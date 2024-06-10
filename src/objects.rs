use std::f64::consts::PI;

use glam::{DQuat, DVec3, Vec3};
use rand::{thread_rng, Rng};
use rayon::{
    iter::{IndexedParallelIterator, IntoParallelIterator},
    slice::ParallelSliceMut,
};

pub const FLOAT_ERROR: f64 = 1e-7;
const BVH_LEAF_MAX: usize = 4;
const BVH_NUM_SPLITS: usize = 40;

pub trait Object: Sync {
    /// Returns the closest intersection of this object with a ray.
    fn intersect(&self, ray: &Ray) -> Option<Hit>;

    /// Returns this object's material.
    fn material(&self) -> &Material;

    /// Samples a point on the surface of this object and returns its probability density.
    fn sample_surface(&self, origin_point: DVec3) -> (Hit, f32);

    /// Returns the probability density of sampling a given surface point (`triangle_point`).
    fn surface_pdf(&self, origin_point: DVec3, triangle_point: DVec3, triangle_id: u32) -> f32;
}

pub struct Ray {
    pub origin: DVec3,
    pub direction: DVec3,
}

pub struct Hit {
    pub id: u32,
    pub triangle_id: u32,
    pub point: DVec3,
    pub distance: f64,
    pub normal: DVec3,
}

#[derive(Clone, Copy)]
pub struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}

pub enum Material {
    Diffuse(Vec3),
    Emitter(Vec3, f32),
    Metal(Vec3, f32),
    Nonmetal(Vec3, f32, f32),
    Glass(Vec3, f32, f32),
    Mix(Box<Material>, Box<Material>, f32),
}

pub struct Sphere {
    id: u32,
    center: Vec3,
    radius: f32,
    material: Material,
}

pub struct Mesh {
    id: u32,
    vertices: Vec<Vec3>,
    indices: Vec<[u32; 3]>,
    bvh: Vec<BoundingBox>,
    material: Material,
}

pub struct BoundingBox {
    start_index: u32,
    end_index: u32,
    bounds: Bounds,
    descendant_count: u32,
}

impl Ray {
    /// Creates a Ray struct from an origin and normalized direction.
    pub fn new(origin: DVec3, direction: DVec3) -> Self {
        Ray { origin, direction }
    }

    /// Finds the point at a given distance from the origin along the ray direction.
    pub fn at(&self, distance: f64) -> DVec3 {
        self.origin + distance * self.direction
    }
}

