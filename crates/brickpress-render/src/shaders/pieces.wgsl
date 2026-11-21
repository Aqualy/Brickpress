// Every placed surface in one instanced draw, shaded from the shape atlas.

const PAD: f32 = 0.25;
const FLAG_OUTLINE: u32 = 1u;
const FLAG_GHOST: u32 = 2u;
const FLAG_INVALID: u32 = 4u;
const FLAG_DIM: u32 = 8u;

@group(1) @binding(0) var atlas_tex: texture_2d<f32>;
@group(1) @binding(1) var atlas_sampler: sampler;

struct Inst {
    @location(0) m0: vec4<f32>,
    @location(1) m1: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) atlas: vec4<f32>,
    @location(4) color: vec4<f32>,
    @location(5) seed: u32,
    @location(6) flags: u32,
    @location(7) opacity: f32,
};

struct PieceOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) world: vec2<f32>,
    @location(2) @interpolate(flat) color: vec4<f32>,
    @location(3) @interpolate(flat) atlas: vec4<f32>,
    @location(4) @interpolate(flat) seed: u32,
    @location(5) @interpolate(flat) flags: u32,
    @location(6) @interpolate(flat) opacity: f32,
};

@vertex
fn vs_piece(@builtin(vertex_index) vi: u32, inst: Inst) -> PieceOut {
    let corner = vec2<f32>(f32(vi & 1u), f32(vi >> 1u));
    let local = mix(vec2<f32>(-PAD), inst.size + vec2<f32>(PAD), corner);
    let world = vec2<f32>(
        inst.m0.x * local.x + inst.m0.z * local.y + inst.m1.x,
        inst.m0.y * local.x + inst.m0.w * local.y + inst.m1.y,
    );
    var out: PieceOut;
    out.pos = to_clip(world);
    out.local = local;
    out.world = world;
    out.color = inst.color;
    out.atlas = inst.atlas;
    out.seed = inst.seed;
    out.flags = inst.flags;
    out.opacity = inst.opacity;
    return out;
}

fn field(atlas: vec4<f32>, local: vec2<f32>) -> f32 {
    let uv = atlas.xy + (local + vec2<f32>(PAD)) * atlas.zw;
    return textureSampleLevel(atlas_tex, atlas_sampler, uv, 0.0).r;
}

struct Shade {
    color: vec3<f32>,
    alpha: f32,
};

// Simulated impression, modelled on real brick letterpress prints:
// - ink lands as a dense, nearly flat film; where it does not transfer the
//   paper shows through as sharp-edged voids, not as a paler tint;
// - voids are fractal flakes and specks whose amount varies a lot from piece
//   to piece and in broad patches within a piece;
// - the rim of each surface holds ink best, so edges stay solid;
// - thin ink shows the paper's tooth as a fine stipple;
// - outlines are slightly ragged and chipped.
// Everything travels with the piece except the paper tooth, which lives in
// board space and is shared by every pass.

// Fractal noise around 0 (sd about 0.22), with octaves faded out once they
// fall below a pixel. `px` is studs per pixel.
fn fbm(p: vec2<f32>, seed: u32, octaves: i32, px: f32) -> f32 {
    return fbm_g(p, seed, octaves, px, 0.5);
}

fn fbm_g(p: vec2<f32>, seed: u32, octaves: i32, px: f32, gain: f32) -> f32 {
    var sum = 0.0;
    var amp = 1.0;
    var q = p;
    var f = 1.0;
    for (var i = 0; i < octaves; i++) {
        let w = 1.0 - smoothstep(0.3, 0.7, px * f);
        sum += gnoise(q, seed + u32(i) * 101u) * amp * w;
        q = mat2x2<f32>(1.6, 1.2, -1.2, 1.6) * q + vec2<f32>(19.7, 7.3);
        amp *= gain;
        f *= 2.0;
    }
    return sum;
}

