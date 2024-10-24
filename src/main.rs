use std::{env, time::Instant};

use image::RgbaImage;
use rayon::iter::{ParallelBridge, ParallelIterator};
use raytracing_improved::{camera::Camera, hittable::{ConstantMedium, Hittable, MovingSphere, Quad, Sphere}, material::Material, math::{interval::Interval, vec3::{Colour, Point, Vec3}}, perlin_noise::PerlinNoise, rng::Seed, texture::Texture};
use sdl2::{event::Event, keyboard::Keycode, pixels::PixelFormatEnum, rect::Rect, render::TextureAccess};
use sti::arena::Arena;

const SENSITIVITY : f32 = 0.05;
const CAMERA_SPEED : f32 = 50.0;

struct CliOptions {
    scene_number: usize,
    render_to_image: bool,
    sample_count: Option<usize>,
    locked: bool,
    resolution_scale: f32,
    display_scale: f32,
    max_depth: usize,
}


fn parse_cli_options() -> Result<CliOptions, String> {
    let mut args = env::args().skip(1);
    let mut options = CliOptions {
        scene_number: 0,
        render_to_image: false,
        sample_count: None,
        locked: false,
        resolution_scale: 100.0,
        display_scale: 1.0,
        max_depth: 25,
    };

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--image" => options.render_to_image = true,

            "--scene" | "-s" => {
                let Some(scene_number) = args.next()
                else { return Err(format!("expected a scene number after cli option '{arg}'")) };

                let Ok(scene_number) = scene_number.parse::<usize>()
                else { return Err(format!("unable to parse '{scene_number}' as an integer")) };

                options.scene_number = scene_number;
            },


            "--sample" => {
                let Some(sample_count) = args.next()
                else { return Err(format!("expected a sample number after cli option '{arg}'")) };

                let Ok(sample_count) = sample_count.parse::<usize>()
                else { return Err(format!("unable to parse '{sample_count}' as an integer")) };

                options.sample_count = Some(sample_count);
            }


            "--depth" => {
                let Some(max_depth) = args.next()
                else { return Err(format!("expected a depth number after cli option '{arg}'")) };

                let Ok(max_depth) = max_depth.parse::<usize>()
                else { return Err(format!("unable to parse '{max_depth}' as an integer")) };

                options.max_depth = max_depth;
            }


            "--resolution" => {
                let Some(res_scale) = args.next()
                else { return Err(format!("expected a resolution scale after cli option '{arg}'")) };

                let Ok(res_scale) = res_scale.parse::<f32>()
                else { return Err(format!("unable to parse '{res_scale}' as a float")) };

                options.resolution_scale = res_scale;
            }


            "--display" => {
                let Some(dis_scale) = args.next()
                else { return Err(format!("expected a display scale after cli option '{arg}'")) };

                let Ok(dis_scale) = dis_scale.parse::<f32>()
                else { return Err(format!("unable to parse '{dis_scale}' as a float")) };

                options.display_scale = dis_scale;
            }


            "--locked" => options.locked = true,

            "--help" => {
                println!("--image => render into an image with a specified sample count instead \
                         of an SDL window (specify the sample count via --sample)");
                println!("--scene | -s {{integer}} => specify a specific scene to be rendered");
                println!("--sample {{integer}} => specify a sample count (only used when rendering into an image)");
                println!("--display {{float}} => specify a display scale where the rendered image will be scaled up by");
                println!("--render {{float}} => specify a resolution scale where the render resolution is scaled up by");
            }


            _ => return Err(format!("unknown cli option '{arg}', --help for more info")),
        }
    };


    if options.sample_count.is_some() && !options.render_to_image {
        println!("WARNING: A sample count has been specified but we're not rendering to an image \
                 thus it goes unused");
    }


    if options.render_to_image && options.locked {
        println!("WARNING: The '--locked' option is set but we're rendering into an image");
    }


    Ok(options)
}


