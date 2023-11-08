struct Uniforms {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct In {
    @location(0) position: vec3<f32>,
    @location(1) color: u32,
}

struct Out {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vertex(in: In) -> Out {
    var out: Out;
    out.clip_position = uniforms.proj * uniforms.view * hom(in.position);
    out.color = unpack4x8unorm(in.color).rgb;
    return out;
}

@fragment
fn fragment(in: Out) -> @location(0) vec4<f32> {
    return vec4(in.color, 1.0);
}
