use std::ops::Sub;

use sti::arena::Arena;

use crate::{math::vec3::{Point, Vec3}, rng::{next, next_f32}};

#[derive(Clone, Copy)]
pub struct PerlinNoise<'a> {
    rand_floats: &'a [Vec3], // size == point_count
    perm_x: &'a [usize], // size == point_count
    perm_y: &'a [usize], // size == point_count
    perm_z: &'a [usize], // size == point_count
}


impl<'a> PerlinNoise<'a> {
    pub fn new(arena: &'a Arena, point_count: usize) -> Self {
        let mut rand_floats = sti::vec::Vec::with_cap_in(arena, point_count);
        let mut perm_x = sti::vec::Vec::with_cap_in(arena, point_count);
        let mut perm_y = sti::vec::Vec::with_cap_in(arena, point_count);
        let mut perm_z = sti::vec::Vec::with_cap_in(arena, point_count);

        for _ in 0..point_count {
            rand_floats.push(Vec3::random_unit());
        }

        perlin_generate_perm(&mut perm_x, point_count);
        perlin_generate_perm(&mut perm_y, point_count);
        perlin_generate_perm(&mut perm_z, point_count);

        Self {
            rand_floats: rand_floats.leak(),
            perm_x: perm_x.leak(),
            perm_y: perm_y.leak(),
            perm_z: perm_z.leak(),
        }
    }

    pub fn noise(&self, p: Vec3) -> f32 {
        let u = p.x - p.x.floor();
        let v = p.y - p.y.floor();
        let w = p.z - p.z.floor();




        let i = p.x.floor() as isize;
        let j = p.y.floor() as isize;
        let k = p.z.floor() as isize;

        let mut c = [[[Vec3::ZERO; 2]; 2]; 2];
        for di in 0..2isize {
            for dj in 0..2isize {
                for dk in 0..2isize {
                    c[di as usize][dj as usize][dk as usize] = self.rand_floats [
                        self.perm_x[((i+di) & 255) as usize % self.rand_floats.len()] ^
                        self.perm_y[((j+dj) & 255) as usize % self.rand_floats.len()] ^
                        self.perm_z[((k+dk) & 255) as usize % self.rand_floats.len()]
                    ];
                }
            }
        }

        let uu = u*u*(3.0-2.0*u);
        let vv = v*v*(3.0-2.0*v);
        let ww = w*w*(3.0-2.0*w);

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


fn perlin_generate_perm(p: &mut sti::vec::Vec<usize, &Arena>, point_count: usize) {
    debug_assert_eq!(p.len(), 0);

    for i in 0..point_count {
        p.push(i)
    }

    permute(p, point_count)
}


fn permute(p: &mut sti::vec::Vec<usize, &Arena>, point_count: usize) {
    for i in 0..point_count {
        let target = next() as usize % (i+1);
        p.swap(i, target)
    }
}

