use crate::{math::{ray::Ray, vec3::{Point, Vec3}, interval::Interval}, rt::materials::Material};

#[derive(Clone, Default)]
pub struct HitRecord {
    pub point: Point,
    pub normal: Vec3,
    pub t: f64,
    pub front_face: bool,
    pub material: Material,
}


pub enum Hittable {
    List(Vec<Hittable>),
    Sphere { centre: Point, radius: f64, mat: Material },
    MovingSphere { centre: Ray, radius: f64, mat: Material },
}


impl HitRecord {
    ///
    /// Sets the hit record normal vector
    /// `outward_normal` is assumed to have unit length
    ///
    fn set_face_normal(&mut self, ray: Ray, outward_normal: Vec3) {
        self.front_face = ray.direction.dot(outward_normal) < 0.0;
        self.normal = if self.front_face { outward_normal } else { -outward_normal };
    }
}


impl Hittable {
    pub fn hit(&self, ray: Ray, t: Interval, rec: &mut HitRecord) -> bool {
        match self {
            Hittable::List(vec) => {
                let mut temp_rec = HitRecord::default();
                let mut hit_anything = false;
                let mut closest_so_far = t.max;

                for obj in vec.iter() {
                    if !obj.hit(ray, Interval::new(t.min, closest_so_far), &mut temp_rec) { continue }

                    hit_anything = true;
                    closest_so_far = temp_rec.t;
                    *rec = temp_rec.clone();
                }

                hit_anything
            },
 
            Hittable::Sphere { centre, radius, mat } => {
                let oc = ray.origin - *centre;
                let a = ray.direction.length_squared();
                let half_b = oc.dot(ray.direction);
                let c = oc.length_squared() - radius*radius;

                let discriminant = half_b*half_b - a*c;
                if discriminant < 0.0 { return false }
                
                let discriminant_sqrt = discriminant.sqrt();

                // Find the nearest root that lies in the acceptable range
                let mut root = (-half_b - discriminant_sqrt) / a;
                if !t.surrounds(root) {
                    root = (-half_b + discriminant_sqrt) / a;
                    if !t.surrounds(root) { return false }
                }

                rec.t = root;
                rec.point = ray.at(rec.t);
                let outward_normal = (rec.point - *centre) / *radius;
                rec.set_face_normal(ray, outward_normal);
                rec.material = *mat;

                true
            },


            Hittable::MovingSphere { centre, radius, mat } => {
                let current_centre = centre.at(ray.time);
                let oc = ray.origin - current_centre;
                let a = ray.direction.length_squared();
                let half_b = oc.dot(ray.direction);
                let c = oc.length_squared() - radius*radius;

                let discriminant = half_b*half_b - a*c;
                if discriminant < 0.0 { return false }
                
                let discriminant_sqrt = discriminant.sqrt();

                // Find the nearest root that lies in the acceptable range
                let mut root = (-half_b - discriminant_sqrt) / a;
                if !t.surrounds(root) {
                    root = (-half_b + discriminant_sqrt) / a;
                    if !t.surrounds(root) { return false }
                }

                rec.t = root;
                rec.point = ray.at(rec.t);
                let outward_normal = (rec.point - current_centre) / *radius;
                rec.set_face_normal(ray, outward_normal);
                rec.material = *mat;

                true

            },
        }
    }
}

