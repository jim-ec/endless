struct Uniforms {
    model: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    camera_translation: vec3<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct In {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
}

struct Out {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
}

@vertex
fn vertex(in: In) -> Out {
    var out: Out;

    out.clip_position = uniforms.proj * uniforms.view * uniforms.model * hom(in.position);
    out.position = dehom(uniforms.model * hom(in.position));

    out.color = clamp(in.color, vec3(0.0), vec3(1.0));

    out.normal = in.normal;

    return out;
}

@fragment
fn fragment(out: Out) -> @location(0) vec4<f32> {
    let ambient_light = 0.05;
    let light_intensity = 1.0;
    
    let v = normalize(out.position - uniforms.camera_translation);
    let nov = clamp(dot(out.normal, v), 0.0, 1.0);

    let color = nov * light_intensity * out.color + ambient_light;
    return vec4(color, 1.0);
}
