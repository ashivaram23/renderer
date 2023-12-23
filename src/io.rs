use std::{
    error::Error,
    fmt::{Debug, Display},
    fs::{self, File},
    io::BufWriter,
    path::Path,
};

use crate::structures::{Camera, Film, Object, Quad, RenderSettings, Scene, Sphere};
use clap::{Arg, Command};
use glam::{vec3, Vec3};
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
    message: String,
}

impl Display for SceneParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for SceneParseError {}

pub fn read_input(filename: &str) -> Result<Scene, SceneParseError> {
    let Ok(scene_json) = fs::read_to_string(Path::new(filename)) else {
        return Err(SceneParseError {
            message: format!("Couldn't open file at {}", filename),
        });
    };

    let Ok(mut scene_params) = from_str::<SceneParams>(&scene_json) else {
        return Err(SceneParseError {
            message: format!("{} isn't in valid JSON format", filename),
        });
    };

    let Ok(camera_params) = from_value::<CameraParams>(scene_params.camera) else {
        return Err(SceneParseError {
            message: "Camera parameters aren't correctly formatted".to_string(),
        });
    };

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
            return Err(SceneParseError {
                message: format!("{} isn't a valid scene object", name),
            });
        };

        let Some((_, object_type)) = object_map.remove_entry("type") else {
            return Err(SceneParseError {
                message: format!("Object {} doesn't have type field", name),
            });
        };

        match object_type.as_str() {
            Some("sphere") => {
                let Ok(sphere_params) = from_value::<SphereParams>(object.clone()) else {
                    return Err(SceneParseError {
                        message: format!("Sphere object {} has invalid parameters", name),
                    });
                };

                objects.push(Box::new(Sphere::new(
                    Vec3::from_array(sphere_params.center),
                    sphere_params.radius,
                    Vec3::from_array(sphere_params.color),
                )));
            }
            Some("quad") => {
                let Ok(quad_params) = from_value::<QuadParams>(object.clone()) else {
                    return Err(SceneParseError {
                        message: format!("Quad object {} has invalid parameters", name),
                    });
                };

                objects.push(Box::new(Quad::new(
                    Vec3::from_array(quad_params.corner),
                    Vec3::from_array(quad_params.u),
                    Vec3::from_array(quad_params.v),
                    Vec3::from_array(quad_params.color),
                )));
            }
            _ => {
                return Err(SceneParseError {
                    message: format!("Object {} has invalid type", name),
                });
            }
        }
    }

    Ok(Scene {
        camera,
        objects,
        environment: vec3(0.3, 0.4, 0.9),
        render_settings: RenderSettings::default(),
    })
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

pub fn save_to_png(film: &Film, filename: &str) -> Result<(), Box<dyn Error>> {
    let rgb_values: Vec<u8> = film
        .pixel_data
        .iter()
        .flat_map(|rgb| {
            let srgb = rgb.powf(1.0 / 2.2);
            [
                (srgb.x.clamp(0.0, 1.0) * 255.0) as u8,
                (srgb.y.clamp(0.0, 1.0) * 255.0) as u8,
                (srgb.z.clamp(0.0, 1.0) * 255.0) as u8,
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
