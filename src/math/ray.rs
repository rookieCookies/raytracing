use core::f32;
use std::simd::{cmp::{SimdPartialEq, SimdPartialOrd}, f32x4, num::{SimdFloat, SimdInt}};

use crate::{hittable::{HitRecord, Hittable, HittableKind}, rng::Seed, utils::Stack};

use super::{vec3::{Point, Vec3, Colour}, interval::Interval};

#[derive(Clone)]
pub struct Ray {
    pub origin: Point,
    pub direction: Vec3,
    pub inv_direction: Vec3,
    pub time: f32,
}

struct Frame {
    ray: Ray,
    depth: usize,
    multiplier: Vec3,
}


impl Ray {
    #[inline(always)]
    pub fn new(origin: Point, direction: Vec3, time: f32) -> Self {
        Self { origin, direction, time, inv_direction: Vec3::new(1.0 / direction[0], 1.0 / direction[1], 1.0 / direction[2]) }
    }

    #[inline(always)]
    pub fn at(&self, t: f32) -> Point { self.origin + t*self.direction }


    #[inline(never)]
    pub fn colour<'a>(&self, seed: &mut Seed, world: &'a Hittable<'a>, hittable_stack: &mut Stack<&'a Hittable<'a>>, depth: usize) -> Colour {
        debug_assert!(hittable_stack.is_empty());
        let mut active_frame = Frame { ray: self.clone(), depth, multiplier: Vec3::ONE };
        let mut rec = HitRecord::default();

        loop {
            let Frame { ray, depth, multiplier } = active_frame;

            if depth == 0 { return Colour::ZERO }

            let mut hit_anything = false;
            let tmin = 0.001;
            let mut tmax = f32::INFINITY;

            hittable_stack.push(world);

            while let Some(hittable) = hittable_stack.pop() {

                let t = Interval::new(tmin, tmax);

                let hit = match &hittable.kind {
                    HittableKind::Sphere(sphere) => sphere.hit(&ray, t, &mut rec),
                    HittableKind::MovingSphere(moving_sphere) => moving_sphere.hit(&ray, t, &mut rec),
                    HittableKind::BVH { 
                        /*
                        aabbs,
                        #[cfg(feature="aabbx4")]
                        regions,
                        */
                        left,
                        right,
                        aabbs
                    } => {
                        
                        /*
                        {
                            let (mins, maxs, mask) = aabbs.hit(&ray, t);

                            let valid = mins.simd_le(maxs) & mask;
                            let mut max = f32x4::from_bits(maxs.to_bits() & valid.to_int().cast());
                            let n = valid.to_bitmask().count_ones();

                            for _ in 0..n {
                                let last = max.reduce_max();
                                let mask = max.simd_eq(f32x4::splat(last)).to_bitmask();
                                let idx = mask.trailing_zeros();
                                if let Some(r) = regions[idx as usize] {
                                    count.0 += 1;
                                    hittable_stack.push(r);
                                }
                                max[idx as usize] = 0.0;
                            }
                        }*/

                        let [(left_t, hit_left), (right_t, hit_right)] = aabbs.hit(&ray, t);

                        if let Some(right) = right {
                            match (hit_left, hit_right) {
                                (true, true) => {
                                    if left_t.max <= right_t.max {
                                        hittable_stack.push(right);
                                        hittable_stack.push(left);
                                    } else {
                                        hittable_stack.push(left);
                                        hittable_stack.push(right);
                                    }
                                }

                                (true, false) => hittable_stack.push(left),
                                (false, true) => hittable_stack.push(right),
                                (false, false) => (),
                            }
                        } else if hit_left {
                            hittable_stack.push(left);
                        };

                        continue;
                    },
                };

                if !hit { continue }

                hit_anything = true;
                tmax = rec.t;
            }

            if !hit_anything {
                let unit_dir = ray.direction.unit();
                let a = 0.5 * (unit_dir[1] + 1.0);
                let colour = (1.0 - a) * Colour::ONE + a * Colour::new(0.5, 0.7, 1.0);
                return multiplier * colour
            }


            let Some((scattered, attenuation)) = rec.material.scatter(seed, &ray, &rec)
            else { return Colour::ZERO };

            let frame = Frame {
                ray: scattered,
                depth: depth - 1,
                multiplier: multiplier * attenuation,
            };

            active_frame = frame;
        }
    }
}

