struct WindowUniform {
    resolution: vec2<f32>,
};
@group(0) @binding(0)
var<uniform> window: WindowUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct FrameUniform {
    frame_index: u32,
}
@group(1) @binding(0)
var<uniform> frame: FrameUniform;

struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(2) @binding(0)
var<uniform> camera: CameraUniform;

struct ModelUniform {
    position: vec3<f32>,
}
@group(3) @binding(0)
var<uniform> model: ModelUniform;

@vertex
fn vs_main(
    @builtin(vertex_index) index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    out.uv = vec2<f32>(
        f32((index << 1u) & 2u),
        f32(index & 2u),
    );
    out.clip_position = vec4<f32>(out.uv * 2.0 - 1.0, 0.0, 1.0);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let coord = (in.uv * 2.0 - 1.0) * vec2(window.resolution.x / window.resolution.y, 1.0);

    let ray_origin = (camera.view_proj * vec4(vec3(0.0) - model.position, 1.0)).xyz;
    let ray_dir = vec3(coord.x, coord.y, -1.0);
    let radius = 0.5;

    var a: f32 = dot(ray_dir, ray_dir);
    var b: f32 = 2.0 * dot(ray_origin, ray_dir);
    var c: f32 = dot(ray_origin, ray_origin) - radius * radius;

    var discriminant: f32 = b * b - 4.0 * a * c;

    if discriminant >= 0.0 {
        return vec4(0.7, 0.3, 0.1, 1.0);
    } else {
        return vec4(0.1, 0.2, 0.3, 1.0);
    }
}

fn random_noise(uv: vec2<f32>) -> vec4<f32> {
    let coord = vec2(uv.x * window.resolution.x, uv.y * window.resolution.y);
    let index = u32(coord.x + (coord.y * window.resolution.y));
    let seed = index * frame.frame_index;

    let color = vec4(
        random_float(u32(seed + 0)) * 0.1 + 0.05,
        random_float(u32(seed + 1)) * 0.2 + 0.25,
        random_float(u32(seed + 2)) * 0.3 + 0.35,
        1.0,
    );

    return color;
}

fn pcg_hash(input: u32) -> u32 {
    let state = input * 747796405 + 2891336453;
    let word = ((state >> ((state >> 28) + 4) ^ state) * 277803737);
    return (word >> 22) ^ word;
}

fn random_float(seed: u32) -> f32 {
    let value = pcg_hash(seed);
    return f32(value) / f32(0xffffffff);
}