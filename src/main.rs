mod math;
mod camera;
pub mod rng;
pub mod utils;
pub mod rt;

use std::{env, fs, time::Instant};

use raylib::{drawing::{RaylibDraw, RaylibTextureModeExt}, math::{Rectangle, Vector2}};
use sti::arena::Arena;

use crate::{camera::Camera, rt::{hittable::Hittable, materials::Material}, math::{interval::Interval, ray::Ray, vec3::{Colour, Point, Vec3}}, rng::{next_f32, next_f32_range}};


const RENDER_RESOLUTION : usize = 1080/8;
const RENDER_RESOLUTION_X : usize = (RENDER_RESOLUTION as f32 * ASPECT_RATIO) as usize;
const DISPLAY_RESOLUTION : usize = 900;
const DISPLAY_RESOLUTION_X : usize = (DISPLAY_RESOLUTION as f32 * ASPECT_RATIO) as usize;
const ASPECT_RATIO : f32 = 16.0 / 9.0;
const SENSITIVITY : f32 = 0.05;
const CAMERA_SPEED : f32 = 3.0;


fn main() {
    println!("Setting up..");
    let time = Instant::now();

    // Camera
    let mut camera = Camera::new(Point::new(-12.0, 4.0, -5.0), Vec3::new(1.0, 0.0, 0.0),
                             ASPECT_RATIO, RENDER_RESOLUTION_X, 100, 50, 20.0,
                             Vec3::new(0.0, 2.0, 0.0), 0.6, 10.0);
    camera.change_pitch_yaw_by(-15.0, 21.0);

    // Rng
    for _ in 0..RENDER_RESOLUTION {
        rng::next_f32();
    }

    // World
    let arena = Arena::new();
    let mut world = sti::vec::Vec::new_in(&arena);

    let material_ground = Material::Lambertian { albedo: Colour::new(0.5, 0.5, 0.5) };
    world.push(Hittable::sphere(Point::new(0.0, -1000.0, 0.0), 1000.0, material_ground));

    
    
    
    /*
    for a in -11..11 {
        for b in -11..11 {
            let choose_mat = next_f32();
            let centre = Vec3::new(a as f32 + 0.9 * next_f32(), 0.2, b as f32 + 0.9 * next_f32());
            let centre_2 = centre + Vec3::new(0.0, next_f32() * 0.5, 0.0);

            if (centre - Point::new(4.0, 0.2, 0.0)).length() <= 0.9 { continue }

            let mat;
            if choose_mat < 0.8 {
                // diffuse
                let albedo = Colour::random() * Colour::random();
                mat = Material::Lambertian { albedo };
            } else if choose_mat < 0.95 {
                let albedo = Colour::random_range(Interval::new(0.5, 1.0));
                let fuzz = next_f32_range(Interval::new(0.0, 0.5));
                mat = Material::Metal { albedo, fuzz_radius: fuzz };
            } else {
                mat = Material::Dielectric { refraction_index: 1.5 }
            }

            world.push(Hittable::moving_sphere(centre, centre_2, 0.2, mat ));
        }
    }*/

    let mat = Material::Dielectric { refraction_index: 1.5 };
    world.push(Hittable::sphere(Point::new(0.0, 1.0, 0.0), 1.0, mat));

    let mat = Material::Lambertian { albedo: Colour::new(0.4, 0.2, 0.1) };
    world.push(Hittable::sphere(Point::new(-4.0, 1.0, 0.0), 1.0, mat));

    let mat = Material::Metal { albedo: Colour::new(0.7, 0.6, 0.5), fuzz_radius: 0.0 };
    world.push(Hittable::sphere(Point::new(4.0, 1.0, 0.0), 1.0, mat));

    let world = Hittable::bvh(&arena, &world);
    // let world = Hittable::list(world);
    
    println!("Set up in {}ms", time.elapsed().as_millis());

    let mut args = env::args();
    args.next();

    if args.next().is_some_and(|x| &x == "image") {
        render_image(camera, world);
        return;
    }

    // else, raylib

    // Raylib
    let (mut rl, th) = raylib::init()
        .size(DISPLAY_RESOLUTION_X as i32, DISPLAY_RESOLUTION as i32)
        .build();
    rl.disable_cursor();


    let mut texture = rl.load_render_texture(&th, RENDER_RESOLUTION_X as u32, RENDER_RESOLUTION as u32).unwrap();
    let mut first_mouse_input = true;
    let mut dt;
    while !rl.window_should_close() {
        dt = rl.get_frame_time();

        let mouse_movement = rl.get_mouse_delta() * SENSITIVITY;
        if !first_mouse_input {
            camera.change_pitch_yaw_by(-mouse_movement.y, mouse_movement.x);
        }

        if mouse_movement != raylib::prelude::Vector2::zero() { first_mouse_input = false }

        if rl.is_key_down(raylib::ffi::KeyboardKey::KEY_LEFT_SHIFT) {
            dt *= 50.0;
        }

        if rl.is_key_down(raylib::ffi::KeyboardKey::KEY_S) {
            camera.move_by(dt * CAMERA_SPEED * camera.backward());
        }

        if rl.is_key_down(raylib::ffi::KeyboardKey::KEY_W) {
            camera.move_by(dt * CAMERA_SPEED * camera.forward());
        }

        if rl.is_key_down(raylib::ffi::KeyboardKey::KEY_A) {
            camera.move_by(dt * CAMERA_SPEED * camera.left());
        }

        if rl.is_key_down(raylib::ffi::KeyboardKey::KEY_D) {
            camera.move_by(dt * CAMERA_SPEED * camera.right());
        }


        let time = Instant::now();
        let data = camera.render(&world);
        println!("Rendered in {}ms", time.elapsed().as_millis());

        let mut brush = rl.begin_drawing(&th);

        let time = Instant::now();
        let mut tex_brush = brush.begin_texture_mode(&th, &mut texture);
        for y in 0..RENDER_RESOLUTION {
            for x in 0..RENDER_RESOLUTION_X {
                let colour = data[y * RENDER_RESOLUTION_X + x];
                let r = (colour.x * 255.999) as u8;
                let g = (colour.y * 255.999) as u8;
                let b = (colour.z * 255.999) as u8;
                tex_brush.draw_pixel(x as i32, (RENDER_RESOLUTION - 1 - y) as i32, raylib::color::Color::new(r, g, b, 255));
            }
        }
        drop(tex_brush);


        brush.clear_background(raylib::color::Color::WHITE);
        brush.draw_texture_pro(&texture,
                               Rectangle::new(0.0, 0.0, RENDER_RESOLUTION_X as f32, RENDER_RESOLUTION as f32),
                               Rectangle::new(0.0, 0.0, DISPLAY_RESOLUTION_X as f32, DISPLAY_RESOLUTION as f32),
                               Vector2::new(0.0, 0.0), 0.0, raylib::color::Color::WHITE);

        brush.draw_fps(0, 0);
        brush.draw_text(format!("pitch: {}, yaw: {}", camera.pitch, camera.yaw).as_str(), 0, 16, 16, raylib::color::Color::RED);
        brush.draw_text(format!("x: {}, y: {}, z: {}", camera.position.x, camera.position.y, camera.position.z).as_str(), 0, 32, 16, raylib::color::Color::RED);
        println!("Drawn in {}ms", time.elapsed().as_millis());
    }

}

fn render_image(mut camera: Camera, world: Hittable) {
    let time = Instant::now();
    let data = camera.render(&world);
    println!("Rendered in {}ms", time.elapsed().as_millis());

    let mut string = String::new();
    string.push_str("P3\n");
    string.push_str(format!("{} {}\n", RENDER_RESOLUTION_X, RENDER_RESOLUTION).as_str());
    string.push_str("255\n");

    for d in data {
        let r = (d.x * 255.999) as u8;
        let g = (d.y * 255.999) as u8;
        let b = (d.z * 255.999) as u8;
        string.push_str(&format!("{} {} {} ", r, g, b));
    }

    fs::write("out.ppm", &string).unwrap();
}


