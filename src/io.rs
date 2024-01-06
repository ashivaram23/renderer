use std::{
    error::Error,
    fmt::{Debug, Display},
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter},
    path::Path,
};

use crate::{
    objects::{Bounds, DiffuseMaterial, Material, Mesh, MirrorMaterial, Object, Sphere, Triangle},
    scene::{Camera, Film, Scene},
};
use clap::{Arg, Command};
use glam::Vec3;
use png::Encoder;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::Deserialize;
use serde_json::{from_str, from_value, Map, Value};

#[derive(Deserialize, Debug)]
struct SceneParams {
    camera: Value,
    settings: Value,
    objects: Map<String, Value>,
}

#[derive(Deserialize, Debug)]
struct CameraParams {
    film_dimensions: [u32; 2],
    origin: [f32; 3],
    look_at: [f32; 3],
    up: [f32; 3],
    field_of_view: f32,
}

#[derive(Deserialize, Debug)]
struct SettingsParams {
    environment: [f32; 3],
    samples_per_pixel: u32,
    max_ray_depth: u32,
}

#[derive(Deserialize, Debug)]
struct SphereParams {
    center: [f32; 3],
    radius: f32,
    material: Value,
}

#[derive(Deserialize, Debug)]
struct TriangleParams {
    point1: [f32; 3],
    point2: [f32; 3],
    point3: [f32; 3],
    material: Value,
}

#[derive(Deserialize, Debug)]
struct MeshParams {
    file: String,
    material: Value,
}

#[derive(Deserialize, Debug)]
struct DiffuseParams {
    reflectance: [f32; 3],
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

pub fn read_input(filename: &str) -> Result<(Scene, u32), SceneParseError> {
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

    let camera = match process_camera(scene_params.camera) {
        Ok(camera) => camera,
        Err(error) => return Err(error),
    };

    let Ok(settings) = from_value::<SettingsParams>(scene_params.settings) else {
        return Err(SceneParseError {
            message: "Scene settings aren't correctly formatted".to_string(),
        });
    };

    let mut primitive_count = 0;
    let mut objects: Vec<Box<dyn Object>> = Vec::new();
    for (name, object_value) in scene_params.objects.iter_mut() {
        let (object, count) = match process_object(name, object_value) {
            Ok((object, count)) => (object, count),
            Err(error) => return Err(error),
        };

        primitive_count += count;
        objects.push(object);
    }

    Ok((
        Scene {
            camera,
            objects,
            environment: Vec3::from_array(settings.environment),
            samples_per_pixel: settings.samples_per_pixel,
            max_ray_depth: settings.max_ray_depth,
        },
        primitive_count,
    ))
}

fn process_camera(camera_value: Value) -> Result<Camera, SceneParseError> {
    let Ok(camera_params) = from_value::<CameraParams>(camera_value) else {
        return Err(SceneParseError {
            message: "Camera parameters aren't correctly formatted".to_string(),
        });
    };

    if camera_params.field_of_view <= 0.0 || camera_params.field_of_view >= 180.0 {
        return Err(SceneParseError {
            message: "Camera field of view must be between 0 and 180 degrees".to_string(),
        });
    }

    Ok(Camera::new(
        camera_params.film_dimensions[0],
        camera_params.film_dimensions[1],
        Vec3::from_array(camera_params.origin),
        Vec3::from_array(camera_params.look_at),
        Vec3::from_array(camera_params.up),
        camera_params.field_of_view,
    ))
}

fn process_object(
    name: &str,
    object_value: &mut Value,
) -> Result<(Box<dyn Object>, u32), SceneParseError> {
    let Some(object_map) = object_value.as_object_mut() else {
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
            let Ok(mut sphere_params) = from_value::<SphereParams>(object_value.clone()) else {
                return Err(SceneParseError {
                    message: format!("Sphere object {} has invalid parameters", name),
                });
            };

            if sphere_params.radius < 0.0 {
                return Err(SceneParseError {
                    message: format!("Sphere object {} has negative radius", name),
                });
            }

            let Some(material) = process_material(&mut sphere_params.material) else {
                return Err(SceneParseError {
                    message: format!("Sphere object {} has invalid material", name),
                });
            };

            Ok((
                Box::new(Sphere::new(
                    Vec3::from_array(sphere_params.center),
                    sphere_params.radius,
                    material,
                )),
                1,
            ))
        }
        Some("triangle") => {
            let Ok(mut triangle_params) = from_value::<TriangleParams>(object_value.clone()) else {
                return Err(SceneParseError {
                    message: format!("Triangle object {} has invalid parameters", name),
                });
            };

            let Some(material) = process_material(&mut triangle_params.material) else {
                return Err(SceneParseError {
                    message: format!("Triangle object {} has invalid material", name),
                });
            };

            Ok((
                Box::new(Triangle::new(
                    Vec3::from_array(triangle_params.point1),
                    Vec3::from_array(triangle_params.point2),
                    Vec3::from_array(triangle_params.point3),
                    material,
                )),
                1,
            ))
        }
        Some("mesh") => {
            let Ok(mut mesh_params) = from_value::<MeshParams>(object_value.clone()) else {
                return Err(SceneParseError {
                    message: format!("Mesh object {} has invalid parameters", name),
                });
            };

            let (vertices, indices) = match read_obj(&mesh_params.file) {
                Ok((vertices, indices)) => (vertices, indices),
                Err(error) => return Err(error),
            };

            let triangle_count = indices.len() as u32;
            let indices_and_bounds: Vec<([u32; 3], Bounds)> = indices
                .into_par_iter()
                .map(|point_indices| {
                    let p1 = vertices[point_indices[0] as usize];
                    let p2 = vertices[point_indices[1] as usize];
                    let p3 = vertices[point_indices[2] as usize];

                    let bounds = Bounds {
                        min: p1.min(p2.min(p3)),
                        max: p1.max(p2.max(p3)),
                    };

                    (point_indices, bounds)
                })
                .collect();

            let Some(material) = process_material(&mut mesh_params.material) else {
                return Err(SceneParseError {
                    message: format!("Mesh object {} has invalid material", name),
                });
            };

            Ok((
                Box::new(Mesh::new(vertices, indices_and_bounds, material)),
                triangle_count,
            ))
        }
        _ => Err(SceneParseError {
            message: format!("Object {} has invalid type", name),
        }),
    }
}

