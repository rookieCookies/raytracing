mod math;
mod camera;
pub mod rng;
pub mod utils;
pub mod rt;
pub mod perlin_noise;

use std::{env, fs, mem::transmute, num::{NonZero, NonZeroU32}, rc::Rc, time::Instant};

use perlin_noise::PerlinNoise;
use sdl2::{event::Event, keyboard::Keycode, pixels::PixelFormatEnum, rect::Rect, render::TextureAccess, sys::SDL_CreateTexture, TimerSubsystem};
use sti::arena::Arena;

use crate::{camera::Camera, math::vec3::{Colour, Point, Vec3}, rt::{hittable::Hittable, materials::Material, texture::Texture}};


const RENDER_RESOLUTION : usize = 1080;
const RENDER_RESOLUTION_X : usize = (RENDER_RESOLUTION as f32 * ASPECT_RATIO) as usize;
const DISPLAY_RESOLUTION : usize = 900;
const DISPLAY_RESOLUTION_X : usize = (DISPLAY_RESOLUTION as f32 * ASPECT_RATIO) as usize;
const MAX_DEPTH : usize = 25;
const ASPECT_RATIO : f32 = 16.0 / 9.0;
const SENSITIVITY : f32 = 0.05;
const CAMERA_SPEED : f32 = 5.0;


fn main() {
    println!("Setting up..");
    let time = Instant::now();

    // Camera
    let mut camera = Camera::new(Point::new(-0.0, 7.0, -0.0), Vec3::new(1.0, 0.0, 0.0),
                             ASPECT_RATIO, RENDER_RESOLUTION_X as usize, MAX_DEPTH, 20.0,
                             Vec3::new(0.0, 2.0, 0.0), 0.0, 10.0);
    camera.change_pitch_yaw_by(-90.0, 0.0);

    // Rng
    for _ in 0..RENDER_RESOLUTION {
        rng::next_f32();
    }

    // World
    let arena = Arena::new();
    let world = bouncing_spheres(&arena);

    camera.set_world(world);
    
    println!("Set up in {}ms", time.elapsed().as_millis());

    let mut args = env::args();
    args.next();

    if args.next().is_some_and(|x| &x == "image") {
        render_image(camera, 50);
        return;
    }

    let sdl_ctx = sdl2::init().unwrap();
    let video_subsystem = sdl_ctx.video().unwrap();

    let mut window = video_subsystem.window("raytracing", DISPLAY_RESOLUTION_X as u32, DISPLAY_RESOLUTION as u32)
        .position_centered()
        .build().unwrap();

    window.set_grab(true);
    window.set_mouse_grab(true);
    sdl_ctx.mouse().set_relative_mouse_mode(true);

    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture(PixelFormatEnum::RGBA32, TextureAccess::Streaming,
                        RENDER_RESOLUTION_X as u32, RENDER_RESOLUTION as u32).unwrap();

    let mut pixels = Box::new([0u32; RENDER_RESOLUTION_X * RENDER_RESOLUTION]);

    canvas.clear();
    canvas.present();

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


        let render_time = timed(&timer, || {
            camera.render(pixels.as_mut_slice());
        });

        let draw_time = timed(&timer, || {
            texture.update(None, unsafe { transmute(pixels.as_slice()) }, RENDER_RESOLUTION_X * size_of::<u32>()).unwrap();

            canvas.clear();
            canvas.copy(&texture,
                        Some(Rect::new(0, 0, RENDER_RESOLUTION_X as u32, RENDER_RESOLUTION as u32)),
                        Some(Rect::new(0, 0, DISPLAY_RESOLUTION_X as u32, DISPLAY_RESOLUTION as u32)))
                .unwrap();
            canvas.present();
        });


        println!("Rendered in {render_time}ms, Drawn in {draw_time}ms");

    }

    // else, raylib
    /*
    let mut window = Window::new("Raytracing", RENDER_RESOLUTION_X, RENDER_RESOLUTION, WindowOptions {
        resize: true,
        scale: minifb::Scale::FitScreen,
        scale_mode: minifb::ScaleMode::AspectRatioStretch,
        ..Default::default()
    }).unwrap();


    let mut last_time = Instant::now();
    let mut last_mouse_pos : Option<(f32, f32)> = None;
    let mut dt;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        dt = last_time.elapsed().as_secs_f32();
        last_time = Instant::now();

        let mouse_movement = match last_mouse_pos {
            Some(last) => {
                match window.get_mouse_pos(minifb::MouseMode::Pass) {
                    Some(v) => {
                        last_mouse_pos = Some(v);
                        (v.0 - last.0, v.1 - last.1)
                    },
                    None => (0.0, 0.0),
                }
            },
            None => {
                last_mouse_pos = window.get_mouse_pos(minifb::MouseMode::Pass);
                (0.0, 0.0)
            },
        };

        let mouse_movement = (mouse_movement.0 * SENSITIVITY, mouse_movement.1 * SENSITIVITY);
        camera.change_pitch_yaw_by(-mouse_movement.1, mouse_movement.0);

        if window.is_key_down(Key::LeftShift) { dt *= 50.0 }
        if window.is_key_down(Key::S) { camera.move_by(dt * CAMERA_SPEED * camera.backward()) };
        if window.is_key_down(Key::W) { camera.move_by(dt * CAMERA_SPEED * camera.forward()) };
        if window.is_key_down(Key::A) { camera.move_by(dt * CAMERA_SPEED * camera.left()) };
        if window.is_key_down(Key::D) { camera.move_by(dt * CAMERA_SPEED * camera.right()) };
        
        let time = Instant::now();
        let data = camera.render(&world);
        println!("Rendered in {}ms", time.elapsed().as_millis());

        let time = Instant::now();
        window.update_with_buffer(&data, RENDER_RESOLUTION_X, RENDER_RESOLUTION).unwrap();
        println!("Drawn in {}ms", time.elapsed().as_millis());
    }*/

}


