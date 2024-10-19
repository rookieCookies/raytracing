use crate::{hittable::HitRecord, math::{ray::Ray, vec3::{Colour, Point, Vec3}}, rng::Seed};

use super::texture::Texture;

#[derive(Default, Clone, Copy)]
pub struct Material<'a> {
    texture: Texture<'a>,
    kind: MaterialKind,
}


#[derive(Default, Clone, Copy)]
pub enum MaterialKind {
    Lambertian,
    Metal {
        fuzz_radius: f32,
    },
    Dielectric {
        refraction_index: f32,
    },
    DiffuseLight,
    Isotropic,

    #[default]
    NotFound,
}


impl<'a> Material<'a> {
    pub fn scatter(self, seed: &mut Seed, ray_in: &Ray, rec: &HitRecord) -> Option<(Ray, Colour)> {
        match self.kind {
            MaterialKind::Lambertian => {
                let mut scatter_dir = rec.normal + Vec3::random_unit(seed);

                if scatter_dir.near_zero() { scatter_dir = rec.normal };

                let scatter_dir = scatter_dir;
                let scattered = Ray::new(rec.point, scatter_dir, ray_in.time);
                Some((scattered, self.texture.value(rec.u, rec.v, rec.point)))
            },

            MaterialKind::Metal { fuzz_radius } => {
                let fuzz_radius = fuzz_radius.min(1.0);
                let reflected = ray_in.direction.unit().reflect(rec.normal);
                let scattered = Ray::new(rec.point, reflected + fuzz_radius * Vec3::random_unit(seed), ray_in.time);

                if scattered.direction.dot(rec.normal) > 0.0 {
                    Some((scattered, self.texture.value(rec.u, rec.v, rec.point)))
                } else { None }
            },

            MaterialKind::Dielectric { refraction_index } => {
                let attenuation = self.texture.value(rec.u, rec.v, rec.point);
                let refraction_ratio = if rec.front_face { 1.0 / refraction_index }
                                       else { refraction_index };

                let unit_dir = ray_in.direction.unit();
                let cos_theta = (-unit_dir).dot(rec.normal).min(1.0);
                let sin_theta = (1.0 - cos_theta*cos_theta).sqrt();

                let cannot_refract = refraction_ratio * sin_theta > 1.0;
                let direction = if cannot_refract || reflectance(cos_theta, refraction_ratio) > seed.next_f32() {
                    unit_dir.reflect(rec.normal)
                } else {
                    unit_dir.refract(rec.normal, refraction_ratio)
                };

                Some((Ray::new(rec.point, direction, ray_in.time), attenuation))
            },

            MaterialKind::Isotropic => {
                Some((Ray::new(rec.point, Vec3::random_unit(seed), ray_in.time), self.texture.value(rec.u, rec.v, rec.point)))
            },

            MaterialKind::DiffuseLight { .. } => None,

            MaterialKind::NotFound => unimplemented!(),
        }
    }


    pub fn emitted(&self, u: f32, v: f32, p: Point) -> Colour {
        match self.kind {
            MaterialKind::DiffuseLight => self.texture.value(u, v, p),
            _ => Colour::ZERO,
        }
    }


    fn new(texture: Texture<'a>, kind: MaterialKind) -> Self {
        Self {
            texture,
            kind,
        }
    }


    pub fn lambertian(texture: Texture<'a>) -> Self {
        Self::new(texture, MaterialKind::Lambertian)
    }

    pub fn metal(texture: Texture<'a>, fuzz_radius: f32) -> Self {
        Self::new(texture, MaterialKind::Metal { fuzz_radius })
    }

    pub fn dielectric(texture: Texture<'a>, refraction_index: f32) -> Self {
        Self::new(texture, MaterialKind::Dielectric { refraction_index })
    }

    pub fn diffuse_light(texture: Texture<'a>) -> Self {
        Self::new(texture, MaterialKind::DiffuseLight)
    }

    pub fn isotropic(texture: Texture<'a>) -> Self {
        Self::new(texture, MaterialKind::Isotropic)
    }
}


fn reflectance(cos: f32, rr: f32) -> f32 {
    // Use Schlic's approximation for reflectance
    let r0 = (1.0-rr) / (1.0+rr);
    let r0 = r0*r0;
    r0 + (1.0-r0)*(1.0-cos).powi(5)
}
