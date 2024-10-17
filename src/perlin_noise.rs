use std::{ops::Sub, simd::{f32x4, StdFloat}};

use sti::arena::Arena;

use crate::{math::vec3::{Point, Vec3}, rng::{Seed}};

#[derive(Clone, Copy)]
pub struct PerlinNoise<'a> {
    rand_floats: &'a [Vec3], // size == point_count
    perm_x: &'a [usize], // size == point_count
    perm_y: &'a [usize], // size == point_count
    perm_z: &'a [usize], // size == point_count
}


impl<'a> PerlinNoise<'a> {
    pub fn new(arena: &'a Arena, seed: &mut Seed, point_count: usize) -> Self {
        let mut rand_floats = sti::vec::Vec::with_cap_in(arena, point_count);
        let mut perm_x = sti::vec::Vec::with_cap_in(arena, point_count);
        let mut perm_y = sti::vec::Vec::with_cap_in(arena, point_count);
        let mut perm_z = sti::vec::Vec::with_cap_in(arena, point_count);

        for _ in 0..point_count {
            rand_floats.push(Vec3::random_unit(seed));
        }

        perlin_generate_perm(seed, &mut perm_x, point_count);
        perlin_generate_perm(seed, &mut perm_y, point_count);
        perlin_generate_perm(seed, &mut perm_z, point_count);

        Self {
            rand_floats: rand_floats.leak(),
            perm_x: perm_x.leak(),
            perm_y: perm_y.leak(),
            perm_z: perm_z.leak(),
        }
    }

    pub fn noise(&self, p: Vec3) -> f32 {
        let uvw = p.axes - p.axes.floor();
        let ijk = p.axes.floor();

        let u = uvw[0];
        let v = uvw[1];
        let w = uvw[2];

        let i = ijk[0] as isize;
        let j = ijk[1] as isize;
        let k = ijk[2] as isize;

        let mut c = [[[Vec3::ZERO; 2]; 2]; 2];
        for di in 0..2isize {
            for dj in 0..2isize {
                for dk in 0..2isize {
                    c[di as usize][dj as usize][dk as usize] = self.rand_floats[
                        (
                            self.perm_x[((i+di) & 255) as usize % self.rand_floats.len()] ^
                            self.perm_y[((j+dj) & 255) as usize % self.rand_floats.len()] ^
                            self.perm_z[((k+dk) & 255) as usize % self.rand_floats.len()]
                        ) % self.rand_floats.len()
                    ];
                }
            }
        }

        let uuvvww = uvw * uvw * (f32x4::splat(3.0) - f32x4::splat(2.0)*uvw);
        let uu = uuvvww[0];
        let vv = uuvvww[1];
        let ww = uuvvww[2];

        let mut accum = 0.0;

        for i in 0..2usize {
            for j in 0..2usize {
                for k in 0..2usize {
                    let weight_v = Vec3::new(u - i as f32, v - j as f32, w - k as f32);
                    accum += (i as f32 * uu + (1.0 - i as f32) * (1.0 - uu))
                              * (j as f32 * vv + (1.0 - j as f32) * (1.0 - vv))
                              * (k as f32 * ww + (1.0 - k as f32) * (1.0 - ww))
                              * c[i][j][k].dot(weight_v);

                }
            }
        }

        accum
    }


    pub fn turbulance(&self, p: Point, depth: usize) -> f32 {
        let mut accum = 0.0;
        let mut temp_p = p;
        let mut weight = 1.0;

        for _ in 0..depth {
            accum += weight * self.noise(temp_p);
            weight *= 0.5;
            temp_p *= 2.0;
        }

        accum.abs()
    }
}


fn perlin_generate_perm(seed: &mut Seed, p: &mut sti::vec::Vec<usize, &Arena>, point_count: usize) {
    debug_assert_eq!(p.len(), 0);

    for i in 0..point_count {
        p.push(i)
    }

    permute(seed, p, point_count)
}


fn permute(seed: &mut Seed, p: &mut sti::vec::Vec<usize, &Arena>, point_count: usize) {
    for i in 0..point_count {
        let target = seed.next() as usize % (i+1);
        p.swap(i, target)
    }
}

