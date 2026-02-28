use std::{fs::File, io::BufReader, sync::Arc};

use image::EncodableLayout;
use l3gion::{
    renderer::{
        LgBuildWithRenderer, LgDrawOp, LgRenderPassBuilder, LgRenderer, LgRendererBuilder,
        LgShader, LgShaderBindGroup, LgShaderBuilder, LgTexelCopyTextureInfo, LgTexture,
        LgTextureBuilder,
    },
    wgpu,
};
use winit::{application::ApplicationHandler, event::WindowEvent, window::Window};

struct RendererData {
    _texture: LgTexture,
    shader: LgShader,
    tex_bind_group: LgShaderBindGroup,
}

struct AppCore {
    window: Arc<Window>,
    renderer: LgRenderer,
    data: RendererData,
}
impl AppCore {
    fn new(window: Arc<Window>, renderer: LgRenderer) -> Self {
        let surface_specs = renderer.get_surface_specs();

        let shader = LgShaderBuilder::from_specs(wgpu::include_wgsl!("shader.wgsl"))
            .with_fragment_state(
                "fs_main",
                &[Some(wgpu::ColorTargetState {
                    format: surface_specs.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            )
            .with_bind_layouts(&[Some(wgpu::BindGroupLayoutDescriptor {
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
            })])
            .build(&renderer);

        let img = BufReader::new(File::open("image.png").unwrap());
        let img_tex = image::load(img, image::ImageFormat::Png)
            .unwrap()
            .to_rgba8();

        let texture = LgTextureBuilder::from_specs(
            wgpu::TextureDescriptor {
                label: Some("Glyph Texture"),
                size: wgpu::Extent3d {
                    width: img_tex.width(),
                    height: img_tex.height(),
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
            img_tex.as_bytes(),
            0,
        )
        .build(&renderer);

        let tex_bind_group = shader
            .bind_group_builder(
                0,
                &[&texture.create_view(&Default::default()), texture.sampler()],
            )
            .unwrap()
            .build(&renderer);

        let data = RendererData {
            _texture: texture,
            shader,
            tex_bind_group,
        };

        Self {
            window,
            renderer,
            data,
        }
    }

    #[inline]
    fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
    }

    fn renderer(&mut self) {
        self.window.request_redraw();

        let surface_texture = self.renderer.get_surface_texture().unwrap();

        let pass = LgRenderPassBuilder::default()
            .with_label("Dev Render Pass")
            .with_color(wgpu::RenderPassColorAttachment {
                view: &surface_texture.texture.create_view(&Default::default()),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })
            .build(&self.renderer);

        pass.set_shader(&self.data.shader)
            .set_bind_group(&self.data.tex_bind_group)
            .draw(LgDrawOp::Vertices(0..6), None)
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
                core.renderer();
            }
            _ => {}
        }
    }
}
