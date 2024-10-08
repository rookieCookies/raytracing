use std::{f32::consts::E, sync::atomic::AtomicUsize};

use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::{math::{ray::Ray, vec3::{Colour, Point, Vec3}}, rng::next_f32, utils::SendPtr, RENDER_RESOLUTION};

use super::hittable::Hittable;

#[derive(Clone)]
pub struct RaytracingCamera {
    pub image: (usize, usize),
    pub centre: Point,
    pub pixel00_loc: Vec3,
    pub pixel_delta_u: Vec3,
    pub pixel_delta_v: Vec3,
    pub max_depth: usize,
    pub defocus_angle: f32,
    pub defocus_disk_u: Vec3,
    pub defocus_disk_v: Vec3,
}

impl RaytracingCamera {
    pub fn new(aspect_ratio: f32, width: usize,
               max_depth: usize, vfov: f32, look_from: Vec3, look_at: Vec3,
               vup: Vec3, defocus_angle: f32, focus_dist: f32) -> Self {

        let height = {
            let val = (width as f32 / aspect_ratio) as usize;
            if val <= 0 { 1 } else { val }
        };

        let centre = look_from;

        // Determine viewport dimensions
        let theta = vfov.to_radians();
        let h = (theta/2.0).tan();
        let viewport_height = 2.0 * h * focus_dist;
        let viewport_width = viewport_height * (width as f32 / height as f32);

        let w = (look_from - look_at).unit();
        let u = vup.cross(w).unit();
        let v = w.cross(u);

        let viewport_u = viewport_width  * u;
        let viewport_v = viewport_height * -v;

        let pixel_delta_u = viewport_u / width as f32;
        let pixel_delta_v = viewport_v / height as f32;


        let viewport_upper_left = centre
                                    - (focus_dist * w)
                                    - viewport_u / 2.0
                                    - viewport_v / 2.0;
        let pixel00_loc = viewport_upper_left + 0.5 * (pixel_delta_u + pixel_delta_v);

        let defocus_radius = focus_dist * (defocus_angle / 2.0).to_radians().tan();
        let defocus_disk_u = defocus_radius * u;
        let defocus_disk_v = defocus_radius * v;

        Self {
            image: (width, height),
            centre,
            pixel00_loc,
            pixel_delta_u,
            pixel_delta_v,
            max_depth,
            defocus_angle,
            defocus_disk_u,
            defocus_disk_v,
        }
    }


    /// # Undefined Behaviour
    /// - If `colours.len()` != image.x * image.y
    pub unsafe fn render(&self, acc_colours: &mut [Colour], final_colours: &mut [u32], samples: usize, world: &Hittable) {
        debug_assert_eq!(acc_colours.len(), self.image.0 * self.image.1);
        debug_assert_eq!(final_colours.len(), self.image.0 * self.image.1);

        {
            let acc_ptr = SendPtr(acc_colours.as_mut_ptr());

            let final_ptr = SendPtr(final_colours.as_mut_ptr());

            let samples = 1.0 / samples as f32;
            // i have never cared less about UB as i have here
            (0..self.image.1).par_bridge()
                .for_each(move |y| {
                    let acc_ptr = acc_ptr;
                    let final_ptr = final_ptr;

                    let mut acc_ptr = unsafe { acc_ptr.0.offset((y*self.image.0) as isize) };
                    let mut final_ptr = unsafe { final_ptr.0.offset((y*self.image.0) as isize) };

                    for x in 0..self.image.0 {
                        let colour = self.colour_of(world, x, y);

                        unsafe { acc_ptr.write(acc_ptr.read() + colour) };
                        
                        {
                            let colour = samples * unsafe { acc_ptr.read() };
                            let r = (colour.x * 255.999) as u32;
                            let g = (colour.y * 255.999) as u32;
                            let b = (colour.z * 255.999) as u32;

                            let val = (r << 16) | (g << 8) | (b);
                            unsafe { final_ptr.write(val) };

                        }

                        //unsafe { final_ptr.write(acc_ptr.read() / samples as f32) };
                        acc_ptr = unsafe { acc_ptr.add(1) };
                        final_ptr = unsafe { final_ptr.add(1) };
                    }

                    //println!("{}/{}, sample: {}", count.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RENDER_RESOLUTION, samples);
                });
        }
    }

    
    fn colour_of(&self, world: &Hittable, x: usize, y: usize) -> Colour {
        // calculate the colour
        let ray = self.get_ray(x, y);
        let mut colour = ray.colour(&world, self.max_depth);
        
        // Linear -> Gamma
        colour.x = linear_to_gamma(colour.x);
        colour.y = linear_to_gamma(colour.y);
        colour.z = linear_to_gamma(colour.z);

        colour
    }



    fn get_ray(&self, x: usize, y: usize) -> Ray {
        let pixel_centre = self.pixel00_loc + (x as f32 * self.pixel_delta_u) + (y as f32 * self.pixel_delta_v);
        let pixel_sample = pixel_centre + self.pixel_sample_square();

        let ray_origin = if self.defocus_angle <= 0.0 { self.centre } else { self.defocus_disk_sample() };
        let ray_direction = pixel_sample - ray_origin;
        let ray_time = next_f32();

        Ray::new(ray_origin, ray_direction, ray_time)

    }

    
    fn defocus_disk_sample(&self) -> Point {
        let p = Vec3::random_in_unit_disk();
        self.centre + (p.x * self.defocus_disk_u) + (p.y * self.defocus_disk_v)
    }


    fn pixel_sample_square(&self) -> Vec3 {
        let px = -0.5 + next_f32();
        let py = -0.5 + next_f32();

        px * self.pixel_delta_u + py * self.pixel_delta_v
    }
}



/// Transforms a colour from linear space to gamma space
#[inline(always)]
fn linear_to_gamma(linear_comp: f32) -> f32 {
    linear_comp.sqrt()
}
