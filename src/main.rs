mod math;
mod camera;
mod hittable;
pub mod rng;
mod materials;
pub mod utils;

use std::time::Instant;

use crate::{math::{vec3::{Vec3, Colour, Point}, interval::Interval}, hittable::Hittable, camera::Camera, materials::Material, rng::{next_f64_range, next_f64}};

fn main() {
    println!("Setting up..");
    let time = Instant::now();

    // Camera
    let camera = Camera::new(16.0 / 10.0, 1080, 10, 100, 20.0, Point::new(13.0, 2.0, 3.0),
                             Point::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), 0.6, 10.0);

    // Rng
    for _ in 0..camera.image.1 {
        rng::next_f64();
    }

    // World
    let mut world = Vec::new();

    let material_ground = Material::Lambertian { albedo: Colour::new(0.5, 0.5, 0.5) };
    world.push(Hittable::Sphere { centre: Point::new(0.0, -1000.0, 0.0), radius: 1000.0, mat: material_ground });

    for a in -11..11 {
        for b in -11..11 {
            let choose_mat = next_f64();
            let centre = Vec3::new(a as f64 + 0.9 * next_f64(), 0.2, b as f64 + 0.9 * next_f64());

            if (centre - Point::new(4.0, 0.2, 0.0)).length() <= 0.9 { continue }

            let mat;
            if choose_mat < 0.8 {
                // diffuse
                let albedo = Colour::random() * Colour::random();
                mat = Material::Lambertian { albedo };
            } else if choose_mat < 0.95 {
                let albedo = Colour::random_range(Interval::new(0.5, 1.0));
                let fuzz = next_f64_range(Interval::new(0.0, 0.5));
                mat = Material::Metal { albedo, fuzz_radius: fuzz };
            } else {
                mat = Material::Dielectric { refraction_index: 1.5 }
            }

            world.push(Hittable::Sphere { centre, radius: 0.2, mat });
        }
    }

    let mat = Material::Dielectric { refraction_index: 1.5 };
    world.push(Hittable::Sphere { centre: Point::new(0.0, 1.0, 0.0), radius: 1.0, mat});

    let mat = Material::Lambertian { albedo: Colour::new(0.4, 0.2, 0.1) };
    world.push(Hittable::Sphere { centre: Point::new(-4.0, 1.0, 0.0), radius: 1.0, mat });

    let mat = Material::Metal { albedo: Colour::new(0.7, 0.6, 0.5), fuzz_radius: 0.0 };
    world.push(Hittable::Sphere { centre: Point::new(4.0, 1.0, 0.0), radius: 1.0, mat });

    let world = Hittable::List(world);
    
    println!("Set up in {}ms", time.elapsed().as_millis());

    // Render
    println!("Rendering");
    let time = Instant::now();

    let data = camera.render(&world);

    println!("Rendered in {}ms", time.elapsed().as_millis());


    // Generate file
    println!("Writing to file");
    std::fs::write("out.ppm", data).unwrap();
}