fn main() {
    let cli_args = match parse_cli_options() {
        Ok(v) => v,
        Err(str) => {
            eprintln!("{str}");
            return;
        },
    };

    // Camera
    let arena = Arena::new();
    let mut camera = match cli_args.scene_number {
        0 => original_bouncing_spheres(&arena, &cli_args),
        1 => bouncing_spheres_night(&arena, &cli_args),
        2 => quads(&arena, &cli_args),
        3 => simple_light(&arena, &cli_args),
        4 => world_sphere(&arena, &cli_args),
        5 => checkered_spheres(&arena, &cli_args),
        6 => cornell_box(&arena, &cli_args),
        7 => cornell_box_fog(&arena, &cli_args),
        8 => the_final_scene(&arena, &cli_args),
        _ => {
            eprintln!("scene number '{}' must be within the range of 0..=8", cli_args.scene_number);
            return;
        }
    };

    if cli_args.render_to_image {
        for i in 1..cli_args.sample_count.unwrap_or(100) {
            camera.render();
            println!("sample {i}");
        }

        let res = camera.display_resolution();
        let mut image = RgbaImage::new(res.0 as u32, res.1 as u32);
        let buffer = camera.render();

        image.enumerate_pixels_mut().par_bridge()
            .for_each(|(x, y, z)|
               z.0 = ((buffer[(y*res.0 as u32 + x) as usize] << 8) + 255).to_be_bytes()
            );

        image.save("out.png").unwrap();

        return;
    }

    // else, SDL
    let sdl_ctx = sdl2::init().unwrap();
    let video_subsystem = sdl_ctx.video().unwrap();

    let (display_width, display_height) = camera.display_resolution();
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
                        camera.render_resolution().0 as u32, camera.render_resolution().1 as u32).unwrap();

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
                    if !cli_args.locked {
                        camera.change_pitch_yaw_by(yrel as f32 * SENSITIVITY, xrel as f32 * SENSITIVITY);
                    }
                }

                Event::KeyDown { keycode, .. } => {
                    let Some(key) = keycode else { continue };
                    
                    match key {
                        Keycode::W => forward = true,
                        Keycode::S => backward = true,
                        Keycode::D => right = true,
                        Keycode::A => left = true,
                        Keycode::Space => speedboost = true,
                        Keycode::Up => camera.set_exposure(camera.exposure() + 0.1),
                        Keycode::Down => camera.set_exposure(camera.exposure() - 0.1),
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

        let render = camera.render();
        texture.update(None, unsafe { core::mem::transmute(render) }, camera.render_resolution().0 * size_of::<u32>()).unwrap();


        canvas.clear();
        canvas.copy(&texture,
                    Some(Rect::new(0, 0, camera.render_resolution().0 as u32, camera.render_resolution().1 as u32)),
                    Some(Rect::new(0, 0, display_width as u32, display_height as u32)))
            .unwrap();
        canvas.present();

        println!("sample {} rendered in {}ms", camera.samples(), time.elapsed().as_millis());

    }
}


