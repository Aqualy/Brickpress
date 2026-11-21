// The tracing guide: a textured quad beneath the pieces.

@group(1) @binding(0) var trace_tex: texture_2d<f32>;
@group(1) @binding(1) var trace_sampler: sampler;

struct TraceUniform {
    rect: vec4<f32>,    // x, y, width, height in studs
    opacity: vec4<f32>,
};
@group(2) @binding(0) var<uniform> trace: TraceUniform;

struct TraceOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_trace(@builtin(vertex_index) vi: u32) -> TraceOut {
    let corner = vec2<f32>(f32(vi & 1u), f32(vi >> 1u));
    var out: TraceOut;
    out.pos = to_clip(trace.rect.xy + corner * trace.rect.zw);
    out.uv = corner;
    return out;
}

@fragment
fn fs_trace(in: TraceOut) -> @location(0) vec4<f32> {
    let c = textureSample(trace_tex, trace_sampler, in.uv);
    let a = c.a * trace.opacity.x;
    return vec4<f32>(c.rgb * a, a);
}
