use core::panic;
use std::collections::HashMap;
use std::fmt::Debug;
use std::{
    cmp::{max, min},
    fmt::format,
    fs::File,
    io::{BufRead, BufReader},
    ops::{Add, Div, Mul, Sub},
};

use bytemuck::{Pod, Zeroable};

#[repr(C, align(16))]
#[derive(Pod, Zeroable, Copy, Clone, Debug)]
pub struct Point {
    pub pos: [f32; 4],
}
#[repr(C, align(32))]
#[derive(Pod, Zeroable, Copy, Clone)]
pub struct BVHNode {
    pub minx: f32,
    pub miny: f32,
    pub minz: f32,
    pub maxx: f32,
    pub maxy: f32,
    pub maxz: f32,
    pub left_first: i32,
    pub count: i32,
}
#[repr(C, align(32))]
#[derive(Pod, Zeroable, Copy, Clone)]
pub struct AABB {
    pub minx: f32,
    pub miny: f32,
    pub minz: f32,
    pub maxx: f32,
    pub maxy: f32,
    pub maxz: f32,
    _padding1: f32,
    _padding2: f32,
}
#[repr(C, align(64))]
#[derive(Pod, Zeroable, Copy, Clone)]
pub struct Ray {
    pub o: Point,
    pub d: Point,
    pub d_r: Point,
    pub t: f32,
    pub prim: u32,
    pub _padding1: u32,
    pub _padding2: u32,
}

pub struct BVH {
    pub vertices: Vec<Point>,
    pub triangles: Vec<[u32; 4]>,
    pub indices: Vec<u32>,
    pub bvh_nodes: Vec<BVHNode>,
    pub centroids: Vec<Point>,
}

impl Debug for AABB {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "({} {} {} {} {} {})",
            self.maxx, self.maxy, self.maxz, self.minx, self.miny, self.minz
        ))
    }
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

impl Add<f32> for Point {
    type Output = Point;

