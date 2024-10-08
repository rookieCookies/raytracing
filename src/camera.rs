use crate::{math::vec3::{Colour, Point, Vec3}, rt::{camera::RaytracingCamera, hittable::Hittable, materials::Material, texture::Texture}};


#[derive(Clone)]
pub struct Camera<'a> {
    pub position: Vec3,
    direction: Vec3,

    pub pitch: f32,
    pub yaw: f32,

    aspect_ratio: f32,
    vfov: f32,
    vup: Vec3,
    focus_dist: f32,
    pub rt_cam: RaytracingCamera,
    

    acc_colours: Vec<Colour>, 
    pub samples: usize,
    world: Hittable<'a>,
}

impl<'a> Camera<'a> {
    pub fn new(position: Vec3, direction: Vec3,
               aspect_ratio: f32, width: usize,
               max_depth: usize, vfov: f32, 
               vup: Vec3, defocus_angle: f32, focus_dist: f32) -> Self {
        let rc = RaytracingCamera::new(aspect_ratio, width, max_depth, vfov, position, position + direction, vup, defocus_angle, focus_dist);

        let height = {
            let val = (width as f32 / aspect_ratio) as usize;
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
            acc_colours: Vec::from_iter((0..width * height).map(|_| Colour::ZERO)),
            pitch: 0.0,
            yaw: 0.0,
            samples: 0,
            world: Hittable::sphere(Point::ONE, 1.0, Material::Lambertian { texture: Texture::SolidColour(Colour::ONE) }),
        }
    }


    pub fn set_world(&mut self, world: Hittable<'a>) {
        self.world = world;
    }


    pub fn render(&mut self, buff: &mut [u32]) {
        self.update_render();
        self.samples += 1;
        unsafe { self.rt_cam.render(&mut self.acc_colours, buff, self.samples, &self.world) };
    }


    fn update_render(&mut self) {
        let direction = Vec3::new(
            self.yaw.to_radians().cos() * self.pitch.to_radians().cos(),
            self.pitch.to_radians().sin(),
            self.yaw.to_radians().sin() * self.pitch.to_radians().cos()
        );

        let render = RaytracingCamera::new(self.aspect_ratio, self.rt_cam.image.0,
                                       self.rt_cam.max_depth,
                                       self.vfov, self.position, self.position + direction,
                                       self.vup, self.rt_cam.defocus_angle, self.focus_dist);
        self.rt_cam = render;

        if self.samples == 0 {
            self.acc_colours.iter_mut()
                .for_each(|x|{
                    *x = Colour::ZERO;
                });
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
}


