fn hom(v: vec3<f32>) -> vec4<f32> {
    return vec4(v, 1.0);
}

fn dehom(v: vec4<f32>) -> vec3<f32> {
    return v.xyz / v.w;
}
