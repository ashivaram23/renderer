use glam::{vec3, Vec3};

pub const FLOAT_ERROR: f32 = 0.001;
pub const CLIP_NEAR: f32 = 0.01;
pub const CLIP_FAR: f32 = 1000.0;

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

pub trait Object {
    fn intersect(&self, ray: &Ray) -> Option<Hit>;
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

    pub fn from_dimensions(width: u32, height: u32) -> Self {
        Camera::new(
            width,
            height,
            Vec3::splat(0.0),
            vec3(0.0, 0.0, 1.0),
            vec3(0.0, 1.0, 0.0),
            1.0,
        )
    }
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Ray {
            origin,
            direction: direction.normalize(),
        }
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

        if discriminant > FLOAT_ERROR {
            Some(Hit {
                distance: 1.0,
                normal: Vec3::splat(0.0),
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
