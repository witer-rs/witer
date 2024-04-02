struct WindowUniform {
    resolution: vec2<f32>,
};
@group(0) @binding(0)
var<uniform> window: WindowUniform;

struct FrameUniform {
    frame_index: u32,
}
@group(1) @binding(0)
var<uniform> frame: FrameUniform;

struct CameraUniform {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
};
@group(2) @binding(0)
var<uniform> camera: CameraUniform;

struct ModelUniform {
    position: vec3<f32>,
}
@group(3) @binding(0)
var<uniform> model: ModelUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) coord: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    var coord = vec2<f32>(
        f32((index << 1u) & 2u),
        f32(index & 2u),
    ) * 2.0 - 1.0;
    let aspect = window.resolution.x / window.resolution.y;

    out.clip_position = vec4<f32>(coord, 0.0, 1.0);
    out.coord = mat2x2(vec2(aspect, 0.0), vec2(0.0, 1.0)) * coord;

    return out;
}

struct Ray {
    origin: vec3<f32>,
    dir: vec3<f32>,
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let view_point: vec3<f32> = (camera.view_proj * vec4(in.coord, 0.0, 1.0)).xyz;

    let radius = 0.5;
    var ray: Ray;
    ray.origin = camera.position;
    ray.dir = normalize(view_point - ray.origin);

    return vec4(ray.dir, 0.0);

    // var a: f32 = dot(ray_dir, ray_dir);
    // var b: f32 = 2.0 * dot(ray_origin, ray_dir);
    // var c: f32 = dot(ray_origin, ray_origin) - (radius * radius);
    // var discriminant: f32 = (b * b) - (4.0 * a * c);

    // var ambient_color = vec4(0.0, 0.0, 0.0, 1.0);

    // if discriminant < 0.0 {
    //     return ambient_color;
    // }

    // // let furthest_t: f32 = (-b + sqrt(discriminant)) / (2.0 * a);
    // let closest_t: f32 = (-b - sqrt(discriminant)) / (2.0 * a);

    // let hit_point: vec3<f32> = ray_origin + (ray_dir * closest_t);

    // let normal = normalize(hit_point);
    // let light_dir = normalize(vec3(-1.0, -1.0, -0.75));

    // let light_intensity: f32 = max(dot(normal, -light_dir), 0.0);

    // var color = vec4(0.7, 0.7, 0.7, 1.0);

    // color *= light_intensity;

    // let final_color = color + ambient_color;

    // return clamp(final_color, vec4(vec3(0.0), 1.0), vec4(1.0)) ;
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