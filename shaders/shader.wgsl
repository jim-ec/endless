struct Camera {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    pos: vec4<f32>,
}

struct Mesh {
    transform: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> mesh: Mesh;

struct VertexInput {
    @location(0) vertex: vec3<f32>,
    @location(1) position: vec3<f32>,
    @location(2) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let position = mesh.transform * vec4(in.vertex + in.position, 1.0);
    out.clip_position = camera.proj * camera.view * position;
    out.position = position.xyz / position.w;

    out.color = in.color;

    return out;
}

@fragment
fn fs_main(frag: VertexOutput) -> @location(0) vec4<f32> {
    let ambient_light = 0.05;
    let light_intensity = 1.0;
    
    let n = normalize(cross(dpdx(frag.position), dpdy(frag.position)));
    let v = normalize(frag.position - camera.pos.xyz / camera.pos.w);
    let nov = dot(n, v);

    let color = nov * light_intensity * frag.color + ambient_light;
    return vec4(color, 1.0);
}