fn process_material(material_value: &mut Value) -> Option<Box<dyn Material>> {
    let Some(material_map) = material_value.as_object_mut() else {
        return None;
    };

    let Some((_, material_type)) = material_map.remove_entry("type") else {
        return None;
    };

    match material_type.as_str() {
        Some("diffuse") => {
            let Ok(diffuse_params) = from_value::<DiffuseParams>(material_value.clone()) else {
                return None;
            };

            Some(Box::new(DiffuseMaterial::new(Vec3::from_array(
                diffuse_params.reflectance,
            ))))
        }
        Some("mirror") => {
            let Ok(diffuse_params) = from_value::<DiffuseParams>(material_value.clone()) else {
                return None;
            };

            Some(Box::new(MirrorMaterial::new(Vec3::from_array(
                diffuse_params.reflectance,
            ))))
        }
        _ => None,
    }
}

fn read_obj(filename: &str) -> Result<(Vec<Vec3>, Vec<[u32; 3]>), SceneParseError> {
    let Ok(file) = File::open(filename) else {
        return Err(SceneParseError {
            message: format!("Couldn't open OBJ file at {}", filename),
        });
    };

    let mut vertices: Vec<Vec3> = Vec::new();
    let mut indices: Vec<[u32; 3]> = Vec::new();

    let invalid_value_error = |i| {
        Err(SceneParseError {
            message: format!("OBJ file {} has invalid values at line {}", filename, i + 1),
        })
    };

    for (i, line) in BufReader::new(file).lines().flatten().enumerate() {
        let tokens: Vec<&str> = line.split(' ').collect();
        if tokens.len() < 4 {
            continue;
        }

        match tokens[0] {
            "v" => {
                let mut points = Vec3::splat(0.0);
                for j in 0..3 {
                    let Ok(value) = tokens[j + 1].parse::<f32>() else {
                        return invalid_value_error(i);
                    };

                    points[j] = value;
                }

                points[0] *= -1.0;
                vertices.push(points);
            }
            "f" => {
                let mut points = [1; 3];
                for j in 0..3 {
                    let value = tokens[j + 1].parse::<u32>();
                    if value.is_ok() && *value.as_ref().unwrap() >= 1 {
                        points[j] = value.unwrap() - 1;
                    } else {
                        return invalid_value_error(i);
                    }
                }

                indices.push(points);
            }
            _ => (),
        }
    }

    for triangle in &indices {
        let highest_index = triangle[0].max(triangle[1]).max(triangle[2]) as usize;
        if highest_index >= vertices.len() {
            return Err(SceneParseError {
                message: format!("OBJ file {} contains indices out of bounds", filename),
            });
        }
    }

    Ok((vertices, indices))
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
