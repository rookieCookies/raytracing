use core::f32;
use std::mem::transmute;

use rayon::iter::{ParallelBridge, ParallelIterator};
//use sdl2::video::WindowPos;
use sti::arena::Arena;

use crate::{hittable::{HitRecord, Hittable, Sphere}, math::{interval::Interval, ray::Ray, vec3::{Colour, Point, Vec3}}, rng::Seed, utils::{SendPtr, Stack}, World};


pub struct Camera<'a> {
    pub position: Vec3,
    direction: Vec3,

    pub pitch: f32,
    pub yaw: f32,

    vfov: f32,
    vup: Vec3,
    focus_dist: f32,
    pub rt_cam: RaytracingCamera,
    
    acc_colours: Vec<Colour>, 
    final_colours: Vec<u32>, // 0RGB
    pub samples: usize,
    world: &'a mut Hittable<'a>,

    seed: Seed,
    arena: &'a Arena,
}


impl<'a> Camera<'a> {
    pub fn new(arena: &'a Arena, position: Vec3, direction: Vec3,
               width: usize, height: usize,
               max_depth: usize, vfov: f32, 
               vup: Vec3, defocus_angle: f32, focus_dist: f32) -> Self {
        let rc = RaytracingCamera::new(width, height, max_depth, vfov, position, position + direction, vup, defocus_angle, focus_dist);
        Self {
            position,
            direction,
            vfov,
            vup,
            focus_dist,
            rt_cam: rc,
            acc_colours: Vec::from_iter((0..width * height).map(|_| Colour::ZERO)),
            final_colours: Vec::from_iter((0..width * height).map(|_| 0)),
            pitch: 0.0,
            yaw: 0.0,
            samples: 0,
            world: arena.alloc_new(Hittable::sphere(Sphere::new(Point::ZERO, 1.0,
                                                                crate::material::Material::Lambertian { texture: crate::texture::Texture::SolidColour(Colour::ZERO) }))),
            seed: Seed([1, 2, 3, 4]),
            arena,
        }
    }


    pub fn set_world(&mut self, world: Hittable<'a>) {
        *self.world = world;
    }


    pub fn render(&mut self) -> &[u32] {
        self.update_raytracing_camera();
        self.samples += 1;
        self.rt_cam.render(self.world, self.samples, &mut self.acc_colours, &mut self.final_colours);
        &self.final_colours
    }


    fn update_raytracing_camera(&mut self) {
        if self.samples != 0 { return }

        let direction = Vec3::new(
            self.yaw.to_radians().cos() * self.pitch.to_radians().cos(),
            self.pitch.to_radians().sin(),
            self.yaw.to_radians().sin() * self.pitch.to_radians().cos()
        );

        let render = RaytracingCamera::new(self.rt_cam.image_dimensions.0, self.rt_cam.image_dimensions.1,
                                       self.rt_cam.max_depth,
                                       self.vfov, self.position, self.position + direction,
                                       self.vup, self.rt_cam.defocus_angle, self.focus_dist);
        self.rt_cam = render;

        self.acc_colours.iter_mut().for_each(|x| *x = Colour::ZERO);
    }



    pub fn change_pitch_yaw_by(&mut self, delta_pitch: f32, delta_yaw: f32) {
        self.pitch += delta_pitch;
        self.yaw += delta_yaw;
        self.direction = Vec3::new(
            self.yaw.to_radians().cos() * self.pitch.to_radians().cos(),
            self.pitch.to_radians().sin(),
            self.yaw.to_radians().sin() * self.pitch.to_radians().cos()
        );
        if delta_pitch != 0.0 || delta_yaw != 0.0 {
            self.samples = 0;
        }
    }


    pub fn move_by(&mut self, step: Vec3) {
        self.position += step;
        if step != Vec3::ZERO {
            self.samples = 0;
        }
    }


    pub fn forward(&self) -> Vec3 {
        self.direction
    }

    pub fn backward(&self) -> Vec3 {
        -self.forward()
    }

    pub fn left(&self) -> Vec3 {
        -self.right()
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(self.up()).unit()
    }

    pub fn up(&self) -> Vec3 {
        Vec3::new(0.0, 1.0, 0.0)
    }
}



#[derive(Clone)]
pub struct RaytracingCamera {
    pub image_dimensions: (usize, usize),
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
    pub fn new(width: usize, height: usize,
               max_depth: usize, vfov: f32, look_from: Vec3, look_at: Vec3,
               vup: Vec3, defocus_angle: f32, focus_dist: f32) -> Self {
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
            image_dimensions: (width, height),
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


    fn render<'a, 'b>(&self, world: &'a Hittable<'a>,
                  n_samples: usize, acc_colours: &'b mut [Colour], final_colours: &'b mut [u32]) {
        assert_eq!(final_colours.len(), self.image_dimensions.0 * self.image_dimensions.1);
        assert_eq!(acc_colours.len(), self.image_dimensions.0 * self.image_dimensions.1);

        let acc_len = acc_colours.len();
        let acc_ptr = SendPtr(acc_colours.as_mut_ptr());
        let final_ptr = SendPtr(final_colours.as_mut_ptr());

        let samples = 1.0 / n_samples as f32;


        (0..self.image_dimensions.1)
            .par_bridge()
            .for_each(move |y| {
                let mut hittable_stack = Stack::new();
                let mut seed = Seed([y as u64, n_samples as u64, acc_len as u64, y as u64]);

                let mut acc_ptr = unsafe { acc_ptr.clone().0.offset((y*self.image_dimensions.0) as isize) };
                let mut final_ptr = unsafe { final_ptr.clone().0.offset((y*self.image_dimensions.0) as isize) };

                for x in 0..self.image_dimensions.0 {
                    let ray = self.get_ray(&mut seed, x, y);
                    let colour = ray.colour(&mut seed, world, &mut hittable_stack, self.max_depth);

                    let colour = unsafe { acc_ptr.read() + colour };
                    unsafe { acc_ptr.write(colour) };

                    let colour = samples * colour;
                    unsafe { final_ptr.write(colour.to_rgba()) };

                    acc_ptr = unsafe { acc_ptr.add(1) };
                    final_ptr = unsafe { final_ptr.add(1) };

                }

            });

    }




    fn get_ray(&self, seed: &mut Seed, x: usize, y: usize) -> Ray {
        let pixel_centre = self.pixel00_loc + (x as f32 * self.pixel_delta_u) + (y as f32 * self.pixel_delta_v);
        let pixel_sample = pixel_centre + self.pixel_sample_square(seed);

        let ray_origin = if self.defocus_angle <= 0.0 { self.centre } else { self.defocus_disk_sample(seed) };
        let ray_direction = pixel_sample - ray_origin;
        let ray_time = seed.next_f32();
        Ray::new(ray_origin, ray_direction, ray_time)

    }

    
    fn defocus_disk_sample(&self, seed: &mut Seed) -> Point {
        let p = Vec3::random_in_unit_disk(seed);
        self.centre + (p[0] * self.defocus_disk_u) + (p[1] * self.defocus_disk_v)

    }


    fn pixel_sample_square(&self, seed: &mut Seed) -> Vec3 {
        let px = -0.5 + seed.next_f32();
        let py = -0.5 + seed.next_f32();

        px * self.pixel_delta_u + py * self.pixel_delta_v
    }

}



/// Transforms a colour from linear space to gamma space
#[inline(always)]
fn linear_to_gamma(linear_comp: f32) -> f32 {
    linear_comp.sqrt()
}

