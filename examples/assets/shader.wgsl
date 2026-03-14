// Vertex shader

struct VertexInput {
  @location(0) position: vec2<f32>,
  @location(1) tex_coords: vec2<f32>,
  @location(2) use_msdf: u32
};

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) tex_coords: vec2<f32>,
  @location(1) use_msdf: u32
};

@vertex
fn vs_main(
  model: VertexInput,
) -> VertexOutput {
  var out: VertexOutput;
  out.tex_coords = model.tex_coords;
  out.clip_position = vec4<f32>(model.position, 0.0, 1.0);
  out.use_msdf = model.use_msdf;
  return out;
}

// Fragment shader

@group(0) @binding(0)
var msdf_texture: texture_2d<f32>;
@group(0) @binding(1)
var sdf_texture: texture_2d<f32>;
@group(0) @binding(2)
var texture_sampler: sampler;


// MSDF
const px_range = 4.0;

fn sqr(x: vec2<f32>) -> vec2<f32> {
  return x * x;
}

fn median(r: f32, g: f32, b: f32) -> f32 {
  return max(min(r, g), min(max(r, g), b));
}

fn unit_range(px_size: f32) -> vec2<f32> {
    let vpx = vec2(px_range);
    let vtd = vec2<f32>(textureDimensions(msdf_texture, 0));
    return vpx / vtd;
}

fn screen_px_range(tex_coord: vec2<f32>) -> f32 {
    let unit_range = unit_range(px_range);
    let screen_tex_size = inverseSqrt(sqr(dpdx(tex_coord)) + sqr(dpdy(tex_coord)));

    return max(0.5 * dot(unit_range, screen_tex_size), 1.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  if in.use_msdf == 1 {
    let msd = textureSample(msdf_texture, texture_sampler, in.tex_coords).rgb;
    let sd = median(msd.r, msd.g, msd.b);
    let screen_px_distance = screen_px_range(in.tex_coords) * (sd - 0.5);
    let opacity = clamp(screen_px_distance + 0.5, 0.0, 1.0);

    let fg_color = vec4(1.0);
    let bg_color = vec4(0.0);

    return mix(bg_color, fg_color, opacity);
  } else {
    let sd = textureSample(sdf_texture, texture_sampler, in.tex_coords).r;
    let fw = fwidth(sd);
    let alpha = smoothstep(0.5 - fw, 0.5 + fw, sd);
    
    return vec4(1.0, 1.0, 1.0, alpha);
  }
}