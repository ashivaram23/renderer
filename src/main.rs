use std::{
    error::Error,
    fs::File,
    io::{BufRead, BufReader, BufWriter},
    path::Path,
};

use clap::{Arg, Command};
use glam::Vec3;
use png::Encoder;
use rand::{thread_rng, Rng};

struct Film {
    screen_width: u32,
    screen_height: u32,
    world_width: f32,
    world_height: f32,
    world_position: Vec3,
    world_u: Vec3,
    world_v: Vec3,
    pixel_data: Vec<Vec3>,
}

struct Camera {
    world_origin: Vec3,
    world_look_at: Vec3,
    world_up: Vec3,
    world_focal_length: f32,
    film: Film,
}

struct Ray {
    origin: Vec3,
    direction: Vec3,
}

struct Sphere {
    center: Vec3,
    radius: f32,
}

trait Object {
    fn intersect(&self, ray: &Ray) -> f32;
}

impl Film {
    fn set_pixel(&mut self, x: u32, y: u32, color: Vec3) {
        if x < self.screen_width && y < self.screen_height {
            let index = (self.screen_height - y - 1) * self.screen_width + x;
            self.pixel_data[index as usize] = color;
        }
    }
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
        let film_world_height = film_world_width * (screen_height as f32) / (screen_width as f32);

        let world_look_at = look_at.normalize();
        let world_up = up.normalize();

        Camera {
            world_origin: origin,
            world_look_at,
            world_up,
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
                world_u: -world_look_at.cross(world_up),
                world_v: world_up,
                pixel_data: vec![Vec3::splat(0.0); (screen_width * screen_height) as usize],
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
}

impl Ray {
    fn new(origin: Vec3, direction: Vec3) -> Self {
        Ray {
            origin,
            direction: direction.normalize(),
        }
    }
}

impl Sphere {
    fn new(center: Vec3, radius: f32) -> Self {
        Sphere { center, radius }
    }
}

impl Object for Sphere {
    fn intersect(&self, ray: &Ray) -> f32 {
        let b = (2.0 * ray.direction).dot(ray.origin - self.center);
        let c = (ray.origin - self.center).dot(ray.origin - self.center) - self.radius.powi(2);
        let discriminant = b * b - 4.0 * c;

        if discriminant >= 0.0 {
            1.0
        } else {
            0.0
        }
    }
}

fn save_to_png(film: Film, filename: &str) -> Result<(), Box<dyn Error>> {
    let rgb_values: Vec<u8> = film
        .pixel_data
        .into_iter()
        .flat_map(|rgb| {
            [
                (rgb.x * 255.0) as u8,
                (rgb.y * 255.0) as u8,
                (rgb.z * 255.0) as u8,
            ]
        })
        .collect();

    let mut encoder = Encoder::new(
        BufWriter::new(File::create(Path::new(filename))?),
        film.screen_width,
        film.screen_height,
    );

    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&rgb_values)?;

    Ok(())
}

fn trace_ray(ray: Ray, objects: &[Box<dyn Object>]) -> Result<Vec3, Box<dyn Error>> {
    for object in objects {
        if object.intersect(&ray) > 0.0 {
            return Ok(Vec3::splat(0.5));
        }
    }

    Ok(Vec3::splat(0.0))
}

fn make_image(camera: &mut Camera, objects: &[Box<dyn Object>]) {
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
                color += trace_ray(camera_ray, objects).unwrap_or(Vec3::splat(0.0));
            }

            film.set_pixel(x, y, color / (samples_per_pixel as f32));
        }
    }

    camera.film.pixel_data[0] = Vec3::splat(0.5);
}

fn read_input(filename: &str) -> Result<(u32, u32), Box<dyn Error>> {
    let mut fields = [640, 480];
    let reader = BufReader::new(File::open(Path::new(filename))?);
    for (line, field) in reader.lines().zip(fields.iter_mut()) {
        *field = line?.parse::<u32>()?;
    }

    Ok((fields[0], fields[1]))
}

fn read_args() -> Option<(String, String)> {
    let matches = Command::new("renderer")
        .arg(Arg::new("input").required(true))
        .arg(Arg::new("output").required(true))
        .arg_required_else_help(true)
        .get_matches();

    let input = matches.get_one::<String>("input")?;
    let output = matches.get_one::<String>("output")?;
    Some((input.clone(), output.clone()))
}

fn main() {
    let (input, output) = read_args().expect("Error reading arguments");
    let (width, height) = read_input(&input).expect("Error reading input file");

    let sphere1 = Sphere::new(Vec3::new(-1.0, 0.0, 4.0), 0.3);
    let sphere2 = Sphere::new(Vec3::new(1.0, 0.0, 4.0), 0.7);
    let objects: Vec<Box<dyn Object>> = vec![Box::new(sphere1), Box::new(sphere2)];

    let mut camera = Camera::from_dimensions(width, height);
    make_image(&mut camera, &objects);

    save_to_png(camera.film, &output).expect("Error writing to png file");
    println!("Saved {}x{} image to {}", width, height, output)
}
