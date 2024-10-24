use std::simd::{num::SimdFloat, StdFloat};

use image::Rgb32FImage;

use crate::{math::{interval::Interval, vec3::{Colour, Point}}, perlin_noise::PerlinNoise};

#[derive(Clone, Copy, Default)]
pub struct Texture<'a>(TextureKind<'a>);


#[derive(Clone, Copy, Default)]
pub enum TextureKind<'a> {
    SolidColour(Colour),


    Checkerboard {
        inv_scale: f32,
        even: &'a Texture<'a>,
        odd: &'a Texture<'a>,
    },


    Image {
        image: &'a Rgb32FImage,
    },

    
    NoiseTexture{ 
        noise: PerlinNoise<'a>,
        scale: f32,
    },


    #[default]
    NotFound,
}


impl<'a> Texture<'a> {
    pub fn value(&self, u: f32, v: f32, p: Point) -> Colour {
        match &self.0 {
            TextureKind::SolidColour(v) => *v,


            TextureKind::Checkerboard { inv_scale, even, odd } => {
                let xyz = (*inv_scale * p).axes.floor();

                let is_even = xyz.reduce_sum() as i32 % 2 == 0;

                if is_even { even } else { odd }.value(u, v, p)
            },


            TextureKind::Image { image  } => {
                // clamp input texture coordinates to 0..1 x 1..0
                let u = Interval::new(0.0, 1.0).clamp(u);
                let v = 1.0 - Interval::new(0.0, 1.0).clamp(v); // flip v to image coords

                let i = (u * (image.width()-1) as f32) as u32;
                let j = (v * (image.height()-1) as f32) as u32;
                let pixel = image.get_pixel(i, j);

                Colour::new(pixel[0].powi(2), pixel[1].powi(2), pixel[2].powi(2))
            },


            TextureKind::NoiseTexture { noise, scale } => {
                (1.0 + (scale * p[2] + 10.0 * noise.turbulance(p, 7)).sin()) * Colour::new(0.5, 0.5, 0.5)
            },

            TextureKind::NotFound => Colour::ZERO,
        }
    }


    fn new(kind: TextureKind) -> Texture {
        Texture(kind)
    }


    pub fn colour(colour: Colour) -> Self {
        Self::new(TextureKind::SolidColour(colour))
    }

    pub fn checkerboard(scale: f32, even: &'a Texture<'a>, odd: &'a Texture<'a>) -> Self {
        Self::new(TextureKind::Checkerboard { inv_scale: 1.0/scale, even, odd })
    }

    pub fn image(image: &'a Rgb32FImage) -> Self {
        Self::new(TextureKind::Image { image })
    }

    pub fn noise(noise: PerlinNoise<'a>, scale: f32) -> Self {
        Self::new(TextureKind::NoiseTexture { noise, scale })
    }




}
