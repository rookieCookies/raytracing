#![feature(portable_simd)]
#![feature(sync_unsafe_cell)]

pub mod camera;
pub mod math;
pub mod rng;
pub mod hittable;
pub mod material;
pub mod texture;
pub mod perlin_noise;
pub mod utils;


struct World {
    image_dimensions: (usize, usize),
    stage_1_buffer: Vec<Stage1>, // len == image.x * image.y
}


struct Stage1 {
}
