#[cfg(feature = "atlas")]
use msdf_font::AtlasGlyphData;
#[cfg(not(feature = "atlas"))]
use msdf_font::GlyphData;
use msdf_font::{GlyphBitmapData, GlyphBounds, GlyphBuilder};
#[cfg(feature = "atlas")]
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
    use_msdf: u32,
}
impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Uint32];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

const INDICES: [u32; 6] = [0, 3, 1, 1, 3, 2];

struct WgpuState {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
    texture_bind_group: wgpu::BindGroup,
    is_surface_configured: bool,
}
impl WgpuState {
    async fn new(window: Arc<Window>, font: &Font) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let msdf_texture_size = wgpu::Extent3d {
            width: font.msdf.width as u32,
            height: font.msdf.height as u32,
            depth_or_array_layers: 1,
        };
        let msdf_texture_bytes = font
            .msdf
            .bytes()
            .chunks_exact(3)
            .flat_map(|px| [px[0], px[1], px[2], 255])
            .collect::<Vec<_>>();

        let sdf_texture_size = wgpu::Extent3d {
            width: font.sdf.width as u32,
            height: font.sdf.height as u32,
            depth_or_array_layers: 1,
        };

        let msdf_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: msdf_texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("msdf_texture"),
            view_formats: &[],
        });

        let sdf_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: sdf_texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("sdf_texture"),
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &msdf_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &msdf_texture_bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * msdf_texture_size.width),
                rows_per_image: Some(msdf_texture_size.height),
            },
            msdf_texture_size,
        );

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &sdf_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &font.sdf.bytes(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(sdf_texture_size.width),
                rows_per_image: Some(sdf_texture_size.height),
            },
            sdf_texture_size,
        );

        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &msdf_texture.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &sdf_texture.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&texture_sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("assets/shader.wgsl"));

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            texture_bind_group,
            is_surface_configured: false,
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }
}

struct Font {
    #[cfg(not(feature = "atlas"))]
    data: GlyphData,
    #[cfg(feature = "atlas")]
    data: HashMap<char, AtlasGlyphData>,
    msdf: GlyphBitmapData<u8, 3>,
    sdf: GlyphBitmapData<u8, 1>,
    #[cfg(feature = "atlas")]
    space_advance: [f32; 2],
    #[cfg(feature = "atlas")]
    line_space: f32,
    #[cfg(feature = "atlas")]
    ascender: f32,

    units_per_em: f32,
}
impl Font {
    fn new() -> Self {
        let face =
            ttf_parser::Face::parse(include_bytes!("assets/OpenSans-Medium.ttf"), 0).unwrap();

        let units_per_em = face.units_per_em() as f32;
        #[cfg(feature = "atlas")]
        let line_space = face.ascender() - face.descender() + face.line_gap();
        #[cfg(feature = "atlas")]
        let ascender = face.ascender() as f32;

        let builder = GlyphBuilder::new(&face).px_range(4).px_size(40);

        #[cfg(not(feature = "atlas"))]
        let mut glyph = builder.build('ç').unwrap();

        #[cfg(not(feature = "atlas"))]
        let (sdf, msdf) = { (glyph.sdf(), glyph.msdf(3.0, true)) };

        #[cfg(feature = "atlas")]
        let (sdf, msdf, atlas) = {
            let chars = (0..0xff).filter_map(char::from_u32);
            let mut atlas = builder.build_atlas(chars).unwrap();

            (atlas.sdf(), atlas.msdf(3.0, true), atlas)
        };

        #[cfg(feature = "atlas")]
        let space_advance = if let Some(gid) = face.glyph_index(' ') {
            [
                face.glyph_hor_advance(gid).unwrap_or(0) as f32,
                face.glyph_ver_advance(gid).unwrap_or(0) as f32,
            ]
        } else {
            [0.0; 2]
        };

        Self {
            #[cfg(not(feature = "atlas"))]
            data: glyph.data,
            #[cfg(feature = "atlas")]
            data: atlas.glyph_table,
            msdf,
            sdf,
            units_per_em,
            #[cfg(feature = "atlas")]
            line_space: line_space as f32,
            #[cfg(feature = "atlas")]
            ascender,
            #[cfg(feature = "atlas")]
            space_advance,
        }
    }
}

