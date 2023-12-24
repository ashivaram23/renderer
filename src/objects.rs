use glam::Vec3;

const FLOAT_ERROR: f32 = 0.001;

pub trait Object {
    fn intersect(&self, ray: &Ray) -> Option<Hit>;
}

pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

pub struct Hit {
    pub distance: f32,
    pub normal: Vec3,
    pub color: Vec3,
}

pub struct Sphere {
    center: Vec3,
    radius: f32,
    color: Vec3,
}

pub struct Triangle {
    point1: Vec3,
    point2: Vec3,
    point3: Vec3,
    color: Vec3,
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

impl Triangle {
    pub fn new(point1: Vec3, point2: Vec3, point3: Vec3, color: Vec3) -> Self {
        Triangle {
            point1,
            point2,
            point3,
            color,
        }
    }
}

impl Object for Triangle {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        //todo!(); // rewrite this properly. also this is VERY SLOW!
        // also something is very wrong with the coloring, things are too dark
        // maybe extra bounces due to self intersections or something? test not parallel? or BACKFACE reintersection problems? or something else?
        let e1 = self.point2 - self.point1;
        let e2 = self.point3 - self.point1;
        let s = ray.origin - self.point1;

        let q = ray.direction.cross(e2);
        let r = s.cross(e1);

        let a = q.dot(e1);
        if a.abs() < FLOAT_ERROR || ray.direction.dot(e2.cross(e1).normalize()) > 0.0 {
            // something here?
            return None;
        }

        let f = 1.0 / q.dot(e1);
        let u = f * s.dot(q);
        let v = f * ray.direction.dot(r);

        if u < 0.0 || v < 0.0 || u + v > 1.0 || f * e2.dot(r) < FLOAT_ERROR {
            None
        } else {
            Some(Hit {
                distance: f * e2.dot(r),
                normal: e2.cross(e1).normalize(),
                color: self.color,
            })
        }
    }
}
