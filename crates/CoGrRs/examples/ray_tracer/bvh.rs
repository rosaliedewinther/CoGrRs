use std::{
    cmp::{max, min},
    fs::File,
    io::{BufRead, BufReader},
    ops::{Add, Div, Mul, Sub},
};

use bytemuck::{Pod, Zeroable};

#[repr(C, align(16))]
#[derive(Pod, Zeroable, Copy, Clone)]
pub struct Point {
    pos: [f32; 4],
}
#[repr(C, align(64))]
#[derive(Pod, Zeroable, Copy, Clone)]
pub struct Triangle {
    points: [Point; 4],
}
#[repr(C, align(32))]
#[derive(Pod, Zeroable, Copy, Clone)]
pub struct BVHNode {
    minx: f32,
    miny: f32,
    minz: f32,
    maxx: f32,
    maxy: f32,
    maxz: f32,
    left_first: i32,
    count: i32,
}
#[repr(C, align(32))]
#[derive(Pod, Zeroable, Copy, Clone)]
pub struct AABB {
    minx: f32,
    miny: f32,
    minz: f32,
    maxx: f32,
    maxy: f32,
    maxz: f32,
    _padding1: f32,
    _padding2: f32,
}

pub struct BVH {
    vertices: Vec<Point>,
    triangles: Vec<[u32; 4]>,
    indices: Vec<u32>,
    bvh_nodes: Vec<BVHNode>,
}

impl Point {
    pub fn new(x: f32, y: f32, z: f32) -> Point {
        Point {
            pos: [x, y, z, 0f32],
        }
    }
}

impl Add for Point {
    type Output = Point;

    fn add(self, other: Point) -> Point {
        Point {
            pos: [
                self.pos[0] + other.pos[0],
                self.pos[1] + other.pos[1],
                self.pos[2] + other.pos[2],
                0f32,
            ],
        }
    }
}

impl Sub for Point {
    type Output = Point;

    fn sub(self, other: Point) -> Point {
        Point {
            pos: [
                self.pos[0] - other.pos[0],
                self.pos[1] - other.pos[1],
                self.pos[2] - other.pos[2],
                0f32,
            ],
        }
    }
}

impl Mul<f32> for Point {
    type Output = Point;

    fn mul(self, scalar: f32) -> Point {
        Point {
            pos: [
                self.pos[0] * scalar,
                self.pos[1] * scalar,
                self.pos[2] * scalar,
                0f32,
            ],
        }
    }
}

impl Mul<Point> for Point {
    type Output = Point;

    fn mul(self, rhs: Point) -> Point {
        Point {
            pos: [
                self.pos[0] * rhs.pos[0],
                self.pos[1] * rhs.pos[1],
                self.pos[2] * rhs.pos[2],
                0f32,
            ],
        }
    }
}

impl Div<f32> for Point {
    type Output = Point;

    fn div(self, scalar: f32) -> Point {
        Point {
            pos: [
                self.pos[0] / scalar,
                self.pos[1] / scalar,
                self.pos[2] / scalar,
                0f32,
            ],
        }
    }
}
impl Point {
    pub fn min(&self, rhs: &Point) -> Point {
        Point {
            pos: [
                f32::min(self.pos[0], rhs.pos[0]),
                f32::min(self.pos[0], rhs.pos[0]),
                f32::min(self.pos[0], rhs.pos[0]),
                0f32,
            ],
        }
    }
    pub fn max(&self, rhs: &Point) -> Point {
        Point {
            pos: [
                f32::max(self.pos[0], rhs.pos[0]),
                f32::max(self.pos[0], rhs.pos[0]),
                f32::max(self.pos[0], rhs.pos[0]),
                0f32,
            ],
        }
    }
}

pub fn dot(a: &Point, b: &Point) -> f32 {
    a.pos[0] * b.pos[0] + a.pos[1] * b.pos[1] + a.pos[2] * b.pos[2]
}

pub fn cross(a: &Point, b: &Point) -> Point {
    Point {
        pos: [
            a.pos[1] * b.pos[2] - a.pos[2] * b.pos[1],
            a.pos[2] * b.pos[0] - a.pos[0] * b.pos[2],
            a.pos[0] * b.pos[1] - a.pos[1] * b.pos[0],
            0f32,
        ],
    }
}

