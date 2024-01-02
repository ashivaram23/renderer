use glam::Vec3;

use crate::objects::Object;

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
    pub samples_per_pixel: u32,
    pub max_ray_depth: u32,
}

impl Camera {
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
            pixel_data: vec![Vec3::splat(0.0); (screen_width * screen_height) as usize],
        };

        Camera {
            world_origin: origin,
            film,
        }
    }
}
