mod io;
mod objects;
mod scene;

use std::{process::exit, time::Instant};

use glam::Vec3;
use objects::{Hit, Object, Ray};
use rand::{thread_rng, Rng};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use scene::Scene;

fn random_direction(normal: Vec3) -> Vec3 {
    let sphere_vec = loop {
        let point = Vec3::from_array(thread_rng().gen()) * 2.0 - 1.0;
        if point.length_squared() < 1.0 {
            break point.normalize();
        }
    };

    if sphere_vec.dot(normal) > 0.0 {
        sphere_vec
    } else {
        -sphere_vec
    }
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
            light *= hit.color; // should be fraction from material's bsdf
            let incident =
                next_ray.direction - 2.0 * (next_ray.direction.dot(hit.normal)) * hit.normal;
            next_ray = Ray::new(
                next_ray.at(hit.distance),
                (0.0 * incident + 4.0 * random_direction(hit.normal)) / 4.0,
            );
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

    let samples_per_pixel = scene.render_settings.samples_per_pixel;
    let pixel_width = film.world_width / (film.screen_width as f32);
    let start_time = Instant::now();

    (0..film.screen_height * film.screen_width)
        .into_par_iter()
        .map(|i| {
            let y = film.screen_height - 1 - (i / film.screen_width);
            let x = i % film.screen_width;

            let mut color = Vec3::splat(0.0);

            for _ in 0..samples_per_pixel {
                let offsets = thread_rng().gen::<(f32, f32)>();
                let u = pixel_width * (offsets.0 + (x as f32));
                let v = pixel_width * (offsets.1 + (y as f32));

                let mut film_pos = film.world_position;
                film_pos += u * film.world_u + v * film.world_v;

                let camera_ray = Ray::new(camera.world_origin, film_pos - camera.world_origin);
                color += ray_light(
                    camera_ray,
                    objects,
                    scene.environment,
                    scene.render_settings.max_ray_depth,
                );
            }

            color / (samples_per_pixel as f32)

            // film.pixel_data[i as usize] = color / (samples_per_pixel as f32);

            // film.set_pixel(x, y, color / (samples_per_pixel as f32));
        })
        .collect_into_vec(&mut film.pixel_data);

    println!("Rendered in {:.2?}", start_time.elapsed());

    film.pixel_data[0] = Vec3::splat(0.5);
}

fn main() {
    let (input, output) = io::read_args().expect("Error reading arguments");
    let mut scene = match io::read_input(&input) {
        Ok(scene) => scene,
        Err(error) => {
            println!("Error reading scene file: {}", error);
            exit(0);
        }
    };

    println!("Rendering scene with {} objects", scene.objects.len());
    render(&mut scene);

    let film = &scene.camera.film;
    let (width, height) = (film.screen_width, film.screen_height);
    io::save_to_png(film, &output).expect("Error writing to png file");
    println!("Saved {}x{} image to {}", width, height, output)
}
