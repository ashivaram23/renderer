mod io;
mod objects;
mod path_trace;
mod scene;

use std::time::Instant;

use glam::Vec3;
use objects::Ray;
use rand::{thread_rng, Rng};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use scene::RenderTask;

//
// Remaining steps:
// - finish environment light sampling and sample_surface/pdf for all objects
//   (ensuring correct area sampling, and sampling likely visible subset)
//   actually just dont include env in light sampling at all, since bsdf good
//   enough for that..
// - materials: diffuse, metal, nonmetal, glass, and mix, including properly
//   handling transmission (and what that means for normals/backfaces/internal
//   rays) and any special cases (eg specular) in path tracing
// - scene object bvh construction, and anything to help performance with bvh
// - updating scene_from_blend to work with 4.1, handle all features (and figure
//   out exact correspondence for eg emission strength as well as materials, and
//   print message when using substitutes), and scene file having ability to
//   refer to same obj file with displacement/rotation, and prettify json dump
//   and make sure will result in no io errors (eg emitters with zero strength)
// - qmc sampling
// - depth of field support
// - firefly reduction issues (making sure the clamping doesnt change the look)
// - straighten out all the intersection precision float etc problems, self
//   intersection things, float error subtraction compounding, etc, ensure fine
// - fix performance inconsistency with sponge/dragon etc, revisit and fix bvh
//   and memory layout and data structure etc to fix that
// - general optimizing and benchmarking, profiling for cpu and memory, revisit
//   threads vs rayon, general strategy like tiling
// - modify sample scenes and set up more, including complex ones, and organize
//   in scenes folder, to help set up routine for comparisons and benchmarking
//   (test with a set of blender scenes that cover all cases/features)
// - cleaned up and commented code, proper structure, ensure neat and efficient
//   and robust (float error, divides by zero, special cases, etc)
// - things like reordering functions so that it makes sense to read (top first)
// - readme and proper documentation with sources and picture samples, and
//   make final git repo neat
//

// Renders a scene by calculating the color of each pixel in the camera's image plane
fn render(task: &mut RenderTask) {
    let camera = &mut task.camera;
    let film = &mut camera.film;
    let scene = &task.scene;

    let (screen_width, screen_height) = (film.screen_width, film.screen_height);
    let pixel_width = film.world_width / (screen_width as f32);

    // Loops over each pixel
    (0..screen_width * screen_height)
        .into_par_iter()
        .map(|pixel| {
            let x = pixel % screen_width;
            let y = screen_height - 1 - (pixel / screen_width);
            let mut color = Vec3::ZERO;

            // Repeats radiance sampling process for multiple samples
            for _ in 0..task.samples_per_pixel {
                // Gets random offsets within pixel square
                let u = pixel_width * (thread_rng().gen::<f32>() + (x as f32));
                let v = pixel_width * (thread_rng().gen::<f32>() + (y as f32));

                // Finds camera ray (going from camera origin through pixel point)
                let film_pos = film.world_position + (u * film.world_u) + (v * film.world_v);
                let camera_ray = Ray::new(camera.world_origin, film_pos - camera.world_origin);

                // Adds estimated radiance to the pixel color, clamped to reduce fireflies
                color += path_trace::radiance(camera_ray, scene, task.max_ray_depth)
                    .clamp(Vec3::ZERO, Vec3::ONE);
            }

            // Divides accumulated radiance by sample count
            color / (task.samples_per_pixel as f32)
        })
        .collect_into_vec(&mut film.pixel_data);
}

// Reads arguments, processes the scene file, renders the scene, and saves an image
fn main() {
    let (input, output) = io::read_args().expect("Error reading arguments");
    let (mut render_task, primitive_count) = match io::read_input(&input) {
        Ok(result) => result,
        Err(error) => {
            println!("Error reading scene file: {}", error);
            return;
        }
    };

    println!(
        "Rendering scene with {} objects ({} primitives), {} samples per pixel",
        render_task.scene.objects.len(),
        primitive_count,
        render_task.samples_per_pixel
    );

    let start_time = Instant::now();
    render(&mut render_task);
    println!("Rendered in {:.2?}", start_time.elapsed());

    let film = &render_task.camera.film;
    let (width, height) = (film.screen_width, film.screen_height);
    io::save_to_png(film, &output).expect("Error writing to png file");
    println!("Saved {}x{} image to {}", width, height, output)
}
