struct Vertex {
  position: vec2<f32>,
  uv: vec2<f32>,
};

struct InstanceInput {
  @location(0) position: vec2<f32>,
  @location(1) size: vec2<f32>,
  @location(2) uv_offset: vec2<f32>,
  @location(3) uv_size: vec2<f32>,
}

struct VertexOutput {
  @location(0) uv: vec2<f32>,
  @builtin(position) position: vec4<f32>,
};

struct Locals {
  screen_size: vec2<f32>,
  _padding: vec2<f32>,
}

var<private> VERTICES: array<Vertex, 6> = array<Vertex, 6>(
  Vertex(vec2<f32>(-1.0, -1.0), vec2<f32>(0.0, 1.0)),
  Vertex(vec2<f32>( 1.0, -1.0), vec2<f32>(1.0, 1.0)),
  Vertex(vec2<f32>(-1.0,  1.0), vec2<f32>(0.0, 0.0)),

  Vertex(vec2<f32>(-1.0,  1.0), vec2<f32>(0.0, 0.0)),
  Vertex(vec2<f32>( 1.0, -1.0), vec2<f32>(1.0, 1.0)),
  Vertex(vec2<f32>( 1.0,  1.0), vec2<f32>(1.0, 0.0)),
);

@group(0) @binding(0) var<uniform> u_locals: Locals;

fn prepare_vertex(
  vertex_pos: vec2<f32>,
  pixel_pos: vec2<f32>,
  pixel_size: vec2<f32>,
) -> vec4<f32> {
  let pos_ndc = vec2<f32>(
    2.0 * pixel_pos.x / u_locals.screen_size.x - 1.0,
    1.0 - pixel_pos.y / u_locals.screen_size.y * 2.0
  );

  let size_ndc = vec2<f32>(
      pixel_size.x / u_locals.screen_size.x,
      pixel_size.y / u_locals.screen_size.y,
  );

  let vertex_ndc = 
      vertex_pos 
      * size_ndc
      - size_ndc
      + pos_ndc;

  return vec4<f32>(vertex_ndc, 0.0, 1.0);
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance_input: InstanceInput,
) -> VertexOutput {
  let vertex = VERTICES[vertex_index];

  var out: VertexOutput;
  let quad_uv = vec2<f32>(1.0 - vertex.uv.x, vertex.uv.y);
  
  out.uv = instance_input.uv_offset + quad_uv * instance_input.uv_size;
  out.position  = prepare_vertex(vertex.position, instance_input.position, instance_input.size);

  return out;
}

// Fragment shader bindings
@group(1) @binding(0) var u_tex_color: texture_2d<f32>;
@group(1) @binding(1) var u_tex_sampler: sampler;

const px_range = 4.0;

fn median(r: f32, g: f32, b: f32) -> f32 {
  return max(min(r, g), min(max(r, g), b));
}

fn sqr(x: vec2<f32>) -> vec2<f32> {
  return x * x;
}

fn screen_px_range(tex_coord: vec2<f32>) -> f32 {
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

    let fg_color = vec4(1.0);
    let bg_color = vec4(1.0, 0.0, 0.0, 1.0);

    return mix(bg_color, fg_color, opacity);
}