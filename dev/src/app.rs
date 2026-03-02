use crate::font_data::FontData;
use l3gion::{
    renderer::{
        LgBuildWithRenderer, LgCreateWithRenderer, LgDrawInstanced, LgDrawOp, LgRenderPassBuilder,
        LgRenderer, LgRendererBuilder, LgShader, LgShaderBindGroup, LgShaderBuilder,
        LgTexelCopyTextureInfo, LgTexture, LgTextureBuilder, LgVertex, LgWriteBufferSpecs,
    },
    wgpu::{self, vertex_attr_array},
};
use msdf_font::BitmapImageType;
use std::sync::Arc;
use winit::{application::ApplicationHandler, event::WindowEvent, window::Window};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
struct ULocals {
    screen_size: [f32; 2],
    _padding: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
struct Instance {
    position: [f32; 2],
    size: [f32; 2],
    uv_offset: [f32; 2],
    uv_size: [f32; 2],
}
impl LgVertex for Instance {
    const STEP_MODE: wgpu::VertexStepMode = wgpu::VertexStepMode::Instance;

    const ATTRIBS: &[wgpu::VertexAttribute] = &vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x2,
        3 => Float32x2,
    ];
}

struct RendererData {
    _texture: LgTexture,
    locals_buffer: wgpu::Buffer,
    shader: LgShader,
    tex_bind_group: LgShaderBindGroup,
    buffer_bind_group: LgShaderBindGroup,
}

