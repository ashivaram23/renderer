mod io;
mod objects;
mod scene;

use std::{f32::consts::PI, time::Instant};

use glam::{Quat, Vec3};
use objects::{Hit, Object, Ray};
use rand::{thread_rng, Rng};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use scene::Scene;

// regular stratified/jitter sampling seems to have no effect on noise, not slower either but restricts spp to square number, do not use
// try owen scrambled sobol? for camera rays, check noise reduction and time increase
// try bvh surface area heuristic!
// later: area lights etc, uvs and textures and normals smooth shade, materials glass etc,
// remember to eventually comment code well with all methods, details (eg left handed), sources etc
// pub trait Material with bsdf() -> Vec3 and random_direction_pdf() -> (Vec3, f32) returning direction and pdf
// or with light_and_direction() -> (Vec3, Vec3) for light multiplier all in one and new direction

fn random_direction(normal: Vec3) -> Vec3 {
    let r = thread_rng().gen::<f32>().sqrt();
    let theta = 2.0 * PI * thread_rng().gen::<f32>();

    let x = r * theta.cos();
    let z = r * theta.sin();
    let y = (1.0 - x * x - z * z).max(0.0).sqrt();

    let rotation = Quat::from_rotation_arc(Vec3::new(0.0, 1.0, 0.0), normal);
    rotation.mul_vec3(Vec3::new(x, y, z))
}

fn ray_light(ray: Ray, objects: &[Box<dyn Object + Sync>], environment: Vec3, depth: u32) -> Vec3 {
    let mut light = Vec3::splat(1.0);
    let mut next_ray = ray;

    for _ in 0..depth {
        let mut best_hit: Option<Hit> = None;
        for object in objects {
            let Some(hit) = object.intersect(&next_ray) else {
                continue;
            };

            if best_hit.is_none() || hit.distance < best_hit.as_ref().unwrap().distance {
                best_hit = Some(hit)
            }
        }

        if let Some(hit) = best_hit {
            light *= hit.color;
            next_ray = Ray::new(next_ray.at(hit.distance), random_direction(hit.normal));
        } else {
            light *= environment;
            break;
        }
    }

    light
}

fn render(scene: &mut Scene) {
    let camera = &mut scene.camera;
    let film = &mut camera.film;
    let objects = &scene.objects;

    let (screen_width, screen_height) = (film.screen_width, film.screen_height);
    let pixel_width = film.world_width / (screen_width as f32);

    (0..film.screen_width * film.screen_height)
        .into_par_iter()
        .map(|pixel| {
            let x = pixel % screen_width;
            let y = screen_height - 1 - (pixel / screen_width);
            let mut color = Vec3::splat(0.0);

            for _ in 0..scene.samples_per_pixel {
                let u = pixel_width * (thread_rng().gen::<f32>() + (x as f32));
                let v = pixel_width * (thread_rng().gen::<f32>() + (y as f32));

                let film_pos = film.world_position + u * film.world_u + v * film.world_v;
                let camera_ray = Ray::new(camera.world_origin, film_pos - camera.world_origin);
                color += ray_light(camera_ray, objects, scene.environment, scene.max_ray_depth);
            }

            color / (scene.samples_per_pixel as f32)
        })
        .collect_into_vec(&mut film.pixel_data);
}

fn main() {
    let (input, output) = io::read_args().expect("Error reading arguments");
    let mut scene = match io::read_input(&input) {
        Ok(scene) => scene,
        Err(error) => {
            println!("Error reading scene file: {}", error);
            return;
        }
    };

    println!(
        "Rendering scene with {} objects, {} samples per pixel",
        scene.objects.len(),
        scene.samples_per_pixel
    );

    let start_time = Instant::now();
    render(&mut scene);
    println!("Rendered in {:.2?}", start_time.elapsed());

    let film = &scene.camera.film;
    let (width, height) = (film.screen_width, film.screen_height);
    io::save_to_png(film, &output).expect("Error writing to png file");
    println!("Saved {}x{} image to {}", width, height, output)
}
