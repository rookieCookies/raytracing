use core::f32;
use std::simd::{cmp::{SimdPartialEq, SimdPartialOrd}, f32x4, num::{SimdFloat, SimdInt}};

use crate::{hittable::{HitRecord, Hittable, HittableKind}, rng::Seed, utils::Stack};

use super::{vec3::{Point, Vec3, Colour}, interval::Interval};

#[derive(Clone)]
pub struct Ray {
    pub origin: Point,
    pub direction: Vec3,
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
        Self { origin, direction, time, /*inv_direction: Vec3::new(1.0 / direction[0], 1.0 / direction[1], 1.0 / direction[2])*/ }
    }

    #[inline(always)]
    pub fn at(&self, t: f32) -> Point { self.origin + t*self.direction }


    pub fn hit_anything<'a>(&self, rec: &mut HitRecord<'a>,
                        world: &'a Hittable<'a>, hittable_stack: &mut Stack<&'a Hittable<'a>>) -> bool {
        let mut hit_anything = false;
        let tmin = 0.001;
        let mut tmax = f32::INFINITY;

        hittable_stack.push(world);

        while let Some(hittable) = hittable_stack.pop() {

            let t = Interval::new(tmin, tmax);

            let hit = match &hittable.kind {
                HittableKind::Sphere(sphere) => sphere.hit(self, t, rec),
                HittableKind::Quad(quad) => quad.hit(self, t, rec),
                HittableKind::MovingSphere(moving_sphere) => moving_sphere.hit(self, t, rec),
                HittableKind::BVH { 
                    left,
                    right,
                    aabbs
                } => {

                    let [(left_t, hit_left), (right_t, hit_right)] = aabbs.hit(self, t);

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

        hit_anything
    }
}

