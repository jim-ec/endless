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

pub fn align(x: usize, align: usize) -> usize {
    (x + align - 1) & !(align - 1)
}
