use glam::Vec3;

const FLOAT_ERROR: f32 = 0.00001;
const BVH_NODE_CHILDREN: usize = 4;
const BVH_LEAF_MAX: usize = 12;

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

pub struct Mesh {
    vertices: Vec<Vec3>,
    indices: Vec<[u32; 3]>,
    color: Vec3,
    bounds: Vec<BoundingBox>,
}

struct BoundingBox {
    start_index: usize,
    end_index: usize,
    bounds_min: Vec3,
    bounds_max: Vec3,
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
                color: self.color,
            })
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
        intersect_triangle(ray, self.point1, self.point2, self.point3).map(|(distance, normal)| {
            Hit {
                distance,
                normal,
                color: self.color,
            }
        })
    }
}

impl Mesh {
    pub fn new(vertices: Vec<Vec3>, mut indices: Vec<[u32; 3]>, color: Vec3) -> Self {
        let length = indices.len();
        let bounds = make_bvh(&vertices, &mut indices, 0, length);

        Mesh {
            vertices,
            indices,
            color,
            bounds,
        }
    }
}

impl Object for Mesh {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let mut best_hit: Option<Hit> = None;

        let mut i = 0;
        while i < self.bounds.len() {
            let near = (self.bounds[i].bounds_min - ray.origin) / ray.direction;
            let far = (self.bounds[i].bounds_max - ray.origin) / ray.direction;

            let min_distance = near.min(far).max_element().max(FLOAT_ERROR);
            let max_distance = far.max(near).min_element().min(f32::INFINITY);
            if min_distance >= max_distance {
                i += self.bounds[i].descendant_count + 1;
                continue;
            }

            if self.bounds[i].descendant_count > 0 {
                i += 1;
                continue;
            }

            let triangles = &self.indices[self.bounds[i].start_index..self.bounds[i].end_index];
            for triangle in triangles {
                let p1 = self.vertices[triangle[0] as usize];
                let p2 = self.vertices[triangle[1] as usize];
                let p3 = self.vertices[triangle[2] as usize];

                let Some((distance, normal)) = intersect_triangle(ray, p1, p2, p3) else {
                    continue;
                };

                if best_hit.is_none() || distance < best_hit.as_ref().unwrap().distance {
                    best_hit = Some(Hit {
                        distance,
                        normal,
                        color: self.color,
                    })
                }
            }

            i += 1;
        }

        best_hit
    }
}

fn intersect_triangle(ray: &Ray, p1: Vec3, p2: Vec3, p3: Vec3) -> Option<(f32, Vec3)> {
    let side1 = p2 - p1;
    let side2 = p3 - p1;

    let normal = side2.cross(side1);
    let ray_cross_side2 = ray.direction.cross(side2);
    let denominator = side1.dot(ray_cross_side2);
    if denominator.abs() < FLOAT_ERROR || ray.direction.dot(normal) > 0.0 {
        return None;
    }

    let fraction = 1.0 / denominator;
    let p1_to_origin = ray.origin - p1;
    let u = fraction * p1_to_origin.dot(ray_cross_side2);
    if u < 0.0 {
        return None;
    }

    let p1_to_origin_cross_side1 = p1_to_origin.cross(side1);
    let v = fraction * ray.direction.dot(p1_to_origin_cross_side1);
    let hit_distance = fraction * side2.dot(p1_to_origin_cross_side1);

    if v < 0.0 || u + v > 1.0 || hit_distance < FLOAT_ERROR {
        None
    } else {
        Some((hit_distance, normal.normalize()))
    }
}

fn make_bvh(
    vertices: &[Vec3],
    indices: &mut [[u32; 3]],
    start: usize,
    end: usize,
) -> Vec<BoundingBox> {
    let mut bounds_min = vertices[indices[start][0] as usize];
    let mut bounds_max = vertices[indices[start][0] as usize];

    for triangle in indices[start..end].iter() {
        for index in triangle {
            let vertex = vertices[*index as usize];
            bounds_min = vertex.min(bounds_min);
            bounds_max = vertex.max(bounds_max);
        }
    }

    bounds_min -= Vec3::splat(FLOAT_ERROR);
    bounds_max += Vec3::splat(FLOAT_ERROR);

    let mut bvh_tree = vec![BoundingBox {
        start_index: start,
        end_index: end,
        bounds_min,
        bounds_max,
        descendant_count: 0,
    }];

    if end - start <= BVH_LEAF_MAX {
        return bvh_tree;
    }

    let range = (bounds_max - bounds_min).to_array();
    let mut best_axis = 0;
    for i in 1..3 {
        if range[i] > range[best_axis] {
            best_axis = i;
        }
    }

    indices[start..end].sort_by(|a, b| {
        let a_average = a
            .iter()
            .map(|i| vertices[*i as usize][best_axis])
            .sum::<f32>();
        let b_average = b
            .iter()
            .map(|i| vertices[*i as usize][best_axis])
            .sum::<f32>();
        a_average
            .partial_cmp(&b_average)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let part_size = (end - start) / BVH_NODE_CHILDREN;
    for i in 0..BVH_NODE_CHILDREN {
        let child_start = start + i * part_size;
        let child_end = if i < BVH_NODE_CHILDREN - 1 {
            child_start + part_size
        } else {
            end
        };

        bvh_tree.append(&mut make_bvh(vertices, indices, child_start, child_end));
    }

    bvh_tree[0].descendant_count = bvh_tree.len() - 1;
    bvh_tree
}
