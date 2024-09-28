use super::{interval::Interval, ray::Ray, vec3::Point};

#[derive(Clone)]
pub struct AABB {
    x: Interval,
    y: Interval,
    z: Interval,
}


impl AABB {
    pub fn new(x: Interval, y: Interval, z: Interval) -> Self {
        Self {
            x, y, z,
        }
    }


    pub fn from_points(a: Point, b: Point) -> Self {
        let x = if a.x <= b.x { Interval::new(a.x, b.x) } else { Interval::new(b.x, a.x) };
        let y = if a.y <= b.y { Interval::new(a.y, b.y) } else { Interval::new(b.y, a.y) };
        let z = if a.z <= b.z { Interval::new(a.z, b.z) } else { Interval::new(b.z, a.z) };

        Self::new(x, y, z)
    }


    pub fn from_aabbs(box1: &AABB, box2: &AABB) -> AABB {
        Self::new(
            Interval::from_intervals(box1.x, box2.x),
            Interval::from_intervals(box1.y, box2.y),
            Interval::from_intervals(box1.z, box2.z),
        )

    }



    pub fn axis_interval(&self, n: usize) -> Interval {
        if n == 1 { return self.y }
        if n == 2 { return self.z }
        self.x
    }


    pub fn hit(&self, ray: Ray, mut ray_t: Interval) -> bool {
        let ray_origin = ray.origin;
        let ray_dir = ray.direction;

        for axis in 0..3 {
            let ax = self.axis_interval(axis);
            let adinv = 1.0 / ray_dir[axis];

            let t0 = (ax.min - ray_origin[axis]) * adinv;
            let t1 = (ax.max - ray_origin[axis]) * adinv;

            if t0 < t1 {
                if t0 > ray_t.min { ray_t.min = t0; }
                if t1 < ray_t.max { ray_t.max = t1; }
            } else {
                if t1 > ray_t.min { ray_t.min = t1; }
                if t0 < ray_t.max { ray_t.max = t0; }
            }

            if ray_t.max <= ray_t.min { return false }
        }

        true
    }

    pub fn longest_axis(&self) -> usize {
        if self.x.size() > self.y.size() { if self.x.size() > self.z.size() { 0 } else { 2 } }
        else { if self.y.size() > self.z.size() { 1 } else { 2 } }
    }
}
