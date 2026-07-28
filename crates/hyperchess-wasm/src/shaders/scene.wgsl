// Shared shader for board tiles, highlight markers, and piece meshes: per-vertex
// position/normal, per-instance model matrix + tint color, one directional light.

struct Camera {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) model_0: vec4<f32>,
    @location(3) model_1: vec4<f32>,
    @location(4) model_2: vec4<f32>,
    @location(5) model_3: vec4<f32>,
    @location(6) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    let model = mat4x4<f32>(in.model_0, in.model_1, in.model_2, in.model_3);
    var out: VertexOut;
    out.clip_position = camera.view_proj * model * vec4<f32>(in.position, 1.0);
    // Valid for the rotation+uniform-scale+translation transforms this renderer
    // ever builds; a full inverse-transpose isn't needed for those.
    let normal_mat = mat3x3<f32>(model[0].xyz, model[1].xyz, model[2].xyz);
    out.world_normal = normalize(normal_mat * in.normal);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let l = normalize(-camera.light_dir.xyz);
    let diff = max(dot(n, l), 0.0);
    let lit = 0.4 + diff * 0.6;
    return vec4<f32>(in.color.rgb * lit, in.color.a);
}
