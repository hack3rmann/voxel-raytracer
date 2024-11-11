use crate::context::*;
use crate::octree::Chunk;
use crate::util::default;
use glam::*;
use naga::ShaderStage;
use std::borrow::Cow;
use std::num::NonZeroU64;
use tracing::error;
use wgpu::util::{BufferInitDescriptor, DeviceExt as _};
use wgpu::*;

pub struct VoxelsDemo {
    pub context: RenderContext,
    pub pipeline: ComputePipeline,
    pub binds_layout: BindGroupLayout,
    pub render_texture: Texture,
    pub buffer: Buffer,
}

impl VoxelsDemo {
    pub fn new(context: RenderContext) -> Self {
        let chunk = Chunk::new_sphere();

        let voxel_buffer = context.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("voxel-data"),
            contents: bytemuck::cast_slice(&chunk.colors),
            usage: BufferUsages::STORAGE,
        });

        let render_texture = context.device.create_texture(&TextureDescriptor {
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            label: Some("voxels-demo"),
            mip_level_count: 1,
            sample_count: 1,
            size: Extent3d {
                width: 1024,
                height: 1024,
                depth_or_array_layers: 1,
            },
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            view_formats: &[TextureFormat::Rgba8Unorm],
        });

        let binds_layout = context
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("voxels-demo"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: Some(
                                NonZeroU64::new(std::mem::size_of_val(&chunk.colors) as u64)
                                    .unwrap(),
                            ),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::ReadWrite,
                            format: TextureFormat::Rgba8Unorm,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            });

        let pipeline_layout = context
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("vexels-demo"),
                bind_group_layouts: &[&binds_layout],
                push_constant_ranges: &[PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..std::mem::size_of::<UVec2>() as u32,
                }],
            });

        let voxels_demo_compute = context.device.create_shader_module(ShaderModuleDescriptor {
            label: Some("voxels-demo"),
            source: ShaderSource::Glsl {
                shader: Cow::Borrowed(include_str!("../assets/shaders/voxels-demo-compute.glsl")),
                stage: ShaderStage::Compute,
                defines: default(),
            },
        });

        let pipeline = context
            .device
            .create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("voxels-demo"),
                layout: Some(&pipeline_layout),
                module: &voxels_demo_compute,
                entry_point: Some("main"),
                compilation_options: default(),
                cache: None,
            });

        Self {
            context,
            pipeline,
            binds_layout,
            render_texture,
            buffer: voxel_buffer,
        }
    }

    pub fn draw(&self) {
        let Ok(cur_texture) = self.context.surface.get_current_texture() else {
            error!("no next swapchain texture");
            return;
        };

        let viewport_size = {
            let extent = cur_texture.texture.size();
            UVec2::new(extent.width, extent.height)
        };

        let screen_view = cur_texture.texture.create_view(&default());
        let mut encoder = self.context.device.create_command_encoder(&default());

        {
            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("voxels-demo"),
                timestamp_writes: None,
            });

            let render_texture_view = self.render_texture.create_view(&default());

            let bind = self.context.device.create_bind_group(&BindGroupDescriptor {
                label: Some("voxels-demo"),
                layout: &self.binds_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: self.buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(&render_texture_view),
                    },
                ],
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.set_push_constants(0, bytemuck::bytes_of(&viewport_size));
            pass.dispatch_workgroups(viewport_size.x / 8, viewport_size.y / 8, 1);
        }

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("voxels-demo"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &screen_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            let vertices = [
                Vec2::new(-1.0, -1.0),
                Vec2::new(1.0, -1.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(-1.0, -1.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(-1.0, 1.0),
            ];

            let vertex_buffer =
                self.context
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        contents: bytemuck::cast_slice(&vertices),
                        label: None,
                        usage: wgpu::BufferUsages::VERTEX,
                    });

            let view = self.render_texture.create_view(&default());
            let sampler = self.context.device.create_sampler(&default());

            let binds_layout =
                self.context
                    .device
                    .create_bind_group_layout(&BindGroupLayoutDescriptor {
                        label: Some("voxels-demo"),
                        entries: &[
                            BindGroupLayoutEntry {
                                binding: 0,
                                visibility: ShaderStages::FRAGMENT,
                                ty: BindingType::Texture {
                                    sample_type: TextureSampleType::Float { filterable: true },
                                    view_dimension: TextureViewDimension::D2,
                                    multisampled: false,
                                },
                                count: None,
                            },
                            BindGroupLayoutEntry {
                                binding: 1,
                                visibility: ShaderStages::FRAGMENT,
                                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                                count: None,
                            },
                        ],
                    });

            let binds = self.context.device.create_bind_group(&BindGroupDescriptor {
                label: Some("voxels-demo"),
                layout: &binds_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&sampler),
                    },
                ],
            });

            let pipeline_layout =
                self.context
                    .device
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: None,
                        bind_group_layouts: &[&binds_layout],
                        push_constant_ranges: &[wgpu::PushConstantRange {
                            stages: wgpu::ShaderStages::VERTEX,
                            range: 0..std::mem::size_of::<UVec2>() as u32,
                        }],
                    });

            let vertex_shader =
                self.context
                    .device
                    .create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: None,
                        source: wgpu::ShaderSource::Glsl {
                            shader: Cow::Borrowed(include_str!(
                                "../assets/shaders/screen-quad-vertex.glsl"
                            )),
                            stage: wgpu::naga::ShaderStage::Vertex,
                            defines: default(),
                        },
                    });

            let fragment_shader =
                self.context
                    .device
                    .create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: None,
                        source: wgpu::ShaderSource::Glsl {
                            shader: Cow::Borrowed(include_str!(
                                "../assets/shaders/screen-quad-fragment.glsl"
                            )),
                            stage: wgpu::naga::ShaderStage::Fragment,
                            defines: default(),
                        },
                    });

            let pipeline =
                self.context
                    .device
                    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: None,
                        layout: Some(&pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &vertex_shader,
                            entry_point: Some("main"),
                            compilation_options: default(),
                            buffers: &[wgpu::VertexBufferLayout {
                                array_stride: std::mem::size_of::<Vec2>() as u64,
                                step_mode: wgpu::VertexStepMode::Vertex,
                                attributes: &wgpu::vertex_attr_array![0 => Float32x2],
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
                            cull_mode: None,
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
            pass.set_bind_group(0, &binds, &[]);
            pass.set_push_constants(
                wgpu::ShaderStages::VERTEX,
                0,
                bytemuck::bytes_of(&viewport_size),
            );
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.draw(0..vertices.len() as u32, 0..1);
        }

        self.context.queue.submit([encoder.finish()]);

        cur_texture.present();
    }
}
