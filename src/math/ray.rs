use core::f32;
use std::{f64::consts::E, simd::{cmp::{SimdPartialEq, SimdPartialOrd}, f32x4, num::{SimdFloat, SimdInt}}};

use crate::{hittable::{ConstantMedium, HitRecord, Hittable, HittableKind}, material::{self, Material}, rng::Seed, texture::Texture, utils::Stack};

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


pub enum Switch<'a> {
    RayMove((Vec3, bool)),
    RayRotateY {
        original_ray: Ray,
        hit_anything_prev: bool,
        sin: f32,
        cos: f32,
    },
    ConstantMediumPhase1 {
        hit_anything_prev: bool,
        original_rec: HitRecord<'a>,
        original_t: Interval,
        medium: &'a ConstantMedium<'a>,

    },
    ConstantMediumPhase2 {
        hit_anything_prev: bool,
        original_rec: HitRecord<'a>,
        original_t: Interval,
        rec_1: HitRecord<'a>,
        medium: &'a ConstantMedium<'a>,
    },
    Hittable(&'a Hittable<'a>),
}


impl Ray {
    #[inline(always)]
    pub fn new(origin: Point, direction: Vec3, time: f32) -> Self {
        Self { origin, direction, time, /*inv_direction: Vec3::new(1.0 / direction[0], 1.0 / direction[1], 1.0 / direction[2])*/ }
    }

    #[inline(always)]
    pub fn at(&self, t: f32) -> Point { self.origin + t*self.direction }


    pub fn hit_anything<'a>(&self, seed: &mut Seed, rec: &mut HitRecord<'a>,
                        world: &'a Hittable<'a>, hittable_stack: &mut Stack<Switch<'a>>) -> bool {

        let mut hit_anything = false;
        let mut tmin = 0.001;
        let mut tmax = f32::INFINITY;
        let mut ray = self.clone();

        hittable_stack.push(Switch::Hittable(world));

