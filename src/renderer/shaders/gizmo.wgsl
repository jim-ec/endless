struct Uniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    camera_translation: vec3<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return uniforms.proj * uniforms.view * hom(position);
}

@fragment
fn fragment() -> @location(0) vec4<f32> {
    return vec4(1.0, 1.0, 0.0, 1.0);
}