struct AppCore {
    window: Arc<Window>,
    renderer: LgRenderer,
    renderer_data: RendererData,
    font_data: FontData,
}
impl AppCore {
    fn new(window: Arc<Window>, renderer: LgRenderer) -> Self {
        let (font_data, mut bitmap_data) = FontData::new("OpenSans.ttf").unwrap();
        // Saving the image just for testing.
        let _ = image::save_buffer(
            "image.png",
            &bitmap_data.bytes,
            bitmap_data.width as u32,
            bitmap_data.height as u32,
            match bitmap_data.image_type {
                BitmapImageType::L8 => image::ColorType::L8,
                BitmapImageType::Rgb8 => image::ColorType::Rgb8,
            },
        )
        .is_ok();

        // Converting to Rgba8 for wgpu;
        bitmap_data.bytes = bitmap_data
            .bytes
            .chunks_exact(3)
            .flat_map(|b| [b[0], b[1], b[2], 255])
            .collect();

        let surface_specs = renderer.get_surface_specs();

        let shader = LgShaderBuilder::from_specs(wgpu::include_wgsl!("shader.wgsl"))
            .with_vertex_state("vs_main", &[Instance::specs()])
            .with_fragment_state(
                "fs_main",
                &[Some(wgpu::ColorTargetState {
                    format: surface_specs.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            )
            .with_bind_layouts(&[
                Some(wgpu::BindGroupLayoutDescriptor {
                    label: Some("Locals Bind"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                }),
                Some(wgpu::BindGroupLayoutDescriptor {
                    label: Some("Texture Bind"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                }),
            ])
            .build(&renderer);

        let texture = LgTextureBuilder::from_specs(
            wgpu::TextureDescriptor {
                label: Some("Glyph Texture"),
                size: wgpu::Extent3d {
                    width: bitmap_data.width as u32,
                    height: bitmap_data.height as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::SamplerDescriptor {
                label: Some("Glyph Texture Sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            },
        )
        .with_write_on_create(
            LgTexelCopyTextureInfo {
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &bitmap_data.bytes,
            0,
        )
        .build(&renderer);

        let screen_size = window.inner_size();
        let locals_buffer: wgpu::Buffer = wgpu::util::BufferInitDescriptor {
            label: Some("Locals Buffer"),
            contents: bytemuck::cast_slice(&[ULocals {
                screen_size: [screen_size.width as f32, screen_size.height as f32],
                _padding: [0.0; 2],
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        }
        .create(&renderer);

        let buffer_bind_group = shader
            .bind_group_builder(0, &[&locals_buffer])
            .unwrap()
            .build(&renderer);

        let tex_bind_group = shader
            .bind_group_builder(
                1,
                &[&texture.create_view(&Default::default()), texture.sampler()],
            )
            .unwrap()
            .build(&renderer);

        let renderer_data = RendererData {
            _texture: texture,
            locals_buffer,
            shader,
            tex_bind_group,
            buffer_bind_group,
        };

        Self {
            window,
            renderer,
            renderer_data,
            font_data,
        }
    }

    #[inline]
    fn resize(&mut self, width: u32, height: u32) {
        LgWriteBufferSpecs {
            data: bytemuck::cast_slice(&[ULocals {
                screen_size: [width as f32, height as f32],
                _padding: [0.0; 2],
            }]),
            buffer: &self.renderer_data.locals_buffer,
            offset: 0,
        }
        .build(&self.renderer);

        self.renderer.resize(width, height);
    }

    fn render(&mut self) {
        self.window.request_redraw();

        let surface_texture = self.renderer.get_surface_texture().unwrap();

        let pass = LgRenderPassBuilder::default()
            .with_label("Dev Render Pass")
            .with_color(wgpu::RenderPassColorAttachment {
                view: &surface_texture.texture.create_view(&Default::default()),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })
            .build(&self.renderer);

        let text_size = 200.0;
        let text = "Open Sans";

        let scale = text_size / self.font_data.units_per_em as f32;
        let mut pos = [0.0, 300.0];
        let instances = text
            .chars()
            .filter_map(|c| {
                self.font_data.glyph_table.get(&c).map(|g_data| {
                    let size = g_data.data.bounds.size();
                    let size = (size.0 * scale, size.1 * scale);

                    let uv_offset = [
                        g_data.offset.0 as f32 / self.font_data.atlas_size.0,
                        g_data.offset.1 as f32 / self.font_data.atlas_size.1,
                    ];
                    let uv_size = [
                        g_data.size.0 as f32 / self.font_data.atlas_size.0,
                        g_data.size.1 as f32 / self.font_data.atlas_size.1,
                    ];

                    let position = [
                        pos[0] + g_data.data.bearing.0 * scale,
                        pos[1] - g_data.data.bearing.1 * scale,
                    ];

                    pos[0] += g_data.data.advance.0 * scale;
                    Instance {
                        position,
                        size: [size.0, size.1],
                        uv_offset,
                        uv_size,
                    }
                })
            })
            .collect::<Vec<_>>();

        let instance_buffer = wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::VERTEX,
        }
        .create(&self.renderer);

        pass.set_shader(&self.renderer_data.shader)
            .set_bind_group(&self.renderer_data.buffer_bind_group)
            .set_bind_group(&self.renderer_data.tex_bind_group)
            .draw(
                LgDrawOp::Vertices(0..6),
                Some(LgDrawInstanced::Buffer {
                    instances: 0..instances.len() as u32,
                    buffer: &instance_buffer,
                    buffer_slot: 0,
                }),
            )
            .end_and_submit(&self.renderer);

        surface_texture.present();
    }
}

#[derive(Default)]
pub(crate) struct App {
    core: Option<AppCore>,
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(event_loop.create_window(Default::default()).unwrap());
        let inner_size = window.inner_size();

        let renderer = pollster::block_on(
            LgRendererBuilder::new(Arc::clone(&window))
                .with_trace(wgpu::Trace::Off)
                .build(|_| wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    width: inner_size.width,
                    height: inner_size.height,
                    present_mode: wgpu::PresentMode::AutoNoVsync,
                    desired_maximum_frame_latency: 2,
                    alpha_mode: wgpu::CompositeAlphaMode::Auto,
                    view_formats: vec![],
                }),
        );

        self.core = Some(AppCore::new(window, renderer))
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
            WindowEvent::Resized(size) => {
                core.resize(size.width, size.height);
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                core.render();
            }
            _ => {}
        }
    }
}
