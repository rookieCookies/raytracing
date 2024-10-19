use std::{cmp::Ordering, f32::{consts::PI, INFINITY, NEG_INFINITY}, marker::PhantomData, simd::{f32x4, num::SimdFloat}};

use sti::arena::Arena;

use crate::{material::Material, math::{aabb::{AABBx2, AABBx4, AABB}, interval::Interval, ray::Ray, vec3::{Point, Vec3}}, texture::Texture};

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
    Quad(Quad<'a>),
    ConstantMedium(ConstantMedium<'a>),
    BVH {
        /*
        aabbs: AABBx4,
        regions: [Option<&'a Hittable<'a>>; 4],
        */
        aabbs: AABBx2,
        left: &'a Hittable<'a>,
        right: Option<&'a Hittable<'a>>,
    },

    Move {
        obj: &'a Hittable<'a>,
        offset: Vec3,
    },

    RotateY {
        obj: &'a Hittable<'a>,
        sin: f32,
        cos: f32,
    },

    List(&'a [Hittable<'a>]),
}


impl<'a> Hittable<'a> {
    pub fn sphere(sphere: Sphere<'a>) -> Self {
        Self { kind: HittableKind::Sphere(sphere) }
    }


    pub fn quad(quad: Quad<'a>) -> Self {
        Self { kind: HittableKind::Quad(quad) }
    }


    pub fn constant_medium(constant_medium: ConstantMedium<'a>) -> Self {
        Self { kind: HittableKind::ConstantMedium(constant_medium) }
    }


    pub fn moving_sphere(sphere: MovingSphere<'a>) -> Self {
        Self { kind: HittableKind::MovingSphere(sphere) }
    }


    pub fn box_of_quads(arena: &'a Arena, a: Point, b: Point, mat: Material<'a>) -> Hittable<'a> {
        let min = unsafe { Point::new_simd(a.axes.simd_min(b.axes)) };
        let max = unsafe { Point::new_simd(a.axes.simd_max(b.axes)) };

        let dx = Vec3::new(max[0] - min[0], 0.0, 0.0);
        let dy = Vec3::new(0.0, max[1] - min[1], 0.0);
        let dz = Vec3::new(0.0, 0.0, max[2] - min[2]);

        let vertexes = arena.alloc_new([
            Hittable::quad(Quad::new(Point::new( min[0],  min[1],  max[2]),  dx,  dy, mat)), // front
            Hittable::quad(Quad::new(Point::new( max[0],  min[1],  max[2]), -dz,  dy, mat)), // right
            Hittable::quad(Quad::new(Point::new( max[0],  min[1],  min[2]), -dx,  dy, mat)), // back
            Hittable::quad(Quad::new(Point::new( min[0],  min[1],  min[2]),  dz,  dy, mat)), // left
            Hittable::quad(Quad::new(Point::new( min[0],  max[1],  max[2]),  dx, -dz, mat)), // top
            Hittable::quad(Quad::new(Point::new( min[0],  min[1],  min[2]),  dx,  dz, mat)), // bottom
        ]);

        let list = Hittable::bvh(arena, vertexes);
        list
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
        if hittables.len() == 1 {
            r1 = &hittables[0];
            r2 = None;
        } else if hittables.len() == 2 {
            r1 = &hittables[0];
            r2 = Some(&hittables[1]);
        } else {
            let mut list = sti::vec::Vec::from_slice_in(arena, hittables);
            list.sort_by(|a, b| if box_comp(a, b, axis) { Ordering::Less } else { Ordering::Greater });

            let middle = list.len() / 2;
            let list = list.leak().split_at(middle);

            r1 = arena.alloc_new(Hittable::bvh(arena, list.0));
            r2 = Some(arena.alloc_new(Hittable::bvh(arena, list.1)));
        }

        let hittable = Hittable {
            kind: HittableKind::BVH {
                aabbs: AABBx2::new(r1.calc_aabb(), r2.map(|x| x.calc_aabb()).unwrap_or(AABB::empty())),
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
        };

        assert_eq!(hittable.calc_aabb(), aabb);
        hittable
    }


