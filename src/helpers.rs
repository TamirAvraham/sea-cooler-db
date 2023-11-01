use std::{cell::Cell, sync::Once, ops::Deref};
/* 
pub struct Lazy<T, F = fn() -> T> {
    value: Option<T>,
    once: Once,
    init: F,
}

impl<T, F> Lazy<T, F> where F: Copy+FnOnce()->T {
    pub const fn new(f: F) -> Self {
        Self {
            value: None,
            once: Once::new(),
            init: f,
        }
    }
    pub fn force(this: &Lazy<T, F>) -> &T {
        this.once.call_once(|| {
            this.value=Some((this.init)());
        });
        match this.value {
            Some(v) => &v,
            None => panic!("init function "),
        }
    }
    pub fn get(&self)->&T{
        match self.value {
            Some(v) => &v,
            None => Self::force(&self),
        }
    }
}
impl<T:Clone,F> Lazy<T,F> where F: FnOnce()->T{
    pub fn get_clone(&self)->T{
        self.get().clone()
    }
}
impl<T,F> Deref for Lazy<T,F> where F: FnOnce()->T{
    type Target = T;

    fn deref(&self) -> &T {
        Self::force(&self)
    }
}
*/