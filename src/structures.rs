use glam::{vec3, Vec3};

pub const FLOAT_ERROR: f32 = 0.001;

pub trait Object {
    fn intersect(&self, ray: &Ray) -> Option<Hit>;
}

pub struct RenderSettings {
    pub samples_per_pixel: u32,
    pub clip_near: f32,
    pub clip_far: f32,
    pub max_ray_depth: u32,
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

pub struct Scene {
    pub camera: Camera,
    pub objects: Vec<Box<dyn Object>>,
    pub environment: Vec3,
    pub render_settings: RenderSettings,
}

pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

pub struct Sphere {
    center: Vec3,
    radius: f32,
    color: Vec3,
}

pub struct Quad {
    corner: Vec3,
    u: Vec3,
    v: Vec3,
    color: Vec3,
}

pub struct Hit {
    pub distance: f32,
    pub normal: Vec3,
    pub color: Vec3,
}

impl RenderSettings {
    pub fn default() -> Self {
        RenderSettings {
            samples_per_pixel: 8,
            clip_near: 0.01,
            clip_far: 1000.0,
            max_ray_depth: 8,
        }
    }
}

impl Film {
    pub fn set_pixel(&mut self, x: u32, y: u32, color: Vec3) {
        if x < self.screen_width && y < self.screen_height {
            let index = (self.screen_height - y - 1) * self.screen_width + x;
            self.pixel_data[index as usize] = color;
        }
    }
}

impl Camera {
    pub fn new(
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
            film: Film {
                screen_width,
                screen_height,
                world_width: film_world_width,
                world_height: film_world_height,
                world_position: origin
                    + vec3(
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
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Ray {
            origin,
            direction: direction.normalize(),
        }
    }

    pub fn at(&self, distance: f32) -> Vec3 {
        self.origin + distance * self.direction
    }
}

impl Sphere {
    pub fn new(center: Vec3, radius: f32, color: Vec3) -> Self {
        Sphere {
            center,
            radius,
            color,
        }
    }
}

impl Object for Sphere {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let b = (2.0 * ray.direction).dot(ray.origin - self.center);
        let c = (ray.origin - self.center).dot(ray.origin - self.center) - self.radius.powi(2);
        let discriminant = b * b - 4.0 * c;

        if discriminant < 0.0 {
            return None;
        }

        let sqrt_discriminant = discriminant.sqrt();
        let first_hit = (-b - sqrt_discriminant) / 2.0;
        let second_hit = (-b + sqrt_discriminant) / 2.0;

        if first_hit > FLOAT_ERROR {
            Some(Hit {
                distance: first_hit,
                normal: (ray.at(first_hit) - self.center).normalize(),
                color: self.color,
            })
        } else if second_hit > FLOAT_ERROR {
            Some(Hit {
                distance: second_hit,
                normal: (ray.at(second_hit) - self.center).normalize(),
                color: self.color,
            })
        } else {
            None
        }
    }
}

impl Quad {
    pub fn new(corner: Vec3, u: Vec3, v: Vec3, color: Vec3) -> Self {
        Quad {
            corner,
            u: u.normalize(),
            v: v.normalize(),
            color,
        }
    }
}

impl Object for Quad {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let normal = self.u.cross(self.v).normalize();
        let denominator = normal.dot(ray.direction);

        if denominator > FLOAT_ERROR {
            Some(Hit {
                distance: (normal.dot(self.corner) - normal.dot(ray.origin)) / denominator,
                normal,
                color: self.color,
            })
        } else {
            None
        }
    }
}
