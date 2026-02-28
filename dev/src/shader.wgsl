struct Vertex {
    position: vec2<f32>,
    uv: vec2<f32>,
};

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};


var<private> VERTICES: array<Vertex, 6> = array<Vertex, 6>(
    Vertex(vec2<f32>(-1.0, -1.0), vec2<f32>(0.0, 1.0)),
    Vertex(vec2<f32>( 1.0, -1.0), vec2<f32>(1.0, 1.0)),
    Vertex(vec2<f32>(-1.0,  1.0), vec2<f32>(0.0, 0.0)),

    Vertex(vec2<f32>(-1.0,  1.0), vec2<f32>(0.0, 0.0)),
    Vertex(vec2<f32>( 1.0, -1.0), vec2<f32>(1.0, 1.0)),
    Vertex(vec2<f32>( 1.0,  1.0), vec2<f32>(1.0, 0.0)),
);

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    let vertex = VERTICES[vertex_index];

    var out: VertexOutput;
    out.uv = vertex.uv;
    out.position = vec4(vertex.position, 0.0, 1.0);

    return out;
}

// Fragment shader bindings

@group(0) @binding(0) var u_tex_color: texture_2d<f32>;
@group(0) @binding(1) var u_tex_sampler: sampler;

fn median(r: f32, g: f32, b: f32) -> f32 {
  return max(min(r, g), min(max(r, g), b));
}

fn sqr(x: vec2<f32>) -> vec2<f32> {
  return x * x;
}

fn screen_px_range(tex_coord: vec2<f32>) -> f32 {
    let px_range = 1.0;
    let vpx = vec2(px_range);
    let vtd = vec2<f32>(textureDimensions(u_tex_color, 0));
    let unit_range = vpx / vtd;
    let screen_tex_size = inverseSqrt(sqr(dpdx(tex_coord)) + sqr(dpdy(tex_coord)));

    return max(0.5 * dot(unit_range, screen_tex_size), 1.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let msd = textureSample(u_tex_color, u_tex_sampler, in.uv).rgb;
    let sd = median(msd.r, msd.g, msd.b);
    let screen_px_distance = screen_px_range(in.uv) * (sd - 0.5);
    let opacity = clamp(screen_px_distance + 0.5, 0.0, 1.0);

    return vec4(1.0, 1.0, 1.0, opacity);
}