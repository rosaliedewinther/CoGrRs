use bytemuck::{Pod, Zeroable};
use core::panic;
use std::fmt::Debug;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    ops::{Add, Div, Mul, Sub},
};

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
pub struct Aabb {
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

pub struct Bvh {
    pub triangles: Vec<[Point; 4]>,
    pub indices: Vec<u32>,
    pub bvh_nodes: Vec<BVHNode>,
    pub centroids: Vec<Point>,
}

impl Debug for Aabb {
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

impl Bvh {
    pub fn new(filename: &str) -> Bvh {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();

        let file = File::open(filename).unwrap();
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line.unwrap();
            let splits: Vec<&str> = line.split(' ').collect();
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
                        let p1 = splits[1].split('/').next().unwrap().parse::<u32>().unwrap() - 1;
                        let p2 = splits[2].split('/').next().unwrap().parse::<u32>().unwrap() - 1;
                        let p3 = splits[3].split('/').next().unwrap().parse::<u32>().unwrap() - 1;
                        triangles.push([p1, p2, p3, 0]);
                    }
                    5 => {
                        let p1 = splits[1].split('/').next().unwrap().parse::<u32>().unwrap() - 1;
                        let p2 = splits[2].split('/').next().unwrap().parse::<u32>().unwrap() - 1;
                        let p3 = splits[4].split('/').next().unwrap().parse::<u32>().unwrap() - 1;
                        triangles.push([p1, p2, p3, 0]);
                        let p1 = splits[2].split('/').next().unwrap().parse::<u32>().unwrap() - 1;
                        let p2 = splits[3].split('/').next().unwrap().parse::<u32>().unwrap() - 1;
                        let p3 = splits[4].split('/').next().unwrap().parse::<u32>().unwrap() - 1;
                        triangles.push([p1, p2, p3, 0]);
                    }
                    _ => panic!("unknown model format"),
                }
            }
        }

        let indices: Vec<u32> = triangles
            .iter()
            .enumerate()
            .map(|(i, _)| i as u32)
            .collect();

        let triangles: Vec<[Point; 4]> = triangles
            .iter()
            .map(|tri| {
                [
                    vertices[tri[0] as usize],
                    vertices[tri[1] as usize],
                    vertices[tri[2] as usize],
                    Point::zeroed(),
                ]
            })
            .collect();

        let bvh_nodes = vec![BVHNode::zeroed(); triangles.len() * 2];

        Bvh {
            triangles,
            indices,
            bvh_nodes,
            centroids: Default::default(),
        }
    }

    pub fn build_bvh(&mut self) {
        self.centroids = self
            .triangles
            .iter()
            .map(|t| (t[0] + t[1] + t[2]) / 3f32)
            .collect();

        self.bvh_nodes[0].left_first = 0;
        self.bvh_nodes[0].count = self.triangles.len() as i32;

        let aabb = self.calculate_bounds(0, self.triangles.len() as u32, false);
        self.set_bound(0, &aabb);

        let mut new_node_index = 2;

        self.subdivide(0, 0, &mut new_node_index);

        self.centroids = Vec::new();
        self.bvh_nodes.truncate(new_node_index as usize);
        self.bvh_nodes.shrink_to_fit();

        self.triangles = self
            .indices
            .iter()
            .map(|index| self.triangles[*index as usize])
            .collect();
    }

    fn subdivide(&mut self, current_bvh_index: usize, start: u32, pool_index: &mut u32) {
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

        self.subdivide(index as usize, start, pool_index);
        self.subdivide(index as usize + 1, pivot, pool_index);
        self.bvh_nodes[current_bvh_index].count = 0;
    }

    fn set_bound(&mut self, bvh_index: usize, aabb: &Aabb) {
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
    fn calculate_bounds(&self, first: u32, amount: u32, centroids: bool) -> Aabb {
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
                for j in 0..3_usize {
                    let vertex = self.triangles[self.indices[i] as usize][j];
                    max_point = Point::max(max_point, vertex);
                    min_point = Point::min(min_point, vertex);
                }
            }
        }
        Aabb {
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
}
