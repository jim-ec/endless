struct Camera {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    pos: vec4<f32>,
}

struct Mesh {
    transform: mat4x4<f32>,
    color: vec3<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> mesh: Mesh;

struct Fragment {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position: vec3<f32>,
}

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> Fragment {
    var frag: Fragment;

    let position = mesh.transform * vec4(position, 1.0);
    frag.clip_position = camera.proj * camera.view * position;
    frag.position = position.xyz / position.w;

    return frag;
}

@fragment
fn fs_main(frag: Fragment) -> @location(0) vec4<f32> {
    let ambient_light = 0.005;
    let light_intensity = 2.0;
    
    let n = normalize(cross(dpdx(frag.position), dpdy(frag.position)));
    let v = normalize(frag.position - camera.pos.xyz / camera.pos.w);
    let nov = dot(n, v);

    let color = nov * light_intensity * mesh.color + ambient_light;
    return vec4(color, 1.0);
}
