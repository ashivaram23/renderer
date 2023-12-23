mod io;
mod structures;

use std::process::exit;

use glam::Vec3;
use rand::{thread_rng, Rng};
use structures::{Hit, Object, Ray, Scene};

fn ray_light(ray: Ray, objects: &[Box<dyn Object>], environment: Vec3) -> Vec3 {
    let mut best_hit: Option<Hit> = None;
    for object in objects {
        let Some(hit) = object.intersect(&ray) else {
            continue;
        };

        if best_hit.is_none() || hit.distance < best_hit.as_ref().unwrap().distance {
            best_hit = Some(hit)
        }
    }

    if let Some(hit) = best_hit {
        hit.color
        // pick random direction on hemisphere
        // calculate incoming radiance (Vec3) from random direction
        // calculate bsdf fraction (Vec3)
        // multiply the two and return
    } else {
        environment
    }
}

fn render(scene: &mut Scene) {
    let camera = &mut scene.camera;
    let film = &mut camera.film;
    let objects = &scene.objects;
    let samples_per_pixel = scene.render_settings.samples_per_pixel;

    for y in 0..film.screen_height {
        for x in 0..film.screen_width {
            let mut color = Vec3::splat(0.0);

            for _ in 0..samples_per_pixel {
                let offsets = thread_rng().gen::<(f32, f32)>();
                let u = film.world_width * (offsets.0 + (x as f32)) / (film.screen_width as f32);
                let v = film.world_height * (offsets.1 + (y as f32)) / (film.screen_height as f32);

                let mut film_pos = film.world_position;
                film_pos += u * film.world_u + v * film.world_v;

                let camera_ray = Ray::new(camera.world_origin, film_pos - camera.world_origin);
                color += ray_light(camera_ray, objects, scene.environment);
            }

            film.set_pixel(x, y, color / (samples_per_pixel as f32));
        }
    }

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