fn the_final_scene<'a>(arena: &'a Arena, opts: &CliOptions) -> Camera<'a> {
    let mut seed = Seed([69, 420, 420, 69]);
    let mut world = sti::vec::Vec::new_in(arena);

    // ground
    let mut boxes_ground = sti::vec::Vec::new_in(arena);
    let ground = Material::lambertian(Texture::colour(Colour::new(0.48, 0.83, 0.53)));

    let boxes_per_side = 20;
    for i in 0..boxes_per_side {
        let i = i as f32;
        for j in 0..boxes_per_side {
            let j = j as f32;
            let w = 100.0;
            let x0 = -1000.0 + i*w;
            let z0 = -1000.0 + j*w;
            let y0 = 0.0;
            let x1 = x0 + w;
            let y1 = seed.next_f32_range(Interval::new(1.0, 101.0));
            let z1 = z0 + w;

            let p0 = Point::new(x0, y0, z0);
            let p1 = Point::new(x1, y1, z1);
            boxes_ground.push(Hittable::box_of_quads(arena, p0, p1, ground));
        }
    }


    world.push(Hittable::bvh(arena, arena.alloc_new(boxes_ground)));

    // light
    let light = Material::diffuse_light(Texture::colour(Colour::new(7.0, 7.0, 7.0)));
    let light = Quad::new(Point::new(123.0, 553.0, 147.0), Vec3::new(300.0, 0.0, 0.0),
                            Vec3::new(0.0, 0.0, 265.0), light);
    world.push(Hittable::quad(light));


    // moving sphere
    let centre1 = Point::new(400.0, 400.0, 200.0);
    let centre2 = centre1 + Point::new(30.0, 0.0, 0.0);
    let sphere_material = Material::lambertian(Texture::colour(Colour::new(0.7, 0.3, 0.1)));
    world.push(Hittable::moving_sphere(MovingSphere::new(centre1, centre2, 50.0, sphere_material)));

    // other spheres
    world.push(Hittable::sphere(Sphere::new(Point::new(260.0, 150.0, 45.0), 50.0,
                                Material::dielectric(Texture::colour(Colour::ONE), 1.5))));
    world.push(Hittable::sphere(Sphere::new(Point::new(0.0, 150.0, 145.0), 50.0,
                                Material::metal(Texture::colour(Colour::new(0.8, 0.8, 0.9)), 1.0))));


    // fog
    let boundary = Sphere::new(Point::new(360.0, 150.0, 145.0), 70.0,
                                Material::dielectric(Texture::colour(Colour::ONE), 1.5));
    let boundary = Hittable::sphere(boundary);
    world.push(boundary.clone());

    let constant_medium = ConstantMedium::new(arena.alloc_new(boundary), 0.2,
                                                Texture::colour(Colour::new(0.2, 0.4, 0.9)));
    world.push(Hittable::constant_medium(constant_medium));


    let boundary = Sphere::new(Point::new(0.0, 0.0, 0.0), 5000.0,
                                Material::dielectric(Texture::colour(Colour::ONE), 1.5));
    let boundary = Hittable::sphere(boundary);
    world.push(boundary.clone());

    let constant_medium = ConstantMedium::new(arena.alloc_new(boundary), 0.0001,
                                                Texture::colour(Colour::ONE));
    world.push(Hittable::constant_medium(constant_medium));


    // earth
    let mut image = image::ImageReader::open("earthmap.jpg").unwrap();
    image.no_limits();
    let image = image.decode().unwrap().into_rgb32f();
    let image = arena.alloc_new(image);

    let earth_material = Material::lambertian(Texture::image(image));
    world.push(Hittable::sphere(Sphere::new(Point::new(400.0, 200.0, 400.0), 100.0, earth_material)));


    // noise
    let perlin_noise = PerlinNoise::new(arena, &mut seed, 256);
    let pertext = Material::lambertian(Texture::noise(perlin_noise, 0.2));
    let sphere = Sphere::new(Point::new(220.0, 280.0, 300.0), 80.0, pertext);
    world.push(Hittable::sphere(sphere));


    // stress balls
    let mut stress_balls = sti::vec::Vec::new_in(arena);
    let white = Material::lambertian(Texture::colour(Colour::ONE));
    let interval = Interval::new(0.0, 165.0);
    let ns = 1000;

    for _ in 0..ns {
        stress_balls.push(Hittable::sphere(Sphere::new(
                    Point::random_range(&mut seed, interval), 10.0, white)));
    }


    let stress_balls = Hittable::bvh(arena, arena.alloc_new(stress_balls))
        .rotate_y_by(arena, 15.0)
        .move_by(arena, Vec3::new(-100.0, 270.0, 395.0));

    world.push(stress_balls);



    let mut camera = Camera::new(arena, Point::new(478.0, 278.0, -600.0), Vec3::new(1.0, 0.0, 0.0),
                             ((10.0 * opts.resolution_scale) as usize, (10.0 * opts.resolution_scale) as usize),
                             opts.display_scale, opts.max_depth, 40.0,
                            Vec3::new(0.0, 1.0, 0.0), 0.0, 10.0, Colour::ZERO);

    camera.set_world(Hittable::bvh(arena, arena.alloc_new(world)));
    camera.change_pitch_yaw_by(0.0, 108.0);


    camera
}


