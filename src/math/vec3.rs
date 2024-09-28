use std::{ops::{Sub, Neg, AddAssign, MulAssign, DivAssign, Add, Mul, Div}, fmt::Display};

use crate::rng::{next_f64, next_f64_range};

use super::{interval::Interval, matrix::Matrix};

pub type Point = Vec3;
pub type Colour = Vec3;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Vec3 {
    pub x : f64,
    pub y : f64,
    pub z : f64,
}


impl Vec3 {
    pub const ZERO : Vec3 = Vec3::new(0.0, 0.0, 0.0);

    #[inline(always)]
    pub const fn new(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3 { x, y, z }
    }

    #[inline(always)]
    pub fn random() -> Vec3 {
        Vec3 { x: next_f64(), y: next_f64(), z: next_f64() }
    }

    #[inline(always)]
    pub fn random_range(r: Interval) -> Vec3 {
        Vec3 { x: next_f64_range(r), y: next_f64_range(r), z: next_f64_range(r) }
    }

    #[inline(always)]
    pub fn random_in_unit_disk() -> Vec3 {
        let range = Interval::new(-1.0, 1.0);
        loop {
            let p = Vec3::new(next_f64_range(range), next_f64_range(range), 0.0);
            if p.length_squared() < 1.0 { return p }
        }
    }

    #[inline(always)]
    pub fn random_in_unit_sphere() -> Vec3 {
        loop {
            let p = Vec3::random_range(Interval::new(-1.0, 1.0));
            if p.length_squared() < 1.0 { return p }
        }
    }

    #[inline(always)]
    pub fn random_unit() -> Vec3 {
        Vec3::random_in_unit_sphere().unit()
    }

    #[inline(always)]
    pub fn random_on_hemisphere(normal: Vec3) -> Vec3 {
        let vec = Vec3::random_unit();

        if vec.dot(normal) > 0.0 { return vec }
        else { return -vec }
    }

    #[inline(always)]
    pub fn near_zero(self) -> bool {
        const TRESHOLD : f64 = 1e-8;
        self.x.abs() < TRESHOLD && self.y.abs() < TRESHOLD && self.z.abs() < TRESHOLD 
    }

    #[inline(always)]
    pub fn reflect(self, oth: Vec3) -> Vec3 {
        self - 2.0 * self.dot(oth) * oth
    }

    #[inline(always)]
    pub fn refract(self, n: Vec3, etai_over_etat: f64) -> Vec3 {
        let cos_theta = (-self).dot(n).min(1.0);
        let r_out_perp = etai_over_etat * (self + cos_theta*n);
        let r_out_parallel = -(1.0 - r_out_perp.length_squared()).abs().sqrt() * n;
        r_out_perp + r_out_parallel
    }

    #[inline(always)]
    pub fn length_squared(self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    #[inline(always)]
    pub fn length(self) -> f64 { self.length_squared().sqrt() }

    #[inline(always)]
    pub fn dot(self, rhs: Vec3) -> f64 {
        self.x * rhs.x +
        self.y * rhs.y +
        self.z * rhs.z
    }

    #[inline(always)]
    pub fn cross(self, rhs: Vec3) -> Vec3 {
        Self::new(self.y * rhs.z - self.z * rhs.y,
                  self.z * rhs.x - self.x * rhs.z,
                  self.x * rhs.y - self.y * rhs.x)
    }

    #[inline(always)]
    pub fn unit(self) -> Vec3 {
        self / self.length()
    }

    #[inline(always)]
    pub fn to_matrix(self) -> Matrix<4, 1, f64> {
        Matrix::new([
            [self.x],
            [self.y],
            [self.z],
            [1.0],
        ])
    }
}

impl Default for Vec3 {
    #[inline(always)]
    fn default() -> Self { Self::new(0.0, 0.0, 0.0) }
}


impl Neg for Vec3 {
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self::Output { Vec3::new(-self.x, -self.y, -self.z) }

}


impl AddAssign for Vec3 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}


impl MulAssign<f64> for Vec3 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: f64) {
        self.x *= rhs;
        self.y *= rhs;
        self.z *= rhs;
    }
}


impl DivAssign<f64> for Vec3 {
    #[inline(always)]
    fn div_assign(&mut self, rhs: f64) {
        *self *= 1.0 / rhs
    }
}


impl Display for Vec3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.x, self.y, self.z)
    }
}


impl Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}


impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}


impl Mul<Vec3> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.x * rhs.x, self.y * rhs.y, self.z * rhs.z)
    }
}


impl Mul<Vec3> for f64 {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Self::Output {
        Vec3::new(self * rhs.x, self * rhs.y, self * rhs.z)
    }
}


impl Div<f64> for Vec3 {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        (1.0 / rhs) * self
    }
}
