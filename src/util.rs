use std::{ops::Range, time::Instant};

use cgmath::{vec3, Vector3};

pub fn perf<R, F: FnOnce() -> R>(label: &str, f: F) -> R {
    let t0 = Instant::now();
    let r = f();
    let t1 = Instant::now();
    let dt = t1 - t0;
    println!("{label} took {dt:.3?}");
    r
}

pub fn rescale(mut x: f64, from: Range<f64>, to: Range<f64>) -> f64 {
    x -= from.start;
    x *= (to.end - to.start) / (from.end - from.start);
    x
}

pub fn rgb(r: usize, g: usize, b: usize) -> Vector3<f32> {
    vec3(r, g, b).map(|x| x as f32 / 255.0)
}
