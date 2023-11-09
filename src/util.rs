use std::{
    f32::consts::TAU,
    ops::{Add, Div, Mul, Range, Sub},
    time::Instant,
};

use cgmath::{vec2, vec3, InnerSpace, Vector2, Vector3};

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
    ((color.x * 255.0) as u32) | ((color.y * 255.0) as u32) << 8 | ((color.z * 255.0) as u32) << 16
}

pub fn xoshiro128(s: &mut [u32; 4]) -> u32 {
    let result = s[0].wrapping_add(s[3]);
    let t = s[1] << 9;
    s[2] ^= s[0];
    s[3] ^= s[1];
    s[1] ^= s[2];
    s[0] ^= s[3];
    s[2] ^= t;
    s[3] = s[3].rotate_left(11);
    result
}

pub fn knuth(n: u32) -> u32 {
    const KNUTH: u32 = 2654435769;
    (n.wrapping_mul(KNUTH)).rotate_right(17)
}

pub fn hash(keys: impl IntoIterator<Item = u32>) -> u32 {
    let mut hash = 0;
    for key in keys {
        hash = knuth(hash ^ key);
    }

    let mut state = [hash; 4];
    const ITER: usize = 3;
    for _ in 0..ITER {
        xoshiro128(&mut state);
    }

    state[0].wrapping_add(state[3])
}

/// Generate a pseudo-random number in [0, 1) by hashing the given keys.
pub fn random(keys: impl IntoIterator<Item = f32>) -> f32 {
    let k = hash(keys.into_iter().map(f32::to_bits));
    // Construct a positive floating point with 0 in the exponent.
    // This is effectively the bits representing positive one.
    let mut bits = 0x3F800000;
    // Fill the mantissa from a substring of the hash.
    // This will yield some number in [0, 1)
    bits |= k & 0x007FFFFF;
    f32::from_bits(bits) - 1.0
}

/// Generate a pseudo-random number in (-1, 1) by hashing the given keys.
pub fn random_signed(keys: impl IntoIterator<Item = f32>) -> f32 {
    let k = hash(keys.into_iter().map(f32::to_bits));
    // Construct a positive floating point with 0 in the exponent.
    // This is effectively the bits representing positive one.
    let mut bits = 0x3F800000;
    // Fill the mantissa from a substring of the hash.
    // This will yield some number in [0, 1)
    bits |= k & 0x007FFFFF;
    // Fill the sign bit from the hash.
    bits = f32::to_bits(f32::from_bits(bits) - 1.0);
    bits |= k & 0x80000000;
    f32::from_bits(bits)
}

/// Perlin noise in 2D, outputting values in the range (-1, 1).
pub fn perlin(p: Vector2<f32>) -> f32 {
    let p0 = p.map(f32::floor);
    let p1 = p0 + Vector2::new(1.0, 0.0);
    let p2 = p0 + Vector2::new(0.0, 1.0);
    let p3 = p0 + Vector2::new(1.0, 1.0);

    // Gradient direction vectors
    let d0 = TAU * random([p0.x, p0.y]);
    let d1 = TAU * random([p1.x, p1.y]);
    let d2 = TAU * random([p2.x, p2.y]);
    let d3 = TAU * random([p3.x, p3.y]);

    // Gradient vectors
    let g0 = vec2(d0.cos(), d0.sin());
    let g1 = vec2(d1.cos(), d1.sin());
    let g2 = vec2(d2.cos(), d2.sin());
    let g3 = vec2(d3.cos(), d3.sin());

    // Interpolate so that the first and second derivative is zero at the end points.
    fn interpolate(a: f32, b: f32, t: f32) -> f32 {
        ((t * (t * 6.0 - 15.0) + 10.0) * t * t * t) * (b - a) + a
    }

    let delta = p - p0;

    let i = (p - p0).dot(g0);
    let j = (p - p1).dot(g1);
    let k = (p - p2).dot(g2);
    let l = (p - p3).dot(g3);

    let u = interpolate(i, j, delta.x);
    let v = interpolate(k, l, delta.x);

    interpolate(u, v, delta.y)
}
