use std::time::Instant;

use raytracing_improved::{camera::{Camera, RaytracingCamera}, hittable::{Hittable, MovingSphere, Quad, Sphere}, material::Material, math::{interval::Interval, vec3::{Colour, Point, Vec3}}, perlin_noise::PerlinNoise, rng::Seed, texture::Texture};
use sdl2::{event::Event, keyboard::Keycode, pixels::PixelFormatEnum, rect::{Rect}, render::TextureAccess, sys::{quad_t, SDL_TouchDeviceType}};
use sti::{arena::Arena, static_assert_eq};

const SENSITIVITY : f32 = 0.05;
const CAMERA_SPEED : f32 = 5.0;

fn main() {
    // Camera
    let arena = Arena::new();


    let mut camera = simple_light(&arena);

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

    let display_width = (camera.render_resolution.0 as f32 * camera.display_scale) as usize;
    let display_height = (camera.render_resolution.1 as f32 * camera.display_scale) as usize;
    let mut window = video_subsystem.window("raytracing", display_width as u32, display_height as u32)
        .position_centered()
        .build().unwrap();

    window.set_grab(true);
    window.set_mouse_grab(true);
    sdl_ctx.mouse().set_relative_mouse_mode(true);

    let mut canvas = window.into_canvas().build().unwrap();

    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture(PixelFormatEnum::RGBA32, TextureAccess::Streaming,
                        camera.render_resolution.0 as u32, camera.render_resolution.1 as u32).unwrap();

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

        println!("hi");
        texture.update(None, unsafe { core::mem::transmute(render) }, camera.render_resolution.0 * size_of::<u32>()).unwrap();
        println!("hi2");

        canvas.clear();
        println!("hi3");
        canvas.copy(&texture,
                    Some(Rect::new(0, 0, camera.render_resolution.0 as u32, camera.render_resolution.1 as u32)),
                    Some(Rect::new(0, 0, display_width as u32, display_height as u32)))
            .unwrap();
        println!("hi4");
        canvas.present();

    }
}


fn simple_light<'a>(arena: &'a Arena) -> Camera<'a> {
    let mut world = sti::vec::Vec::new_in(arena);

    let pertext = Texture::NoiseTexture(PerlinNoise::new(arena, &mut Seed([1, 2, 3, 4]), 100), 4.0);
    world.push(Hittable::sphere(Sphere::new(Point::new(0.0, -1000.0, 0.0), 1000.0, Material::Lambertian { texture: pertext })));
    world.push(Hittable::sphere(Sphere::new(Point::new(0.0, 2.0, 0.0), 2.0, Material::Lambertian { texture: pertext })));

    let diff_light = Material::DiffuseLight { texture: Texture::SolidColour(Colour::new(4.0, 4.0, 4.0)) };
    world.push(Hittable::quad(Quad::new(Point::new(3.0, 1.0, -2.0), Vec3::new(2.0, 0.0, 0.0), Vec3::new(0.0, 2.0, 0.0), diff_light)));


    let mut camera = Camera::new(&arena, Point::new(26.0, 2.0, 6.0), Vec3::new(1.0, 0.0, 0.0),
                             (1920, 1080), 1.0, 50, 20.0,
                             Vec3::new(0.0, 2.0, 0.0), 0.0, 10.0, Colour::ZERO);
    camera.change_pitch_yaw_by(0.0, -90.0);

    let world = Hittable::bvh(arena, world.leak());
    camera.set_world(world);
    camera
}


