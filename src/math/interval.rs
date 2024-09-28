#[derive(Clone, Copy)]
pub struct Interval {
    pub min: f32,
    pub max: f32,
}


impl Interval {
    pub const EMPTY    : Self = Self::new( f32::INFINITY, -f32::INFINITY);
    pub const UNIVERSE : Self = Self::new(-f32::INFINITY,  f32::INFINITY);

    #[inline(always)]
    pub const fn new(min: f32, max: f32) -> Self { Self { min, max } }


    #[inline(always)]
    pub fn from_intervals(a: Interval, b: Interval) -> Interval {
        Self::new(
            a.min.min(b.min),
            a.max.max(b.max)
        )
    }


    #[inline(always)]
    pub fn contains(self, x: f32) -> bool { 
        self.min <= x && x <= self.max
    }


    #[inline(always)]
    pub fn surrounds(self, x: f32) -> bool { 
        self.min < x && x < self.max
    }


    #[inline(always)]
    pub fn clamp(self, x: f32) -> f32 {
        x.clamp(self.min, self.max)
    }


    pub fn size(self) -> f32 {
        (self.max - self.min).abs()
    }
}
