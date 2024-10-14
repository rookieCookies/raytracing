use core::cell::UnsafeCell;

use crate::math::interval::Interval;


thread_local! {
pub static SEED : UnsafeCell<[u64; 4]> = UnsafeCell::new([6, 9, 4, 20]);
}


#[inline(always)]
fn rotl(x: u64, k: u64) -> u64{
    (x << k) | (x >> (64 - k))
}


#[derive(Clone)]
pub struct Seed(pub [u64; 4]);


impl Seed {
    #[inline(always)]
    pub fn next(&mut self) -> u64 {
        let s = &mut self.0;
        let result = rotl(s[0].wrapping_add(s[3]), 23).wrapping_add(s[0]);
        
        let t = s[1] << 17;

        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];

        s[2] ^= t;

        s[3] = rotl(s[3], 45);

        result
    }


    #[inline(always)]
    pub fn next_f32(&mut self) -> f32 {
        const FRACTION_BITS : u64 = 52;

        let float_size = std::mem::size_of::<f64>() as u64 * 8;
        let precision : u64 = FRACTION_BITS + 1;
        let scale = 1.0 / ((1u64 << precision) as f64);

        let value : u64 = self.next();
        let value = value >> (float_size - precision);
        (scale * (value as f64)) as f32
    }


    #[inline(always)]
    pub fn next_f32_range(&mut self, r: Interval) -> f32 {
        r.min + (r.max - r.min) * self.next_f32()
    }
}