    fn add(self, other: f32) -> Point {
        Point {
            pos: [
                self.pos[0] + other,
                self.pos[1] + other,
                self.pos[2] + other,
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

impl Sub<f32> for Point {
    type Output = Point;

    fn sub(self, other: f32) -> Point {
        Point {
            pos: [
                self.pos[0] - other,
                self.pos[1] - other,
                self.pos[2] - other,
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
impl Div<Point> for f32 {
    type Output = Point;

    fn div(self, point: Point) -> Point {
        Point {
            pos: [
                self / point.pos[0],
                self / point.pos[1],
                self / point.pos[2],
                0f32,
            ],
        }
    }
}
impl Point {
    pub fn min(self, rhs: Point) -> Point {
        Point {
            pos: [
                f32::min(self.pos[0], rhs.pos[0]),
                f32::min(self.pos[1], rhs.pos[1]),
                f32::min(self.pos[2], rhs.pos[2]),
                0f32,
            ],
        }
    }
    pub fn max(self, rhs: Point) -> Point {
        Point {
            pos: [
                f32::max(self.pos[0], rhs.pos[0]),
                f32::max(self.pos[1], rhs.pos[1]),
                f32::max(self.pos[2], rhs.pos[2]),
                0f32,
            ],
        }
    }
}

pub fn dot(a: Point, b: Point) -> f32 {
    a.pos[0] * b.pos[0] + a.pos[1] * b.pos[1] + a.pos[2] * b.pos[2]
}

pub fn cross(a: Point, b: Point) -> Point {
    Point {
        pos: [
            a.pos[1] * b.pos[2] - a.pos[2] * b.pos[1],
            a.pos[2] * b.pos[0] - a.pos[0] * b.pos[2],
            a.pos[0] * b.pos[1] - a.pos[1] * b.pos[0],
            0f32,
        ],
    }
}

pub fn length(point: Point) -> f32 {
    (point.pos[0] * point.pos[0] + point.pos[1] * point.pos[1] + point.pos[2] * point.pos[2]).sqrt()
}

pub fn normalize(point: Point) -> Point {
    point / length(point)
}

pub fn distance(a: Point, b: Point) -> f32 {
    length(a - b)
}

impl Debug for BVHNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "{} {} {} {} {} {} {} {}",
            self.count,
            self.left_first,
            self.maxx,
            self.maxy,
            self.maxz,
            self.minx,
            self.miny,
            self.minz
        ))
    }
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
                match splits.len() {
                    4 => {
                        let p1 = splits[1].split("/").next().unwrap().parse::<u32>().unwrap() - 1;
                        let p2 = splits[2].split("/").next().unwrap().parse::<u32>().unwrap() - 1;
                        let p3 = splits[3].split("/").next().unwrap().parse::<u32>().unwrap() - 1;
                        triangles.push([p1, p2, p3, 0]);
                    }
                    5 => {
                        let p1 = splits[1].split("/").next().unwrap().parse::<u32>().unwrap() - 1;
                        let p2 = splits[2].split("/").next().unwrap().parse::<u32>().unwrap() - 1;
                        let p3 = splits[4].split("/").next().unwrap().parse::<u32>().unwrap() - 1;
                        triangles.push([p1, p2, p3, 0]);
                        let p1 = splits[2].split("/").next().unwrap().parse::<u32>().unwrap() - 1;
                        let p2 = splits[3].split("/").next().unwrap().parse::<u32>().unwrap() - 1;
                        let p3 = splits[4].split("/").next().unwrap().parse::<u32>().unwrap() - 1;
                        triangles.push([p1, p2, p3, 0]);
                    }
                    _ => panic!("unknown model format"),
                }
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

        let indices: Vec<u32> = triangles
            .iter()
            .enumerate()
            .map(|(i, _)| i as u32)
            .collect();

        let bvh_nodes = vec![BVHNode::zeroed(); triangles.len() * 2];

        BVH {
            vertices,
            triangles,
            indices,
            bvh_nodes,
            centroids: Default::default(),
        }
    }

    pub fn get_bvh_statistics(&self, node_width: u32) -> String {
        format!(
            "max depth: {}\ntotal_area: {}\ntotal_internal_nodes: {}\ntriangle_count: {}\nvertex_count: {}",
            self.get_max_depth(0, 0, node_width),
            self.get_total_area(0, node_width),
            self.total_internal_nodes(0, node_width),
            self.triangles.len(),
            self.vertices.len()
        )
    }

    pub fn get_max_depth(&self, node_id: u32, depth: u32, width: u32) -> u32 {
        if self.bvh_nodes[node_id as usize].count > 0
            || self.bvh_nodes[node_id as usize].left_first == 0
        {
            return depth;
        }
        let mut global_max = 0;
        let left = self.bvh_nodes[node_id as usize].left_first;
        for i in 0..width {
            global_max = max(
                global_max,
                self.get_max_depth(left as u32 + i, depth + 1, width),
            );
        }
        global_max
    }
    pub fn get_total_area(&self, node_id: u32, width: u32) -> f64 {
        let node = &self.bvh_nodes[node_id as usize];
        let mut area = Self::get_area(
            node.maxx, node.maxy, node.maxz, node.minx, node.miny, node.minz,
        ) as f64;
        if self.bvh_nodes[node_id as usize].count > 0
            || self.bvh_nodes[node_id as usize].left_first == 0
        {
            return area;
        }
        let left = self.bvh_nodes[node_id as usize].left_first;
        for i in 0..width {
            area += self.get_total_area(left as u32 + i, width);
        }

        area
    }
    pub fn total_internal_nodes(&self, node_id: u32, width: u32) -> u32 {
        if self.bvh_nodes[node_id as usize].count > 0
            || self.bvh_nodes[node_id as usize].left_first == 0
        {
            return 0;
        }
        let left = self.bvh_nodes[node_id as usize].left_first;
        let mut count = 1;
        for i in 0..width {
            count += self.total_internal_nodes(left as u32 + i, width);
        }

        count
    }

    pub fn build_bvh(&mut self) {
        self.centroids = self
            .triangles
            .iter()
            .map(|t| {
                (self.vertices[t[0] as usize]
                    + self.vertices[t[1] as usize]
                    + self.vertices[t[2] as usize])
                    / 3f32
            })
            .collect();

        self.bvh_nodes[0].left_first = 0;
        self.bvh_nodes[0].count = self.triangles.len() as i32;

        let aabb = self.calculate_bounds(0, self.triangles.len() as u32, false);
        self.set_bound(0, &aabb);

        let mut new_node_index = 2;

        self.subdivide(0, 0, &mut new_node_index, 0);
        println!("done building bvh");

        self.centroids = Vec::new();
        self.bvh_nodes.truncate(new_node_index as usize);
        self.bvh_nodes.shrink_to_fit();
    }

    fn print_tree(&self, index: u32, depth: u32) {
        println!(
            "{}{}: {} {} {} {} {} {} {} {}",
            "\t".repeat(depth as usize),
            index,
            self.bvh_nodes[index as usize].maxx,
            self.bvh_nodes[index as usize].maxy,
            self.bvh_nodes[index as usize].maxz,
            self.bvh_nodes[index as usize].minx,
            self.bvh_nodes[index as usize].miny,
            self.bvh_nodes[index as usize].minz,
            self.bvh_nodes[index as usize].count,
            self.bvh_nodes[index as usize].left_first,
        );
        if self.bvh_nodes[index as usize].count > 0 {
            return;
        }
        self.print_tree(self.bvh_nodes[index as usize].left_first as u32, depth + 1);
        self.print_tree(
            (self.bvh_nodes[index as usize].left_first + 1) as u32,
            depth + 1,
        );
    }

    //loop invariants:
    // current_bvh_index.count = the number of primitives which still have to be divided
    // current_bvh_index.aabb = correct
    // current_bvh_index.left_first = ???
    // start = start in indices buffer
    // pool_index = first free spot in bvh_nodes

    // count performance results
    // 2 = 0.1866s
    // 3 = 0.1857s
    // 4 = 0.187s
    // 5 = 0.1901s
    fn subdivide(
        &mut self,
        current_bvh_index: usize,
        start: u32,
        pool_index: &mut u32,
        depth: u32,
    ) {
        if self.bvh_nodes[current_bvh_index].count <= 3 {
            self.bvh_nodes[current_bvh_index].left_first = start as i32;
            return;
        }
        let index = *pool_index;
        *pool_index += 2;
        self.bvh_nodes[current_bvh_index].left_first = index as i32;

        let pivot = self.partition(start, self.bvh_nodes[current_bvh_index].count as u32);
        let left_count = pivot - start;
        self.bvh_nodes[index as usize].count = left_count as i32;
        let bounds = self.calculate_bounds(start, left_count, false);
        self.set_bound(index as usize, &bounds);

        let right_count = self.bvh_nodes[current_bvh_index].count - left_count as i32;
        self.bvh_nodes[index as usize + 1].count = right_count;
        let bounds = self.calculate_bounds(pivot, right_count as u32, false);
        self.set_bound(index as usize + 1, &bounds);

        self.subdivide(index as usize, start, pool_index, depth + 1);
        self.subdivide(index as usize + 1, pivot, pool_index, depth + 1);
        self.bvh_nodes[current_bvh_index].count = 0;
    }

    fn set_bound(&mut self, bvh_index: usize, aabb: &AABB) {
        self.bvh_nodes[bvh_index].maxx = aabb.maxx;
        self.bvh_nodes[bvh_index].maxy = aabb.maxy;
        self.bvh_nodes[bvh_index].maxz = aabb.maxz;
        self.bvh_nodes[bvh_index].minx = aabb.minx;
        self.bvh_nodes[bvh_index].miny = aabb.miny;
        self.bvh_nodes[bvh_index].minz = aabb.minz;
    }

    fn partition(&mut self, start: u32, count: u32) -> u32 {
        let bins = 8;
        let mut optimal_axis = 0;
        let mut optimal_pos = 0f32;
        let mut optimal_pivot = 0;
        let mut optimal_cost = f32::MAX;

        let aabb = self.calculate_bounds(start, count, true);

        for axis in 0..3 {
            for b in 1..bins {
                let pos = match axis {
                    0 => Self::lerp(aabb.minx, aabb.maxx, (b as f32) / (bins as f32)),
                    1 => Self::lerp(aabb.miny, aabb.maxy, (b as f32) / (bins as f32)),
                    2 => Self::lerp(aabb.minz, aabb.maxz, (b as f32) / (bins as f32)),
                    _ => panic!("error when partitioning"),
                };
                let pivot = self.partition_shuffle(axis, pos, start, count);

                let bb1_count = pivot - start;
                let bb2_count = count - bb1_count;

                let bb1 = self.calculate_bounds(start, bb1_count, false);
                let bb2 = self.calculate_bounds(pivot, bb2_count, false);

                let half_area1 =
                    Self::get_area(bb1.maxx, bb1.maxy, bb1.maxz, bb1.minx, bb1.miny, bb1.minz);
                let half_area2 =
                    Self::get_area(bb2.maxx, bb2.maxy, bb2.maxz, bb2.minx, bb2.miny, bb2.minz);

                let cost = half_area1 * bb1_count as f32 + half_area2 * bb2_count as f32;
                if cost < optimal_cost {
                    optimal_axis = axis;
                    optimal_pos = pos;
                    optimal_cost = cost;
                    optimal_pivot = pivot;
                }
            }
        }
        self.partition_shuffle(optimal_axis, optimal_pos, start, count);
        optimal_pivot
    }

    fn get_area(maxx: f32, maxy: f32, maxz: f32, minx: f32, miny: f32, minz: f32) -> f32 {
        ((maxx - minx) * (maxy - miny)
            + (maxx - minx) * (maxz - minz)
            + (maxy - miny) * (maxz - minz))
            * 2f32
    }

    fn partition_shuffle(&mut self, axis: usize, pos: f32, start: u32, count: u32) -> u32 {
        let mut end = (start + count - 1) as i32;
        let mut i = start as i32;

        while i < end {
            //println!("{} {}", i, end);
            if self.centroids[self.indices[i as usize] as usize].pos[axis] < pos {
                i += 1;
            } else {
                self.indices.swap(i as usize, end as usize);
                end -= 1;
            }
        }

        i as u32
    }

    // return min and max point
    fn calculate_bounds(&self, first: u32, amount: u32, centroids: bool) -> AABB {
        let mut max_point = Point {
            pos: [-100000000f32, -100000000f32, -100000000f32, 0f32],
        };
        let mut min_point = Point {
            pos: [100000000f32, 100000000f32, 100000000f32, 0f32],
        };
        for i in first..(first + amount) {
            let i = i as usize;
            if centroids {
                let vertex = self.centroids[self.indices[i] as usize];
                max_point = Point::max(max_point, vertex);
                min_point = Point::min(min_point, vertex);
            } else {
                for j in 0..3 as usize {
                    let vertex =
                        self.vertices[self.triangles[self.indices[i] as usize][j] as usize];
                    max_point = Point::max(max_point, vertex);
                    min_point = Point::min(min_point, vertex);
                }
            }
        }
        AABB {
            maxx: max_point.pos[0],
            maxy: max_point.pos[1],
            maxz: max_point.pos[2],
            minx: min_point.pos[0],
            miny: min_point.pos[1],
            minz: min_point.pos[2],
            _padding1: 0f32,
            _padding2: 0f32,
        }
    }

    fn lerp(a: f32, b: f32, p: f32) -> f32 {
        a + (b - a) * p
    }
    pub fn intersects_triangle(&self, ray: &mut Ray, triangle_index: u32) {
        let a = &self.vertices[self.triangles[triangle_index as usize][0] as usize];
        let b = &self.vertices[self.triangles[triangle_index as usize][1] as usize];
        let c = &self.vertices[self.triangles[triangle_index as usize][2] as usize];
        let a_to_b = *b - *a;
        let a_to_c = *c - *a;

        // Begin calculating determinant - also used to calculate u parameter
        // u_vec lies in view plane
        // length of a_to_c in view_plane = |u_vec| = |a_to_c|*sin(a_to_c, dir)
        let u_vec = cross(ray.d, a_to_c);

        // If determinant is near zero, ray lies in plane of triangle
        // The determinant corresponds to the parallelepiped volume:
        // det = 0 => [dir, a_to_b, a_to_c] not linearly independant
        let det = dot(a_to_b, u_vec);

        // Only testing positive bound, thus enabling backface culling
        // If backface culling is not desired write:
        // det < 0.0001 && det > -0.0001
        if det < f32::EPSILON && det > -f32::EPSILON {
            //return;
        }

        let inv_det = 1.0 / det;

        // Vector from point a to ray origin
        let a_to_origin = ray.o - *a;

        // Calculate u parameter
        let u = dot(a_to_origin, u_vec) * inv_det;

        // Test bounds: u < 0 || u > 1 => outside of triangle
        if u < 0f32 || u > 1f32 {
            return;
        }

        // Prepare to test v parameter
        let v_vec = cross(a_to_origin, a_to_b);

        // Calculate v parameter and test bound
        let v = dot(ray.d, v_vec) * inv_det;
        // The intersection lies outside of the triangle
        if v < 0.0 || u + v > 1.0 {
            return;
        }

        let dist = dot(a_to_c, v_vec) * inv_det;

        if dist > 0.0000001 && dist < ray.t {
            ray.t = dist;
            ray.prim = triangle_index;
        }
    }
    // returns nea/far
    pub fn intersect_aabb(&self, ray: &mut Ray, bvh_node: u32) -> f32 {
        let bvh_node = &self.bvh_nodes[bvh_node as usize];
        let vMax = Point {
            pos: [bvh_node.maxx, bvh_node.maxy, bvh_node.maxz, 0f32],
        };
        let vMin = Point {
            pos: [bvh_node.minx, bvh_node.miny, bvh_node.minz, 0f32],
        };
        let tMin = (vMin - ray.o) * ray.d_r;
        let tMax = (vMax - ray.o) * ray.d_r;
        let t1 = Point::min(tMin, tMax);
        let t2 = Point::max(tMin, tMax);
        let tNear = f32::max(f32::max(t1.pos[0], t1.pos[1]), t1.pos[2]);
        let tFar = f32::min(f32::min(t2.pos[0], t2.pos[1]), t2.pos[2]);
        if tFar >= tNear && tNear < ray.t && tFar > 0f32 {
            tNear
        } else {
            f32::MAX
        }
    }

    pub fn intersect(&self, ray: &mut Ray) {
        for triangle_id in 0..self.triangles.len() {
            self.intersects_triangle(ray, triangle_id as u32);
        }
    }

    pub fn triangle_normal(&self, triangle_index: u32) -> Point {
        let triangle = self.triangles[triangle_index as usize];
        let p1 = self.vertices[triangle[1] as usize] - self.vertices[triangle[0] as usize];
        let p2 = self.vertices[triangle[1] as usize] - self.vertices[triangle[2] as usize];
        normalize(cross(normalize(p1), normalize(p2)))
    }
    pub fn fast_intersect(&self, ray: &mut Ray) {
        let mut stack = [(0usize, 0f32); 32];
        let mut node_index = 0;
        let mut stack_ptr = 0;

        let mut loop_counter = 0;

        'outer: loop {
            loop_counter += 1;
            //println!("{:?}", self.bvh_nodes[node_index]);
            //println!("{} {} {:?}", node_index, stack_ptr, stack);
            if self.bvh_nodes[node_index].count > 0 {
                for i in 0..self.bvh_nodes[node_index].count {
                    self.intersects_triangle(
                        ray,
                        (self.indices[(self.bvh_nodes[node_index].left_first + i) as usize]) as u32,
                    )
                }
                if stack_ptr == 0 {
                    break;
                } else {
                    let mut t = f32::MAX;
                    while t >= ray.t {
                        if stack_ptr == 0 {
                            break 'outer;
                        }
                        stack_ptr -= 1;
                        (node_index, t) = stack[stack_ptr];
                    }
                    continue;
                }
            }
            let mut child1 = self.bvh_nodes[node_index].left_first as u32;
            let mut child2 = self.bvh_nodes[node_index].left_first as u32 + 1;

            let mut dist1 = self.intersect_aabb(ray, child1);
            let mut dist2 = self.intersect_aabb(ray, child2);
            if dist1 > dist2 {
                std::mem::swap(&mut dist1, &mut dist2);
                std::mem::swap(&mut child1, &mut child2);
            }
            if dist1 == f32::MAX {
                if stack_ptr == 0 {
                    break;
                } else {
                    let mut t = f32::MAX;
                    while t >= ray.t {
                        if stack_ptr == 0 {
                            break 'outer;
                        }
                        stack_ptr -= 1;
                        (node_index, t) = stack[stack_ptr];
                    }
                    //println!("stack_ptr -= 1");
                }
            } else {
                node_index = child1 as usize;
                if dist2 != f32::MAX {
                    stack[stack_ptr] = (child2 as usize, dist2);
                    stack_ptr += 1;
                    //println!("stack_ptr += 1");
                }
            }
        }
        //ray.t = loop_counter as f32;
    }
}
