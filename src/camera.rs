use raylib::color::Color;

use crate::{math::{matrix::Matrix, vec3::{Colour, Vec3}}, rt::{camera::RaytracingCamera, hittable::Hittable}};


#[derive(Clone)]
pub struct Camera {
    position: Vec3,
    direction: Vec3,

    pub pitch: f64,
    pub yaw: f64,

    aspect_ratio: f64,
    vfov: f64,
    vup: Vec3,
    focus_dist: f64,
    pub rt_cam: RaytracingCamera,
    
    colours: Vec<Colour>,
}

impl Camera {
    pub fn new(position: Vec3, direction: Vec3,
               aspect_ratio: f64, width: usize, samples_per_pixel: usize,
               max_depth: usize, vfov: f64, 
               vup: Vec3, defocus_angle: f64, focus_dist: f64) -> Self {
        let rc = RaytracingCamera::new(aspect_ratio, width, samples_per_pixel, max_depth, vfov, position, position + direction, vup, defocus_angle, focus_dist);

        let height = {
            let val = (width as f64 / aspect_ratio) as usize;
            if val <= 0 { 1 } else { val }
        };

        Self {
            position,
            direction,
            aspect_ratio,
            vfov,
            vup,
            focus_dist,
            rt_cam: rc,
            colours: Vec::from_iter((0..width * height).map(|_| Colour::ZERO)),
            pitch: 0.0,
            yaw: 0.0,
        }
    }


    pub fn render(&mut self, world: &Hittable) -> &[Colour] {
        self.update_render();
        unsafe { self.rt_cam.render(&mut self.colours, world) };
        &self.colours
    }


    fn update_render(&mut self) {
        let direction = Vec3::new(
            self.yaw.to_radians().cos() * self.pitch.to_radians().cos(),
            self.pitch.to_radians().sin(),
            self.yaw.to_radians().sin() * self.pitch.to_radians().cos()
        );

        let render = RaytracingCamera::new(self.aspect_ratio, self.rt_cam.image.0,
                                       self.rt_cam.samples_per_pixel, self.rt_cam.max_depth,
                                       self.vfov, self.position, self.position + direction,
                                       self.vup, self.rt_cam.defocus_angle, self.focus_dist);
        self.rt_cam = render;
    }


    pub fn move_by(&mut self, step: Vec3) {
        self.position += step;
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


    pub fn change_pitch_yaw_by(&mut self, delta_pitch: f64, delta_yaw: f64) {
        self.pitch += delta_pitch;
        self.yaw += delta_yaw;
        self.direction = Vec3::new(
            self.yaw.to_radians().cos() * self.pitch.to_radians().cos(),
            self.pitch.to_radians().sin(),
            self.yaw.to_radians().sin() * self.pitch.to_radians().cos()
        );
    }
}