fn world_sphere<'a>(arena: &'a Arena) -> Hittable<'a> {
    let mut world = sti::vec::Vec::new_in(arena);

    let mut image = image::ImageReader::open("earthmap3.png").unwrap();
    image.no_limits();
    let image = image.decode().unwrap().into_rgb32f();
    let image = arena.alloc_new(image);
    let material_ground = Material::Lambertian { texture: Texture::Image { image } };
    world.push(Hittable::sphere(Point::new(0.0, 0.0, 0.0), 2.0, material_ground));

    let world = Hittable::bvh(&arena, world.leak());
    world
}


fn checkered_spheres<'a>(arena: &'a Arena) -> Hittable<'a> {
    let mut world = sti::vec::Vec::new_in(arena);

    let material_ground = Material::Lambertian { texture: Texture::Checkerboard { inv_scale: 1.0, even: arena.alloc_new(Texture::SolidColour(Colour::ZERO)), odd: arena.alloc_new(Texture::SolidColour(Colour::ONE)) } };
    world.push(Hittable::sphere(Point::new(0.0, -10.0, 0.0), 10.0, material_ground));
    world.push(Hittable::sphere(Point::new(0.0, 10.0, 0.0), 10.0, material_ground));

    let world = Hittable::bvh(&arena, world.leak());
    world
}


fn test<'a>(arena: &'a Arena) -> Hittable<'a> {
    let mut world = sti::vec::Vec::new_in(arena);

    let material_ground = Material::Lambertian { texture: Texture::NoiseTexture(PerlinNoise::new(arena, 256*16), 0.1) };
    world.push(Hittable::sphere(Point::new(0.0, -1000.0, 0.0), 1000.0, material_ground));

   
    let mat = Material::Dielectric { refraction_index: 1.5, texture: Texture::SolidColour(Colour::ONE)};
    world.push(Hittable::sphere(Point::new(0.0, 1.0, 0.0), 1.0, mat));

    let mat = Material::Lambertian { texture: Texture::SolidColour(Colour::new(0.4, 0.2, 0.1)) };
    world.push(Hittable::sphere(Point::new(-4.0, 1.0, 0.0), 1.0, mat));

    let mat = Material::Metal { texture: Texture::SolidColour(Colour::new(0.7, 0.6, 0.5)), fuzz_radius: 0.0 };
    world.push(Hittable::sphere(Point::new(4.0, 1.0, 0.0), 1.0, mat));
    let world = Hittable::bvh(&arena, world.leak());
    world
}




