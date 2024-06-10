use crate::objects::{Hit, Material, Object, Ray, FLOAT_ERROR};
use glam::{DVec3, Vec3};
use rand::{seq::SliceRandom, thread_rng};

pub struct Scene {
    pub environment: Vec3,
    pub objects: Vec<Box<dyn Object>>,
    pub lights: Vec<u32>,
}

pub struct Film {
    pub screen_width: u32,
    pub screen_height: u32,
    pub world_width: f32,
    pub world_height: f32,
    pub world_position: Vec3,
    pub world_u: Vec3,
    pub world_v: Vec3,
    pub pixel_data: Vec<Vec3>,
}

pub struct Camera {
    pub world_origin: Vec3,
    pub film: Film,
}

pub struct RenderTask {
    pub scene: Scene,
    pub camera: Camera,
    pub samples_per_pixel: u32,
    pub max_ray_depth: u32,
}

impl Scene {
    /// Creates a Scene struct from a Vec of objects and an environment color.
    pub fn new(objects: Vec<Box<dyn Object>>, environment: Vec3) -> Self {
        let mut lights = Vec::new();
        for (i, object) in objects.iter().enumerate() {
            if let Material::Emitter(_, _) = object.material() {
                lights.push(i as u32);
            }
        }

        Scene {
            environment,
            objects,
            lights,
        }
    }

    /// Finds the closest intersection of a ray in the scene.
    pub fn trace_ray(&self, ray: &Ray) -> Option<Hit> {
        let mut best_hit: Option<Hit> = None;
        for object in &self.objects {
            if let Some(hit) = object.intersect(ray) {
                if best_hit.is_none() || hit.distance < best_hit.as_ref().unwrap().distance {
                    best_hit = Some(hit)
                }
            };
        }

        best_hit
    }

    /// Samples a point on the surface of the scene's lights and returns a ray pointing to it along
    /// with its probability density and emitted radiance.
    pub fn sample_lights(&self, origin_point: DVec3) -> Option<(Ray, f32, Vec3)> {
        let chosen_light_index = self.lights.choose(&mut thread_rng())?;
        let chosen_light = &self.objects[*chosen_light_index as usize];

        let (light_hit, light_pdf) = chosen_light.sample_surface(origin_point);
        let light_direction = (light_hit.point - origin_point).normalize();
        let light_ray = Ray::new(origin_point, light_direction);

        let light_is_visible = self.trace_ray(&light_ray).is_some_and(|ray_hit| {
            ray_hit.id == light_hit.id && ray_hit.point.abs_diff_eq(light_hit.point, FLOAT_ERROR)
        });

        if light_is_visible {
            let emission = chosen_light.material().emitted(&-light_direction);
            Some((light_ray, light_pdf / self.lights.len() as f32, emission))
        } else {
            None
        }
    }

    /// Returns an object given its ID (index in the objects array).
    pub fn object_id(&self, id: u32) -> &dyn Object {
        &*self.objects[id as usize]
    }
}

impl Camera {
    /// Creates a Camera struct.
    pub fn new(
        screen_width: u32,
        screen_height: u32,
        origin: Vec3,
        look_at: Vec3,
        up: Vec3,
        field_of_view: f32,
    ) -> Self {
        let world_width = 2.0;
        let world_height = world_width * (screen_height as f32) / (screen_width as f32);
        let focal_length = (field_of_view / 2.0).to_radians().tan().recip();

        let world_forwards = (look_at - origin).normalize();
        let world_left = world_forwards.cross(up).normalize();
        let world_up = world_left.cross(world_forwards).normalize();

        let world_u = world_up.cross(world_forwards);
        let world_v = world_up;

        let world_position = origin
            + (-world_width * world_u / 2.0)
            + (-world_height * world_v / 2.0)
            + focal_length * world_forwards;

        let film = Film {
            screen_width,
            screen_height,
            world_width,
            world_height,
            world_position,
            world_u,
            world_v,
            pixel_data: vec![Vec3::ZERO; (screen_width * screen_height) as usize],
        };

        Camera {
            world_origin: origin,
            film,
        }
    }
}
