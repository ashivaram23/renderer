use std::{
    error::Error,
    fmt::{Debug, Display},
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter},
    path::Path,
};

use crate::{
    objects::{Bounds, Material, Mesh, Object, Sphere},
    scene::{Camera, Film, RenderTask, Scene},
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
struct MeshParams {
    file: String,
    material: Value,
}

#[derive(Deserialize, Debug)]
struct DiffuseParams {
    color: [f32; 3],
}

#[derive(Deserialize, Debug)]
struct EmitterParams {
    color: [f32; 3],
    strength: f32,
}

#[derive(Deserialize, Debug)]
struct MetalParams {
    color: [f32; 3],
    roughness: f32,
}

#[derive(Deserialize, Debug)]
struct NonmetalParams {
    color: [f32; 3],
    roughness: f32,
    specular: f32,
}

#[derive(Deserialize, Debug)]
struct GlassParams {
    color: [f32; 3],
    roughness: f32,
    specular: f32,
}

#[derive(Deserialize, Debug)]
struct MixParams {
    first: Value,
    second: Value,
    factor: f32,
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

pub fn read_input(filename: &str) -> Result<(RenderTask, u32), SceneParseError> {
    let Ok(scene_json) = fs::read_to_string(Path::new(filename)) else {
        return Err(SceneParseError {
            message: format!("couldn't open file at {}", filename),
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
            message: "scene settings aren't correctly formatted".to_string(),
        });
    };

    let mut primitive_count = 0;
    let mut objects: Vec<Box<dyn Object>> = Vec::new();
    for (i, (name, object_value)) in scene_params.objects.iter_mut().enumerate() {
        let (object, count) = match process_object(name, object_value, i) {
            Ok((object, count)) => (object, count),
            Err(error) => return Err(error),
        };

        primitive_count += count;
        objects.push(object);
    }

    Ok((
        RenderTask {
            scene: Scene::new(objects, Vec3::from_array(settings.environment)),
            camera,
            samples_per_pixel: settings.samples_per_pixel,
            max_ray_depth: settings.max_ray_depth,
        },
        primitive_count,
    ))
}

fn process_camera(camera_value: Value) -> Result<Camera, SceneParseError> {
    let Ok(camera_params) = from_value::<CameraParams>(camera_value) else {
        return Err(SceneParseError {
            message: "camera parameters aren't correctly formatted".to_string(),
        });
    };

    if camera_params.field_of_view <= 0.0 || camera_params.field_of_view >= 180.0 {
        return Err(SceneParseError {
            message: "camera field of view must be between 0 and 180 degrees".to_string(),
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
    id: usize,
) -> Result<(Box<dyn Object>, u32), SceneParseError> {
    let Some(object_map) = object_value.as_object_mut() else {
        return Err(SceneParseError {
            message: format!("{} isn't a valid scene object", name),
        });
    };

    let Some((_, object_type)) = object_map.remove_entry("type") else {
        return Err(SceneParseError {
            message: format!("object {} doesn't have type field", name),
        });
    };

    match object_type.as_str() {
        Some("sphere") => {
            let Ok(mut sphere_params) = from_value::<SphereParams>(object_value.clone()) else {
                return Err(SceneParseError {
                    message: format!("sphere object {} has invalid parameters", name),
                });
            };

            if sphere_params.radius < 0.0 {
                return Err(SceneParseError {
                    message: format!("sphere object {} has negative radius", name),
                });
            }

            let material = process_material(
                &format!("sphere object {}", name),
                &mut sphere_params.material,
            )?;

            Ok((
                Box::new(Sphere::new(
                    id,
                    Vec3::from_array(sphere_params.center),
                    sphere_params.radius,
                    material,
                )),
                1,
            ))
        }
        Some("mesh") => {
            let Ok(mut mesh_params) = from_value::<MeshParams>(object_value.clone()) else {
                return Err(SceneParseError {
                    message: format!("mesh object {} has invalid parameters", name),
                });
            };

            let (vertices, indices) = match read_obj(&mesh_params.file) {
                Ok((vertices, indices)) => (vertices, indices),
                Err(error) => return Err(error),
            };

            let triangle_count = indices.len() as u32;
            if triangle_count == 0 {
                return Err(SceneParseError {
                    message: format!("mesh object {} is empty", name),
                });
            }

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

            let material =
                process_material(&format!("mesh object {}", name), &mut mesh_params.material)?;

            Ok((
                Box::new(Mesh::new(id, vertices, indices_and_bounds, material)),
                triangle_count,
            ))
        }
        _ => Err(SceneParseError {
            message: format!("object {} has invalid type", name),
        }),
    }
}

fn process_material(
    object_name: &str,
    material_value: &mut Value,
) -> Result<Material, SceneParseError> {
    let invalid_material_error = Err(SceneParseError {
        message: format!("{} has invalid material", object_name),
    });

    let Some(material_map) = material_value.as_object_mut() else {
        return invalid_material_error;
    };

    let Some((_, material_type)) = material_map.remove_entry("type") else {
        return invalid_material_error;
    };

    match material_type.as_str() {
        Some("diffuse") => {
            let Ok(diffuse_params) = from_value::<DiffuseParams>(material_value.clone()) else {
                return invalid_material_error;
            };

            Ok(Material::Diffuse(Vec3::from_array(diffuse_params.color)))
        }
        Some("emitter") => {
            let Ok(emitter_params) = from_value::<EmitterParams>(material_value.clone()) else {
                return invalid_material_error;
            };

            let emits_light = emitter_params.color.iter().any(|value| *value > 0.0)
                && emitter_params.strength > 0.0;
            if !emits_light {
                return Err(SceneParseError {
                    message: format!(
                        "emitter material for {} should have nonzero strength and color",
                        object_name
                    ),
                });
            }

            Ok(Material::Emitter(
                Vec3::from_array(emitter_params.color),
                emitter_params.strength,
            ))
        }
        Some("metal") => {
            let Ok(metal_params) = from_value::<MetalParams>(material_value.clone()) else {
                return invalid_material_error;
            };

            Ok(Material::Metal(
                Vec3::from_array(metal_params.color),
                metal_params.roughness,
            ))
        }
        Some("nonmetal") => {
            let Ok(nonmetal_params) = from_value::<NonmetalParams>(material_value.clone()) else {
                return invalid_material_error;
            };

            Ok(Material::Nonmetal(
                Vec3::from_array(nonmetal_params.color),
                nonmetal_params.roughness,
                nonmetal_params.specular,
            ))
        }
        Some("glass") => {
            let Ok(glass_params) = from_value::<GlassParams>(material_value.clone()) else {
                return invalid_material_error;
            };

            Ok(Material::Glass(
                Vec3::from_array(glass_params.color),
                glass_params.roughness,
                glass_params.specular,
            ))
        }
        Some("mix") => {
            let Ok(mut mix_params) = from_value::<MixParams>(material_value.clone()) else {
                return invalid_material_error;
            };

            let first = process_material(object_name, &mut mix_params.first)?;
            if let Material::Emitter(_, _) = first {
                return Err(SceneParseError {
                    message: format!("mix material for {} should't contain emitters", object_name),
                });
            }

            let second = process_material(object_name, &mut mix_params.second)?;
            if let Material::Emitter(_, _) = second {
                return Err(SceneParseError {
                    message: format!("mix material for {} should't contain emitters", object_name),
                });
            }

            Ok(Material::Mix(
                Box::new(first),
                Box::new(second),
                mix_params.factor,
            ))
        }
        _ => invalid_material_error,
    }
}

fn read_obj(filename: &str) -> Result<(Vec<Vec3>, Vec<[u32; 3]>), SceneParseError> {
    let Ok(file) = File::open(filename) else {
        return Err(SceneParseError {
            message: format!("couldn't open OBJ file at {}", filename),
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
                let mut points = Vec3::ZERO;
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
            let srgb = rgb.powf(1.0 / 2.2).clamp(Vec3::ZERO, Vec3::ONE) * 255.0;
            [srgb.x as u8, srgb.y as u8, srgb.z as u8]
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
