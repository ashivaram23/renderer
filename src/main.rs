mod io;
mod objects;
mod scene;

use std::{f32::consts::PI, process::exit, thread, time::Instant};

use glam::{Quat, Vec3};
use objects::{Hit, Object, Ray};
use rand::{thread_rng, Rng};
use scene::Scene;

// faster aabb tests and other insights from profiler
// try bvh surface area heuristic
// see if basic parallel is faster than rayon
// later: area lights etc, uvs and textures and normals smooth shade, materials glass etc,

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

    let (world_position, world_u, world_v) = (film.world_position, film.world_u, film.world_v);
    let (environment, max_ray_depth) = (scene.environment, scene.render_settings.max_ray_depth);
    let (screen_width, screen_height) = (film.screen_width, film.screen_height);
    let samples_per_pixel = scene.render_settings.samples_per_pixel;
    let world_origin = camera.world_origin;

    let num_threads = match thread::available_parallelism() {
        Ok(count) => usize::from(count),
        Err(_) => 4,
    };

    let pixel_width = film.world_width / (screen_width as f32);
    let pixel_count = (film.screen_width * film.screen_height) as usize;
    let chunk_size = (pixel_count + num_threads - 1) / num_threads;

    thread::scope(|scope| {
        for thread_number in 0..num_threads {
            let range_min = thread_number * chunk_size;
            let range_max = (range_min + chunk_size).min(pixel_count);

            scope.spawn(move || {
                for pixel in range_min..range_max {
                    let x = (pixel as u32) % screen_width;
                    let y = screen_height - 1 - ((pixel as u32) / screen_width);
                    let mut color = Vec3::splat(0.0);

                    for _ in 0..samples_per_pixel {
                        let u = pixel_width * (thread_rng().gen::<f32>() + (x as f32));
                        let v = pixel_width * (thread_rng().gen::<f32>() + (y as f32));

                        let film_pos = world_position + u * world_u + v * world_v;
                        let camera_ray = Ray::new(world_origin, film_pos - world_origin);

                        color += ray_light(camera_ray, objects, environment, max_ray_depth);
                    }

                    // color / (samples_per_pixel as f32)
                }
            });
        }
    });
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

    println!(
        "Rendering scene with {} objects, {} samples per pixel",
        scene.objects.len(),
        scene.render_settings.samples_per_pixel
    );

    let start_time = Instant::now();
    render(&mut scene);
    println!("Rendered in {:.2?}", start_time.elapsed());

    let film = &scene.camera.film;
    let (width, height) = (film.screen_width, film.screen_height);
    io::save_to_png(film, &output).expect("Error writing to png file");
    println!("Saved {}x{} image to {}", width, height, output)
}