        while let Some(hittable) = hittable_stack.pop() {
            let hittable = match hittable {
                Switch::RayMove((vec3, hit)) => {
                    ray.origin = ray.origin + vec3;
                    if hit_anything {
                        rec.point += vec3;
                    }

                    hit_anything = hit_anything || hit;

                    continue;
                },

                Switch::RayRotateY { original_ray, hit_anything_prev, sin, cos } => {
                    ray = original_ray;

                    if hit_anything {
                        rec.point = Point::new(
                            (cos * rec.point[0]) + (sin * rec.point[2]),
                            rec.point[1],
                            (-sin * rec.point[0]) + (cos * rec.point[2]),
                        );

                        rec.normal = Vec3::new(
                            (cos * rec.normal[0]) + (sin * rec.normal[2]),
                            rec.normal[1],
                            (-sin * rec.normal[0]) + (cos * rec.normal[2]),
                        );
                    }

                    hit_anything = hit_anything || hit_anything_prev;
                    continue;
                },


                Switch::ConstantMediumPhase1 { original_rec, original_t, medium, hit_anything_prev } => {
                    if !hit_anything {
                        hit_anything = hit_anything_prev;
                        *rec = original_rec;
                        tmin = original_t.min;
                        tmax = original_t.max;
                        continue;
                    }

                    hittable_stack.push(Switch::ConstantMediumPhase2 {
                        original_rec, original_t, rec_1: rec.clone(), medium,
                        hit_anything_prev, });
                    hittable_stack.push(Switch::Hittable(medium.boundary));

                    tmin = 0.0001 + rec.t;
                    tmax = Interval::UNIVERSE.max;
                    *rec = HitRecord::default();
                    hit_anything = false;
                    continue;
                },


                Switch::ConstantMediumPhase2 { original_rec, original_t, mut rec_1, medium, hit_anything_prev } => {
                    let mut rec_2 = core::mem::replace(rec, original_rec);
                    tmin = original_t.min;
                    tmax = original_t.max;
                    if !hit_anything {
                        hit_anything = hit_anything_prev;
                        continue;
                    }
                    hit_anything = hit_anything_prev;

                    if rec_1.t < tmin { rec_1.t = tmin }
                    if rec_2.t > tmax { rec_2.t = tmax }

                    if rec_1.t >= rec_2.t { continue }
                    rec_1.t = rec_1.t.max(0.0);

                    let ray_len = ray.direction.length();
                    let distance_inside_bounds = (rec_2.t - rec_1.t) * ray_len;
                    let hit_distance = medium.neg_inv_density * seed.next_f32().log(E as f32); 

                    if hit_distance > distance_inside_bounds { continue }

                    rec.t = rec_1.t + hit_distance/ray_len;
                    tmax = rec.t;
                    rec.point = ray.at(rec.t);

                    rec.normal = Vec3::new(1.0, 0.0, 0.0); // arbitrary
                    rec.front_face = true; // also arbitrary
                    rec.material = medium.phase_function;
                    hit_anything = true;

                    continue;
                },


                Switch::Hittable(v) => v,


            };

            let t = Interval::new(tmin, tmax);

            let hit = match &hittable.kind {
                HittableKind::Sphere(sphere) => sphere.hit(&ray, t, rec),
                HittableKind::Quad(quad) => quad.hit(&ray, t, rec),
                HittableKind::MovingSphere(moving_sphere) => moving_sphere.hit(&ray, t, rec),
                HittableKind::BVH { 
                    left,
                    right,
                    aabbs,
                } => {

                    let [(left_t, hit_left), (right_t, hit_right)] = aabbs.hit(&ray, t);

                    if let Some(right) = right {
                        match (hit_left, hit_right) {
                            (true, true) => {
                                if left_t.max <= right_t.max {
                                    hittable_stack.push(Switch::Hittable(right));
                                    hittable_stack.push(Switch::Hittable(left));
                                } else {
                                    hittable_stack.push(Switch::Hittable(left));
                                    hittable_stack.push(Switch::Hittable(right));
                                }
                            }

                            (true, false) => hittable_stack.push(Switch::Hittable(left)),
                            (false, true) => hittable_stack.push(Switch::Hittable(right)),
                            (false, false) => (),
                        }
                    } else if hit_left {
                        hittable_stack.push(Switch::Hittable(left));
                    };

                    continue;
                },


                HittableKind::Move { obj, offset } => {
                    ray.origin = ray.origin - *offset;
                    hittable_stack.push(Switch::RayMove((*offset, hit_anything)));
                    hittable_stack.push(Switch::Hittable(obj));

                    hit_anything = false;
                    continue;
                },

                HittableKind::RotateY { obj, sin, cos } => {
                    let origin = Point::new(
                        (cos * ray.origin[0]) - (sin * ray.origin[2]),
                        ray.origin[1],
                        (sin * ray.origin[0]) + (cos * ray.origin[2]),
                    );


                    let direction = Point::new(
                        (cos * ray.direction[0]) - (sin * ray.direction[2]),
                        ray.direction[1],
                        (sin * ray.direction[0]) + (cos * ray.direction[2]),
                    );

                    let original_ray = ray.clone();

                    ray.origin = origin;
                    ray.direction = direction;

                    hittable_stack.push(Switch::RayRotateY {
                        original_ray, hit_anything_prev: hit_anything,
                        sin: *sin, cos: *cos,
                    });

                    hittable_stack.push(Switch::Hittable(obj));

                    continue;


                },

                HittableKind::List(hittables) => {
                    for h in hittables.iter() { hittable_stack.push(Switch::Hittable(h)) };
                    continue
                },


                HittableKind::ConstantMedium(constant_medium) => {
                    hittable_stack.push(Switch::ConstantMediumPhase1 {
                        original_rec: rec.clone(),
                        original_t: t,
                        medium: constant_medium,
                        hit_anything_prev: hit_anything,
                    });
                    hittable_stack.push(Switch::Hittable(constant_medium.boundary));

                    *rec = HitRecord::default();
                    hit_anything = false;
                    tmin = Interval::UNIVERSE.min;
                    tmax = Interval::UNIVERSE.max;
                    continue
                },
            };

            if !hit { continue }

            hit_anything = true;
            tmax = rec.t;
        }

        hit_anything
    }
}

