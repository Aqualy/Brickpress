// The paper and stud grid, drawn as one quad covering the board.

struct BoardOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) world: vec2<f32>,
};

@vertex
fn vs_board(@builtin(vertex_index) vi: u32) -> BoardOut {
    let corner = vec2<f32>(f32(vi & 1u), f32(vi >> 1u));
    let world = corner * g.board.xy;
    var out: BoardOut;
    out.pos = to_clip(world);
    out.world = world;
    return out;
}

// Coverage of a line of `width_studs` at distance `d_studs`, at least one pixel wide.
fn line(d_studs: f32, width_studs: f32, px: f32) -> f32 {
    let w = max(width_studs * px, 1.0);
    let alpha = min(width_studs * px, 1.0);
    return clamp(w * 0.5 + 0.5 - d_studs * px, 0.0, 1.0) * max(alpha, 0.35);
}

fn fibers(p: vec2<f32>, density: f32, seed: u32, px: f32) -> f32 {
    let cell_size = 0.5;
    let base = vec2<i32>(floor(p / cell_size));
    var ink = 0.0;
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let c = base + vec2<i32>(dx, dy);
            let h = hash2(c, seed ^ 0x51f15eedu);
            if (rand01(h) >= density) {
                continue;
            }
            let start = (vec2<f32>(c) + vec2<f32>(rand01(pcg(h)), rand01(pcg(h + 1u)))) * cell_size;
            let a = rand01(pcg(h + 2u)) * 3.14159;
            let len = 0.05 + rand01(pcg(h + 3u)) * 0.06;
            let end = start + vec2<f32>(cos(a), sin(a)) * len;
            let ab = end - start;
            let t = clamp(dot(p - start, ab) / dot(ab, ab), 0.0, 1.0);
            let d = length(p - start - ab * t);
            ink = max(ink, clamp(0.5 - (d - 0.004) * px, 0.0, 1.0));
        }
    }
    return ink;
}

@fragment
fn fs_board(in: BoardOut) -> @location(0) vec4<f32> {
    let p = in.world;
    let px = g.frame.x;
    let printed = g.frame.y > 0.5;
    let paper_on = g.frame.z > 0.5;
    var color = g.paper.rgb;

    if (printed && paper_on) {
        // Paper grain: fine, desaturated noise that fades out below a pixel.
        let seed = g.seeds.x;
        let fine = fbm2(p * 32.0 + seed_offset(seed), seed);
        let coarse = fbm2(p * 5.0 + seed_offset(seed + 7u), seed + 7u);
        let fade = clamp(px / 64.0, 0.0, 1.0);
        let grain = mix(0.5, fine, fade) * 0.7 + coarse * 0.3;
        color = mix(color, vec3<f32>(grain), g.paper.a * 0.12);
        // Tooth: tiny pits that catch the light.
        let tooth = fbm2(p * 18.0 + seed_offset(seed + 3u), seed + 3u);
        color *= 1.0 - g.paper2.x * 0.05 * (tooth - 0.5) * fade;
        // Fibres: short, faint strands.
        let f = fibers(p, g.paper2.y * 1.2 * 0.25, seed, px);
        color = mix(color, vec3<f32>(0.396, 0.361, 0.286), f * 0.12 * fade);
    }

    var ink = 0.0;
    if (g.frame.w > 0.5) {
        let style = i32(g.board.w + 0.5);
        let cell = fract(p);
        let embossed = g.paper2.z > 0.5 && !printed && paper_on;
        if (embossed && style != 0) {
            // Raised stud: soft shadow, then a lit dome with a rim.
            let shadow = length(cell - vec2<f32>(0.51, 0.54));
            color = mix(color, vec3<f32>(0.067, 0.094, 0.153), 0.1 * clamp((0.235 - shadow) * px, 0.0, 1.0));
            let d = length(cell - vec2<f32>(0.5, 0.5));
            let dome = clamp((0.225 - d) * px + 0.5, 0.0, 1.0);
            let light = clamp(1.0 - length(cell - vec2<f32>(0.42, 0.38)) / 0.3, 0.0, 1.0);
            color = mix(color, min(color * 1.04 + vec3<f32>(light * 0.06), vec3<f32>(1.0)), dome);
            let rim = clamp(1.0 - abs(d - 0.225) * px / max(0.015 * px, 1.0), 0.0, 1.0);
            color = mix(color, vec3<f32>(1.0), rim * 0.3);
        }
        if (style == 1 && !embossed) {
            let d = length(cell - vec2<f32>(0.5, 0.5));
            ink = clamp((0.04 - d) * px + 0.5, 0.0, 1.0) * 0.75;
        } else if (style >= 2) {
            let width = select(0.007, 0.012, style == 3);
            let to_line = min(min(cell.x, 1.0 - cell.x), min(cell.y, 1.0 - cell.y));
            ink = line(to_line, width, px) * 0.55;
            let major = max(g.board.z, 1.0);
            let mc = fract(p / major) * major;
            let to_major = min(min(mc.x, major - mc.x), min(mc.y, major - mc.y));
            ink = max(ink, line(to_major, 0.015, px) * 0.8);
        }
        ink *= g.grid_color.a;
    }
    if (!paper_on) {
        // Transparent export: only the grid, premultiplied.
        return vec4<f32>(g.grid_color.rgb * ink, ink);
    }
    return vec4<f32>(mix(color, g.grid_color.rgb, ink), 1.0);
}