// Like fbm_g, but scaled to unit variance from the octaves that are still
// resolved, so a threshold keeps the same area fraction at any zoom.
fn fbm_unit(p: vec2<f32>, seed: u32, octaves: i32, px: f32, gain: f32) -> f32 {
    var sum = 0.0;
    var var_sum = 0.0;
    var amp = 1.0;
    var q = p;
    var f = 1.0;
    for (var i = 0; i < octaves; i++) {
        let w = 1.0 - smoothstep(0.3, 0.7, px * f);
        sum += gnoise(q, seed + u32(i) * 101u) * amp * w;
        var_sum += amp * amp * w * w;
        q = mat2x2<f32>(1.6, 1.2, -1.2, 1.6) * q + vec2<f32>(19.7, 7.3);
        amp *= gain;
        f *= 2.0;
    }
    return sum / (0.19 * sqrt(max(var_sum, 1e-4)));
}

// Area that takes ink, for noise `n` (unit variance) and void fraction `v`.
// Once even the coarsest octave is below a pixel, the mean coverage is used.
fn covered(n: f32, v: f32, px_f: f32) -> f32 {
    let t = probit(v);
    let soft = 0.05 + px_f * 1.5;
    let sharp = smoothstep(t - soft, t + soft, n);
    return mix(sharp, 1.0 - v, smoothstep(0.5, 1.1, px_f));
}

// Approximate inverse normal CDF.
fn probit(v: f32) -> f32 {
    let x = clamp(v, 1e-4, 1.0 - 1e-4);
    return log(x / (1.0 - x)) / 1.702;
}

fn print_shade(in: PieceOut, aa: f32, tooth_w: f32) -> Shade {
    let ink_amount = g.print0.x;
    let pressure = g.print0.y;
    let coverage = g.print0.z;
    let roller = g.print0.w;
    let wear = g.print1.x;
    let tooth = g.print1.z + g.paper2.x;
    let scale = max(g.print1.w, 0.05);
    let erosion = g.print2.y;
    let roughness = g.print2.z;

    let p = (in.local + seed_offset(in.seed)) / scale;
    let paper_p = (in.world + seed_offset(g.seeds.x)) / scale;
    let px = aa / scale;
    let starve = 1.15 - pressure * 0.3;
    // A clean press shows none of the texture below.
    let distress = clamp((1.0 - coverage) * 6.0 + wear + (1.0 - ink_amount) * 2.0 + g.print1.z + (1.0 - roller), 0.0, 1.0);

    // Pieces differ a lot: some print almost solid, some starved.
    let r = rand01(pcg(in.seed ^ 0x51f3u));
    let piece_k = exp2((r - 0.5) * 2.0 * (0.6 + (1.0 - roller) * 1.4));
    // Broad patches within a piece where the film is thinner.
    let patches = fbm_unit(p * 0.9 + vec2<f32>(3.1, 8.7), in.seed + 900u, 3, px * 0.9, 0.5);
    let patch_k = exp2(patches * (0.5 + (1.0 - roller) * 0.8));

    // Outline: eroded, slightly ragged, chipped where the surface is worn.
    let warp = vec2<f32>(fbm(p * 16.0, in.seed + 7u, 3, px * 16.0), fbm(p * 16.0 + 31.7, in.seed + 8u, 3, px * 16.0));
    let chip = fbm(p * 7.0 + 13.1, in.seed + 9u, 4, px * 7.0);
    let d = field(in.atlas, in.local + warp * roughness * 0.022 * scale) + erosion
        + max(chip - 0.18, 0.0) * roughness * 0.12;
    let edge = clamp(0.5 - d / aa, 0.0, 1.0);

    // The rim holds ink best.
    let rim = smoothstep(0.02, 0.09 + pressure * 0.08, -d);
    let rim_k = mix(0.1, 1.0, rim);

    // Flakes: a worn or under-inked surface misses whole irregular areas.
    let flake_v = clamp(
        ((1.0 - coverage) * 0.25 + wear * wear * wear * 0.6 + max(0.0, 0.85 - ink_amount) * 0.35) * starve * piece_k * patch_k * rim_k,
        0.0,
        0.55,
    );
    let flake_n = fbm_unit(p * 7.5, in.seed + 17u, 6, px * 7.5, 0.66);
    let flakes = covered(flake_n, flake_v, px * 7.5);

    // Speckle: the paper's tooth. Its low points miss the ink, in clusters
    // where the film is thin.
    let speck_v = clamp(
        (tooth * 0.03 + max(0.0, 0.9 - ink_amount) * 0.3 + (1.0 - pressure) * 0.03 + (1.0 - coverage) * 0.4)
            * piece_k * rim_k * (0.6 + wear) * distress
            * (0.1 + 2.7 * smoothstep(-0.3, 1.8, patches)),
        0.0,
        0.8,
    );
    let speck_n = fbm_unit(paper_p * 18.0, g.seeds.x ^ 0xa5a5u, 3, px * 18.0, 0.5);
    let specks = covered(speck_n, speck_v, px * 18.0);

    // Film density: dense, with a little variation.
    let film_n = fbm(p * 1.8 + vec2<f32>(5.3, 1.1), in.seed + 63u, 2, px * 1.8);
    let density = clamp(0.97 + film_n * (0.05 + (1.0 - roller) * 0.08) * distress, 0.0, 1.0);
    let body = mix(0.8, 1.0, clamp((in.opacity - 0.5) * 2.0, 0.0, 1.0))
        * (1.0 - rand01(pcg(in.seed ^ 0x7e11u)) * (1.0 - roller) * 0.15);

    let alpha = edge * flakes * specks * density * body;
    return Shade(in.color.rgb, clamp(alpha, 0.0, 1.0));
}