fn bouncing_spheres<'a>(arena: &'a Arena) -> Hittable<'a> {
    let mut world = sti::vec::Vec::new_in(arena);

    let material_ground = Material::Lambertian { texture: Texture::Checkerboard { inv_scale: 0.64, even: arena.alloc_new(Texture::SolidColour(Colour::ZERO)), odd: arena.alloc_new(Texture::SolidColour(Colour::ONE)) } };
    world.push(Hittable::sphere(Point::new(0.0, -1000.0, 0.0), 1000.0, material_ground));

    
    /*
    for a in -11..11 {
        for b in -11..11 {
            let choose_mat = next_f32();
            let centre = Vec3::new(a as f32 + 9.0 * next_f32(), 0.2, b as f32 + 9.0 * next_f32());
            let centre_2 = centre + Vec3::new(0.0, next_f32() * 0.2, 0.0);

            if (centre - Point::new(4.0, 0.2, 0.0)).length() <= 0.9 { continue }

            let mat
            if choose_mat < 0.8 {
                // diffuse
                let albedo = Colour::random() * Colour::random();
                mat = Material::Lambertian { texture: Texture::SolidColour(albedo) };
            } else if choose_mat < 0.95 {
                let albedo = Colour::random_range(Interval::new(0.5, 1.0));
                let fuzz = next_f32_range(Interval::new(0.0, 0.5));
                mat = Material::Metal { texture: Texture::SolidColour(albedo), fuzz_radius: fuzz };
            } else {
                mat = Material::Dielectric { refraction_index: 1.5, texture: Texture::SolidColour(Colour::ONE) }
            }

            world.push(Hittable::moving_sphere(centre, centre_2, 0.2, mat ));
        }
    }*/

    let mat = Material::Dielectric { refraction_index: 1.5, texture: Texture::SolidColour(Colour::ONE)};
    world.push(Hittable::sphere(Point::new(0.0, 1.0, 0.0), 1.0, mat));

    let mat = Material::Lambertian { texture: Texture::SolidColour(Colour::new(0.4, 0.2, 0.1)) };
    world.push(Hittable::sphere(Point::new(-4.0, 1.0, 0.0), 1.0, mat));

    let mat = Material::Metal { texture: Texture::SolidColour(Colour::new(0.7, 0.6, 0.5)), fuzz_radius: 0.0 };
    world.push(Hittable::sphere(Point::new(4.0, 1.0, 0.0), 1.0, mat));

    let world = Hittable::bvh(&arena, world.leak());
    world
}


fn render_image(mut camera: Camera, samples: usize) {
    let time = Instant::now();
    let mut buff = vec![0; (RENDER_RESOLUTION * RENDER_RESOLUTION_X) as usize];
    for _ in 0..(samples-1) { camera.render(&mut buff); }

    let data = camera.render(&mut buff);
    println!("Rendered in {}ms", time.elapsed().as_millis());

    let mut string = String::new();
    string.push_str("P3\n");
    string.push_str(format!("{} {}\n", RENDER_RESOLUTION_X, RENDER_RESOLUTION).as_str());
    string.push_str("255\n");

    //for d in data {
    //    let r = (d.x * 255.999) as u8;
    //    let g = (d.y * 255.999) as u8;
    //    let b = (d.z * 255.999) as u8;
    //    string.push_str(&format!("{} {} {} ", r, g, b));
    //}

    fs::write("out.ppm", &string).unwrap();
}


fn timed<F: FnOnce() -> ()>(timer: &TimerSubsystem, f: F) -> usize {
    let last = timer.performance_counter();
    f();
    let now = timer.performance_counter();
    ((now - last) as f64 / timer.performance_frequency() as f64 * 1000.0) as usize
}
