#![feature(portable_simd)]
#![feature(sync_unsafe_cell)]

use hittable::Hittable;
use material::MaterialMap;

pub mod camera;
pub mod math;
pub mod rng;
pub mod hittable;
pub mod material;
pub mod texture;
pub mod perlin_noise;
pub mod utils;


pub struct World<'a> {
    entry: &'a Hittable<'a>,
    material_map: MaterialMap<'a>,
}

impl<'a> World<'a> {
    pub fn new(entry: &'a Hittable<'a>, material_map: MaterialMap<'a>) -> Self {
        Self { entry, material_map }
    }
}
