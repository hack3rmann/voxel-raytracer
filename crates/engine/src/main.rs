#![allow(dead_code)]

use std::borrow::Cow;
use std::error::Error;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::*;
use pollster::FutureExt as _;
use wgpu::util::DeviceExt as _;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use tracing::{debug, error, warn};

pub fn default<T: Default>() -> T {
    T::default()
}

struct RenderContext {
    instance: Arc<wgpu::Instance>,
    surface: Arc<wgpu::Surface<'static>>,
    adapter: Arc<wgpu::Adapter>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
}

impl RenderContext {
    pub fn new(
        instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        adapter: wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
    ) -> Self {
        Self {
            instance: Arc::new(instance),
            surface: Arc::new(surface),
            adapter: Arc::new(adapter),
            device: Arc::new(device),
            queue: Arc::new(queue),
        }
    }
}

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    render_context: Option<RenderContext>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        );

        let window_size = window.inner_size();

        debug!(size = ?window_size, "window created");

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            flags: wgpu::InstanceFlags::DEBUG
                | wgpu::InstanceFlags::VALIDATION
                | wgpu::InstanceFlags::GPU_BASED_VALIDATION,
            // TODO(hack3rmann): Support for DirectX12 DCX compiler
            // and ship the program with additional dlls
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });

        let surface = instance.create_surface(Arc::clone(&window)).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .block_on()
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("raytrace-device"),
                    required_features: wgpu::Features::PUSH_CONSTANTS,
                    // TODO(hack3rmann): require better limits as needed
                    required_limits: default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .block_on()
            .unwrap();

        let surface_config = surface
            .get_default_config(&adapter, window_size.width, window_size.height)
            .unwrap();

        surface.configure(&device, &surface_config);

        self.window.replace(window);
        self.render_context.replace(RenderContext::new(
            instance, surface, adapter, device, queue,
        ));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                debug!("closing window and exiting event loop");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // Redraw the application.
                //
                // It's preferable for applications that do not render continuously to render in
                // this event rather than in AboutToWait, since rendering in here allows
                // the program to gracefully handle redraws requested by the OS.

                'render: {
                    let Some(context) = self.render_context.as_ref() else {
                        warn!("no render context to draw with");
                        break 'render;
                    };

                    let Ok(cur_texture) = context.surface.get_current_texture() else {
                        error!("no next swapchain texture");
                        break 'render;
                    };

                    let view = cur_texture.texture.create_view(&default());
                    let mut encoder = context.device.create_command_encoder(&default());

                    {
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("rainbow-triangle"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            occlusion_query_set: None,
                            timestamp_writes: None,
                        });

                        #[repr(C)]
                        #[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
                        struct Vertex {
                            position: Vec2,
                            color: Vec3,
                        }

                        let vertices = [
                            Vertex {
                                position: Vec2::new(-0.5, -0.5),
                                color: Vec3::X,
                            },
                            Vertex {
                                position: Vec2::new(0.5, -0.5),
                                color: Vec3::Y,
                            },
                            Vertex {
                                position: Vec2::new(0.0, 0.5),
                                color: Vec3::Z,
                            },
                        ];

                        let vertex_buffer =
                            context
                                .device
                                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                    contents: bytemuck::cast_slice(&vertices),
                                    label: None,
                                    usage: wgpu::BufferUsages::VERTEX,
                                });

                        let pipeline_layout = context.device.create_pipeline_layout(
                            &wgpu::PipelineLayoutDescriptor {
                                label: None,
                                bind_group_layouts: &[],
                                push_constant_ranges: &[],
                            },
                        );

                        let vertex_shader =
                            context
                                .device
                                .create_shader_module(wgpu::ShaderModuleDescriptor {
                                    label: None,
                                    source: wgpu::ShaderSource::Glsl {
                                        shader: Cow::Borrowed(include_str!(
                                            "../assets/shaders/triangle-vertex.glsl"
                                        )),
                                        stage: wgpu::naga::ShaderStage::Vertex,
                                        defines: default(),
                                    },
                                });

                        let fragment_shader =
                            context
                                .device
                                .create_shader_module(wgpu::ShaderModuleDescriptor {
                                    label: None,
                                    source: wgpu::ShaderSource::Glsl {
                                        shader: Cow::Borrowed(include_str!(
                                            "../assets/shaders/triangle-fragment.glsl"
                                        )),
                                        stage: wgpu::naga::ShaderStage::Fragment,
                                        defines: default(),
                                    },
                                });

                        let pipeline = context.device.create_render_pipeline(
                            &wgpu::RenderPipelineDescriptor {
                                label: None,
                                layout: Some(&pipeline_layout),
                                vertex: wgpu::VertexState {
                                    module: &vertex_shader,
                                    entry_point: Some("main"),
                                    compilation_options: default(),
                                    buffers: &[wgpu::VertexBufferLayout {
                                        array_stride: std::mem::size_of::<Vertex>() as u64,
                                        step_mode: wgpu::VertexStepMode::Vertex,
                                        attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x3],
                                    }],
                                },
                                fragment: Some(wgpu::FragmentState {
                                    module: &fragment_shader,
                                    entry_point: Some("main"),
                                    compilation_options: default(),
                                    targets: &[Some(wgpu::ColorTargetState {
                                        format: cur_texture.texture.format(),
                                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                                        write_mask: wgpu::ColorWrites::ALL,
                                    })],
                                }),
                                cache: None,
                                primitive: wgpu::PrimitiveState {
                                    topology: wgpu::PrimitiveTopology::TriangleList,
                                    strip_index_format: None,
                                    front_face: wgpu::FrontFace::Ccw,
                                    cull_mode: Some(wgpu::Face::Back),
                                    unclipped_depth: false,
                                    polygon_mode: wgpu::PolygonMode::Fill,
                                    conservative: false,
                                },
                                depth_stencil: None,
                                multisample: wgpu::MultisampleState {
                                    count: 1,
                                    mask: !0,
                                    alpha_to_coverage_enabled: false,
                                },
                                multiview: None,
                            },
                        );

                        pass.set_pipeline(&pipeline);
                        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                        pass.draw(0..vertices.len() as u32, 0..1);
                    }

                    context.queue.submit([encoder.finish()]);
                    cur_texture.present();
                };

                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw in
                // applications which do not always need to. Applications that redraw continuously
                // can render here instead.
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::Resized(size) => 'event: {
                let Some(context) = self.render_context.as_ref() else {
                    break 'event;
                };

                let Some(config) =
                    context
                        .surface
                        .get_default_config(&context.adapter, size.width, size.height)
                else {
                    break 'event;
                };

                context.surface.configure(&context.device, &config);
            }
            _ => (),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new().unwrap();

    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::DEBUG)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut App::default())?;

    Ok(())
}
