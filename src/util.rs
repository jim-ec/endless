use std::{ops::Range, time::Instant};

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
