mod io;
mod objects;
mod scene;

use std::time::Instant;

use glam::Vec3;
use objects::{Hit, Object, Ray};
use rand::{thread_rng, Rng};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use scene::Scene;

// Minimal remaining steps:
// - make the monte carlo sampling accumulate correctly with weights etc and rr,
//   and the camera tonemap stuff etc ie ensure complete and not temporary hack
// - implement pbr materials and lights
// - noise reduction sobol sampling etc
// - all the intersection precision float etc problems straightened out (incl
//   maybe give triangle id on hit for intersect (and maybe material) purposes),
//   clarify how inside hits are supposed to work (including if transmission)
// - something making performance very very bad at scale with the sponge in
//   particular, fix up bvh and acceleration structure handling to improve that
// - cleaned up and commented code, proper structure, ensure neat and efficient
//   and do more profiling for cpu AND for memory/heap
// - readme and proper documentation with sources etc, and with good looking
//   picture samples of interesting scenes (run those on cs machines through ssh
//   for much better performance), for final presentable product

// Other things
// - textures and texturedmesh type and getting the normals, uv, smooth shading
//   from obj files etc
// - environment maps hdris etc
// - other techniques like multiple importance sampling, other integrators like
//   bidirectional, metropolis, etc etc
// - spectral? allowing better caustics, dispersion, glass effects etc things
// - remaining features like depth of field, etc
// - volumetric, physical media etc
// - gpu...

fn ray_light(ray: Ray, objects: &[Box<dyn Object>], environment: Vec3, depth: u32) -> Vec3 {
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
            let hit_point = next_ray.at(hit.distance);
            let incoming = (hit_point - next_ray.origin).normalize();
            let (multiplier, outgoing) = hit.material.light_and_direction(incoming, hit.normal);

            light *= multiplier;
            next_ray = Ray::new(hit_point, outgoing);
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

    film.pixel_data = (0..screen_width * screen_height)
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
        .collect();
}

fn main() {
    let (input, output) = io::read_args().expect("Error reading arguments");
    let (mut scene, primitive_count) = match io::read_input(&input) {
        Ok((scene, count)) => (scene, count),
        Err(error) => {
            println!("Error reading scene file: {}", error);
            return;
        }
    };

    println!(
        "Rendering scene with {} objects ({} primitives), {} samples per pixel",
        scene.objects.len(),
        primitive_count,
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
