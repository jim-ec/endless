use std::{
    ops::{Add, Div, Mul, Range, Sub},
    time::Instant,
};

use cgmath::{vec3, Vector3};

pub fn profile<R>(label: &str, f: impl FnOnce() -> R) -> R {
    let t0 = Instant::now();
    let r = f();
    let t1 = Instant::now();
    let dt = t1 - t0;
    println!("{label} took {dt:.2?}");
    r
}

pub fn rescale<T>(x: T, from: Range<T>, to: Range<T>) -> T
where
    T: Copy + Add<T, Output = T> + Sub<T, Output = T> + Mul<T, Output = T> + Div<T, Output = T>,
{
    (x - from.start) * (to.end - to.start) / (from.end - from.start) + to.start
}

#[test]
fn test_rescale() {
    dbg!(rescale(0.0, -1.0..1.0, 1.0..2.0));
}

pub fn rgb(r: usize, g: usize, b: usize) -> Vector3<f32> {
    vec3(r, g, b).map(|x| x as f32 / 255.0)
}

pub const fn align(x: usize, align: usize) -> usize {
    (x + align - 1) & !(align - 1)
}

pub const fn stride_of<T>() -> usize {
    align(std::mem::size_of::<T>(), std::mem::align_of::<T>())
}

pub struct Counter {
    pub measures: Vec<f32>,
    pub smoothed: f32,
}

impl Default for Counter {
    fn default() -> Self {
        Self {
            measures: Vec::with_capacity(MAX_COUNTER_HISTORY),
            smoothed: f32::NAN,
        }
    }
}

pub const MAX_COUNTER_HISTORY: usize = 100;

impl Counter {
    pub fn push(&mut self, measure: f32) {
        self.measures.push(measure);
        if self.measures.len() > MAX_COUNTER_HISTORY {
            self.measures.remove(0);
        }

        if self.smoothed.is_nan() {
            self.smoothed = measure;
        } else {
            let stiffness = 0.2;
            self.smoothed = stiffness * measure + (1.0 - stiffness) * self.smoothed;
        }
    }
}

pub fn pack(color: Vector3<f32>) -> u32 {
    ((color.x * 255.0) as u32) << 16 | ((color.y * 255.0) as u32) << 8 | ((color.z * 255.0) as u32)
}