fn cornell_box<'a>(arena: &'a Arena, opts: &CliOptions) -> Camera<'a> {
    let mut world = sti::vec::Vec::new_in(arena);

    let red = Material::lambertian(Texture::colour(Colour::new(0.65, 0.05, 0.05)));
    let white = Material::lambertian(Texture::colour(Colour::new(0.73, 0.73, 0.73)));
    let green = Material::lambertian(Texture::colour(Colour::new(0.12, 0.45, 0.15)));
    let light = Material::diffuse_light(Texture::colour(Colour::new(15.0, 15.0, 15.0)));


    world.push(Hittable::quad(Quad::new(Point::new(555.0, 0.0, 0.0), Vec3::new(0.0, 555.0, 0.0), Vec3::new(0.0, 0.0, 555.0), green)));
    world.push(Hittable::quad(Quad::new(Point::new(0.0, 0.0, 0.0), Vec3::new(0.0, 555.0, 0.0), Vec3::new(0.0, 0.0, 555.0), red)));
    world.push(Hittable::quad(Quad::new(Point::new(343.0, 554.0, 332.0), Vec3::new(-130.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -105.0), light)));
    world.push(Hittable::quad(Quad::new(Point::new(0.0, 0.0, 0.0), Vec3::new(555.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 555.0), white)));
    world.push(Hittable::quad(Quad::new(Point::new(555.0, 555.0, 555.0), Vec3::new(-555.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -555.0), white)));
    world.push(Hittable::quad(Quad::new(Point::new(0.0, 0.0, 555.0), Vec3::new(555.0, 0.0, 0.0), Vec3::new(0.0, 555.0, 0.0), white)));

    let box1 = Hittable::box_of_quads(arena, Point::new(0.0, 0.0, 0.0), Point::new(165.0, 330.0, 165.0), white)
               .rotate_y_by(arena, 15.0)
               .move_by(arena, Vec3::new(265.0, 0.0, 295.0));
    let box2 = Hittable::box_of_quads(arena, Point::new(0.0, 0.0, 0.0), Point::new(165.0, 165.0, 165.0), white)
               .rotate_y_by(arena, -18.0)
               .move_by(arena, Vec3::new(130.0, 0.0, 65.0));
    world.push(box1);
    world.push(box2);

    let mut camera = Camera::new(&arena, Point::new(278.0, 278.0, -800.0), Vec3::new(1.0, 0.0, 0.0),
                             ((10.0 * opts.resolution_scale) as usize, (10.0 * opts.resolution_scale) as usize),
                             opts.display_scale, opts.max_depth, 40.0,
                             Vec3::new(0.0, 2.0, 0.0), 0.0, 10.0, Colour::ZERO);
    camera.change_pitch_yaw_by(0.0, 90.0);

    let world = Hittable::bvh(arena, world.leak());
    camera.set_world(world);
    camera
}


fn cornell_box_fog<'a>(arena: &'a Arena, opts: &CliOptions) -> Camera<'a> {
    let mut world = sti::vec::Vec::new_in(arena);

    let red = Material::lambertian(Texture::colour(Colour::new(0.65, 0.05, 0.05)));
    let white = Material::lambertian(Texture::colour(Colour::new(0.73, 0.73, 0.73)));
    let green = Material::lambertian(Texture::colour(Colour::new(0.12, 0.45, 0.15)));
    let light = Material::diffuse_light(Texture::colour(Colour::new(15.0, 15.0, 15.0)));


    world.push(Hittable::quad(Quad::new(Point::new(555.0, 0.0, 0.0), Vec3::new(0.0, 555.0, 0.0), Vec3::new(0.0, 0.0, 555.0), green)));
    world.push(Hittable::quad(Quad::new(Point::new(0.0, 0.0, 0.0), Vec3::new(0.0, 555.0, 0.0), Vec3::new(0.0, 0.0, 555.0), red)));
    world.push(Hittable::quad(Quad::new(Point::new(343.0, 554.0, 332.0), Vec3::new(-130.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -105.0), light)));
    world.push(Hittable::quad(Quad::new(Point::new(0.0, 0.0, 0.0), Vec3::new(555.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 555.0), white)));
    world.push(Hittable::quad(Quad::new(Point::new(555.0, 555.0, 555.0), Vec3::new(-555.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -555.0), white)));
    world.push(Hittable::quad(Quad::new(Point::new(0.0, 0.0, 555.0), Vec3::new(555.0, 0.0, 0.0), Vec3::new(0.0, 555.0, 0.0), white)));

    let box1 = Hittable::box_of_quads(arena, Point::new(0.0, 0.0, 0.0), Point::new(165.0, 330.0, 165.0), white)
               .rotate_y_by(arena, 15.0)
               .move_by(arena, Vec3::new(265.0, 0.0, 295.0));
    let box1 = Hittable::constant_medium(ConstantMedium::new(
                                            arena.alloc_new(box1), 0.01,
                                            Texture::colour(Colour::ZERO)));

    let box2 = Hittable::box_of_quads(arena, Point::new(0.0, 0.0, 0.0), Point::new(165.0, 165.0, 165.0), white)
               .rotate_y_by(arena, -18.0)
               .move_by(arena, Vec3::new(130.0, 0.0, 65.0));
    let box2 = Hittable::constant_medium(ConstantMedium::new(
                                            arena.alloc_new(box2), 0.01,
                                            Texture::colour(Colour::ONE)));

    world.push(box1);
    world.push(box2);

    let mut camera = Camera::new(&arena, Point::new(278.0, 278.0, -800.0), Vec3::new(1.0, 0.0, 0.0),
                             ((10.0 * opts.resolution_scale) as usize, (10.0 * opts.resolution_scale) as usize),
                             opts.display_scale, opts.max_depth, 40.0,
                             Vec3::new(0.0, 2.0, 0.0), 0.0, 10.0, Colour::ZERO);
    camera.change_pitch_yaw_by(0.0, 90.0);

    let world = Hittable::bvh(arena, world.leak());
    camera.set_world(world);
    camera
}


