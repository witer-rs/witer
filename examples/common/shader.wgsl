struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(2) @binding(0)
var<uniform> camera: CameraUniform;

struct FrameUniform {
    frame_index: u32,
}
@group(1) @binding(0)
var<uniform> frame: FrameUniform;

struct WindowUniform {
    resolution: vec2<f32>,
};
@group(0) @binding(0)
var<uniform> window: WindowUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

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
    let unnorm = vec2(in.uv.x * window.resolution.x, in.uv.y * window.resolution.y);
    let index = u32(unnorm.x + (unnorm.y * window.resolution.y));
    let seed = index * frame.frame_index;
    // let value = f32(index) / (window.resolution.x * window.resolution.y);

    // return vec4<f32>(0.7, 0.3, 0.1, 1.0);
    // return vec4<f32>(in.uv, 0.0, 1.0);
    let r = random_float(u32(seed + 0));
    let g = random_float(u32(seed + 1));
    let b = random_float(u32(seed + 2));
    return vec4(r, g, b, 1.0);
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