fn shade(in: PieceOut) -> Shade {
    // Derivatives first, in uniform control flow.
    let fw = fwidth(in.local);
    let aa = max(length(fw) * 0.7071, 1e-5);
    let tooth_w = length(fwidth(in.local * 14.0 / max(g.print1.w, 0.05)));

    if ((in.flags & FLAG_OUTLINE) != 0u) {
        let d = field(in.atlas, in.local);
        // A two-pixel band just outside the edge, plus a faint inner tint.
        let outer = clamp((d + aa * 0.5) / aa, 0.0, 1.0) * clamp((aa * 2.5 - d) / aa, 0.0, 1.0);
        let inner = clamp(0.5 - d / aa, 0.0, 1.0) * 0.12;
        return Shade(in.color.rgb, max(outer, inner));
    }
    if (g.frame.y > 0.5 && (in.flags & FLAG_GHOST) == 0u) {
        return print_shade(in, aa, tooth_w);
    }
    let d = field(in.atlas, in.local) + g.paper2.w * 0.5;
    var alpha = clamp(0.5 - d / aa, 0.0, 1.0) * in.color.a;
    if ((in.flags & FLAG_DIM) != 0u) {
        alpha *= 0.55;
    }
    if ((in.flags & FLAG_GHOST) != 0u) {
        // Dashed-looking edge so previews read as previews.
        let edge = clamp(1.0 - abs(d) / (aa * 1.5), 0.0, 1.0);
        let dash = step(0.5, fract((in.local.x + in.local.y) * 4.0));
        alpha = max(alpha, edge * mix(0.45, 0.95, dash));
    }
    return Shade(in.color.rgb, alpha);
}

// Normal "over" compositing, premultiplied.
@fragment
fn fs_piece_over(in: PieceOut) -> @location(0) vec4<f32> {
    let s = shade(in);
    return vec4<f32>(s.color * s.alpha, s.alpha);
}

// Overprint: ink multiplies the paper and the inks beneath it.
@fragment
fn fs_piece_multiply(in: PieceOut) -> @location(0) vec4<f32> {
    let s = shade(in);
    return vec4<f32>(mix(vec3<f32>(1.0), s.color, s.alpha), 1.0);
}
