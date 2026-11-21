// Shared uniforms and noise. Prepended to board.wgsl, pieces.wgsl and trace.wgsl.

struct Globals {
    // board studs -> clip space: ndc = board * view.xy + view.zw
    view: vec4<f32>,
    // device pixels per stud, mode (0 design, 1 print), paper on, grid shown
    frame: vec4<f32>,
    // width, height, major grid, grid style (0 off, 1 points, 2 lines, 3 squares)
    board: vec4<f32>,
    // paper rgb, grain
    paper: vec4<f32>,
    // paper tooth, fibres, embossed, design gap (studs)
    paper2: vec4<f32>,
    // ink amount, pressure, ink coverage, roller consistency
    print0: vec4<f32>,
    // piece wear, edge wear, paper tooth (press), texture scale
    print1: vec4<f32>,
    // misalignment, erosion (studs), print roughness, unused
    print2: vec4<f32>,
    // grid ink rgb, grid alpha
    grid_color: vec4<f32>,
    // paper seed, unused...
    seeds: vec4<u32>,
};

@group(0) @binding(0) var<uniform> g: Globals;

fn to_clip(p: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(p * g.view.xy + g.view.zw, 0.0, 1.0);
}

// PCG-based integer hash: deterministic on every GPU.
fn pcg(v: u32) -> u32 {
    let state = v * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn hash2(p: vec2<i32>, seed: u32) -> u32 {
    return pcg(u32(p.x) ^ pcg(u32(p.y) ^ pcg(seed)));
}

fn rand01(h: u32) -> f32 {
    return f32(h & 0x00ffffffu) / 16777216.0;
}

fn gradient(p: vec2<i32>, seed: u32) -> vec2<f32> {
    let a = rand01(hash2(p, seed)) * 6.2831853;
    return vec2<f32>(cos(a), sin(a));
}

// 2D gradient noise in roughly [-0.7, 0.7].
fn gnoise(p: vec2<f32>, seed: u32) -> f32 {
    let i = vec2<i32>(floor(p));
    let f = fract(p);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let a = dot(gradient(i, seed), f);
    let b = dot(gradient(i + vec2<i32>(1, 0), seed), f - vec2<f32>(1.0, 0.0));
    let c = dot(gradient(i + vec2<i32>(0, 1), seed), f - vec2<f32>(0.0, 1.0));
    let d = dot(gradient(i + vec2<i32>(1, 1), seed), f - vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Two-octave fractal noise mapped to about [0, 1], like SVG fractalNoise.
fn fbm2(p: vec2<f32>, seed: u32) -> f32 {
    let n = gnoise(p, seed) + 0.5 * gnoise(p * 2.0 + vec2<f32>(19.7, 7.3), seed + 101u);
    return clamp(n + 0.5, 0.0, 1.0);
}

fn seed_offset(seed: u32) -> vec2<f32> {
    return vec2<f32>(rand01(pcg(seed)), rand01(pcg(seed ^ 0x9e3779b9u))) * 251.0;
}
