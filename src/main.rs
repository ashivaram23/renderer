use std::{
    env,
    error::Error,
    fs::File,
    io::{BufRead, BufReader, BufWriter},
    path::Path,
    process::exit,
};

use glam::Vec3;
use png::Encoder;

struct Film {
    screen_width: u32,
    screen_height: u32,
    world_width: f32,
    world_height: f32,
    world_position: Vec3,
    world_u: Vec3,
    world_v: Vec3,
    color_data: Vec<Vec3>,
}

struct Camera {
    world_origin: Vec3,
    world_look_at: Vec3,
    world_up: Vec3,
    world_focal_length: f32,
    film: Film,
}

impl Camera {
    fn new(
        screen_width: u32,
        screen_height: u32,
        origin: Vec3,
        look_at: Vec3,
        up: Vec3,
        focal_length: f32,
    ) -> Self {
        let film_world_width = 2.0;
        let film_world_height = film_world_width * (screen_height / screen_width) as f32;

        Camera {
            world_origin: origin,
            world_look_at: look_at,
            world_up: up,
            world_focal_length: focal_length,
            film: Film {
                screen_width,
                screen_height,
                world_width: film_world_width,
                world_height: film_world_height,
                world_position: origin
                    + Vec3::new(
                        -film_world_width / 2.0,
                        -film_world_height / 2.0,
                        focal_length,
                    ),
                world_u: look_at.cross(up),
                world_v: up,
                color_data: vec![Vec3::splat(0.0); (screen_width * screen_height) as usize],
            },
        }
    }

    fn from_dimensions(width: u32, height: u32) -> Self {
        Camera::new(
            width,
            height,
            Vec3::splat(0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 1.0, 0.0),
            1.0,
        )
    }

    fn default() -> Self {
        Camera::new(
            640,
            480,
            Vec3::splat(0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 1.0, 0.0),
            1.0,
        )
    }
}

fn rgb_f32_to_u8(rgb: Vec3) -> [u8; 3] {
    [
        (rgb.x * 255.0) as u8,
        (rgb.y * 255.0) as u8,
        (rgb.z * 255.0) as u8,
    ]
}

fn save_to_png(image: Film, filename: &str) -> Result<(), Box<dyn Error>> {
    let rgb_values: Vec<u8> = image
        .color_data
        .into_iter()
        .flat_map(rgb_f32_to_u8)
        .collect();

    let mut encoder = Encoder::new(
        BufWriter::new(File::create(Path::new(filename))?),
        image.screen_width,
        image.screen_height,
    );

    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&rgb_values)?;

    Ok(())
}

fn make_image(image: &mut Film) {
    image.color_data[0] = Vec3::splat(0.5);
}

fn read_input(filename: &str) -> Result<(u32, u32), Box<dyn Error>> {
    let mut fields = [640, 480];
    let reader = BufReader::new(File::open(Path::new(filename))?);
    for (line, field) in reader.lines().zip(fields.iter_mut()) {
        *field = line?.parse::<u32>()?;
    }

    Ok((fields[0], fields[1]))
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Usage: {} input output", args[0]);
        exit(0)
    }

    let input = &args[1];
    let output = &args[2];
    let (width, height) = read_input(input).expect("Error reading input file");

    let mut camera = Camera::from_dimensions(width, height);
    make_image(&mut camera.film);

    save_to_png(camera.film, output).expect("Error writing to png file");
    println!("Saved {}x{} image to {}", width, height, output)
}