fn quads<'a>(arena: &'a Arena) -> Camera<'a> {
    let left_red = Material::Lambertian { texture: Texture::SolidColour(Colour::new(1.0, 0.2, 0.2)) };
    let back_green = Material::DiffuseLight { texture: Texture::SolidColour(Colour::new(0.2, 1.0, 0.2)) };
    let right_blue = Material::Lambertian { texture: Texture::SolidColour(Colour::new(0.2, 0.2, 1.0)) };
    let upper_orange = Material::Lambertian { texture: Texture::SolidColour(Colour::new(1.0, 0.5, 0.0)) };
    let lower_teal = Material::Lambertian { texture: Texture::SolidColour(Colour::new(0.2, 0.8, 0.8)) };

    
    let mut world = sti::vec::Vec::new_in(arena);

    world.push(Hittable::quad(Quad::new(Point::new(-3.0, -2.0, 5.0), Point::new( 0.0, 0.0, -4.0), Point::new(0.0, 4.0,  0.0), left_red)));
    world.push(Hittable::quad(Quad::new(Point::new(-2.0, -2.0, 0.0), Point::new( 4.0, 0.0,  0.0), Point::new(0.0, 4.0,  0.0), back_green)));
    world.push(Hittable::quad(Quad::new(Point::new( 3.0, -2.0, 1.0), Point::new( 0.0, 0.0,  4.0), Point::new(0.0, 4.0,  0.0), right_blue)));
    world.push(Hittable::quad(Quad::new(Point::new(-2.0,  3.0, 1.0), Point::new( 4.0, 0.0,  0.0), Point::new(0.0, 0.0,  4.0), upper_orange)));
    world.push(Hittable::quad(Quad::new(Point::new(-2.0, -3.0, 5.0), Point::new( 4.0, 0.0,  0.0), Point::new(0.0, 0.0, -4.0), lower_teal)));


    let mut camera = Camera::new(&arena, Point::new(0.0, 0.0, 9.0), Vec3::new(1.0, 0.0, 0.0),
                             (1080, 1080), 1.0, 50, 80.0,
                             Vec3::new(0.0, 2.0, 0.0), 0.0, 10.0, Colour::new(0.01, 0.01, 0.01));
    camera.change_pitch_yaw_by(0.0, -90.0);

    let world = Hittable::bvh(arena, world.leak());
    camera.set_world(world);
    camera
}


fn bouncing_spheres<'a>(arena: &'a Arena) -> Camera<'a> {
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

    let mut camera = Camera::new(&arena, Point::new(-10.0, 5.0, -10.0), Vec3::new(1.0, 0.0, 0.0),
                             (1728, 1080), 1.0, 25, 20.0,
                             Vec3::new(0.0, 2.0, 0.0), 0.0, 10.0, Colour::ZERO);

    camera.set_world(world);
    camera.change_pitch_yaw_by(-15.0, 45.0);
    camera
}


fn world_sphere<'a>(arena: &'a Arena) -> Camera<'a> {
    let mut world = sti::vec::Vec::new_in(arena);

    let mut image = image::ImageReader::open("earthmap3.png").unwrap();
    image.no_limits();
    let image = image.decode().unwrap().into_rgb32f();
    let image = arena.alloc_new(image);
    let material_ground = Material::Lambertian { texture: Texture::Image { image } };
    world.push(Hittable::sphere(Sphere::new(Point::new(0.0, 0.0, 0.0), 2.0, material_ground)));

    let world = Hittable::bvh(&arena, world.leak());

    let mut camera = Camera::new(&arena, Point::new(-10.0, 5.0, -10.0), Vec3::new(1.0, 0.0, 0.0),
                             (1728, 1080), 1.0, 25, 20.0,
                             Vec3::new(0.0, 2.0, 0.0), 0.0, 10.0, Colour::ZERO);

    camera.set_world(world);
    camera
}


fn checkered_spheres<'a>(arena: &'a Arena) -> Camera<'a> {
    let mut world = sti::vec::Vec::new_in(arena);

    let material_ground = Material::Lambertian { texture: Texture::Checkerboard { inv_scale: 1.0, even: arena.alloc_new(Texture::SolidColour(Colour::ZERO)), odd: arena.alloc_new(Texture::SolidColour(Colour::ONE)) } };
    world.push(Hittable::sphere(Sphere::new(Point::new(0.0, -10.0, 0.0), 10.0, material_ground)));
    world.push(Hittable::sphere(Sphere::new(Point::new(0.0, 10.0, 0.0), 10.0, material_ground)));

    let world = Hittable::bvh(&arena, world.leak());
    let mut camera = Camera::new(&arena, Point::new(-10.0, 5.0, -10.0), Vec3::new(1.0, 0.0, 0.0),
                             (1728, 1080), 1.0, 25, 20.0,
                             Vec3::new(0.0, 2.0, 0.0), 0.0, 10.0, Colour::ZERO);

    camera.set_world(world);
    camera
}
