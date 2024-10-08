use image::Rgb32FImage;

use crate::{math::{interval::Interval, vec3::{Colour, Point}}, perlin_noise::PerlinNoise};

#[derive(Clone, Copy)]
pub enum Texture<'a> {
    SolidColour(Colour),


    Checkerboard {
        inv_scale: f32,
        even: &'a Texture<'a>,
        odd: &'a Texture<'a>,
    },


    Image {
        image: &'a Rgb32FImage,
    },

    
    NoiseTexture(PerlinNoise<'a>, f32),
}


impl<'a> Texture<'a> {
    pub fn value(&self, u: f32, v: f32, p: Point) -> Colour {
        match self {
            Texture::SolidColour(v) => *v,


            Texture::Checkerboard { inv_scale, even, odd } => {
                let x = (inv_scale * p.x).floor() as i32;
                let y = (inv_scale * p.y).floor() as i32;
                let z = (inv_scale * p.z).floor() as i32;

                let is_even = (x + y + z) % 2 == 0;

                if is_even { even } else { odd }.value(u, v, p)
            },


            Texture::Image { image  } => {
                // clamp input texture coordinates to 0..1 x 1..0
                let u = Interval::new(0.0, 1.0).clamp(u);
                let v = 1.0 - Interval::new(0.0, 1.0).clamp(v); // flip v to image coords

                let i = (u * (image.width()-1) as f32) as u32;
                let j = (v * (image.height()-1) as f32) as u32;
                let pixel = image.get_pixel(i, j);

                Colour::new(pixel[0].powi(2), pixel[1].powi(2), pixel[2].powi(2))
            },


            Texture::NoiseTexture(noise, scale) => {
                (1.0 + (scale * p.z + 10.0 * noise.turbulance(p, 7)).sin()) * Colour::new(0.5, 0.5, 0.5)
            },
        }
    }

}