fn simple_light<'a>(arena: &'a Arena, opts: &CliOptions) -> Camera<'a> {
    let mut world = sti::vec::Vec::new_in(arena);

    let pertext = Texture::noise(PerlinNoise::new(arena, &mut Seed([1, 2, 3, 4]), 100), 4.0);
    world.push(Hittable::sphere(Sphere::new(Point::new(0.0, -1000.0, 0.0), 1000.0, Material::lambertian(pertext))));
    world.push(Hittable::sphere(Sphere::new(Point::new(0.0, 2.0, 0.0), 2.0, Material::lambertian(pertext))));

    let diff_light = Material::diffuse_light(Texture::colour(Colour::new(4.0, 4.0, 4.0)));
    world.push(Hittable::quad(Quad::new(Point::new(3.0, 1.0, -2.0), Vec3::new(2.0, 0.0, 0.0), Vec3::new(0.0, 2.0, 0.0), diff_light)));


    let mut camera = Camera::new(&arena, Point::new(26.0, 2.0, 6.0), Vec3::new(1.0, 0.0, 0.0),
                             ((16.0 * opts.resolution_scale) as usize, (9.0 * opts.resolution_scale) as usize),
                             opts.display_scale, opts.max_depth, 20.0,
                             Vec3::new(0.0, 2.0, 0.0), 0.0, 10.0, Colour::ZERO);
    camera.change_pitch_yaw_by(0.0, -90.0);

    let world = Hittable::bvh(arena, world.leak());
    camera.set_world(world);
    camera
}


fn quads<'a>(arena: &'a Arena, opts: &CliOptions) -> Camera<'a> {
    let left_red = Material::lambertian(Texture::colour(Colour::new(1.0, 0.2, 0.2)));
    let back_green = Material::diffuse_light(Texture::colour(Colour::new(0.2, 1.0, 0.2)));
    let right_blue = Material::lambertian(Texture::colour(Colour::new(0.2, 0.2, 1.0)));
    let upper_orange = Material::lambertian(Texture::colour(Colour::new(1.0, 0.5, 0.0)));
    let lower_teal = Material::lambertian(Texture::colour(Colour::new(0.2, 0.8, 0.8)));

    
    let mut world = sti::vec::Vec::new_in(arena);

    world.push(Hittable::quad(Quad::new(Point::new(-3.0, -2.0, 5.0), Point::new( 0.0, 0.0, -4.0), Point::new(0.0, 4.0,  0.0), left_red)));
    world.push(Hittable::quad(Quad::new(Point::new(-2.0, -2.0, 0.0), Point::new( 4.0, 0.0,  0.0), Point::new(0.0, 4.0,  0.0), back_green)));
    world.push(Hittable::quad(Quad::new(Point::new( 3.0, -2.0, 1.0), Point::new( 0.0, 0.0,  4.0), Point::new(0.0, 4.0,  0.0), right_blue)));
    world.push(Hittable::quad(Quad::new(Point::new(-2.0,  3.0, 1.0), Point::new( 4.0, 0.0,  0.0), Point::new(0.0, 0.0,  4.0), upper_orange)));
    world.push(Hittable::quad(Quad::new(Point::new(-2.0, -3.0, 5.0), Point::new( 4.0, 0.0,  0.0), Point::new(0.0, 0.0, -4.0), lower_teal)));


    let mut camera = Camera::new(&arena, Point::new(0.0, 0.0, 9.0), Vec3::new(1.0, 0.0, 0.0),
                             ((10.0 * opts.resolution_scale) as usize, (10.0 * opts.resolution_scale) as usize),
                             opts.display_scale, opts.max_depth, 80.0,
                             Vec3::new(0.0, 2.0, 0.0), 0.0, 10.0, Colour::new(0.1, 0.1, 0.1));
    camera.change_pitch_yaw_by(0.0, -90.0);

    let world = Hittable::bvh(arena, world.leak());
    camera.set_world(world);
    camera
}


