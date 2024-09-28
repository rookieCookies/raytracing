use crate::rt::hittable::{HitRecord, Hittable};

use super::{vec3::{Point, Vec3, Colour}, interval::Interval};

#[derive(Clone, Copy)]
pub struct Ray {
    pub origin: Point,
    pub direction: Vec3,
    pub time: f32,
}


impl Ray {
    #[inline(always)]
    pub fn new(origin: Point, direction: Vec3, time: f32) -> Self {
        Self { origin, direction, time }
    }

    #[inline(always)]
    pub fn at(self, t: f32) -> Point { self.origin + t*self.direction }


    #[inline(always)]
    pub fn colour(self, world: &Hittable, depth: usize) -> Colour {
        if depth == 0 { return Colour::new(0.0, 0.0, 0.0) }

        let mut rec = HitRecord::default();
        if world.hit(self, Interval::new(0.001, f32::INFINITY), &mut rec) {
            if let Some((scattered, attenuation)) = rec.material.scatter(self, &rec) {
                return attenuation * scattered.colour(world, depth - 1);
            }

            return Colour::new(0.0, 0.0, 0.0)
        }

        let unit_dir = self.direction.unit();
        let a = 0.5 * (unit_dir.y + 1.0);
        return (1.0 - a) * Colour::new(1.0, 1.0, 1.0) + a * Colour::new(0.5, 0.7, 1.0);
    }
}

