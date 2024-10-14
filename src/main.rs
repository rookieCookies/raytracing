use std::{thread, time::{Duration, Instant}};

use raytracing_improved::{camera::{Camera, RaytracingCamera}, hittable::{Hittable, MovingSphere, Sphere}, material::Material, math::{interval::Interval, vec3::{Colour, Point, Vec3}}, rng::Seed, texture::Texture};
use sdl2::{event::Event, keyboard::Keycode, pixels::PixelFormatEnum, rect::Rect, render::TextureAccess, sys::SDL_TouchDeviceType};
use sti::{arena::Arena, static_assert_eq};

const ASPECT_RATIO : f32 = 16.0/10.0;

const RENDER_HEIGHT : usize = 1080;
const RENDER_WIDTH : usize = (RENDER_HEIGHT as f32 * ASPECT_RATIO) as usize;

const DISPLAY_HEIGHT : usize = 640;
const DISPLAY_WIDTH : usize = (DISPLAY_HEIGHT as f32 * ASPECT_RATIO) as usize;

const SENSITIVITY : f32 = 0.05;
const CAMERA_SPEED : f32 = 5.0;

static_assert_eq!(DISPLAY_HEIGHT, ((DISPLAY_HEIGHT as f32 * ASPECT_RATIO) as usize as f32 / ASPECT_RATIO) as usize);
static_assert_eq!(RENDER_HEIGHT, ((RENDER_HEIGHT as f32 * ASPECT_RATIO) as usize as f32 / ASPECT_RATIO) as usize);


fn main() {
    // Camera
    let arena = Arena::new();
    let mut camera = Camera::new(&arena, Point::new(-10.0, 5.0, -10.0), Vec3::new(1.0, 0.0, 0.0),
                             RENDER_WIDTH, RENDER_HEIGHT as usize, 25, 20.0,
                             Vec3::new(0.0, 2.0, 0.0), 0.0, 10.0);

    camera.change_pitch_yaw_by(-15.0, 45.0);

    camera.set_world(bouncing_spheres(&arena));

    #[cfg(feature="miri")]
    {
        let time = Instant::now();
        //println!("rendering");
        for _ in 0..1 {
            let render = camera.render();
        }
        //println!("Rendered in {}ms", time.elapsed().as_millis());
        return
    }

    // SDL
    let sdl_ctx = sdl2::init().unwrap();
    let video_subsystem = sdl_ctx.video().unwrap();

    let mut window = video_subsystem.window("raytracing", DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32)
        .position_centered()
        .build().unwrap();

    window.set_grab(true);
    window.set_mouse_grab(true);
    sdl_ctx.mouse().set_relative_mouse_mode(true);

    let mut canvas = window.into_canvas().build().unwrap();

    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture(PixelFormatEnum::RGBA32, TextureAccess::Streaming,
                        RENDER_WIDTH as u32, RENDER_HEIGHT as u32).unwrap();

    let mut event_pump = sdl_ctx.event_pump().unwrap();
    let timer = sdl_ctx.timer().unwrap();

    let mut forward = false;
    let mut backward = false;
    let mut left = false;
    let mut right = false;
    let mut speedboost = false;

    let mut last = timer.performance_counter();
    'main: loop {
        let now = timer.performance_counter();
        let dt = (now - last) as f32 / timer.performance_frequency() as f32;
        last = now;


        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'main,
                Event::MouseMotion { xrel, yrel, .. } => {
                    camera.change_pitch_yaw_by(yrel as f32 * SENSITIVITY, xrel as f32 * SENSITIVITY);
                }

                Event::KeyDown { keycode, .. } => {
                    let Some(key) = keycode else { continue };
                    
                    match key {
                        Keycode::W => forward = true,
                        Keycode::S => backward = true,
                        Keycode::D => right = true,
                        Keycode::A => left = true,
                        Keycode::Space => speedboost = true,
                        _ => (),
                    };
                }

                Event::KeyUp { keycode, .. } => {
                    let Some(key) = keycode else { continue };
                    
                    match key {
                        Keycode::W => forward = false,
                        Keycode::S => backward = false,
                        Keycode::D => right = false,
                        Keycode::A => left = false,
                        Keycode::Space => speedboost = false,
                        _ => (),
                    };
                }
                _ => (),
            }
        }


        let mut cam_speed = CAMERA_SPEED * dt as f32;

        if speedboost { cam_speed *= 5.0 }
        if forward { camera.move_by(cam_speed * camera.forward()) }
        if backward { camera.move_by(cam_speed * camera.backward()) }
        if left { camera.move_by(cam_speed * camera.left()) }
        if right { camera.move_by(cam_speed * camera.right()) }

        let time = Instant::now();
        println!("rendering");
        let render = camera.render();
        println!("Rendered in {}ms", time.elapsed().as_millis());

        texture.update(None, unsafe { core::mem::transmute(render) }, RENDER_WIDTH * size_of::<u32>()).unwrap();

        canvas.clear();
        canvas.copy(&texture,
                    Some(Rect::new(0, 0, RENDER_WIDTH as u32, RENDER_HEIGHT as u32)),
                    Some(Rect::new(0, 0, DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32)))
            .unwrap();
        canvas.present();

    }
}