pub fn length(point: &Point) -> f32 {
    (point.pos[0] * point.pos[0] + point.pos[1] * point.pos[1] + point.pos[2] * point.pos[2]).sqrt()
}

pub fn normalize(point: &Point) -> Point {
    *point / length(point)
}

pub fn distance(a: &Point, b: &Point) -> f32 {
    length(&(*a - *b))
}

impl BVH {
    pub fn construct(filename: &str) -> BVH {
        println!("reading .obj file");

        let mut vertices = Vec::new();
        let mut triangles = Vec::new();

        let file = File::open(filename).unwrap();
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line.unwrap();
            let splits: Vec<&str> = line.split(" ").collect();
            if splits[0] == "v" {
                let p1 = splits[1].parse::<f32>().unwrap();
                let p2 = splits[2].parse::<f32>().unwrap();
                let p3 = splits[3].parse::<f32>().unwrap();
                vertices.push(Point {
                    pos: [p1, p2, p3, 0f32],
                });
            }
            if splits[0] == "f" {
                let p1 = splits[1].split("/").next().unwrap().parse::<u32>().unwrap() - 1;
                let p2 = splits[2].split("/").next().unwrap().parse::<u32>().unwrap() - 1;
                let p3 = splits[3].split("/").next().unwrap().parse::<u32>().unwrap() - 1;
                triangles.push([p1, p2, p3, 0]);
            }
        }

        println!("len v: {}", vertices.len());
        println!(
            "max f: {}",
            triangles
                .iter()
                .map(|val| max(max(max(val[0], val[1]), val[2]), val[3]))
                .max()
                .unwrap()
        );

        println!("done with parsing .obj file");

        let mut indices: Vec<u32> = triangles
            .iter()
            .enumerate()
            .map(|(i, _)| i as u32)
            .collect();

        let aabb =
            Self::calculate_bounds(&vertices, &triangles, &indices, 0, triangles.len() as u32);

        let scale_factor = Point {
            pos: [
                1f32 / (aabb.maxx - aabb.minx),
                1f32 / (aabb.maxy - aabb.miny),
                1f32 / (aabb.maxz - aabb.minz),
                0f32,
            ],
        };
        let offset = Point {
            pos: [
                0f32 - (aabb.maxx + aabb.minx) * scale_factor.pos[0] / 2f32,
                0f32 - (aabb.maxy + aabb.miny) * scale_factor.pos[1] / 2f32,
                0f32 - (aabb.maxz + aabb.minz) * scale_factor.pos[2] / 2f32,
                0f32,
            ],
        };
        vertices
            .iter_mut()
            .for_each(|vertex| *vertex = (*vertex) * scale_factor + offset);

        let mut bvh_nodes = vec![BVHNode::zeroed(); triangles.len() * 2];

        let centers: Vec<Point> = triangles
            .iter()
            .map(|t| {
                (vertices[t[0] as usize]
                    + vertices[t[1] as usize]
                    + vertices[t[2] as usize]
                    + vertices[t[3] as usize])
                    / 4f32
            })
            .collect();

        bvh_nodes[0].left_first = 0;
        bvh_nodes[0].count = triangles.len() as i32;

        let aabb =
            Self::calculate_bounds(&vertices, &triangles, &indices, 0, vertices.len() as u32);
        bvh_nodes[0].maxx = aabb.maxx;
        bvh_nodes[0].maxy = aabb.maxy;
        bvh_nodes[0].maxz = aabb.maxz;
        bvh_nodes[0].minx = aabb.minx;
        bvh_nodes[0].miny = aabb.miny;
        bvh_nodes[0].minz = aabb.minz;

        Self::subdivide(
            &vertices,
            &triangles,
            &mut indices,
            &centers,
            &mut bvh_nodes,
            0,
            0,
            &mut 2,
        );

        println!("done building bvh");

