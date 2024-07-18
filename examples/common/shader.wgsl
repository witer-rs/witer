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
        f32((index << 1) & 2),
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

struct HitInfo {
    did_hit: bool,
    dist: f32,
    hit_pt: vec3<f32>,
    normal: vec3<f32>,
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let view_point: vec3<f32> = vec3(in.coord, 1.0);

    var ray: Ray;
    ray.origin = camera.position;
    ray.dir = normalize(view_point - ray.origin);

    let hit: HitInfo = ray_sphere(ray, vec3(0.0), 0.75);

    var ambient_color = vec4(0.2, 0.2, 0.2, 1.0);
    if !hit.did_hit {
        return ambient_color;
    }

    var color = vec4(0.7, 0.7, 0.7, 1.0);

    let light_dir = vec3(-1.0, -1.0, -1.0);
    let light_intensity: f32 = dot(hit.normal, -light_dir);
    color *= light_intensity;
    let final_color = color + ambient_color;

    return clamp(final_color, vec4(vec3(0.0), 1.0), vec4(1.0));
}

fn ray_sphere(ray: Ray, sphere_center: vec3<f32>, sphere_radius: f32) -> HitInfo {
    var hit_info: HitInfo;
    var offset_ray_origin = ray.origin - sphere_center;

    var a: f32 = dot(ray.dir, ray.dir);
    var b: f32 = 2.0 * dot(offset_ray_origin, ray.dir);
    var c: f32 = dot(offset_ray_origin, offset_ray_origin) - (sphere_radius * sphere_radius);

    var discriminant: f32 = (b * b) - (4.0 * a * c);
    if discriminant >= 0.0 {
        let dist: f32 = (-b - sqrt(discriminant)) / (2.0 * a);
        if dist >= 0.0 {
            hit_info.did_hit = true;
            hit_info.dist = dist;
            hit_info.hit_pt = ray.origin + ray.dir * dist;
            hit_info.normal = normalize(hit_info.hit_pt - sphere_center);
        }
    }

    return hit_info;
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