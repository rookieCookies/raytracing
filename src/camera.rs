use core::f32;
use std::{mem::transmute, ops::Div, simd::StdFloat};

use rayon::iter::{ParallelBridge, ParallelIterator};
//use sdl2::video::WindowPos;
use sti::arena::Arena;

use crate::{hittable::{HitRecord, Hittable, Sphere}, material::Material, math::{interval::Interval, ray::{Ray, Switch}, vec3::{Colour, Point, Vec3}}, rng::Seed, texture::Texture, utils::{SendPtr, Stack}, World};


pub struct Camera<'a> {
    position: Vec3,
    direction: Vec3,

    display_scale: f32,
    render_resolution: (usize, usize),

    pitch: f32,
    yaw: f32,

    vfov: f32,
    vup: Vec3,
    focus_dist: f32,
    rt_cam: RaytracingCamera,
    
    acc_colours: Vec<Colour>, 
    final_colours: Vec<u32>, // 0RGB
    samples: usize,
    world: &'a mut Hittable<'a>,

    background_colour: Colour,
    exposure: f32,
}


impl<'a> Camera<'a> {
    pub fn new(arena: &'a Arena, position: Vec3, direction: Vec3,
               (width, height): (usize, usize), display_scale: f32,
               max_depth: usize, vfov: f32,  vup: Vec3, defocus_angle: f32,
               focus_dist: f32, background_colour: Colour) -> Self {
        let rc = RaytracingCamera::new(width, height, max_depth, vfov, position, position + direction, vup, defocus_angle, focus_dist, background_colour, 1.0);
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
                                                                Material::lambertian(Texture::colour(Colour::ZERO))))),
            display_scale,
            render_resolution: (width, height),
            background_colour,
            exposure: 1.0,
        }
    }


    pub fn set_exposure(&mut self, exposure: f32) {
        self.force_update_raytracing_camera();
        self.exposure = exposure;
    }




    pub fn set_world(&mut self, world: Hittable<'a>) {
        self.samples = 0;
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

        self.force_update_raytracing_camera();
        self.acc_colours.iter_mut().for_each(|x| *x = Colour::ZERO);
    }


    fn force_update_raytracing_camera(&mut self) {
        let direction = Vec3::new(
            self.yaw.to_radians().cos() * self.pitch.to_radians().cos(),
            self.pitch.to_radians().sin(),
            self.yaw.to_radians().sin() * self.pitch.to_radians().cos()
        );

        let render = RaytracingCamera::new(self.rt_cam.image_dimensions.0, self.rt_cam.image_dimensions.1,
                                       self.rt_cam.max_depth,
                                       self.vfov, self.position, self.position + direction,
                                       self.vup, self.rt_cam.defocus_angle, self.focus_dist, self.background_colour,
                                       self.exposure);
        self.rt_cam = render;
    }



    pub fn change_pitch_yaw_by(&mut self, delta_pitch: f32, delta_yaw: f32) {
        if delta_pitch == 0.0 && delta_yaw == 0.0 { return }
        self.pitch += delta_pitch;
        self.yaw += delta_yaw;
        self.direction = Vec3::new(
            self.yaw.to_radians().cos() * self.pitch.to_radians().cos(),
            self.pitch.to_radians().sin(),
            self.yaw.to_radians().sin() * self.pitch.to_radians().cos()
        );

        self.samples = 0;
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

    pub fn exposure(&self) -> f32 { self.exposure }
    pub fn samples(&self) -> usize { self.samples }
    pub fn pitch(&self) -> f32 { self.pitch }
    pub fn yaw(&self) -> f32 { self.yaw}
    pub fn display_scale(&self) -> f32 { self.display_scale }
    pub fn render_resolution(&self) -> (usize, usize) { self.render_resolution }
    pub fn display_resolution(&self) -> (usize, usize) {
        let x = (self.render_resolution.0 as f32 * self.display_scale) as usize;
        let y = (self.render_resolution.1 as f32 * self.display_scale) as usize;
        (x, y)
    }
}



#[derive(Clone)]
struct RaytracingCamera {
    image_dimensions: (usize, usize),
    centre: Point,
    pixel00_loc: Vec3,
    pixel_delta_u: Vec3,
    pixel_delta_v: Vec3,
    max_depth: usize,
    defocus_angle: f32,
    defocus_disk_u: Vec3,
    defocus_disk_v: Vec3,
    background_colour: Colour,
    exposure: f32,
}


impl RaytracingCamera {
    pub fn new(width: usize, height: usize,
               max_depth: usize, vfov: f32, look_from: Vec3, look_at: Vec3,
               vup: Vec3, defocus_angle: f32, focus_dist: f32, background_colour: Colour,
               exposure: f32) -> Self {
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
            background_colour,
            exposure,
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
                    let colour = self.ray_colour(&mut seed, ray, world, &mut hittable_stack);

                    let colour = unsafe { acc_ptr.read() + colour };
                    unsafe { acc_ptr.write(colour) };

                    let colour = samples * colour;
                    let mut mapped = Vec3::ONE.axes - (self.exposure * -colour).axes.exp();
                    mapped[3] = 0.0;
                    let mapped = unsafe { Vec3::new_simd(mapped) };
                    unsafe { final_ptr.write(mapped.to_rgba()) };

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


    fn ray_colour<'a>(&self, seed: &mut Seed, ray: Ray,
                  world: &'a Hittable<'a>, hittable_stack: &mut Stack<Switch<'a>>) -> Colour {

        struct Frame {
            ray: Ray,
            depth: usize,
            multiplier: Vec3,
        }


        debug_assert!(hittable_stack.is_empty());
        let mut active_frame = Frame { ray, depth: self.max_depth, multiplier: Vec3::ONE };
        let mut rec = HitRecord::default();

        loop {
            let Frame { ray, depth, multiplier } = active_frame;

            if depth == 0 { return Colour::ZERO }

            let hit_anything = ray.hit_anything(seed, &mut rec, world, hittable_stack);

            // If the ray hits nothing, return the skybox
            if !hit_anything {
                return multiplier * self.background_colour 
            }


            let colour_from_emission = rec.material.emitted(rec.u, rec.v, rec.point);
            let Some((scattered, attenuation)) = rec.material.scatter(seed, &ray, &rec)
            else { return multiplier * colour_from_emission };

            let frame = Frame {
                ray: scattered,
                depth: depth - 1,
                multiplier: multiplier * attenuation + colour_from_emission,
            };

            active_frame = frame;
        }
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