struct AppCore {
    wgpu_state: WgpuState,
    font: Font,
    window: Arc<Window>,
    text_size: f32,
}
impl AppCore {
    async fn new(window: Window) -> Self {
        let font = Font::new();
        let window = Arc::new(window);
        let wgpu_state = WgpuState::new(Arc::clone(&window), &font).await;

        Self {
            window,
            wgpu_state,
            font,
            text_size: 100.0,
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.wgpu_state.resize(width, height);
    }

    fn render(&mut self) {
        self.window.request_redraw();

        if !self.wgpu_state.is_surface_configured {
            return;
        }

        let output = self.wgpu_state.surface.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.wgpu_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        // let mut vertices = vec![];
        let quads = self.get_draw_data();

        let indices = (0..quads.len() as u32)
            .flat_map(|i| INDICES.map(|index| index + (i * 4)))
            .collect::<Vec<_>>();

        let vertices = quads.into_iter().flat_map(|quad| quad).collect::<Vec<_>>();

        let vertex_buffer =
            self.wgpu_state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        let index_buffer =
            self.wgpu_state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Index Buffer"),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.wgpu_state.render_pipeline);
            render_pass.set_bind_group(0, &self.wgpu_state.texture_bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        }

        self.get_draw_data();
        self.wgpu_state
            .queue
            .submit(std::iter::once(encoder.finish()));
        output.present();
    }

    fn event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if !event.state.is_pressed() {
                    return;
                }

                if let PhysicalKey::Code(code) = event.physical_key {
                    match code {
                        KeyCode::KeyW => {
                            self.text_size += 1.0;
                            println!("Text Size: {}px", self.text_size);
                        }
                        KeyCode::KeyS => {
                            self.text_size -= 1.0;
                            println!("Text Size: {}px", self.text_size);
                        }

                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn get_draw_data(&self) -> Vec<[Vertex; 4]> {
        let scale = self.text_size * self.window.scale_factor() as f32 / self.font.units_per_em;
        let window_size = self.window.inner_size();
        let window_size = [window_size.width as f32, window_size.height as f32];

        #[cfg(not(feature = "atlas"))]
        return {
            let plane_bounds = self.font.data.plane_bounds;
            let bearing = self.font.data.bearing;
            let bearing = [bearing[0] as f32 * scale, bearing[1] as f32 * scale];

            let cursor = [0.0; 2];

            vec![
                self.get_draw_glyph_data(
                    1,
                    scale,
                    plane_bounds,
                    window_size,
                    [cursor[0] + bearing[0], cursor[1]],
                    [0.0; 2],
                    [1.0; 2],
                ),
                self.get_draw_glyph_data(
                    0,
                    scale,
                    plane_bounds,
                    window_size,
                    [window_size[0] / 2.0 + cursor[0] + bearing[0], cursor[1]],
                    [0.0; 2],
                    [1.0; 2],
                ),
            ]
        };

        #[cfg(feature = "atlas")]
        return {
            let atlas_size = [self.font.msdf.width as f32, self.font.msdf.height as f32];
            let mut cursor = [0.0, self.font.ascender * scale];
            let new_line = self.font.line_space * scale;

            (0..0xff)
                .filter_map(char::from_u32)
                .chain(include_str!("assets/lorem_ipsum.txt").chars())
                .filter_map(|c| {
                    if c == ' ' {
                        cursor[0] += self.font.space_advance[0] * scale;
                        None
                    } else if c == '\n' {
                        cursor[0] = 0.0;
                        cursor[1] += new_line;
                        None
                    } else {
                        self.font.data.get(&c).map(|g_data| {
                            let plane_bounds = g_data.data.plane_bounds;
                            let size = plane_bounds.size();
                            let size = [size[0] * scale, size[1] * scale];

                            if size[0] + cursor[0] > window_size[0] / 2.0 {
                                cursor[0] = 0.0;
                                cursor[1] += new_line;
                            }

                            let bearing = g_data.data.bearing;
                            let bearing = [bearing[0] as f32 * scale, bearing[1] as f32 * scale];

                            let mut uv_offset = [0.0; 2];
                            let mut uv_size = [0.0; 2];

                            for i in 0..2 {
                                uv_offset[i] = g_data.atlas_bounds.min[i] as f32 / atlas_size[i];

                                uv_size[i] = g_data.atlas_bounds.max[i] as f32 / atlas_size[i]
                                    - uv_offset[i];
                            }

                            let result = [
                                self.get_draw_glyph_data(
                                    1,
                                    scale,
                                    plane_bounds,
                                    window_size,
                                    [cursor[0] + bearing[0], cursor[1] - bearing[1]],
                                    uv_offset,
                                    uv_size,
                                ),
                                self.get_draw_glyph_data(
                                    0,
                                    scale,
                                    plane_bounds,
                                    window_size,
                                    [
                                        window_size[0] / 2.0 + cursor[0] + bearing[0],
                                        cursor[1] - bearing[1],
                                    ],
                                    uv_offset,
                                    uv_size,
                                ),
                            ];

                            cursor[0] += g_data.data.advance[0] as f32 * scale;

                            result
                        })
                    }
                })
                .flatten()
                .collect()
        };
    }

    // Cursor, in pixels.
    fn get_draw_glyph_data(
        &self,
        field_type: u32,
        scale: f32,
        plane_bounds: GlyphBounds<f32>,
        window_size: [f32; 2],
        cursor: [f32; 2],
        uv_offset: [f32; 2],
        uv_size: [f32; 2],
    ) -> [Vertex; 4] {
        // Scaling the glyph.
        let scaled_min = plane_bounds.min.map(|v| v * scale);
        let scaled_max = plane_bounds.max.map(|v| v * scale);
        let scaled_bounds = GlyphBounds {
            min: scaled_min,
            max: scaled_max,
        };

        // Getting the pixel glyph size.
        let scaled_size = scaled_bounds.size();

        // Converting to ndc.
        let tl = Self::to_ndc([cursor[0], cursor[1]], window_size);
        let tr = Self::to_ndc([cursor[0] + scaled_size[0], cursor[1]], window_size);
        let br = Self::to_ndc(
            [cursor[0] + scaled_size[0], cursor[1] + scaled_size[1]],
            window_size,
        );
        let bl = Self::to_ndc([cursor[0], cursor[1] + scaled_size[1]], window_size);

        [
            Vertex {
                position: tl,
                tex_coords: uv_offset,
                use_msdf: field_type,
            },
            Vertex {
                position: tr,
                tex_coords: [uv_offset[0] + uv_size[0], uv_offset[1]],
                use_msdf: field_type,
            },
            Vertex {
                position: br,
                tex_coords: [uv_offset[0] + uv_size[0], uv_offset[1] + uv_size[1]],
                use_msdf: field_type,
            },
            Vertex {
                position: bl,
                tex_coords: [uv_offset[0], uv_offset[1] + uv_size[1]],
                use_msdf: field_type,
            },
        ]
    }

    #[inline]
    const fn to_ndc(val: [f32; 2], window_size: [f32; 2]) -> [f32; 2] {
        [
            2.0 * val[0] / window_size[0] - 1.0,
            1.0 - val[1] / window_size[1] * 2.0,
        ]
    }
}

#[derive(Default)]
struct App {
    core: Option<AppCore>,
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.core = Some(pollster::block_on(AppCore::new(
            event_loop
                .create_window(WindowAttributes::default().with_title("WGPU msdf_font Example"))
                .unwrap(),
        )))
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let core = if let Some(core) = &mut self.core {
            core
        } else {
            return;
        };

        match event {
            winit::event::WindowEvent::Resized(size) => {
                core.resize(size.width, size.height);
            }
            winit::event::WindowEvent::CloseRequested => event_loop.exit(),
            winit::event::WindowEvent::RedrawRequested => core.render(),
            _ => {
                core.event(event);
            }
        }
    }
}

fn main() {
    let el = winit::event_loop::EventLoop::new().unwrap();
    el.run_app(&mut App::default()).unwrap();
}
