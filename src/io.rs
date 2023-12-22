use std::{
    error::Error,
    fmt::{Debug, Display},
    fs::{self, File},
    io::BufWriter,
    path::Path,
};

use crate::structures::{Camera, Film, Object, Quad, Sphere};
use clap::{Arg, Command};
use glam::Vec3;
use png::Encoder;
use serde::Deserialize;
use serde_json::{from_str, from_value, Map, Value};

#[derive(Deserialize, Debug)]
struct SceneParams {
    camera: Value,
    objects: Map<String, Value>,
}

#[derive(Deserialize, Debug)]
struct CameraParams {
    film_dimensions: [u32; 2],
    origin: [f32; 3],
    look_at: [f32; 3],
    up: [f32; 3],
}

#[derive(Deserialize, Debug)]
struct SphereParams {
    center: [f32; 3],
    radius: f32,
    color: [f32; 3],
}

#[derive(Deserialize, Debug)]
struct QuadParams {
    corner: [f32; 3],
    u: [f32; 3],
    v: [f32; 3],
    color: [f32; 3],
}

#[derive(Debug)]
pub struct SceneParseError {
    element: String,
    message: String,
}

impl Display for SceneParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error parsing {}: {}", self.element, self.message)
    }
}

impl Error for SceneParseError {}

type SceneResult = (Camera, Vec<Box<dyn Object>>);

pub fn read_input(filename: &str) -> Result<SceneResult, Box<dyn Error>> {
    let scene_json = fs::read_to_string(Path::new(filename))?;
    let mut scene_params: SceneParams = from_str(&scene_json)?;

    let camera_params: CameraParams = from_value(scene_params.camera)?;
    let camera = Camera::new(
        camera_params.film_dimensions[0],
        camera_params.film_dimensions[1],
        Vec3::from_array(camera_params.origin),
        Vec3::from_array(camera_params.look_at),
        Vec3::from_array(camera_params.up),
        1.0,
    );

    let mut objects: Vec<Box<dyn Object>> = Vec::new();
    for (name, object) in scene_params.objects.iter_mut() {
        let Some(object_map) = object.as_object_mut() else {
            return Err(Box::new(SceneParseError {
                element: name.to_string(),
                message: "not a valid scene object".to_string(),
            }));
        };

        let Some((_, object_type)) = object_map.remove_entry("type") else {
            return Err(Box::new(SceneParseError {
                element: name.to_string(),
                message: "object doesn't have type".to_string(),
            }));
        };

        match object_type.as_str() {
            Some("sphere") => {
                let sphere_params: SphereParams = from_value(object.clone())?;
                objects.push(Box::new(Sphere::new(
                    Vec3::from_array(sphere_params.center),
                    sphere_params.radius,
                    Vec3::from_array(sphere_params.color),
                )));
            }
            Some("quad") => {
                let quad_params: QuadParams = from_value(object.clone())?;
                objects.push(Box::new(Quad::new(
                    Vec3::from_array(quad_params.corner),
                    Vec3::from_array(quad_params.u),
                    Vec3::from_array(quad_params.v),
                    Vec3::from_array(quad_params.color),
                )));
            }
            _ => {
                return Err(Box::new(SceneParseError {
                    element: name.to_string(),
                    message: "object type invalid".to_string(),
                }));
            }
        }
    }

    Ok((camera, objects))
}

pub fn read_args() -> Option<(String, String)> {
    let matches = Command::new("renderer")
        .arg(Arg::new("input").required(true))
        .arg(Arg::new("output").required(true))
        .arg_required_else_help(true)
        .get_matches();

    let input = matches.get_one::<String>("input")?;
    let output = matches.get_one::<String>("output")?;
    Some((input.clone(), output.clone()))
}

pub fn save_to_png(film: Film, filename: &str) -> Result<(), Box<dyn Error>> {
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