    pub fn list(list: &'a [Hittable<'a>]) -> Hittable<'a> {
        Hittable {
            kind: HittableKind::List(list),
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


            HittableKind::Quad(quad) => {
                let bbox_diag1 = AABB::from_points(quad.q, quad.q + quad.u + quad.v);
                let bbox_diag2 = AABB::from_points(quad.q + quad.u, quad.q + quad.v);
                AABB::from_aabbs(&bbox_diag1, &bbox_diag2)
            },


            HittableKind::BVH { aabbs, right, left } => {
                if right.is_some() {
                    AABB::from_aabbs(&aabbs.aabb1(), &aabbs.aabb2())
                } else {
                    aabbs.aabb1()
                }
            },


            HittableKind::ConstantMedium(constant_medium) => constant_medium.boundary.calc_aabb(),


            HittableKind::Move { obj, offset } => obj.calc_aabb().offset(*offset),


            HittableKind::RotateY { obj, sin, cos } => {
                let mut min = Point::new(INFINITY, INFINITY, INFINITY);
                let mut max = Point::new(NEG_INFINITY, NEG_INFINITY, NEG_INFINITY);
                let bbox = obj.calc_aabb();

                for i in 0..2 {
                    for j in 0..2 {
                        for k in 0..2 {
                            let i = i as f32;
                            let j = j as f32;
                            let k = k as f32;

                            let x = i*bbox.x().max + (1.0 - i)*bbox.x().min;
                            let y = j*bbox.y().max + (1.0 - j)*bbox.y().min;
                            let z = k*bbox.z().max + (1.0 - k)*bbox.z().min;

                            let newx =  cos*x + sin*z;
                            let newz = -sin*x + cos*z;

                            let tester = Vec3::new(newx, y, newz);
                            for c in 0..3 {
                                min[c] = min[c].min(tester[c]);
                                max[c] = max[c].max(tester[c]);
                            }
                        }
                    }
                }

                AABB::from_points(min, max)
            },


            HittableKind::List(hittables) => {
                let mut aabb = AABB::new(Interval::EMPTY, Interval::EMPTY, Interval::EMPTY);
                for l in hittables.iter() {
                    aabb = AABB::from_aabbs(&aabb, &l.calc_aabb());
                }
                aabb
            }


        }
    }


    pub fn move_by(self, arena: &'a Arena, offset: Vec3) -> Hittable<'a> {
        Hittable {
            kind: HittableKind::Move { obj: arena.alloc_new(self), offset },
        }
    }


    pub fn rotate_y_by(self, arena: &'a Arena, offset: f32) -> Hittable<'a> {
        let rads = offset.to_radians();

        let sin = rads.sin();
        let cos = rads.cos();

        Hittable {
            kind: HittableKind::RotateY { obj: arena.alloc_new(self), sin, cos },
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


#[derive(Clone)]
pub struct Quad<'a> {
    q: Point,
    u: Vec3,
    v: Vec3,
    w: Vec3,
    normal: Vec3,
    d: f32,
    material: Material<'a>
}

impl<'a> Quad<'a> {
    pub fn new(q: Point, u: Vec3, v: Vec3, material: Material<'a>) -> Self {
        let n = u.cross(v);
        let normal = n.unit();
        let d = normal.dot(q);
        let w = n / n.dot(n);
        Self { q, u, v, normal, d, material, w }
    }



    pub fn hit(&self, ray: &Ray, ray_t: Interval, rec: &mut HitRecord<'a>) -> bool {
        let denom = self.normal.dot(ray.direction);

        if denom.abs() < 1e-8 {
            return false;
        }

        let t = (self.d - self.normal.dot(ray.origin)) / denom;
        if !ray_t.contains(t) {
            return false;
        }

        let intersection = ray.at(t);
        let planar_hitpt_vec = intersection - self.q;
        let alpha = self.w.dot(planar_hitpt_vec.cross(self.v));
        let beta = self.w.dot(self.u.cross(planar_hitpt_vec));

        {
            // Given the hit point in plane coordinates, return false if it is outside the
            // primitive, otherwise set the hit record UV coordinates and return true.
            if !Interval::UNIT.contains(alpha) || !Interval::UNIT.contains(beta) {
                return false;
            }
        }

        rec.t = t;
        rec.u = alpha;
        rec.v = beta;
        rec.point = intersection;
        rec.material = self.material;
        rec.set_face_normal(ray, self.normal);

        true
    }
}


#[derive(Clone)]
pub struct ConstantMedium<'a> {
    pub phase_function : Material<'a>,
    pub boundary: &'a Hittable<'a>,
    pub neg_inv_density: f32,
}

impl<'a> ConstantMedium<'a> {
    pub fn new(boundary: &'a Hittable<'a>, density: f32, texture: Texture<'a>) -> Self {
        let phase_function = Material::isotropic(texture);
        let neg_inv_density = -1.0 / density;
        Self { phase_function, boundary, neg_inv_density }
    }
}