impl Bounds {
    /// Adds other bounds to these bounds (in-place union).
    fn add_bounds(&mut self, other: &Bounds) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }

    /// Expands these bounds by a certain amount on all sides.
    fn expand(&mut self, addition: Vec3) {
        self.min -= addition;
        self.max += addition;
    }

    /// Returns the size of the box contained by these bounds.
    fn size(&self) -> Vec3 {
        self.max - self.min
    }

    /// Returns the union of these bounds with other bounds.
    fn union(&self, other: &Bounds) -> Self {
        Bounds {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
}

impl Material {
    /// Returns BSDF * cos(theta) at a point. `incident` points towards the source of incoming
    /// light and `exitant` points where the light travels.
    pub fn bsdf_multiplier(&self, incident: &DVec3, exitant: &DVec3, normal: &DVec3) -> Vec3 {
        match self {
            Material::Diffuse(color) => (color.as_dvec3() * normal.dot(*incident) / PI).as_vec3(),
            Material::Emitter(_, _) => Vec3::ZERO,
            Material::Metal(color, roughness) => {
                let color = color.as_dvec3();

                let normal_dot_incident = normal.dot(*incident);
                let normal_dot_exitant = normal.dot(*exitant);

                let half = (*incident + *exitant).normalize();
                let half_dot_normal = half.dot(*normal);
                let half_dot_incident = half.dot(*incident);

                let fresnel = color + (1.0 - color) * (1.0 - half_dot_incident).powi(5);

                let masking = todo!();

                let alpha_sq = roughness.powi(4);

                // fresnel * masking * ggx / (4.0 * normal_dot_exitant)

                // account for multiple scattering?

                todo!()
            }
            Material::Nonmetal(_, _, _) => todo!(),
            Material::Glass(_, _, _) => todo!(),
            Material::Mix(_, _, _) => todo!(),
        }
    }

    /// Returns the radiance emitted by this material in a given direction.
    pub fn emitted(&self, _exitant: &DVec3) -> Vec3 {
        match self {
            Material::Emitter(color, strength) => *color * *strength,
            _ => Vec3::ZERO,
        }
    }

    /// Samples a new direction from the BSDF and returns its probability density.
    pub fn sample_direction(&self, normal: &DVec3) -> (DVec3, f32) {
        match self {
            Material::Diffuse(_) | Material::Emitter(_, _) => {
                let r_sq = thread_rng().gen::<f64>();
                let r = r_sq.sqrt();
                let theta = 2.0 * PI * thread_rng().gen::<f64>();

                let direction = DVec3::new(r * theta.cos(), (1.0 - r_sq).sqrt(), r * theta.sin());
                let rotated = DQuat::from_rotation_arc(DVec3::Y, *normal).mul_vec3(direction);
                (rotated, (direction.dot(DVec3::Y) / PI) as f32)
            }
            Material::Metal(_, _) => todo!(),
            Material::Nonmetal(_, _, _) => todo!(),
            Material::Glass(_, _, _) => todo!(),
            Material::Mix(_, _, _) => todo!(),
        }
    }

    /// Returns the probability density of sampling a given direction.
    pub fn direction_pdf(&self, direction: &DVec3, normal: &DVec3) -> f32 {
        match self {
            Material::Diffuse(_) | Material::Emitter(_, _) => (direction.dot(*normal) / PI) as f32,
            Material::Metal(_, _) => todo!(),
            Material::Nonmetal(_, _, _) => todo!(),
            Material::Glass(_, _, _) => todo!(),
            Material::Mix(_, _, _) => todo!(),
        }
    }
}

impl Sphere {
    /// Creates a Sphere struct.
    pub fn new(id: usize, center: Vec3, radius: f32, material: Material) -> Self {
        Sphere {
            id: id as u32,
            center,
            radius,
            material,
        }
    }
}

impl Object for Sphere {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let radius_sq = self.radius as f64 * self.radius as f64;
        let origin_to_center = self.center.as_dvec3() - ray.origin;
        let origin_to_center_sq = origin_to_center.length_squared();

        let projection_on_ray = origin_to_center.dot(ray.direction);
        if projection_on_ray < 0.0 || origin_to_center_sq < radius_sq {
            return None;
        }

        let center_to_projection_sq = origin_to_center_sq - projection_on_ray * projection_on_ray;
        if center_to_projection_sq > radius_sq {
            return None;
        }

        let projection_to_hit = (radius_sq - center_to_projection_sq).sqrt();
        let hit_distance = projection_on_ray - projection_to_hit;
        if hit_distance < FLOAT_ERROR {
            return None;
        }

        let hit_location = ray.at(hit_distance - FLOAT_ERROR);
        let mut normal = (hit_location - self.center.as_dvec3()).normalize();
        if ray.direction.dot(normal) > 0.0 {
            normal *= -1.0;
        }

        Some(Hit {
            id: self.id,
            triangle_id: 0,
            point: hit_location,
            distance: hit_distance - FLOAT_ERROR,
            normal,
        })
    }

    fn material(&self) -> &Material {
        &self.material
    }

    fn sample_surface(&self, origin_point: DVec3) -> (Hit, f32) {
        // later, try to only sample smaller potentially visible part, and update surface_pdf too
        todo!()
    }

    fn surface_pdf(&self, origin_point: DVec3, triangle_point: DVec3, triangle_id: u32) -> f32 {
        todo!()
    }
}

impl Mesh {
    /// Creates a Mesh struct.
    pub fn new(
        id: usize,
        vertices: Vec<Vec3>,
        mut indices_and_bounds: Vec<([u32; 3], Bounds)>,
        material: Material,
    ) -> Self {
        let mut full_bounds = indices_and_bounds[0].1;
        for triangle in &indices_and_bounds {
            full_bounds.add_bounds(&triangle.1);
        }

        let length = indices_and_bounds.len();
        let bvh = make_bvh(&mut indices_and_bounds, full_bounds, 0, length);

        let mut indices: Vec<[u32; 3]> = Vec::with_capacity(length);
        let mut bounds: Vec<Bounds> = Vec::with_capacity(length);
        indices_and_bounds
            .into_par_iter()
            .unzip_into_vecs(&mut indices, &mut bounds);

        Mesh {
            id: id as u32,
            vertices,
            indices,
            bvh,
            material,
        }
    }
}

impl Object for Mesh {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let mut best_distance_normal: Option<(f64, DVec3)> = None;
        let mut best_triangle_id = 0;
        let ray_direction_recip = ray.direction.recip();

