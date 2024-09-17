use std::{fmt::Write, sync::atomic::AtomicUsize};

use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::{hittable::Hittable, math::{ray::Ray, vec3::{Colour, Point, Vec3}}, rng::next_f64, utils::SendPtr};

pub struct Camera {
    pub image: (usize, usize),
    pub centre: Point,
    pub pixel00_loc: Vec3,
    pub pixel_delta_u: Vec3,
    pub pixel_delta_v: Vec3,
    pub samples_per_pixel: usize,
    max_depth: usize,
    defocus_angle: f64,
    defocus_disk_u: Vec3,
    defocus_disk_v: Vec3,
}


impl Camera {
    pub fn new(aspect_ratio: f64, width: usize, samples_per_pixel: usize,
               max_depth: usize, vfov: f64, look_from: Vec3, look_at: Vec3,
               vup: Vec3, defocus_angle: f64, focus_dist: f64) -> Self {

        let height = {
            let val = (width as f64 / aspect_ratio) as usize;
            if val <= 0 { 1 } else { val }
        };

        let centre = look_from;

        // Determine viewport dimensions
        let theta = vfov.to_radians();
        let h = (theta/2.0).tan();
        let viewport_height = 2.0 * h * focus_dist;
        let viewport_width = viewport_height * (width as f64 / height as f64);

        let w = (look_from - look_at).unit();
        let u = vup.cross(w).unit();
        let v = w.cross(u);

        let viewport_u = viewport_width  * u;
        let viewport_v = viewport_height * -v;

        let pixel_delta_u = viewport_u / width as f64;
        let pixel_delta_v = viewport_v / height as f64;


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
            samples_per_pixel,
            max_depth,
            defocus_angle,
            defocus_disk_u,
            defocus_disk_v,
        }
    }


    pub fn render(&self, world: &Hittable) -> String {

        let mut buffer = String::new();
        writeln!(buffer, "P3\n {} {}\n255", self.image.0, self.image.1).unwrap();

        let cap = (3 * 3 + 2 * 1 + 1) * self.image.0 * self.image.1;
        buffer.reserve(cap);

        let cap = buffer.capacity();
        let mut colours : Vec<Colour> = Vec::with_capacity(self.image.0 * self.image.1);

        {
            let ptr = SendPtr(colours.as_mut_ptr());
            let counter = AtomicUsize::new(0);


            // i have never cared less about UB as i have here
            (0..self.image.1).par_bridge()
                .for_each(move |y| {
                    let ptr = ptr;
                    let mut ptr = unsafe { ptr.0.offset((y*self.image.0) as isize) };
                    for x in 0..self.image.0 {
                        let colour = self.colour_of(world, x, y);
                        unsafe { ptr.write(colour) };
                        ptr = unsafe { ptr.add(1) };
                    }

                    let count = counter.fetch_add(1, std::sync::atomic::Ordering::Release);
                    println!("{}/{}", count, self.image.1);
                });
            
            unsafe { colours.set_len(self.image.0 * self.image.1) };

        }
        
        for colour in colours {
            writeln!(buffer, "{} {} {}",
                     (colour.x * 255.999) as u8,
                     (colour.y * 255.999) as u8,
                     (colour.z * 255.999) as u8).unwrap();

        }

        assert!(buffer.capacity() == cap);

        buffer
    }

    
    fn colour_of(&self, world: &Hittable, x: usize, y: usize) -> Colour {
        // calculate the colour
        let mut colour = Colour::new(0.0, 0.0, 0.0);
        for _ in 0..self.samples_per_pixel {
            let ray = self.get_ray(x, y);
            colour += ray.colour(&world, self.max_depth);
        }

        // finalise
        let scale = 1.0 / self.samples_per_pixel as f64;
        colour.x *= scale;
        colour.y *= scale;
        colour.z *= scale;


        // Linear -> Gamma
        colour.x = linear_to_gamma(colour.x);
        colour.y = linear_to_gamma(colour.y);
        colour.z = linear_to_gamma(colour.z);

        colour
    }



    fn get_ray(&self, x: usize, y: usize) -> Ray {
        let pixel_centre = self.pixel00_loc + (x as f64 * self.pixel_delta_u) + (y as f64 * self.pixel_delta_v);
        let pixel_sample = pixel_centre + self.pixel_sample_square();

        let ray_origin = if self.defocus_angle <= 0.0 { self.centre } else { self.defocus_disk_sample() };
        let ray_direction = pixel_sample - ray_origin;

        Ray::new(ray_origin, ray_direction)

    }

    
    fn defocus_disk_sample(&self) -> Point {
        let p = Vec3::random_in_unit_disk();
        self.centre + (p.x * self.defocus_disk_u) + (p.y * self.defocus_disk_v)
    }


    fn pixel_sample_square(&self) -> Vec3 {
        let px = -0.5 + next_f64();
        let py = -0.5 + next_f64();

        px * self.pixel_delta_u + py * self.pixel_delta_v
    }
}



/// Transforms a colour from linear space to gamma space
#[inline(always)]
fn linear_to_gamma(linear_comp: f64) -> f64 {
    if linear_comp > 0.0 { linear_comp.sqrt() }
    else { linear_comp }
}
