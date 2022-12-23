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
    pub pos: [f32; 4],
}
#[repr(C, align(64))]
#[derive(Pod, Zeroable, Copy, Clone)]
pub struct Triangle {
    points: [Point; 4],
}
#[repr(C, align(32))]
#[derive(Pod, Zeroable, Copy, Clone, Debug)]
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
#[repr(C, align(32))]
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
                f32::min(self.pos[1], rhs.pos[1]),
                f32::min(self.pos[2], rhs.pos[2]),
                0f32,
            ],
        }
    }
    pub fn max(&self, rhs: &Point) -> Point {
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

    pub fn build_bvh(&mut self) {
        //let aabb = self.calculate_bounds(0, self.triangles.len() as u32);

        //let scale_factor = Point {
        //    pos: [
        //        1f32 / (aabb.maxx - aabb.minx),
        //        1f32 / (aabb.maxy - aabb.miny),
        //        1f32 / (aabb.maxz - aabb.minz),
        //        0f32,
        //    ],
        //};
        //let offset = Point {
        //    pos: [
        //        0f32 - (aabb.maxx + aabb.minx) * scale_factor.pos[0] / 2f32,
        //        0f32 - (aabb.maxy + aabb.miny) * scale_factor.pos[1] / 2f32,
        //        0f32 - (aabb.maxz + aabb.minz) * scale_factor.pos[2] / 2f32,
        //        0f32,
        //    ],
        //};
        //self.vertices
        //    .iter_mut()
        //    .for_each(|vertex| *vertex = (*vertex) * scale_factor + offset);

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

        let aabb = self.calculate_bounds(0, self.triangles.len() as u32);
        self.set_bound(0, &aabb);

        self.subdivide(0, 0, &mut 2);

        //self.print_tree(0, 0);
    }

    fn print_tree(&self, index: u32, depth: u32) {
        println!(
            "{}{}: {:?}",
            "\t".repeat(depth as usize),
            index,
            self.bvh_nodes[index as usize]
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
    fn subdivide(&mut self, current_bvh_index: usize, start: u32, pool_index: &mut u32) {
        if self.bvh_nodes[current_bvh_index].count <= 3 {
            self.bvh_nodes[current_bvh_index].left_first = start as i32;
            return;
        }
        let index = *pool_index;
        *pool_index += 2;
        self.bvh_nodes[current_bvh_index].left_first = index as i32;

        let pivot = self.partition(
            current_bvh_index,
            start,
            self.bvh_nodes[current_bvh_index].count as u32,
        );
        let left_count = pivot - start;
        self.bvh_nodes[index as usize].count = left_count as i32;
        let bounds = self.calculate_bounds(start, left_count);
        self.set_bound(index as usize, &bounds);

        let right_count = self.bvh_nodes[current_bvh_index].count - left_count as i32;
        self.bvh_nodes[index as usize + 1].count = right_count;
        let bounds = self.calculate_bounds(pivot, right_count as u32);
        self.set_bound(index as usize + 1, &bounds);

        self.subdivide(index as usize, start, pool_index);
        self.subdivide(index as usize + 1, pivot, pool_index);
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

    fn partition(&mut self, current_bvh_index: usize, start: u32, count: u32) -> u32 {
        let bins = 8;
        let mut optimal_axis = 0;
        let mut optimal_pos = 0f32;
        let mut optimal_cost = f32::MAX;
        for axis in 0..3 {
            for b in 1..bins {
                let pos = match axis {
                    0 => Self::lerp(
                        self.bvh_nodes[current_bvh_index].minx,
                        self.bvh_nodes[current_bvh_index].maxx,
                        (b as f32) / (bins as f32),
                    ),
                    1 => Self::lerp(
                        self.bvh_nodes[current_bvh_index].miny,
                        self.bvh_nodes[current_bvh_index].maxy,
                        (b as f32) / (bins as f32),
                    ),
                    2 => Self::lerp(
                        self.bvh_nodes[current_bvh_index].minz,
                        self.bvh_nodes[current_bvh_index].maxz,
                        (b as f32) / (bins as f32),
                    ),
                    _ => panic!("error when partitioning"),
                };
                let pivot = self.partition_shuffle(axis, pos, start, count);

                let bb1_count = pivot - start;
                let bb2_count = count - bb1_count;

                let bb1 = self.calculate_bounds(start, bb1_count);
                let bb2 = self.calculate_bounds(pivot, bb2_count);

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

        self.partition_shuffle(optimal_axis, optimal_pos, start, count)
    }

    fn partition_shuffle(&mut self, axis: usize, pos: f32, start: u32, count: u32) -> u32 {
        let mut end = (start + count - 1) as i32;
        let mut i = start as i32;

        while i <= end {
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
    fn calculate_bounds(&self, first: u32, amount: u32) -> AABB {
        let mut max_point = Point {
            pos: [f32::MIN, f32::MIN, f32::MIN, 0f32],
        };
        let mut min_point = Point {
            pos: [f32::MAX, f32::MAX, f32::MAX, 0f32],
        };
        for i in first..(first + amount) {
            let i = i as usize;
            for j in 0..3 as usize {
                let vertex = &self.vertices[self.triangles[self.indices[i] as usize][j] as usize];
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
    pub fn intersects_triangle(&self, ray: &mut Ray, triangle_index: u32) {
        let a = &self.vertices[self.triangles[triangle_index as usize][0] as usize];
        let b = &self.vertices[self.triangles[triangle_index as usize][1] as usize];
        let c = &self.vertices[self.triangles[triangle_index as usize][2] as usize];
        let a_to_b = *b - *a;
        let a_to_c = *c - *a;

        // Begin calculating determinant - also used to calculate u parameter
        // u_vec lies in view plane
        // length of a_to_c in view_plane = |u_vec| = |a_to_c|*sin(a_to_c, dir)
        let u_vec = cross(&ray.d, &a_to_c);

        // If determinant is near zero, ray lies in plane of triangle
        // The determinant corresponds to the parallelepiped volume:
        // det = 0 => [dir, a_to_b, a_to_c] not linearly independant
        let det = dot(&a_to_b, &u_vec);

        // Only testing positive bound, thus enabling backface culling
        // If backface culling is not desired write:
        // det < 0.0001 && det > -0.0001
        if det < 0.0001 && det > -0.0001 {
            return;
        }

        let inv_det = 1.0 / det;

        // Vector from point a to ray origin
        let a_to_origin = ray.o - *a;

        // Calculate u parameter
        let u = dot(&a_to_origin, &u_vec) * inv_det;

        // Test bounds: u < 0 || u > 1 => outside of triangle
        if u < 0f32 || u > 1f32 {
            return;
        }

        // Prepare to test v parameter
        let v_vec = cross(&a_to_origin, &a_to_b);

        // Calculate v parameter and test bound
        let v = dot(&ray.d, &v_vec) * inv_det;
        // The intersection lies outside of the triangle
        if v < 0.0 || u + v > 1.0 {
            return;
        }

        let dist = dot(&a_to_c, &v_vec) * inv_det;

        if dist > 0.0001 && dist < ray.t {
            ray.t = dist;
            ray.prim = triangle_index;
        }
    }
    // returns nea/far
    pub fn intersect_aabb(&self, ray: &mut Ray, bvh_node: u32) -> f32 {
        let bvh_node = &self.bvh_nodes[bvh_node as usize];
        let tx1 = (bvh_node.minx - ray.o.pos[0]) * ray.d_r.pos[0];
        let tx2 = (bvh_node.maxx - ray.o.pos[0]) * ray.d_r.pos[0];
        let mut tmin = f32::min(tx1, tx2);
        let mut tmax = f32::max(tx1, tx2);
        let ty1 = (bvh_node.miny - ray.o.pos[1]) * ray.d_r.pos[1];
        let ty2 = (bvh_node.maxy - ray.o.pos[1]) * ray.d_r.pos[1];
        tmin = f32::max(tmin, f32::min(ty1, ty2));
        tmax = f32::min(tmax, f32::max(ty1, ty2));
        let tz1 = (bvh_node.minz - ray.o.pos[2]) * ray.d_r.pos[2];
        let tz2 = (bvh_node.maxz - ray.o.pos[2]) * ray.d_r.pos[2];
        tmin = f32::max(tmin, f32::min(tz1, tz2));
        tmax = f32::min(tmax, f32::max(tz1, tz2));
        if tmax >= tmin && tmin < ray.t && tmax > 0f32 {
            tmin
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
        normalize(&cross(&normalize(&p1), &normalize(&p2)))
    }
    pub fn fast_intersect(&self, ray: &mut Ray) {
        let mut stack = [0; 128];
        let mut node_index = 0;
        let mut stack_ptr = 0;

        //let mut loop_counter = 0;

        loop {
            //loop_counter += 1;
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
                    stack_ptr -= 1;
                    node_index = stack[stack_ptr];
                    //println!("stack_ptr -= 1");
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
                    stack_ptr -= 1;
                    node_index = stack[stack_ptr];
                    //println!("stack_ptr -= 1");
                }
            } else {
                node_index = child1 as usize;
                if dist2 != f32::MAX {
                    stack[stack_ptr] = child2 as usize;
                    stack_ptr += 1;
                    //println!("stack_ptr += 1");
                }
            }
        }
        //ray.t = loop_counter as f32;
    }
}
