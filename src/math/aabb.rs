use std::{mem::transmute, ops::MulAssign, simd::{cmp::SimdPartialOrd, f32x16, f32x4, f32x8, num::SimdFloat, u32x4, u8x4, Mask}};

use super::{interval::Interval, ray::Ray, vec3::{Point, Vec3}};

#[derive(Clone, PartialEq, Debug)]
pub struct AABB {
    pub mins: f32x4,
    pub maxs: f32x4,
}


#[derive(Clone)]
pub struct AABBx2 {
    mins: f32x8,
    maxs: f32x8,
}


#[derive(Clone)]
pub struct AABBx4 {
    mins1: f32x8,
    mins2: f32x8,
    maxs1: f32x8,
    maxs2: f32x8,
}


impl AABB {
    pub fn empty() -> Self { Self::new(Interval::EMPTY, Interval::EMPTY, Interval::EMPTY) }

    pub fn new(x: Interval, y: Interval, z: Interval) -> Self {
        let mut slf = Self {
            mins: f32x4::from_array([x.min, y.min, z.min, 1.0]),
            maxs: f32x4::from_array([x.max, y.max, z.max, 1.0]),
        };

        slf.pad_to_minimums();
        slf
    }


    #[inline(always)]
    pub fn hit(&self, ray: &Ray, inv_dir: f32x4, ray_t: &mut Interval) -> bool {
        let ray_origin = ray.origin;
        let ray_origin = f32x4::from_array([ray_origin[0], ray_origin[1], ray_origin[2], 0.0]);
        let ray_idir = f32x4::from_array([inv_dir[0], inv_dir[1], inv_dir[2], ray_t.min]);
        let ray_idir2 = f32x4::from_array([inv_dir[0], inv_dir[1], inv_dir[2], ray_t.max]);

        // self.mins & self.maxs's 4th element is 1 so we can multiply by ray_idir(2)
        // to set the 4th element as ray_t.min & ray_t.max respectively
        let t1 = (self.mins - ray_origin) * ray_idir;
        let t2 = (self.maxs - ray_origin) * ray_idir2;

        ray_t.min = t1.simd_min(t2).reduce_max();
        ray_t.max = t1.simd_max(t2).reduce_min();

        ray_t.min <= ray_t.max
    }


    pub fn from_points(a: Point, b: Point) -> Self {
        let x = if a[0] <= b[0] { Interval::new(a[0], b[0]) } else { Interval::new(b[0], a[0]) };
        let y = if a[1] <= b[1] { Interval::new(a[1], b[1]) } else { Interval::new(b[1], a[1]) };
        let z = if a[2] <= b[2] { Interval::new(a[2], b[2]) } else { Interval::new(b[2], a[2]) };

        Self::new(x, y, z)
    }


    pub fn from_aabbs(box1: &AABB, box2: &AABB) -> AABB {
        Self::new(
            Interval::from_intervals(box1.x(), box2.x()),
            Interval::from_intervals(box1.y(), box2.y()),
            Interval::from_intervals(box1.z(), box2.z()),
        )

    }



    pub fn longest_axis(&self) -> usize {
        if self.x().size() > self.y().size() { if self.x().size() > self.z().size() { 0 } else { 2 } }
        else { if self.y().size() > self.z().size() { 1 } else { 2 } }
    }


    pub fn pad_to_minimums(&mut self) {
        let delta = 0.0001;
        let delta_half = delta * 0.5;

        if self.x().size() < delta {
            self.mins[0] = self.mins[0] - delta_half;
            self.maxs[0] = self.maxs[0] + delta_half;
        }
        
        if self.y().size() < delta {
            self.mins[1] = self.mins[1] - delta_half;
            self.maxs[1] = self.maxs[1] + delta_half;
        }

        if self.z().size() < delta {
            self.mins[2] = self.mins[2] - delta_half;
            self.maxs[2] = self.maxs[2] + delta_half;
        }

    }

    pub fn x(&self) -> Interval { self.axis_interval(0) }
    pub fn y(&self) -> Interval { self.axis_interval(1) }
    pub fn z(&self) -> Interval { self.axis_interval(2) }
    pub fn pos(&self) -> Vec3 { Vec3::new(self.mins[0], self.mins[1], self.mins[2]) }

    pub fn axis_interval(&self, axis: usize) -> Interval {
        Interval::new(self.mins[axis], self.maxs[axis])
    }

    pub fn offset(&self, offset: Vec3) -> AABB {
        AABB { mins: self.mins + offset.axes, maxs: self.maxs + offset.axes }
    }
}


impl AABBx2 {
    pub fn new(aabb1: AABB, aabb2: AABB) -> Self {
        Self {
            mins: f32x8::from_array([aabb1.mins[0], aabb1.mins[1], aabb1.mins[2], aabb1.mins[3],
                                     aabb2.mins[0], aabb2.mins[1], aabb2.mins[2], aabb2.mins[3]]),
            maxs: f32x8::from_array([aabb1.maxs[0], aabb1.maxs[1], aabb1.maxs[2], aabb1.maxs[3],
                                     aabb2.maxs[0], aabb2.maxs[1], aabb2.maxs[2], aabb2.maxs[3]]),
        }
    }


    #[inline(always)]
    pub fn hit(&self, ray: &Ray, ray_t: Interval) -> [(Interval, bool); 2] {
        let inv_dir = f32x4::splat(1.0) / ray.direction.axes; 
        let ray_origin = f32x8::from_array([ray.origin[0], ray.origin[1], ray.origin[2], 0.0,
                                            ray.origin[0], ray.origin[1], ray.origin[2], 0.0]);
        let ray_idir = f32x8::from_array([inv_dir[0], inv_dir[1], inv_dir[2], ray_t.min,
                                            inv_dir[0], inv_dir[1], inv_dir[2], ray_t.min]);
        let ray_idir2 = f32x8::from_array([inv_dir[0], inv_dir[1], inv_dir[2], ray_t.max,
                                            inv_dir[0], inv_dir[1], inv_dir[2], ray_t.max]);

        // self.mins & self.maxs's 4th element is 1 so we can multiply by ray_idir(2)
        // to set the 4th element as ray_t.min & ray_t.max respectively
        let t1 = (self.mins - ray_origin) * ray_idir;
        let t2 = (self.maxs - ray_origin) * ray_idir2;

        let t1 = unsafe { core::mem::transmute::<f32x8, [f32x4; 2]>(t1) };
        let t2 = unsafe { core::mem::transmute::<f32x8, [f32x4; 2]>(t2) };

        let left = {
            let t1 = t1[0];
            let t2 = t2[0];

            let min = t1.simd_min(t2).reduce_max();
            let max = t1.simd_max(t2).reduce_min();

            (Interval::new(min, max), min <= max)
        };

        let right = {
            let t1 = t1[1];
            let t2 = t2[1];

            let min = t1.simd_min(t2).reduce_max();
            let max = t1.simd_max(t2).reduce_min();

            (Interval::new(min, max), min <= max)
        };

        [left, right]
    }


    pub fn aabb1(&self) -> AABB {
        AABB::new(
            Interval::new(self.mins[0], self.maxs[0]),
            Interval::new(self.mins[1], self.maxs[1]),
            Interval::new(self.mins[2], self.maxs[2]),
        )
    }


    pub fn aabb2(&self) -> AABB {
        AABB::new(
            Interval::new(self.mins[4], self.maxs[4]),
            Interval::new(self.mins[5], self.maxs[5]),
            Interval::new(self.mins[6], self.maxs[6]),
        )
    }
}