        let mut i = 0;
        while i < self.bvh.len() {
            let near = (self.bvh[i].bounds.min.as_dvec3() - ray.origin) * ray_direction_recip;
            let far = (self.bvh[i].bounds.max.as_dvec3() - ray.origin) * ray_direction_recip;

            let min_distance = near.min(far).max_element().max(FLOAT_ERROR);
            let max_distance = far.max(near).min_element().min(f64::INFINITY);
            if min_distance >= max_distance {
                i += (self.bvh[i].descendant_count + 1) as usize;
                continue;
            }

            if self.bvh[i].descendant_count > 0 {
                i += 1;
                continue;
            }

            let triangles =
                &self.indices[self.bvh[i].start_index as usize..self.bvh[i].end_index as usize];
            for (triangle_id, triangle) in triangles.iter().enumerate() {
                let p1 = self.vertices[triangle[0] as usize].as_dvec3();
                let p2 = self.vertices[triangle[1] as usize].as_dvec3();
                let p3 = self.vertices[triangle[2] as usize].as_dvec3();

                let Some((distance, normal)) = intersect_triangle(ray, p1, p2, p3) else {
                    continue;
                };

                if best_distance_normal.is_none() || distance < best_distance_normal.unwrap().0 {
                    best_distance_normal = Some((distance, normal));
                    best_triangle_id = triangle_id;
                }
            }

            i += 1;
        }

        let (distance, mut normal) = best_distance_normal?;
        if ray.direction.dot(normal) > 0.0 {
            normal *= -1.0;
        }

        Some(Hit {
            id: self.id,
            triangle_id: best_triangle_id as u32,
            point: ray.at(distance - FLOAT_ERROR),
            distance: distance - FLOAT_ERROR,
            normal,
        })
    }

    fn material(&self) -> &Material {
        &self.material
    }

    fn sample_surface(&self, point: DVec3) -> (Hit, f32) {
        // todo: later consider trying to sample smaller subset
        let chosen_triangle_id = thread_rng().gen_range(0..self.indices.len());

        let u = 1.0 - thread_rng().gen::<f64>().sqrt();
        let v = thread_rng().gen::<f64>() * (1.0 - u);

        let triangle = self.indices[chosen_triangle_id];
        let p1 = self.vertices[triangle[0] as usize].as_dvec3();
        let p2 = self.vertices[triangle[1] as usize].as_dvec3();
        let p3 = self.vertices[triangle[2] as usize].as_dvec3();

        let triangle_point = u * p1 + v * p2 + (1.0 - u - v) * p3;
        let side2_cross_side1 = (p3 - p1).cross(p2 - p1);

        let triangle_point_to_point = (point - triangle_point).normalize();
        let mut normal = side2_cross_side1.normalize();
        if triangle_point_to_point.dot(normal) < 0.0 {
            normal *= -1.0;
        }

        let surface_pdf = 2.0 / side2_cross_side1.length();
        let distance = triangle_point.distance(point);
        let area_to_solid_angle = distance.powi(2) / normal.dot(triangle_point_to_point);

        (
            Hit {
                id: self.id,
                triangle_id: chosen_triangle_id as u32,
                point: triangle_point + FLOAT_ERROR * triangle_point_to_point,
                distance,
                normal,
            },
            (area_to_solid_angle * surface_pdf / self.indices.len() as f64) as f32,
        )
    }

    fn surface_pdf(&self, origin_point: DVec3, triangle_point: DVec3, triangle_id: u32) -> f32 {
        let triangle = self.indices[triangle_id as usize];
        let p1 = self.vertices[triangle[0] as usize].as_dvec3();
        let p2 = self.vertices[triangle[1] as usize].as_dvec3();
        let p3 = self.vertices[triangle[2] as usize].as_dvec3();

        let side2_cross_side1 = (p3 - p1).cross(p2 - p1);
        let mut normal = side2_cross_side1.normalize();
        if (origin_point - triangle_point).dot(normal) < 0.0 {
            normal *= -1.0;
        }

        let surface_pdf = 2.0 / side2_cross_side1.length();
        let distance = triangle_point.distance(origin_point);
        let area_to_solid_angle =
            distance.powi(2) / normal.dot((origin_point - triangle_point).normalize());

        (area_to_solid_angle * surface_pdf / self.indices.len() as f64) as f32
    }
}

