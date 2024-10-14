use std::{cmp::Ordering, f32::consts::PI, marker::PhantomData};

use sti::arena::Arena;

use crate::{material::Material, math::{aabb::{AABBx2, AABBx4, AABB}, interval::Interval, ray::Ray, vec3::{Point, Vec3}}};

#[derive(Clone, Default)]
pub struct HitRecord<'a> {
    pub point: Point,
    pub normal: Vec3,
    pub t: f32,
    pub front_face: bool,
    pub material: Material<'a>,
    pub u: f32,
    pub v: f32,
}
impl HitRecord<'_> {
    ///
    /// Sets the hit record normal vector
    /// `outward_normal` is assumed to have unit length
    ///
    fn set_face_normal(&mut self, ray: &Ray, outward_normal: Vec3) {
        self.front_face = ray.direction.dot(outward_normal) < 0.0;
        self.normal = if self.front_face { outward_normal } else { -outward_normal };
    }
}


#[derive(Clone)]
pub struct Hittable<'a> {
    pub kind: HittableKind<'a>,
}


#[derive(Clone)]
pub enum HittableKind<'a> {
    Sphere(Sphere<'a>),
    MovingSphere(MovingSphere<'a>),
    BVH {
        /*
        aabbs: AABBx4,
        regions: [Option<&'a Hittable<'a>>; 4],
        */
        aabbs: AABBx2,
        left: &'a Hittable<'a>,
        right: Option<&'a Hittable<'a>>,
    },
}


impl<'a> Hittable<'a> {
    pub fn sphere(sphere: Sphere<'a>) -> Self {
        Self { kind: HittableKind::Sphere(sphere) }
    }


    pub fn moving_sphere(sphere: MovingSphere<'a>) -> Self {
        Self { kind: HittableKind::MovingSphere(sphere) }
    }


    pub fn bvh(arena: &'a Arena, hittables: &'a [Hittable<'a>]) -> Self {
        fn box_comp(a: &Hittable, b: &Hittable, axis: usize) -> bool {
            let a_axis_interval = a.calc_aabb().axis_interval(axis);
            let b_axis_interval = b.calc_aabb().axis_interval(axis);

            a_axis_interval.min < b_axis_interval.min
        }

        let mut aabb = AABB::new(Interval::EMPTY, Interval::EMPTY, Interval::EMPTY);
        for l in hittables {
            aabb = AABB::from_aabbs(&aabb, &l.calc_aabb());
        }

        let axis = aabb.longest_axis();

        let r1;
        let r2;
        //let r3 : Option<&Hittable>;
        //let r4 : Option<&Hittable>;
        if hittables.len() == 1 {
            r1 = &hittables[0];
            r2 = None;
        } else if hittables.len() == 2 {
            r1 = &hittables[0];
            r2 = Some(&hittables[1]);
            /*
            {
                r3 = None;
                r4 = None;
            }
            */
        } else if hittables.len() == 3 {
            r1 = &hittables[0];
            r2 = Some(&hittables[1]);
            /*
            {
                r3 = Some(&hittables[3]);
                r4 = None;
            }
            */
        } /*else if hittables.len() == 4 {
            r1 = &hittables[0];
            r2 = Some(&hittables[1]);
            #[cfg(feature="aabbx4")]
            {
                r3 = Some(&hittables[3]);
                r4 = Some(&hittables[4]);
            }
        }*/ else {
            let mut list = sti::vec::Vec::from_slice_in(arena, hittables);
            list.sort_by(|a, b| if box_comp(a, b, axis) { Ordering::Less } else { Ordering::Greater });

            let middle = list.len() / 2;
            let list = list.leak().split_at(middle);
            /*
            {
                let list = (list.0.split_at(list.0.len()/2), list.1.split_at(list.1.len()/2));

                r1 = arena.alloc_new(Hittable::bvh(arena, list.0.0));
                r2 = Some(arena.alloc_new(Hittable::bvh(arena, list.0.1)));
                r3 = Some(arena.alloc_new(Hittable::bvh(arena, list.1.0)));
                r4 = Some(arena.alloc_new(Hittable::bvh(arena, list.1.1)));
            }
            */

            r1 = arena.alloc_new(Hittable::bvh(arena, list.0));
            r2 = Some(arena.alloc_new(Hittable::bvh(arena, list.1)));
        }

        Hittable {
            kind: HittableKind::BVH {
                aabbs: AABBx2::new(r1.calc_aabb(), r2.map(|x| x.calc_aabb()).unwrap_or(AABB::EMPTY)),
                left: r1,
                right: r2,

                /*
                regions: [Some(r1), r2, r3, r4],
                aabbs: AABBx4::new(r1.calc_aabb(),
                        r2.map(Hittable::calc_aabb).unwrap_or(AABB::EMPTY),
                        r3.map(Hittable::calc_aabb).unwrap_or(AABB::EMPTY),
                        r4.map(Hittable::calc_aabb).unwrap_or(AABB::EMPTY))
                        */
            },
        }
    }


    pub fn calc_aabb(&self) -> AABB {
        match &self.kind {
            HittableKind::Sphere(sphere) => {
                let rvec = Vec3::new(sphere.radius, sphere.radius, sphere.radius);
                AABB::from_points(sphere.centre - rvec, sphere.centre + rvec)
            },


            HittableKind::MovingSphere(sphere) => {
                let rvec = Vec3::new(sphere.radius, sphere.radius, sphere.radius);
                let box1 = AABB::from_points(sphere.centre.at(0.0) - rvec, sphere.centre.at(0.0) + rvec);
                let box2 = AABB::from_points(sphere.centre.at(1.0) - rvec, sphere.centre.at(1.0) + rvec);
                AABB::from_aabbs(&box1, &box2)
            },


            HittableKind::BVH { aabbs, .. } => AABB::from_aabbs(&aabbs.aabb1(), &aabbs.aabb2()),
        }
    }
}


#[derive(Clone)]
pub struct Sphere<'a> {
    centre: Point,
    radius: f32,
    material: Material<'a>,
}


impl<'a> Sphere<'a> {
    pub fn new(centre: Point, radius: f32, material: Material<'a>) -> Self {
        Self { centre, radius, material }
    }


    pub fn hit(&self, ray: &Ray, t: Interval, rec: &mut HitRecord<'a>) -> bool {
        let oc = ray.origin - self.centre;
        let a = ray.direction.length_squared();
        let half_b = oc.dot(ray.direction);
        let c = oc.length_squared() - self.radius*self.radius;

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
        let outward_normal = (rec.point - self.centre) / self.radius;
        rec.set_face_normal(&ray, outward_normal);
        (rec.u, rec.v) = get_sphere_uv(outward_normal);
        rec.material = self.material;

        true
    }

}


#[derive(Clone)]
pub struct MovingSphere<'a> {
    centre: Ray,
    radius: f32,
    material: Material<'a>,
}


