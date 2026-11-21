// 3D view: extruded pieces on a studded baseplate, with one shadowing light.

struct Uniforms {
    view_proj: mat4x4<f32>,
    light_view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    // direction toward the light
    light_dir: vec4<f32>,
    // board width, height, shadows on, unused
    board: vec4<f32>,
    accent: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var shadow_map: texture_depth_2d;
@group(0) @binding(2) var shadow_sampler: sampler_comparison;

const FLAG_SELECTED: u32 = 1u;
const FLAG_GHOST: u32 = 2u;

struct VIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct Inst {
    @location(2) m: vec4<f32>,
    @location(3) t: vec2<f32>,
    @location(4) color: vec4<f32>,
    @location(5) flags: u32,
};

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) @interpolate(flat) color: vec4<f32>,
    @location(3) @interpolate(flat) flags: u32,
};

fn to_world(p: vec3<f32>, inst: Inst) -> vec3<f32> {
    let bx = inst.m.x * p.x + inst.m.z * p.y + inst.t.x;
    let by = inst.m.y * p.x + inst.m.w * p.y + inst.t.y;
    return vec3<f32>(bx - u.board.x * 0.5, p.z, by - u.board.y * 0.5);
}

@vertex
fn vs_main(v: VIn, inst: Inst) -> VOut {
    let world = to_world(v.position, inst);
    var out: VOut;
    out.pos = u.view_proj * vec4<f32>(world, 1.0);
    out.world = world;
    out.normal = normalize(vec3<f32>(
        inst.m.x * v.normal.x + inst.m.z * v.normal.y,
        v.normal.z,
        inst.m.y * v.normal.x + inst.m.w * v.normal.y,
    ));
    out.color = inst.color;
    out.flags = inst.flags;
    return out;
}

@vertex
fn vs_shadow(v: VIn, inst: Inst) -> @builtin(position) vec4<f32> {
    return u.light_view_proj * vec4<f32>(to_world(v.position, inst), 1.0);
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045));
}

fn to_srgb(c: vec3<f32>) -> vec3<f32> {
    let x = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    return select(1.055 * pow(x, vec3<f32>(1.0 / 2.4)) - 0.055, x * 12.92, x <= vec3<f32>(0.0031308));
}

fn shadow(world: vec3<f32>, n: vec3<f32>) -> f32 {
    if (u.board.z < 0.5) {
        return 1.0;
    }
    let p = u.light_view_proj * vec4<f32>(world + n * 0.02, 1.0);
    let ndc = p.xyz / p.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
        return 1.0;
    }
    let texel = 1.0 / 2048.0;
    var sum = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            sum += textureSampleCompareLevel(shadow_map, shadow_sampler, uv + vec2<f32>(f32(x), f32(y)) * texel, ndc.z - 0.0015);
        }
    }
    return sum / 9.0;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    var n = normalize(in.normal);
    let view = normalize(u.eye.xyz - in.world);
    if (dot(n, view) < 0.0) {
        n = -n;
    }
    var base = to_linear(in.color.rgb);
    if ((in.flags & FLAG_SELECTED) != 0u) {
        base = mix(base, to_linear(u.accent.rgb), 0.35);
    }
    let l = normalize(u.light_dir.xyz);
    let lit = shadow(in.world, n);
    let sky = vec3<f32>(1.0);
    let ground = to_linear(vec3<f32>(0.663, 0.635, 0.627));
    let hemi = mix(ground, sky, n.y * 0.5 + 0.5) * 0.52;
    let diffuse = max(dot(n, l), 0.0) * 0.72 * lit;
    let h = normalize(l + view);
    let specular = pow(max(dot(n, h), 0.0), 56.0) * 0.22 * lit;
    var color = base * (hemi + diffuse) + vec3<f32>(specular);
    // Rim light on selected pieces.
    if ((in.flags & FLAG_SELECTED) != 0u) {
        let rim = pow(1.0 - max(dot(n, view), 0.0), 3.0);
        color += to_linear(u.accent.rgb) * rim * 0.6;
    }
    var alpha = in.color.a;
    if ((in.flags & FLAG_GHOST) != 0u) {
        alpha = 0.5;
    }
    return vec4<f32>(to_srgb(color) * alpha, alpha);
}

