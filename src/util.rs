use std::{
    ops::{Div, MulAssign, Range, Sub, SubAssign},
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

pub fn rescale<T>(mut x: T, from: Range<T>, to: Range<T>) -> T
where
    T: Copy + SubAssign + MulAssign + Sub<T, Output = T> + Div<T, Output = T>,
{
    x -= from.start;
    x *= (to.end - to.start) / (from.end - from.start);
    x
}

pub fn rgb(r: usize, g: usize, b: usize) -> Vector3<f32> {
    vec3(r, g, b).map(|x| x as f32 / 255.0)
}
