use std::f32::consts::PI;

use glam::{Quat, Vec3};
use rand::{thread_rng, Rng};
use rayon::{
    iter::{IndexedParallelIterator, IntoParallelIterator},
    slice::ParallelSliceMut,
};

const FLOAT_ERROR: f32 = 0.000001;
const BVH_LEAF_MAX: usize = 8;
const BVH_NUM_SPLITS: usize = 20;

pub trait Object: Sync {
    fn intersect(&self, ray: &Ray) -> Option<Hit>;
}

pub trait Material: Sync {
    fn light_and_direction(&self, incoming: Vec3, normal: Vec3) -> (Vec3, Vec3);
}

pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

pub struct Hit<'a> {
    pub distance: f32,
    pub normal: Vec3,
    pub material: &'a dyn Material,
}

#[derive(Clone, Copy)]
pub struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}

pub struct DiffuseMaterial {
    reflectance: Vec3,
}

pub struct MirrorMaterial {
    reflectance: Vec3,
}

pub struct Sphere {
    center: Vec3,
    radius: f32,
    material: Box<dyn Material>,
}

pub struct Triangle {
    point1: Vec3,
    point2: Vec3,
    point3: Vec3,
    material: Box<dyn Material>,
}

pub struct Mesh {
    vertices: Vec<Vec3>,
    indices: Vec<[u32; 3]>,
    bvh: Vec<BoundingBox>,
    material: Box<dyn Material>,
}

struct BoundingBox {
    start_index: usize,
    end_index: usize,
    bounds: Bounds,
    descendant_count: usize,
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

impl Bounds {
    fn add_bounds(&mut self, other: &Bounds) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }

    fn expand(&mut self, addition: Vec3) {
        self.min -= addition;
        self.max += addition;
    }

    fn size(&self) -> Vec3 {
        self.max - self.min
    }

    fn union(&self, other: &Bounds) -> Self {
        Bounds {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
}

impl DiffuseMaterial {
    pub fn new(reflectance: Vec3) -> Self {
        DiffuseMaterial { reflectance }
    }
}

impl Material for DiffuseMaterial {
    fn light_and_direction(&self, _incoming: Vec3, normal: Vec3) -> (Vec3, Vec3) {
        let r = thread_rng().gen::<f32>().sqrt();
        let theta = 2.0 * PI * thread_rng().gen::<f32>();

        let x = r * theta.cos();
        let z = r * theta.sin();
        let y = (1.0 - x * x - z * z).max(0.0).sqrt();

        let rotation = Quat::from_rotation_arc(Vec3::new(0.0, 1.0, 0.0), normal);
        let outgoing = rotation.mul_vec3(Vec3::new(x, y, z));
        (self.reflectance, outgoing)
    }
}

impl MirrorMaterial {
    pub fn new(reflectance: Vec3) -> Self {
        MirrorMaterial { reflectance }
    }
}

impl Material for MirrorMaterial {
    fn light_and_direction(&self, incoming: Vec3, normal: Vec3) -> (Vec3, Vec3) {
        let outgoing = incoming - 2.0 * normal * incoming.dot(normal);
        (self.reflectance, outgoing)
    }
}

impl Sphere {
    pub fn new(center: Vec3, radius: f32, material: Box<dyn Material>) -> Self {
        Sphere {
            center,
            radius,
            material,
        }
    }
}

impl Object for Sphere {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let radius_sq = self.radius * self.radius;
        let origin_to_center = self.center - ray.origin;
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
            None
        } else {
            Some(Hit {
                distance: hit_distance,
                normal: (ray.at(hit_distance) - self.center).normalize(),
                material: &*self.material,
            })
        }
    }
}

impl Triangle {
    pub fn new(point1: Vec3, point2: Vec3, point3: Vec3, material: Box<dyn Material>) -> Self {
        Triangle {
            point1,
            point2,
            point3,
            material,
        }
    }
}

impl Object for Triangle {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        intersect_triangle(ray, self.point1, self.point2, self.point3).map(|(distance, normal)| {
            Hit {
                distance,
                normal,
                material: &*self.material,
            }
        })
    }
}

impl Mesh {
    pub fn new(
        vertices: Vec<Vec3>,
        mut indices_and_bounds: Vec<([u32; 3], Bounds)>,
        material: Box<dyn Material>,
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
            vertices,
            indices,
            bvh,
            material,
        }
    }
}

impl Object for Mesh {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let mut best_distance_normal: Option<(f32, Vec3)> = None;
        let ray_direction_recip = ray.direction.recip();

        let mut i = 0;
        while i < self.bvh.len() {
            let near = (self.bvh[i].bounds.min - ray.origin) * ray_direction_recip;
            let far = (self.bvh[i].bounds.max - ray.origin) * ray_direction_recip;

            let min_distance = near.min(far).max_element().max(FLOAT_ERROR);
            let max_distance = far.max(near).min_element().min(f32::INFINITY);
            if min_distance >= max_distance {
                i += self.bvh[i].descendant_count + 1;
                continue;
            }

            if self.bvh[i].descendant_count > 0 {
                i += 1;
                continue;
            }

            let triangles = &self.indices[self.bvh[i].start_index..self.bvh[i].end_index];
            for triangle in triangles {
                let p1 = self.vertices[triangle[0] as usize];
                let p2 = self.vertices[triangle[1] as usize];
                let p3 = self.vertices[triangle[2] as usize];

                let Some((distance, normal)) = intersect_triangle(ray, p1, p2, p3) else {
                    continue;
                };

                if best_distance_normal.is_none() || distance < best_distance_normal.unwrap().0 {
                    best_distance_normal = Some((distance, normal));
                }
            }

            i += 1;
        }

        if let Some((distance, normal)) = best_distance_normal {
            Some(Hit {
                distance,
                normal,
                material: &*self.material,
            })
        } else {
            None
        }
    }
}

fn intersect_triangle(ray: &Ray, p1: Vec3, p2: Vec3, p3: Vec3) -> Option<(f32, Vec3)> {
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

fn make_bvh(
    indices_and_bounds: &mut [([u32; 3], Bounds)],
    mut full_bounds: Bounds,
    start: usize,
    end: usize,
) -> Vec<BoundingBox> {
    full_bounds.expand(Vec3::splat(FLOAT_ERROR));
    let mut bvh_tree = vec![BoundingBox {
        start_index: start,
        end_index: end,
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

    bvh_tree[0].descendant_count = bvh_tree.len() - 1;
    bvh_tree
}
