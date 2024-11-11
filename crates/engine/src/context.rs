use crate::util::default;
use bytemuck::{Pod, Zeroable};
use glam::*;
use pollster::FutureExt as _;
use std::{borrow::Cow, sync::Arc};
use thiserror::Error;
use tracing::error;
use wgpu::util::DeviceExt as _;
use winit::dpi::PhysicalSize;

pub use wgpu::{Adapter, Device, Instance, Queue, Surface};

#[derive(Clone)]
pub struct RenderContext {
    pub instance: Arc<Instance>,
    pub surface: Arc<Surface<'static>>,
    pub adapter: Arc<Adapter>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
}

impl RenderContext {
    pub fn new(window: &Arc<winit::window::Window>) -> Self {
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

        let surface = instance.create_surface(Arc::clone(window)).unwrap();

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
                    required_features: wgpu::Features::PUSH_CONSTANTS
                        | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    // TODO(hack3rmann): require better limits as needed
                    required_limits: wgpu::Limits {
                        max_push_constant_size: 128,
                        ..default()
                    },
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .block_on()
            .unwrap();

        let window_size = window.inner_size();

        let surface_config = surface
            .get_default_config(&adapter, window_size.width, window_size.height)
            .unwrap();

        surface.configure(&device, &surface_config);

        Self {
            instance: Arc::new(instance),
            surface: Arc::new(surface),
            adapter: Arc::new(adapter),
            device: Arc::new(device),
            queue: Arc::new(queue),
        }
    }

    pub fn resize(&self, viewport_size: PhysicalSize<u32>) -> Result<(), SurfaceUnsupported> {
        let config = self
            .surface
            .get_default_config(&self.adapter, viewport_size.width, viewport_size.height)
            .ok_or(SurfaceUnsupported)?;

        self.surface.configure(&self.device, &config);

        Ok(())
    }

    pub fn draw_demo(&self) {
        let Ok(cur_texture) = self.surface.get_current_texture() else {
            error!("no next swapchain texture");
            return;
        };

        let viewport_size = {
            let extent = cur_texture.texture.size();
            UVec2::new(extent.width, extent.height)
        };

        let view = cur_texture.texture.create_view(&default());
        let mut encoder = self.device.create_command_encoder(&default());

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
                    position: Vec2::new(-0.5, -f32::sqrt(3.0) / 6.0),
                    color: Vec3::X,
                },
                Vertex {
                    position: Vec2::new(0.5, -f32::sqrt(3.0) / 6.0),
                    color: Vec3::Y,
                },
                Vertex {
                    position: Vec2::new(0.0, f32::sqrt(3.0) / 3.0),
                    color: Vec3::Z,
                },
            ];

            let vertex_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    contents: bytemuck::cast_slice(&vertices),
                    label: None,
                    usage: wgpu::BufferUsages::VERTEX,
                });

            let pipeline_layout =
                self.device
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: None,
                        bind_group_layouts: &[],
                        push_constant_ranges: &[wgpu::PushConstantRange {
                            stages: wgpu::ShaderStages::VERTEX,
                            range: 0..std::mem::size_of::<UVec2>() as u32,
                        }],
                    });

            let vertex_shader = self
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

            let fragment_shader = self
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

            let pipeline = self
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                });

            pass.set_pipeline(&pipeline);
            pass.set_push_constants(
                wgpu::ShaderStages::VERTEX,
                0,
                bytemuck::bytes_of(&viewport_size),
            );
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.draw(0..vertices.len() as u32, 0..1);
        }

        self.queue.submit([encoder.finish()]);
        cur_texture.present();
    }
}

#[derive(Debug, Error)]
#[error("surface is unsupported")]
pub struct SurfaceUnsupported;