fn original_bouncing_spheres<'a>(arena: &'a Arena, opts: &CliOptions) -> Camera<'a> {
    let mut world = sti::vec::Vec::new_in(arena);

    let material_ground = Material::lambertian(Texture::checkerboard(0.64, arena.alloc_new(Texture::colour(Colour::ZERO)), arena.alloc_new(Texture::colour(Colour::ONE))));

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
                mat = Material::lambertian(Texture::colour(albedo));
            } else if choose_mat < 0.95 {
                let albedo = Colour::random_range(&mut seed, Interval::new(0.5, 1.0));
                let fuzz = seed.next_f32_range(Interval::new(0.0, 0.5));
                mat = Material::metal(Texture::colour(albedo), fuzz);
            } else {
                mat = Material::dielectric(Texture::colour(Colour::ONE), 1.5)
            }

            let hittable = Hittable::moving_sphere(MovingSphere::new(centre, centre_2, 0.2, mat));
            world.push(hittable);
        }
    }

    let mat = Material::dielectric(Texture::colour(Colour::new(1.0, 1.0, 1.0)), 1.5);
    world.push(Hittable::sphere(Sphere::new(Point::new(0.0, 1.0, 0.0), 1.0, mat)));

    let mat = Material::lambertian(Texture::colour(Colour::new(0.4, 0.2, 0.1)));
    world.push(Hittable::sphere(Sphere::new(Point::new(-4.0, 1.0, 0.0), 1.0, mat)));

    let mat = Material::metal(Texture::colour(Colour::new(0.7, 0.6, 0.5)), 0.0);
    world.push(Hittable::sphere(Sphere::new(Point::new(4.0, 1.0, 0.0), 1.0, mat)));

    let world = Hittable::bvh(arena, world.leak());
    let mut camera = Camera::new(&arena, Point::new(-10.0, 5.0, -10.0), Vec3::new(1.0, 0.0, 0.0),
                             ((16.0 * opts.resolution_scale) as usize, (9.0 * opts.resolution_scale) as usize),
                             opts.display_scale, opts.max_depth, 20.0,
                             Vec3::new(0.0, 2.0, 0.0), 0.0, 10.0, Colour::new(0.7, 0.7, 0.7));


    camera.set_world(world);
    camera.change_pitch_yaw_by(-15.0, 45.0);
    camera
}


