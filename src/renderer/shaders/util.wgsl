fn hom(v: vec3<f32>) -> vec4<f32> {
    return vec4(v, 1.0);
}

fn dehom(v: vec4<f32>) -> vec3<f32> {
    return v.xyz / v.w;
}

fn unpack(n: u32) -> vec3<f32> {
    let r = f32((n & (0xffu << 16u)) >> 16u) / 255.0;
    let g = f32((n & (0xffu << 8u)) >> 8u) / 255.0;
    let b = f32(n & 0xffu) / 255.0;
    return vec3(r, g, b);
}