/// Finds the intersection of a ray and triangle, returning the distance and normal vector.
fn intersect_triangle(ray: &Ray, p1: DVec3, p2: DVec3, p3: DVec3) -> Option<(f64, DVec3)> {
    let side1 = p2 - p1;
    let side2 = p3 - p1;

    let ray_cross_side2 = ray.direction.cross(side2);
    let denominator = side1.dot(ray_cross_side2);
    if denominator.abs() < FLOAT_ERROR {
        return None;
    }

    let fraction = denominator.recip();
    let p1_to_origin = ray.origin - p1;
    let u = fraction * p1_to_origin.dot(ray_cross_side2);
    if u < 0.0 {
        return None;
    }

    let p1_to_origin_cross_side1 = p1_to_origin.cross(side1);
    let v = fraction * ray.direction.dot(p1_to_origin_cross_side1);
    let hit_distance = fraction * side2.dot(p1_to_origin_cross_side1);
    let normal = side2.cross(side1).normalize();

    if v < 0.0 || u + v > 1.0 || hit_distance < FLOAT_ERROR {
        None
    } else {
        Some((hit_distance, normal))
    }
}

/// Constructs a bounding volume hierarchy for triangles in a mesh.
fn make_bvh(
    indices_and_bounds: &mut [([u32; 3], Bounds)],
    mut full_bounds: Bounds,
    start: usize,
    end: usize,
) -> Vec<BoundingBox> {
    full_bounds.expand(Vec3::splat(1e-6));
    let mut bvh_tree = vec![BoundingBox {
        start_index: start as u32,
        end_index: end as u32,
        bounds: full_bounds,
        descendant_count: 0,
    }];

    let length = end - start;
    if length <= BVH_LEAF_MAX {
        return bvh_tree;
    }

    let range = full_bounds.size().to_array();
    let mut best_axis = 0;
    for axis in 1..3 {
        if range[axis] > range[best_axis] {
            best_axis = axis;
        }
    }

    indices_and_bounds[start..end]
        .par_sort_unstable_by(|a, b| a.1.min[best_axis].total_cmp(&b.1.min[best_axis]));

    let num_chunks = length.min(BVH_NUM_SPLITS + 1);
    let chunk_size = length / num_chunks;

    let mut chunk_ends: Vec<usize> = vec![0; num_chunks];
    for chunk in 0..num_chunks {
        let previous = if chunk == 0 {
            start
        } else {
            chunk_ends[chunk - 1]
        };

        let extra = if chunk < length % num_chunks { 1 } else { 0 };
        chunk_ends[chunk] = previous + chunk_size + extra;
    }

    let mut bounds_per_chunk: Vec<Bounds> = Vec::with_capacity(num_chunks);
    for chunk in 0..num_chunks {
        let chunk_start = if chunk == 0 {
            start
        } else {
            chunk_ends[chunk - 1]
        };

        let mut chunk_bounds = indices_and_bounds[chunk_start].1;
        for triangle in &indices_and_bounds[chunk_start..chunk_ends[chunk]] {
            chunk_bounds.add_bounds(&triangle.1);
        }

        bounds_per_chunk.push(chunk_bounds);
    }

    let mut bounds_from_left = vec![bounds_per_chunk[0]; num_chunks];
    for chunk in 1..num_chunks {
        bounds_from_left[chunk] = bounds_from_left[chunk - 1].union(&bounds_per_chunk[chunk]);
    }

    let mut bounds_from_right = vec![bounds_per_chunk[num_chunks - 1]; num_chunks];
    for chunk in (0..num_chunks - 1).rev() {
        bounds_from_right[chunk] = bounds_from_right[chunk + 1].union(&bounds_per_chunk[chunk]);
    }

    let mut best_split = 0;
    let mut best_cost = f32::INFINITY;

    for chunk in 0..(num_chunks - 1) {
        let left_count = chunk_ends[chunk] - start;
        let left_bounds = bounds_from_left[chunk].size();
        let left_bounds_shifted = Vec3::new(left_bounds.y, left_bounds.z, left_bounds.x);
        let left_cost = left_bounds.dot(left_bounds_shifted) * left_count as f32;

        let right_bounds = bounds_from_right[chunk + 1].size();
        let right_bounds_shifted = Vec3::new(right_bounds.y, right_bounds.z, right_bounds.x);
        let right_cost = right_bounds.dot(right_bounds_shifted) * (length - left_count) as f32;

        if left_cost + right_cost < best_cost {
            best_split = chunk;
            best_cost = left_cost + right_cost;
        }
    }

    let left_bounds = bounds_from_left[best_split];
    bvh_tree.append(&mut make_bvh(
        indices_and_bounds,
        left_bounds,
        start,
        chunk_ends[best_split],
    ));

    let right_bounds = bounds_from_right[best_split + 1];
    bvh_tree.append(&mut make_bvh(
        indices_and_bounds,
        right_bounds,
        chunk_ends[best_split],
        end,
    ));

    bvh_tree[0].descendant_count = (bvh_tree.len() - 1) as u32;
    bvh_tree
}
