use crate::{math::{ray::Ray, vec3::{Colour, Vec3}}, rt::hittable::HitRecord, rng::next_f32};

#[derive(Default, Clone, Copy)]
pub enum Material {
    Lambertian {
        albedo: Colour,
    },

    Metal {
        albedo: Colour,
        fuzz_radius: f32,
    },

    Dielectric {
        refraction_index: f32,
    },

    #[default]
    Unknown,
}


impl Material {
    pub fn scatter(self, ray_in: Ray, rec: &HitRecord) -> Option<(Ray, Colour)> {
        match self {
            Material::Lambertian { albedo } => {
                let mut scatter_dir = rec.normal + Vec3::random_unit();

                if scatter_dir.near_zero() { scatter_dir = rec.normal };

                let scatter_dir = scatter_dir;
                let scattered = Ray::new(rec.point, scatter_dir, ray_in.time);
                Some((scattered, albedo))
            },

            Material::Metal { albedo, fuzz_radius } => {
                let fuzz_radius = fuzz_radius.min(1.0);
                let reflected = ray_in.direction.unit().reflect(rec.normal);
                let scattered = Ray::new(rec.point, reflected + fuzz_radius * Vec3::random_unit(), ray_in.time);

                if scattered.direction.dot(rec.normal) > 0.0 {
                    Some((scattered, albedo))
                } else { None }
            },

            Material::Dielectric { refraction_index } => {
                let attenuation = Colour::new(1.0, 1.0, 1.0);
                let refraction_ratio = if rec.front_face { 1.0 / refraction_index }
                                       else { refraction_index };

                let unit_dir = ray_in.direction.unit();
                let cos_theta = (-unit_dir).dot(rec.normal).min(1.0);
                let sin_theta = (1.0 - cos_theta*cos_theta).sqrt();

                let cannot_refract = refraction_ratio * sin_theta > 1.0;
                let direction = if cannot_refract || reflectance(cos_theta, refraction_ratio) > next_f32() {
                    unit_dir.reflect(rec.normal)
                } else {
                    unit_dir.refract(rec.normal, refraction_ratio)
                };

                Some((Ray::new(rec.point, direction, ray_in.time), attenuation))
            },

            Material::Unknown => unimplemented!(),
        }
    }
}


fn reflectance(cos: f32, rr: f32) -> f32 {
    // Use Schlic's approximation for reflectance
    let r0 = (1.0-rr) / (1.0+rr);
    let r0 = r0*r0;
    r0 + (1.0-r0)*(1.0-cos).powi(5)
}
