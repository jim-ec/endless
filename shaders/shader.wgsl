struct Camera {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    pos: vec4<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;

struct VoxelOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
}

@vertex
fn voxel_vertex(
    @location(0) vertex: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) position: vec3<f32>,
    @location(3) color: vec3<f32>,
) -> VoxelOut {
    var out: VoxelOut;

    let position = vec4(vertex + position, 1.0);
    out.clip_position = camera.proj * camera.view * position;
    out.position = position.xyz / position.w;

    out.color = clamp(color, vec3(0.0), vec3(1.0));

    out.normal = normal;

    return out;
}

@fragment
fn voxel_fragment(frag: VoxelOut) -> @location(0) vec4<f32> {
    let ambient_light = 0.05;
    let light_intensity = 1.0;
    
    let v = normalize(frag.position - camera.pos.xyz / camera.pos.w);
    let nov = clamp(dot(frag.normal, v), 0.0, 1.0);

    let color = nov * light_intensity * frag.color + ambient_light;
    return vec4(color, 1.0);
}

struct LineOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position: vec3<f32>,
}

@vertex
fn line_vertex(
    @location(0) position: vec3<f32>,
) -> LineOut {
    var frag: LineOut;

    let position = camera.view * vec4(position, 1.0);
    frag.clip_position = camera.proj * position;
    frag.position = position.xyz / position.w;

    return frag;
}

@fragment
fn line_fragment(frag: LineOut) -> @location(0) vec4<f32> {
    return vec4(1.0, 1.0, 0.0, 1.0);
}

@vertex
fn water_vertex(
    @location(0) position: vec3<f32>,
) -> @builtin(position) vec4<f32> {
    return camera.proj * camera.view * vec4(position, 1.0);
}

@fragment
fn water_fragment() -> @location(0) vec4<f32> {
    return vec4(0.0, 0.2, 1.0, 0.5);
}
