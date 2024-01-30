#[derive(Clone, Copy)]
pub struct Interval {
    pub min: f64,
    pub max: f64,
}


impl Interval {
    pub const EMPTY    : Self = Self::new( f64::INFINITY, -f64::INFINITY);
    pub const UNIVERSE : Self = Self::new(-f64::INFINITY,  f64::INFINITY);

    #[inline(always)]
    pub const fn new(min: f64, max: f64) -> Self { Self { min, max } }

    #[inline(always)]
    pub fn contains(self, x: f64) -> bool { 
        self.min <= x && x <= self.max
    }

    #[inline(always)]
    pub fn surrounds(self, x: f64) -> bool { 
        self.min < x && x < self.max
    }

    #[inline(always)]
    pub fn clamp(self, x: f64) -> f64 {
        x.clamp(self.min, self.max)
    }
}