        BVH {
            vertices,
            triangles,
            indices,
            bvh_nodes,
        }
    }

    fn subdivide(
        vertices: &Vec<Point>,
        triangles: &Vec<[u32; 4]>,
        indices: &mut Vec<u32>,
        centers: &Vec<Point>,
        bvh_nodes: &mut Vec<BVHNode>,
        current_bvh_index: usize,
        start: u32,
        pool_index: &mut u32,
    ) {
        if bvh_nodes[current_bvh_index].count <= 3 {
            bvh_nodes[current_bvh_index].left_first = start as i32;
            return;
        }
        let index = *pool_index;
        *pool_index += 2;
        bvh_nodes[current_bvh_index].left_first = index as i32;
        let pivot = Self::partition(
            vertices,
            triangles,
            indices,
            centers,
            bvh_nodes,
            current_bvh_index,
            start,
            bvh_nodes[current_bvh_index].count as u32,
        );
        let left_count = pivot - start;
        bvh_nodes[index as usize].count = left_count as i32;
        let bounds = Self::calculate_bounds(vertices, triangles, indices, start, left_count);
        bvh_nodes[index as usize].maxx = bounds.maxx;
        bvh_nodes[index as usize].maxy = bounds.maxy;
        bvh_nodes[index as usize].maxz = bounds.maxz;
        bvh_nodes[index as usize].minx = bounds.minx;
        bvh_nodes[index as usize].miny = bounds.miny;
        bvh_nodes[index as usize].minz = bounds.minz;

        let right_count = bvh_nodes[current_bvh_index].count - bvh_nodes[index as usize].count;
        bvh_nodes[index as usize + 1].count = right_count;
        let bounds =
            Self::calculate_bounds(vertices, triangles, indices, pivot, right_count as u32);
        bvh_nodes[index as usize + 1].maxx = bounds.maxx;
        bvh_nodes[index as usize + 1].maxy = bounds.maxy;
        bvh_nodes[index as usize + 1].maxz = bounds.maxz;
        bvh_nodes[index as usize + 1].minx = bounds.minx;
        bvh_nodes[index as usize + 1].miny = bounds.miny;
        bvh_nodes[index as usize + 1].minz = bounds.minz;

        Self::subdivide(
            vertices,
            triangles,
            indices,
            centers,
            bvh_nodes,
            index as usize,
            start,
            pool_index,
        );
        Self::subdivide(
            vertices,
            triangles,
            indices,
            centers,
            bvh_nodes,
            index as usize + 1,
            pivot,
            pool_index,
        );
    }

    fn partition(
        vertices: &Vec<Point>,
        triangles: &Vec<[u32; 4]>,
        indices: &mut Vec<u32>,
        centers: &Vec<Point>,
        bvh_nodes: &Vec<BVHNode>,
        current_bvh_index: usize,
        start: u32,
        count: u32,
    ) -> u32 {
        let bins = 8;
        let mut optimal_axis = 0;
        let mut optimal_pos = 0f32;
        let mut optimal_cost = f32::MAX;
        for axis in 0..3 {
            for b in 1..bins {
                let pos = match axis {
                    0 => Self::lerp(
                        bvh_nodes[current_bvh_index].minx,
                        bvh_nodes[current_bvh_index].maxx,
                        (b as f32) / (bins as f32),
                    ),
                    1 => Self::lerp(
                        bvh_nodes[current_bvh_index].miny,
                        bvh_nodes[current_bvh_index].maxy,
                        (b as f32) / (bins as f32),
                    ),
                    2 => Self::lerp(
                        bvh_nodes[current_bvh_index].minz,
                        bvh_nodes[current_bvh_index].maxz,
                        (b as f32) / (bins as f32),
                    ),
                    _ => panic!("error when partitioning"),
                };
                let pivot = Self::partition_shuffle(indices, centers, axis, pos, start, count);

                let bb1_count = pivot - start;
                let bb2_count = count - bb1_count;

                let bb1 = Self::calculate_bounds(vertices, triangles, indices, start, bb1_count);
                let bb2 = Self::calculate_bounds(vertices, triangles, indices, pivot, bb2_count);

                let half_area1 = (bb1.maxx - bb1.minx) * (bb1.maxy - bb1.miny)
                    + (bb1.maxx - bb1.minx) * (bb1.maxz - bb1.minz)
                    + (bb1.maxy - bb1.miny) * (bb1.maxz - bb1.minz);
                let half_area2 = (bb2.maxx - bb2.minx) * (bb2.maxy - bb2.miny)
                    + (bb2.maxx - bb2.minx) * (bb2.maxz - bb2.minz)
                    + (bb2.maxy - bb2.miny) * (bb2.maxz - bb2.minz);

                let cost = half_area1 * bb1_count as f32 + half_area2 * bb2_count as f32;
                if cost < optimal_cost {
                    optimal_axis = axis;
                    optimal_pos = pos;
                    optimal_cost = cost;
                }
            }
        }

        Self::partition_shuffle(indices, centers, optimal_axis, optimal_pos, start, count)
    }

    fn partition_shuffle(
        indices: &mut Vec<u32>,
        centers: &Vec<Point>,
        axis: usize,
        pos: f32,
        start: u32,
        count: u32,
    ) -> u32 {
        let mut end = (start + count - 1) as usize;
        let mut i = start as usize;

        while i < end {
            if centers[indices[i] as usize].pos[axis] > pos {
                //we have to swap
                let temp = indices[i];
                indices[i] = indices[end];
                indices[end] = temp;
                end -= 1;
            }
            i += 1;
        }

        i as u32
    }

    // return min and max point
    fn calculate_bounds(
        vertices: &Vec<Point>,
        triangles: &Vec<[u32; 4]>,
        indices: &Vec<u32>,
        first: u32,
        amount: u32,
    ) -> AABB {
        let mut max_point = Point {
            pos: [f32::MIN, f32::MIN, f32::MIN, 0f32],
        };
        let mut min_point = Point {
            pos: [f32::MAX, f32::MAX, f32::MAX, 0f32],
        };
        for i in first..first + amount {
            let i = i as usize;
            for j in 0..4 as usize {
                let vertex = &vertices[triangles[indices[i] as usize][j] as usize];
                max_point = Point::max(&max_point, vertex);
                min_point = Point::min(&min_point, vertex);
            }
        }
        AABB {
            minx: min_point.pos[0],
            miny: min_point.pos[1],
            minz: min_point.pos[2],
            maxx: max_point.pos[0],
            maxy: max_point.pos[1],
            maxz: max_point.pos[2],
            _padding1: 0f32,
            _padding2: 0f32,
        }
    }

    fn lerp(a: f32, b: f32, p: f32) -> f32 {
        a * (1f32 - p) + (b * p)
    }
    pub fn intersects_triangle(
        &self,
        ray_origin: &Point,
        ray_direction: &Point,
        triangle: &[u32; 4],
    ) -> Option<f32> {
        let a = &self.vertices[triangle[0] as usize];
        let b = &self.vertices[triangle[1] as usize];
        let c = &self.vertices[triangle[2] as usize];
        let a_to_b = *b - *a;
        let a_to_c = *c - *a;

        // Begin calculating determinant - also used to calculate u parameter
        // u_vec lies in view plane
        // length of a_to_c in view_plane = |u_vec| = |a_to_c|*sin(a_to_c, dir)
        let u_vec = cross(ray_direction, &a_to_c);

        // If determinant is near zero, ray lies in plane of triangle
        // The determinant corresponds to the parallelepiped volume:
        // det = 0 => [dir, a_to_b, a_to_c] not linearly independant
        let det = dot(&a_to_b, &u_vec);

        // Only testing positive bound, thus enabling backface culling
        // If backface culling is not desired write:
        // det < 0.0001 && det > -0.0001
        if det < 0.0001 && det > -0.0001 {
            return None;
        }

        let inv_det = 1.0 / det;

        // Vector from point a to ray origin
        let a_to_origin = *ray_origin - *a;

        // Calculate u parameter
        let u = dot(&a_to_origin, &u_vec) * inv_det;

        // Test bounds: u < 0 || u > 1 => outside of triangle
        if !(0.0..=1.0).contains(&u) {
            return None;
        }

        // Prepare to test v parameter
        let v_vec = cross(&a_to_origin, &a_to_b);

        // Calculate v parameter and test bound
        let v = dot(ray_direction, &v_vec) * inv_det;
        // The intersection lies outside of the triangle
        if v < 0.0 || u + v > 1.0 {
            return None;
        }

        let dist = dot(&a_to_c, &v_vec) * inv_det;

        if dist > 0.0001 {
            Some(dist)
        } else {
            None
        }
    }

    pub fn intersect(&self, origin: &Point, direction: &Point) -> Option<([u32; 4], f32)> {
        let mut closest_hit_triangle = None;
        let mut t = f32::MAX;
        for triangle in &self.triangles {
            match self.intersects_triangle(origin, direction, triangle) {
                None => (),
                Some(new_t) => {
                    if new_t < t {
                        closest_hit_triangle = Some(triangle);
                        t = new_t;
                    }
                }
            }
        }
        match closest_hit_triangle {
            None => None,
            Some(triangle) => Some((*triangle, t)),
        }
    }
}
