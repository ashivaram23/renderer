mod io;
mod structures;

use glam::Vec3;
use rand::{thread_rng, Rng};
use structures::{Camera, Object, Ray};

fn ray_hit_color(ray: Ray, objects: &[Box<dyn Object>]) -> Vec3 {
    for object in objects {
        if let Some(hit) = object.intersect(&ray) {
            return hit.color;
        }
    }

    Vec3::splat(0.0)
}

fn send_rays(camera: &mut Camera, objects: &[Box<dyn Object>]) {
    let film = &mut camera.film;
    let samples_per_pixel = 10;

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
                color += ray_hit_color(camera_ray, objects);
            }

            film.set_pixel(x, y, color / (samples_per_pixel as f32));
        }
    }

    camera.film.pixel_data[0] = Vec3::splat(0.5);
}

fn main() {
    let (input, output) = io::read_args().expect("Error reading arguments");
    let (mut camera, objects) = io::read_input(&input).expect("Error reading input file");

    let (width, height) = (camera.film.screen_width, camera.film.screen_height);
    println!("Rendering image with {} objects", objects.len());

    send_rays(&mut camera, &objects);

    io::save_to_png(camera.film, &output).expect("Error writing to png file");
    println!("Saved {}x{} image to {}", width, height, output)
}