impl<'a> MovingSphere<'a> {
    pub fn new(centre_1: Point, centre_2: Point, radius: f32, material: Material<'a>) -> Self {
        let centre = Ray::new(centre_1, centre_2 - centre_1, 0.0);

        Self { centre, radius, material }
    }


    pub fn hit(&self, ray: &Ray, t: Interval, rec: &mut HitRecord<'a>) -> bool {
        let current_centre = self.centre.at(ray.time);
        let oc = ray.origin - current_centre;
        let a = ray.direction.length_squared();
        let half_b = oc.dot(ray.direction);
        let c = oc.length_squared() - self.radius*self.radius;

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
        let outward_normal = (rec.point - current_centre) / self.radius;
        rec.set_face_normal(ray, outward_normal);
        (rec.u, rec.v) = get_sphere_uv(outward_normal);
        rec.material = self.material;

        true
    }

}


fn get_sphere_uv(p: Point) -> (f32, f32) {
    // p: a given point on the sphere of radius one, centered at the origin.
    // u: returned value [0,1] of angle around the Y axis from X=-1.
    // v: returned value [0,1] of angle from Y=-1 to Y=+1.
    //     <1 0 0> yields <0.50 0.50>       <-1  0  0> yields <0.00 0.50>
    //     <0 1 0> yields <0.50 1.00>       < 0 -1  0> yields <0.50 0.00>
    //     <0 0 1> yields <0.25 0.50>       < 0  0 -1> yields <0.75 0.50>

    let theta = (-p[1]).acos();
    let phi = (-p[2]).atan2(p[0]) + PI;
    (phi/(2.0*PI), theta/PI)
}
