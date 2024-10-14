use std::mem::MaybeUninit;

#[derive(Clone, Copy)]
pub struct SendPtr<T>(pub *mut T);
unsafe impl<T> Send for SendPtr<T> {}
unsafe impl<T> Sync for SendPtr<T> {}


pub struct Stack<T> {
    buffer: Vec<MaybeUninit<T>>,
    sp: usize,
}

impl<T> Stack<T> {
    pub fn new() -> Self {
        Self { buffer: Vec::new(), sp: 0 }
    }


    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.sp == 0
    }


    #[inline(always)]
    pub fn make_space_for(&mut self, n: usize) {
        if self.buffer.len() < self.sp + n {
            self.buffer.extend((0..n).map(|_| MaybeUninit::uninit()));
        }
    }


    #[inline(always)]
    pub fn push(&mut self, v: T) {
        self.make_space_for(1);
        unsafe { *self.buffer.get_unchecked_mut(self.sp) = MaybeUninit::new(v) };
        self.sp += 1;
    }


    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() { return None }
        self.sp -= 1;
        let val = unsafe { Some(self.buffer.get_unchecked(self.sp).assume_init_read()) };
        val
    }

    #[inline(always)]
    pub unsafe fn write_to(&mut self, n: usize, v: T) {
        self.buffer[n] = MaybeUninit::new(v);
    }


    #[inline(always)]
    pub unsafe fn inc_sp_by(&mut self, n: usize) {
        self.sp += n;
    }


    #[inline(always)]
    pub fn get_sp(&self) -> usize { self.sp }
}
