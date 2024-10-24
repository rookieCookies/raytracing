use std::{fmt::Display, ops::{Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub}, simd::{cmp::SimdPartialOrd, f32x4, num::SimdFloat, StdFloat}};

use crate::rng::Seed;

use super::{interval::Interval, matrix::Matrix};

pub type Point = Vec3;
pub type Colour = Vec3;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Vec3 {
    // the 4th axis is 0
    pub axes: f32x4,
}


impl Vec3 {
    pub const ZERO : Vec3 = Vec3::new(0.0, 0.0, 0.0);
    pub const ONE  : Vec3 = Vec3::new(1.0, 1.0, 1.0);

    #[inline(always)]
    pub const fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { axes: f32x4::from_array([x, y, z, 0.0]) }
    }


    /// # Safety
    /// The 4th element should be 0.0
    #[inline(always)]
    pub unsafe fn new_simd(simd: f32x4) -> Vec3 {
        debug_assert_eq!(simd[3], 0.0);

        Vec3 { axes: simd }
    }


    #[inline(always)]
    pub fn random(seed: &mut Seed) -> Vec3 {
        Self::new(seed.next_f32(), seed.next_f32(), seed.next_f32())
    }

    #[inline(always)]
    pub fn random_range(seed: &mut Seed, r: Interval) -> Vec3 {
        Self::new(seed.next_f32_range(r), seed.next_f32_range(r), seed.next_f32_range(r))
    }

    #[inline(always)]
    pub fn random_in_unit_disk(seed: &mut Seed) -> Vec3 {
        let range = Interval::new(-1.0, 1.0);
        loop {
            let p = Vec3::new(seed.next_f32_range(range), seed.next_f32_range(range), 0.0);
            if p.length_squared() < 1.0 { return p }
        }
    }

    #[inline(always)]
    pub fn random_in_unit_sphere(seed: &mut Seed) -> Vec3 {
        loop {
            let p = Vec3::random_range(seed, Interval::new(-1.0, 1.0));
            if p.length_squared() < 1.0 { return p }
        }
    }

    #[inline(always)]
    pub fn random_unit(seed: &mut Seed) -> Vec3 {
        Vec3::random_in_unit_sphere(seed).unit()
    }

    #[inline(always)]
    pub fn random_on_hemisphere(seed: &mut Seed, normal: Vec3) -> Vec3 {
        let vec = Vec3::random_unit(seed);

        if vec.dot(normal) > 0.0 { return vec }
        else { return -vec }
    }

    #[inline(always)]
    pub fn near_zero(self) -> bool {
        const TRESHOLD : f32 = 1e-8;
        self.axes.abs().simd_lt(f32x4::splat(TRESHOLD)).all()
    }

    #[inline(always)]
    pub fn reflect(self, oth: Vec3) -> Vec3 {
        self - 2.0 * self.dot(oth) * oth
    }

    #[inline(always)]
    pub fn refract(self, n: Vec3, etai_over_etat: f32) -> Vec3 {
        let cos_theta = (-self).dot(n).min(1.0);
        let r_out_perp = etai_over_etat * (self + cos_theta*n);
        let r_out_parallel = -(1.0 - r_out_perp.length_squared()).abs().sqrt() * n;
        r_out_perp + r_out_parallel
    }

    #[inline(always)]
    pub fn length_squared(self) -> f32 {
        self.axes.mul(self.axes).reduce_sum()
    }

    #[inline(always)]
    pub fn length(self) -> f32 { self.length_squared().sqrt() }

    #[inline(always)]
    pub fn dot(self, rhs: Vec3) -> f32 {
        self.axes.mul(rhs.axes).reduce_sum()
    }

    #[inline(always)]
    pub fn cross(self, rhs: Vec3) -> Vec3 {
        let lyzx = f32x4::from_array([self.axes[1], self.axes[2], self.axes[0], 0.0]);
        let ryzx = f32x4::from_array([rhs.axes[1], rhs.axes[2], rhs.axes[0], 0.0]);
        let lzxy = f32x4::from_array([self.axes[2], self.axes[0], self.axes[1], 0.0]);
        let rzxy = f32x4::from_array([rhs.axes[2], rhs.axes[0], rhs.axes[1], 0.0]);

        let res = lyzx * rzxy - lzxy * ryzx;

        unsafe { Self::new_simd(res) }
    }


    #[inline(always)]
    pub fn unit(self) -> Vec3 {
        self / self.length()
    }


    #[inline(always)]
    pub fn to_matrix(self) -> Matrix<4, 1, f32> {
        Matrix::new([
            [self.axes[0]],
            [self.axes[1]],
            [self.axes[2]],
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
    fn neg(self) -> Self::Output { unsafe { Vec3::new_simd(self.axes.neg()) } }

}


impl AddAssign for Vec3 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.axes += rhs.axes;
    }
}


impl MulAssign<f32> for Vec3 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: f32) {
        self.axes *= f32x4::splat(rhs);
    }
}


impl DivAssign<f32> for Vec3 {
    #[inline(always)]
    fn div_assign(&mut self, rhs: f32) {
        *self *= 1.0 / rhs
    }
}


impl Display for Vec3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self[0], self[1], self[2])
    }
}


impl Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        unsafe { Self::new_simd(self.axes + rhs.axes) }
    }
}


impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        unsafe { Self::new_simd(self.axes - rhs.axes) }
    }
}


impl Mul<Vec3> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        unsafe { Self::new_simd(self.axes * rhs.axes) }
    }
}


impl Mul<Vec3> for f32 {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Self::Output {
        unsafe { Vec3::new_simd(f32x4::splat(self) * rhs.axes) }
    }
}


impl Div<f32> for Vec3 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        (1.0 / rhs) * self
    }
}


impl Index<usize> for Vec3 {
    type Output = f32;

    fn index(&self, index: usize) -> &Self::Output {
        if index == 0 { return &self.axes[0] }
        if index == 1 { return &self.axes[1] }
        if index == 2 { return &self.axes[2] }
        unreachable!()
    }
}

impl IndexMut<usize> for Vec3 {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index == 0 { return &mut self.axes[0] }
        if index == 1 { return &mut self.axes[1] }
        if index == 2 { return &mut self.axes[2] }
        unreachable!()
    }
}


impl Colour {
    pub fn to_rgba(mut self) -> u32 {
        self.axes = self.axes.sqrt();

        self.axes = self.axes * f32x4::splat(255.999);
        let rgb0 = self.axes.cast::<u32>();
        (rgb0[0] << 0) | (rgb0[1] << 8) | (rgb0[2] << 16)
    }
}
