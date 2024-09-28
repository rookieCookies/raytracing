use std::cmp::Ordering;

use sti::{arena::Arena, traits::FromIn};

use crate::{math::{aabb::AABB, interval::Interval, ray::Ray, vec3::{Point, Vec3}}, rng::next, rt::materials::Material};

#[derive(Clone, Default)]
pub struct HitRecord {
    pub point: Point,
    pub normal: Vec3,
    pub t: f32,
    pub front_face: bool,
    pub material: Material,
}


#[derive(Clone)]
pub struct Hittable<'a> {
    aabb: AABB,
    kind: HittableKind<'a>,
}


#[derive(Clone)]
pub enum HittableKind<'a> {
    List(&'a [Hittable<'a>]),
    Sphere { centre: Point, radius: f32, mat: Material },
    MovingSphere { centre: Ray, radius: f32, mat: Material },
    BVH { left: &'a Hittable<'a>, right: &'a Hittable<'a> }
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



impl Hittable<'_> {
    pub fn sphere<'a>(centre: Point, radius: f32, mat: Material) -> Hittable<'a> {
        let rvec = Vec3::new(radius, radius, radius);
        let aabb = AABB::from_points(centre - rvec, centre + rvec);
        Hittable {
            aabb,
            kind: HittableKind::Sphere { centre, radius, mat },
        }
    }


    pub fn moving_sphere<'a>(centre1: Point, centre2: Point, radius: f32, mat: Material) -> Hittable<'a> {
        let centre = Ray::new(centre1, centre2 - centre1, 0.0);

        let rvec = Vec3::new(radius, radius, radius);
        let box1 = AABB::from_points(centre.at(0.0) - rvec, centre.at(0.0) + rvec);
        let box2 = AABB::from_points(centre.at(1.0) - rvec, centre.at(1.0) + rvec);
        let aabb = AABB::from_aabbs(&box1, &box2);

        Hittable {
            aabb,
            kind: HittableKind::MovingSphere { centre, radius, mat },
        }
    }


    pub fn list<'a>(list: &'a [Hittable<'a>]) -> Hittable<'a> {
        let mut aabb = AABB::new(Interval::EMPTY, Interval::EMPTY, Interval::EMPTY);

        for l in list {
            aabb = AABB::from_aabbs(&aabb, l.bounding_box());
        }

        Hittable {
            aabb,
            kind: HittableKind::List(list),
        }
    }

    pub fn bvh<'a>(arena: &'a Arena, list: &'a [Hittable<'a>]) -> Hittable<'a> {
        fn box_comp(a: &Hittable, b: &Hittable, axis: usize) -> bool {
            let a_axis_interval = a.bounding_box().axis_interval(axis);
            let b_axis_interval = b.bounding_box().axis_interval(axis);

            a_axis_interval.min < b_axis_interval.min
        }

        let mut aabb = AABB::new(Interval::EMPTY, Interval::EMPTY, Interval::EMPTY);
        for l in list {
            aabb = AABB::from_aabbs(&aabb, l.bounding_box());
        }

        let axis = aabb.longest_axis();

        let left;
        let right;
        if list.len() == 1 {
            left = list[0].clone();
            right = left.clone();
        } else if list.len() == 2 {
            left = list[0].clone();
            right = list[1].clone();
        } else {
            let mut list = sti::vec::Vec::from_slice_in(arena, list);
            list.sort_by(|a, b| if box_comp(a, b, axis) { Ordering::Less } else { Ordering::Greater });

            let middle = list.len() / 2;
            let list = list.leak().split_at(middle);

            left = Hittable::bvh(arena, list.0);
            right = Hittable::bvh(arena, list.1);
        }

        Hittable {
            aabb,
            kind: HittableKind::BVH { left: arena.alloc_new(left), right: arena.alloc_new(right) }
        }
    }


    pub fn hit(&self, ray: Ray, t: Interval, rec: &mut HitRecord) -> bool {
        match &self.kind {
            HittableKind::List(vec) => {
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
 
            HittableKind::Sphere { centre, radius, mat } => {
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


            HittableKind::MovingSphere { centre, radius, mat } => {
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


            HittableKind::BVH { left, right } => {
                if !self.bounding_box().hit(ray, t) {
                    return false;
                }

                let hit_left = left.hit(ray, t, rec);
                let hit_right = right.hit(ray, Interval::new(t.min, if hit_left { rec.t } else { t.max }), rec);

                hit_left || hit_right
            }
        }
    }


    pub fn bounding_box(&self) -> &AABB {
        &self.aabb
    }
}