fn bouncing_spheres<'a>(arena: &'a Arena) -> Hittable<'a> {
    let mut world = sti::vec::Vec::new_in(arena);

    let material_ground = Material::Lambertian { texture: Texture::Checkerboard { inv_scale: 0.64, even: arena.alloc_new(Texture::SolidColour(Colour::ZERO)), odd: arena.alloc_new(Texture::SolidColour(Colour::ONE)) } };
    world.push(Hittable::sphere(Sphere::new(Point::new(0.0, -1000.0, 0.0), 1000.0, material_ground)));

    let mut seed = Seed([69, 420, 420, 69]);
    
    for a in -11..11 {
        for b in -11..11 {
            let choose_mat = seed.next_f32();
            let centre = Vec3::new(a as f32 + 9.0 * seed.next_f32(), 0.2, b as f32 + 9.0 * seed.next_f32());
            let centre_2 = centre + Vec3::new(0.0, seed.next_f32() * 0.2, 0.0);

            if (centre - Point::new(4.0, 0.2, 0.0)).length() <= 0.9 { continue }

            let mat;
            if choose_mat < 0.8 {
                // diffuse
                let albedo = Colour::random(&mut seed) * Colour::random(&mut seed);
                mat = Material::Lambertian { texture: Texture::SolidColour(albedo) };
            } else if choose_mat < 0.95 {
                let albedo = Colour::random_range(&mut seed, Interval::new(0.5, 1.0));
                let fuzz = seed.next_f32_range(Interval::new(0.0, 0.5));
                mat = Material::Metal { texture: Texture::SolidColour(albedo), fuzz_radius: fuzz };
            } else {
                mat = Material::Dielectric { refraction_index: 1.5, texture: Texture::SolidColour(Colour::ONE) }
            }

            world.push(Hittable::moving_sphere(MovingSphere::new(centre, centre_2, 0.2, mat)));
        }
    }

    let mat = Material::Dielectric { refraction_index: 1.5, texture: Texture::SolidColour(Colour::ONE)};
    world.push(Hittable::sphere(Sphere::new(Point::new(0.0, 1.0, 0.0), 1.0, mat)));

    let mat = Material::Lambertian { texture: Texture::SolidColour(Colour::new(0.4, 0.2, 0.1)) };
    world.push(Hittable::sphere(Sphere::new(Point::new(-4.0, 1.0, 0.0), 1.0, mat)));

    let mat = Material::Metal { texture: Texture::SolidColour(Colour::new(0.7, 0.6, 0.5)), fuzz_radius: 0.0 };
    world.push(Hittable::sphere(Sphere::new(Point::new(4.0, 1.0, 0.0), 1.0, mat)));

    let world = Hittable::bvh(arena, world.leak());
    world
}