fn bouncing_spheres_night<'a>(arena: &'a Arena, opts: &CliOptions) -> Camera<'a> {
    let mut world = sti::vec::Vec::new_in(arena);

    let material_ground = Material::lambertian(Texture::checkerboard(0.64, arena.alloc_new(Texture::colour(Colour::ZERO)), arena.alloc_new(Texture::colour(Colour::ONE))));

    world.push(Hittable::sphere(Sphere::new(Point::new(0.0, -1000.0, 0.0), 1000.0, material_ground)));

    let mut seed = Seed([69, 420, 420, 69]);
    
    for a in -11..11 {
        for b in -11..11 {
            let choose_mat = seed.next_f32();
            let centre = Vec3::new(a as f32 + 9.0 * seed.next_f32(), 0.2, b as f32 + 9.0 * seed.next_f32());
            let centre_2 = centre + Vec3::new(0.0, seed.next_f32() * 0.2, 0.0);

            if (centre - Point::new(4.0, 0.2, 0.0)).length() <= 0.9 { continue }

            let mat;
            if choose_mat < 0.5 {
                // diffuse
                let albedo = Colour::random(&mut seed) * Colour::random(&mut seed);
                mat = Material::lambertian(Texture::colour(albedo));
            } else if choose_mat < 0.8 {
                // diffuse
                let albedo = Colour::random(&mut seed) * Colour::random(&mut seed);
                mat = Material::diffuse_light(Texture::colour(25.0*albedo));
            } else if choose_mat < 0.95 {
                let albedo = Colour::random_range(&mut seed, Interval::new(0.5, 1.0));
                let fuzz = seed.next_f32_range(Interval::new(0.0, 0.5));
                mat = Material::metal(Texture::colour(albedo), fuzz);
            } else {
                mat = Material::dielectric(Texture::colour(Colour::ONE), 1.5)
            }

            let mut hittable = Hittable::moving_sphere(MovingSphere::new(centre, centre_2, 0.2, mat));

            if seed.next_f32() < 0.125 {
                let noise = PerlinNoise::new(arena, &mut seed, 64);
                let texture = Texture::noise(noise, seed.next_f32()*2.0);
                let medium = ConstantMedium::new(arena.alloc_new(hittable), seed.next_f32(), texture);
                hittable = Hittable::constant_medium(medium)
            }

            world.push(hittable);
        }
    }

    let mat = Material::dielectric(Texture::colour(Colour::new(1.0, 1.0, 1.0)), 1.5);
    world.push(Hittable::sphere(Sphere::new(Point::new(0.0, 1.0, 0.0), 1.0, mat)));

    let mat = Material::lambertian(Texture::colour(Colour::new(0.4, 0.2, 0.1)));
    world.push(Hittable::sphere(Sphere::new(Point::new(-4.0, 1.0, 0.0), 1.0, mat)));

    let mat = Material::metal(Texture::colour(Colour::new(0.7, 0.6, 0.5)), 0.0);
    world.push(Hittable::sphere(Sphere::new(Point::new(4.0, 1.0, 0.0), 1.0, mat)).move_by(arena, Vec3::ONE));

    let world = Hittable::bvh(arena, world.leak());

    let mut camera = Camera::new(&arena, Point::new(-10.0, 5.0, -10.0), Vec3::new(1.0, 0.0, 0.0),
                             ((16.0 * opts.resolution_scale) as usize, (9.0 * opts.resolution_scale) as usize),
                             opts.display_scale, opts.max_depth, 20.0,
                             Vec3::new(0.0, 2.0, 0.0), 0.0, 10.0, Colour::new(0.0, 0.0, 0.0));

    camera.set_world(world);
    camera.change_pitch_yaw_by(-15.0, 45.0);
    camera
}


fn world_sphere<'a>(arena: &'a Arena, opts: &CliOptions) -> Camera<'a> {
    let mut world = sti::vec::Vec::new_in(arena);

    let mut image = image::ImageReader::open("earthmap.jpg").unwrap();
    image.no_limits();
    let image = image.decode().unwrap().into_rgb32f();
    let image = arena.alloc_new(image);
    let material_ground = Material::diffuse_light(Texture::image(image));
    world.push(Hittable::sphere(Sphere::new(Point::new(0.0, 0.0, 0.0), 2.0, material_ground)));

    let world = Hittable::bvh(&arena, world.leak());

    let mut camera = Camera::new(&arena, Point::new(-10.0, 5.0, -10.0), Vec3::new(1.0, 0.0, 0.0),
                             ((16.0 * opts.resolution_scale) as usize, (10.0 * opts.resolution_scale) as usize),
                             opts.display_scale, opts.max_depth, 20.0,
                             Vec3::new(0.0, 2.0, 0.0), 0.0, 10.0, Colour::new(0.7, 0.7, 0.7));

    camera.set_world(world);
    camera
}


fn checkered_spheres<'a>(arena: &'a Arena, opts: &CliOptions) -> Camera<'a> {
    let mut world = sti::vec::Vec::new_in(arena);

    let material_ground = Material::lambertian(Texture::checkerboard(1.0, arena.alloc_new(Texture::colour(Colour::ZERO)), arena.alloc_new(Texture::colour(Colour::ONE))));
    world.push(Hittable::sphere(Sphere::new(Point::new(0.0, -10.0, 0.0), 10.0, material_ground)));
    world.push(Hittable::sphere(Sphere::new(Point::new(0.0, 10.0, 0.0), 10.0, material_ground)));

    let world = Hittable::bvh(&arena, world.leak());
    let mut camera = Camera::new(&arena, Point::new(-10.0, 5.0, -10.0), Vec3::new(1.0, 0.0, 0.0),
                             ((16.0 * opts.resolution_scale) as usize, (10.0 * opts.resolution_scale) as usize),
                             opts.display_scale, opts.max_depth, 20.0,
                             Vec3::new(0.0, 2.0, 0.0), 0.0, 10.0, Colour::new(0.7, 0.7, 0.7));

    camera.set_world(world);
    camera
}
