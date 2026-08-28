//! WGPU backend for the Paint VM runtime.
//!
//! This backend consumes the shared `paint-vm-gpu-core` render plan and draws
//! solid meshes into an offscreen WGPU texture. It is the first concrete GPU
//! consumer of the shared tessellation layer; glyph atlases and filters remain
//! deliberately outside this Tier 1 slice.

use std::sync::mpsc;

use paint_instructions::{PaintScene, PixelContainer};
use paint_vm_gpu_core::{
    plan_scene, GpuApiFamily, GpuBackendProfile, GpuBlendMode, GpuColor, GpuCommand, GpuFilter,
    GpuImageUpload, GpuLayer, GpuMesh, GpuPaintPlan, GpuPlanSeverity, GpuReadbackStrategy, GpuRect,
    GpuRenderPath, GpuTextureFilter,
};
use paint_vm_runtime::{
    PaintAcceleration, PaintBackendCapabilities, PaintBackendDescriptor, PaintBackendFamily,
    PaintBackendTier, PaintPlatformSupport, PaintRenderError, PaintRenderer, SupportLevel,
};
use wgpu::util::DeviceExt;

pub const VERSION: &str = "0.1.0";

const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

pub struct WgpuPaintBackend;

pub fn descriptor() -> PaintBackendDescriptor {
    PaintBackendDescriptor {
        id: "paint-vm-wgpu",
        display_name: "Paint VM WGPU",
        family: PaintBackendFamily::Wgpu,
        acceleration: PaintAcceleration::Gpu,
        tier: PaintBackendTier::Tier1Smoke,
        platforms: PaintPlatformSupport::gpu_portable(),
        capabilities: PaintBackendCapabilities {
            rect: SupportLevel::Supported,
            line: SupportLevel::Supported,
            ellipse: SupportLevel::Supported,
            path: SupportLevel::Supported,
            path_arc_to: SupportLevel::Unsupported,
            glyph_run: SupportLevel::Unsupported,
            text: SupportLevel::Unsupported,
            image: SupportLevel::Supported,
            clip: SupportLevel::Supported,
            group: SupportLevel::Supported,
            group_transform: SupportLevel::Supported,
            group_opacity: SupportLevel::Supported,
            layer: SupportLevel::Supported,
            layer_opacity: SupportLevel::Supported,
            layer_filters: SupportLevel::Supported,
            layer_blend_modes: SupportLevel::Supported,
            linear_gradient: SupportLevel::Supported,
            radial_gradient: SupportLevel::Supported,
            antialiasing: SupportLevel::Unsupported,
            offscreen_pixels: SupportLevel::Supported,
        },
        priority: 55,
    }
}

pub fn profile() -> GpuBackendProfile {
    GpuBackendProfile::tier1_textured(
        "paint-vm-wgpu",
        GpuApiFamily::Wgpu,
        GpuRenderPath::GraphicsPipeline,
        "WGSL",
        GpuReadbackStrategy::TextureCopyToBuffer,
    )
    .with_isolated_layers()
}

pub fn renderer() -> WgpuPaintBackend {
    WgpuPaintBackend
}

pub fn render(scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
    renderer().render(scene)
}

impl PaintRenderer for WgpuPaintBackend {
    fn descriptor(&self) -> PaintBackendDescriptor {
        descriptor()
    }

    fn render(&self, scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
        render_scene(scene)
    }
}

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 2]>() * 2) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

struct PreparedMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    texture_id: Option<usize>,
}

struct PreparedTexture {
    bind_group: wgpu::BindGroup,
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
}

struct RenderTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

struct LayerFrame {
    parent_target: usize,
    descriptor: GpuLayer,
    composite_clip: GpuRect,
}

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct FilterParams {
    kind: u32,
    padding: [u32; 3],
    params: [f32; 4],
    color: [f32; 4],
    matrix: [[f32; 4]; 4],
    bias: [f32; 4],
}

#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct CompositeParams {
    blend_mode: u32,
    padding: [u32; 3],
    opacity: f32,
    opacity_padding: [f32; 3],
    clip: [f32; 4],
}

struct LayerPipelines {
    filter_layout: wgpu::BindGroupLayout,
    composite_layout: wgpu::BindGroupLayout,
    filter_pipeline: wgpu::ComputePipeline,
    composite_pipeline: wgpu::ComputePipeline,
}

fn render_scene(scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
    let plan = plan_scene(scene);
    validate_plan(&plan)?;
    if plan.width == 0 || plan.height == 0 {
        return Ok(PixelContainer::new(plan.width, plan.height));
    }
    pollster::block_on(render_plan(plan))
}

async fn render_plan(plan: GpuPaintPlan) -> Result<PixelContainer, PaintRenderError> {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .ok_or(PaintRenderError::BackendUnavailable {
            backend: "paint-vm-wgpu",
            reason: "no WGPU adapter available for offscreen rendering",
        })?;
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("paint-vm-wgpu-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )
        .await
        .map_err(|err| PaintRenderError::RenderFailed {
            backend: "paint-vm-wgpu",
            message: format!("failed to create WGPU device: {err}"),
        })?;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("paint-vm-wgpu-textured-shader"),
        source: wgpu::ShaderSource::Wgsl(TEXTURED_SHADER.into()),
    });
    let viewport_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("paint-vm-wgpu-viewport"),
        contents: bytemuck::cast_slice(&[plan.width as f32, plan.height as f32]),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let viewport_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("paint-vm-wgpu-viewport-bind-group-layout"),
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
        });
    let texture_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("paint-vm-wgpu-texture-bind-group-layout"),
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
        });
    let viewport_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("paint-vm-wgpu-viewport-bind-group"),
        layout: &viewport_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: viewport_buffer.as_entire_binding(),
        }],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("paint-vm-wgpu-pipeline-layout"),
        bind_group_layouts: &[&viewport_bind_group_layout, &texture_bind_group_layout],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("paint-vm-wgpu-textured-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[Vertex::layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: TARGET_FORMAT,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    });
    let layer_pipelines = prepare_layer_pipelines(&device);

    let mut targets = vec![create_render_target(
        &device,
        plan.width,
        plan.height,
        "paint-vm-wgpu-root-target",
    )];
    let prepared_meshes = prepare_meshes(&device, &plan.meshes);
    let (white_texture, prepared_textures) =
        prepare_textures(&device, &queue, &texture_bind_group_layout, &plan.images);
    let row_bytes = plan.width * 4;
    let padded_row_bytes = align_to(row_bytes, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let readback_size = padded_row_bytes as u64 * plan.height as u64;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("paint-vm-wgpu-readback"),
        size: readback_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("paint-vm-wgpu-encoder"),
    });
    clear_target(
        &mut encoder,
        &targets[0].view,
        to_wgpu_color(plan.background),
    );
    let full_clip = GpuRect {
        x: 0.0,
        y: 0.0,
        width: plan.width as f32,
        height: plan.height as f32,
    };
    let mut clip_stack = vec![full_clip];
    let mut layer_stack: Vec<LayerFrame> = Vec::new();
    let mut current_target = 0;

    for command in &plan.commands {
        match command {
            GpuCommand::DrawMesh { mesh_id } => {
                let clip = *clip_stack.last().unwrap();
                if clip.width <= 0.0 || clip.height <= 0.0 {
                    continue;
                }
                if let Some(mesh) = prepared_meshes.get(*mesh_id) {
                    let texture = mesh
                        .texture_id
                        .and_then(|texture_id| prepared_textures.get(texture_id))
                        .unwrap_or(&white_texture);
                    draw_mesh(
                        &mut encoder,
                        &targets[current_target].view,
                        &pipeline,
                        &viewport_bind_group,
                        mesh,
                        texture,
                        clip,
                        plan.width,
                        plan.height,
                    );
                }
            }
            GpuCommand::PushClip { rect } => {
                let clipped = intersect_rect(*clip_stack.last().unwrap(), *rect);
                clip_stack.push(clipped);
            }
            GpuCommand::PopClip => {
                if clip_stack.len() > 1 {
                    clip_stack.pop();
                }
            }
            GpuCommand::BeginLayer(descriptor) => {
                let layer_target = targets.len();
                targets.push(create_render_target(
                    &device,
                    plan.width,
                    plan.height,
                    "paint-vm-wgpu-layer-target",
                ));
                clear_target(
                    &mut encoder,
                    &targets[layer_target].view,
                    wgpu::Color::TRANSPARENT,
                );
                layer_stack.push(LayerFrame {
                    parent_target: current_target,
                    descriptor: descriptor.clone(),
                    composite_clip: *clip_stack.last().unwrap(),
                });
                current_target = layer_target;
            }
            GpuCommand::EndLayer => {
                let Some(frame) = layer_stack.pop() else {
                    continue;
                };
                let mut filtered = current_target;
                for filter in &frame.descriptor.filters {
                    let destination = targets.len();
                    targets.push(create_render_target(
                        &device,
                        plan.width,
                        plan.height,
                        "paint-vm-wgpu-filter-target",
                    ));
                    apply_filter(
                        &device,
                        &mut encoder,
                        &layer_pipelines,
                        &targets[filtered].view,
                        &targets[destination].view,
                        filter,
                        plan.width,
                        plan.height,
                    );
                    filtered = destination;
                }
                let destination = targets.len();
                targets.push(create_render_target(
                    &device,
                    plan.width,
                    plan.height,
                    "paint-vm-wgpu-composite-target",
                ));
                composite_layer(
                    &device,
                    &mut encoder,
                    &layer_pipelines,
                    &targets[filtered].view,
                    &targets[frame.parent_target].view,
                    &targets[destination].view,
                    &frame.descriptor,
                    frame.composite_clip,
                    plan.width,
                    plan.height,
                );
                current_target = destination;
            }
            GpuCommand::DrawText(_) | GpuCommand::DrawGlyphRun(_) => {}
        }
    }
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &targets[current_target].texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &readback,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_row_bytes),
                rows_per_image: Some(plan.height),
            },
        },
        wgpu::Extent3d {
            width: plan.width,
            height: plan.height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));

    let slice = readback.slice(..);
    let (tx, rx) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv()
        .map_err(|err| PaintRenderError::RenderFailed {
            backend: "paint-vm-wgpu",
            message: format!("readback callback failed: {err}"),
        })?
        .map_err(|err| PaintRenderError::RenderFailed {
            backend: "paint-vm-wgpu",
            message: format!("failed to map readback buffer: {err:?}"),
        })?;

    let mapped = slice.get_mapped_range();
    let mut data = vec![0u8; row_bytes as usize * plan.height as usize];
    for row in 0..plan.height as usize {
        let src_start = row * padded_row_bytes as usize;
        let src_end = src_start + row_bytes as usize;
        let dst_start = row * row_bytes as usize;
        data[dst_start..dst_start + row_bytes as usize]
            .copy_from_slice(&mapped[src_start..src_end]);
    }
    drop(mapped);
    readback.unmap();

    Ok(PixelContainer::from_data(plan.width, plan.height, data))
}

fn create_render_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    label: &'static str,
) -> RenderTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: TARGET_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    RenderTarget { texture, view }
}

fn clear_target(encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, color: wgpu::Color) {
    let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("paint-vm-wgpu-clear-pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(color),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
    });
}

#[allow(clippy::too_many_arguments)]
fn draw_mesh(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    viewport_bind_group: &wgpu::BindGroup,
    mesh: &PreparedMesh,
    texture: &PreparedTexture,
    clip: GpuRect,
    width: u32,
    height: u32,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("paint-vm-wgpu-draw-pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, viewport_bind_group, &[]);
    set_scissor(&mut pass, clip, width, height);
    pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
    pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
    pass.set_bind_group(1, &texture.bind_group, &[]);
    pass.draw_indexed(0..mesh.index_count, 0, 0..1);
}

fn prepare_layer_pipelines(device: &wgpu::Device) -> LayerPipelines {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("paint-vm-wgpu-layer-shader"),
        source: wgpu::ShaderSource::Wgsl(LAYER_SHADER.into()),
    });
    let filter_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("paint-vm-wgpu-filter-layout"),
        entries: &[
            sampled_texture_entry(0),
            storage_texture_entry(1),
            uniform_entry(2),
        ],
    });
    let composite_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("paint-vm-wgpu-composite-layout"),
        entries: &[
            sampled_texture_entry(0),
            sampled_texture_entry(1),
            storage_texture_entry(2),
            uniform_entry(3),
        ],
    });
    let filter_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("paint-vm-wgpu-filter-pipeline-layout"),
        bind_group_layouts: &[&filter_layout],
        push_constant_ranges: &[],
    });
    let composite_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("paint-vm-wgpu-composite-pipeline-layout"),
            bind_group_layouts: &[&composite_layout],
            push_constant_ranges: &[],
        });
    let filter_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("paint-vm-wgpu-filter-pipeline"),
        layout: Some(&filter_pipeline_layout),
        module: &shader,
        entry_point: "paint_filter",
    });
    let composite_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("paint-vm-wgpu-composite-pipeline"),
        layout: Some(&composite_pipeline_layout),
        module: &shader,
        entry_point: "paint_composite",
    });
    LayerPipelines {
        filter_layout,
        composite_layout,
        filter_pipeline,
        composite_pipeline,
    }
}

fn sampled_texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn storage_texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::StorageTexture {
            access: wgpu::StorageTextureAccess::WriteOnly,
            format: TARGET_FORMAT,
            view_dimension: wgpu::TextureViewDimension::D2,
        },
        count: None,
    }
}

fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_filter(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    pipelines: &LayerPipelines,
    source: &wgpu::TextureView,
    destination: &wgpu::TextureView,
    filter: &GpuFilter,
    width: u32,
    height: u32,
) {
    let params = filter_params(filter);
    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("paint-vm-wgpu-filter-params"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("paint-vm-wgpu-filter-bind-group"),
        layout: &pipelines.filter_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(source),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(destination),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: params_buffer.as_entire_binding(),
            },
        ],
    });
    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("paint-vm-wgpu-filter-pass"),
        timestamp_writes: None,
    });
    pass.set_pipeline(&pipelines.filter_pipeline);
    pass.set_bind_group(0, &bind_group, &[]);
    pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
}

#[allow(clippy::too_many_arguments)]
fn composite_layer(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    pipelines: &LayerPipelines,
    source: &wgpu::TextureView,
    backdrop: &wgpu::TextureView,
    destination: &wgpu::TextureView,
    layer: &GpuLayer,
    clip: GpuRect,
    width: u32,
    height: u32,
) {
    let params = CompositeParams {
        blend_mode: blend_mode_index(layer.blend_mode),
        padding: [0; 3],
        opacity: layer.opacity,
        opacity_padding: [0.0; 3],
        clip: [clip.x, clip.y, clip.width, clip.height],
    };
    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("paint-vm-wgpu-composite-params"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("paint-vm-wgpu-composite-bind-group"),
        layout: &pipelines.composite_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(source),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(backdrop),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(destination),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: params_buffer.as_entire_binding(),
            },
        ],
    });
    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("paint-vm-wgpu-composite-pass"),
        timestamp_writes: None,
    });
    pass.set_pipeline(&pipelines.composite_pipeline);
    pass.set_bind_group(0, &bind_group, &[]);
    pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
}

fn filter_params(filter: &GpuFilter) -> FilterParams {
    let mut params = FilterParams {
        kind: 0,
        padding: [0; 3],
        params: [0.0; 4],
        color: [0.0; 4],
        matrix: [[0.0; 4]; 4],
        bias: [0.0; 4],
    };
    match filter {
        GpuFilter::Blur { radius } => params.params[0] = *radius,
        GpuFilter::DropShadow {
            dx,
            dy,
            blur,
            color,
        } => {
            params.kind = 1;
            params.params = [*dx, *dy, 0.0, *blur];
            params.color = [color.r, color.g, color.b, color.a];
        }
        GpuFilter::ColorMatrix { matrix } => {
            params.kind = 2;
            for row in 0..4 {
                let offset = row * 5;
                params.matrix[row].copy_from_slice(&matrix[offset..offset + 4]);
                params.bias[row] = matrix[offset + 4];
            }
        }
        GpuFilter::Brightness { amount } => {
            params.kind = 3;
            params.params[0] = *amount;
        }
        GpuFilter::Contrast { amount } => {
            params.kind = 4;
            params.params[0] = *amount;
        }
        GpuFilter::Saturate { amount } => {
            params.kind = 5;
            params.params[0] = *amount;
        }
        GpuFilter::HueRotate { angle_degrees } => {
            params.kind = 6;
            params.params[0] = *angle_degrees;
        }
        GpuFilter::Invert { amount } => {
            params.kind = 7;
            params.params[0] = *amount;
        }
        GpuFilter::Opacity { amount } => {
            params.kind = 8;
            params.params[0] = *amount;
        }
    }
    params
}

fn blend_mode_index(mode: GpuBlendMode) -> u32 {
    match mode {
        GpuBlendMode::Normal => 0,
        GpuBlendMode::Multiply => 1,
        GpuBlendMode::Screen => 2,
        GpuBlendMode::Overlay => 3,
        GpuBlendMode::Darken => 4,
        GpuBlendMode::Lighten => 5,
        GpuBlendMode::ColorDodge => 6,
        GpuBlendMode::ColorBurn => 7,
        GpuBlendMode::HardLight => 8,
        GpuBlendMode::SoftLight => 9,
        GpuBlendMode::Difference => 10,
        GpuBlendMode::Exclusion => 11,
        GpuBlendMode::Hue => 12,
        GpuBlendMode::Saturation => 13,
        GpuBlendMode::Color => 14,
        GpuBlendMode::Luminosity => 15,
    }
}

fn validate_plan(plan: &GpuPaintPlan) -> Result<(), PaintRenderError> {
    if let Some(diagnostic) = plan
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity == GpuPlanSeverity::Unsupported)
    {
        return Err(PaintRenderError::RenderFailed {
            backend: "paint-vm-wgpu",
            message: format!("unsupported GPU plan feature: {}", diagnostic.message),
        });
    }
    if plan.commands.iter().any(|command| {
        matches!(
            command,
            GpuCommand::DrawText(_) | GpuCommand::DrawGlyphRun(_)
        )
    }) {
        return Err(PaintRenderError::RenderFailed {
            backend: "paint-vm-wgpu",
            message: "text and glyph atlas rendering are not wired in the WGPU backend yet"
                .to_string(),
        });
    }
    Ok(())
}

fn prepare_meshes(device: &wgpu::Device, meshes: &[GpuMesh]) -> Vec<PreparedMesh> {
    meshes
        .iter()
        .map(|mesh| {
            let vertices: Vec<Vertex> = mesh
                .vertices
                .iter()
                .map(|vertex| Vertex {
                    position: [vertex.position.x, vertex.position.y],
                    uv: vertex.uv,
                    color: [
                        vertex.color.r,
                        vertex.color.g,
                        vertex.color.b,
                        vertex.color.a,
                    ],
                })
                .collect();
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("paint-vm-wgpu-mesh-vertices"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("paint-vm-wgpu-mesh-indices"),
                contents: bytemuck::cast_slice(&mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            PreparedMesh {
                vertex_buffer,
                index_buffer,
                index_count: mesh.indices.len() as u32,
                texture_id: mesh.texture_id,
            }
        })
        .collect()
}

fn prepare_textures(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    images: &[GpuImageUpload],
) -> (PreparedTexture, Vec<PreparedTexture>) {
    let white_texture = prepare_texture(
        device,
        queue,
        layout,
        "paint-vm-wgpu-white-texture",
        1,
        1,
        &[255, 255, 255, 255],
        GpuTextureFilter::Nearest,
    );
    let textures = images
        .iter()
        .enumerate()
        .map(|(index, image)| {
            prepare_texture(
                device,
                queue,
                layout,
                texture_label(index),
                image.width,
                image.height,
                &image.data,
                image.filter,
            )
        })
        .collect();
    (white_texture, textures)
}

// GPU texture upload genuinely needs all of device/queue/layout/label/dims/
// data/filter; bundling them into a struct would only move the argument list.
#[allow(clippy::too_many_arguments)]
fn prepare_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    label: &'static str,
    width: u32,
    height: u32,
    data: &[u8],
    filter: GpuTextureFilter,
) -> PreparedTexture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: TARGET_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let filter_mode = match filter {
        GpuTextureFilter::Nearest => wgpu::FilterMode::Nearest,
        GpuTextureFilter::Linear => wgpu::FilterMode::Linear,
    };
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some(label),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: filter_mode,
        min_filter: filter_mode,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..wgpu::SamplerDescriptor::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    PreparedTexture {
        bind_group,
        _texture: texture,
        _view: view,
        _sampler: sampler,
    }
}

fn texture_label(index: usize) -> &'static str {
    match index {
        0 => "paint-vm-wgpu-image-texture-0",
        1 => "paint-vm-wgpu-image-texture-1",
        2 => "paint-vm-wgpu-image-texture-2",
        _ => "paint-vm-wgpu-image-texture",
    }
}

fn to_wgpu_color(color: GpuColor) -> wgpu::Color {
    wgpu::Color {
        r: color.r as f64,
        g: color.g as f64,
        b: color.b as f64,
        a: color.a as f64,
    }
}

fn align_to(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}

fn intersect_rect(a: GpuRect, b: GpuRect) -> GpuRect {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width);
    let bottom = (a.y + a.height).min(b.y + b.height);
    GpuRect {
        x: left,
        y: top,
        width: (right - left).max(0.0),
        height: (bottom - top).max(0.0),
    }
}

fn set_scissor(pass: &mut wgpu::RenderPass<'_>, rect: GpuRect, width: u32, height: u32) {
    let x = rect.x.floor().max(0.0).min(width as f32) as u32;
    let y = rect.y.floor().max(0.0).min(height as f32) as u32;
    let right = (rect.x + rect.width).ceil().max(0.0).min(width as f32) as u32;
    let bottom = (rect.y + rect.height).ceil().max(0.0).min(height as f32) as u32;
    pass.set_scissor_rect(x, y, right.saturating_sub(x), bottom.saturating_sub(y));
}

const TEXTURED_SHADER: &str = r#"
struct Viewport {
    size: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> viewport: Viewport;

@group(1) @binding(0)
var image_texture: texture_2d<f32>;

@group(1) @binding(1)
var image_sampler: sampler;

struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var output: VertexOut;
    let clip_x = input.position.x / viewport.size.x * 2.0 - 1.0;
    let clip_y = 1.0 - input.position.y / viewport.size.y * 2.0;
    output.position = vec4<f32>(clip_x, clip_y, 0.0, 1.0);
    output.uv = input.uv;
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return input.color * textureSample(image_texture, image_sampler, input.uv);
}
"#;

const LAYER_SHADER: &str = r#"
struct FilterParams {
    kind: u32,
    padding0: u32,
    padding1: u32,
    padding2: u32,
    params: vec4<f32>,
    color: vec4<f32>,
    matrix0: vec4<f32>,
    matrix1: vec4<f32>,
    matrix2: vec4<f32>,
    matrix3: vec4<f32>,
    bias: vec4<f32>,
};

struct CompositeParams {
    blend_mode: u32,
    padding0: u32,
    padding1: u32,
    padding2: u32,
    opacity: f32,
    opacity_padding0: f32,
    opacity_padding1: f32,
    opacity_padding2: f32,
    clip: vec4<f32>,
};

@group(0) @binding(0) var filter_source: texture_2d<f32>;
@group(0) @binding(1) var filter_destination: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> filter_params: FilterParams;

fn straight_color(value: vec4<f32>) -> vec4<f32> {
    if (value.a > 0.000001) {
        return vec4<f32>(value.rgb / value.a, value.a);
    }
    return vec4<f32>(0.0);
}

fn premultiplied_color(value: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(value.rgb * value.a, value.a);
}

fn hue_rotated(color: vec3<f32>, degrees: f32) -> vec3<f32> {
    let angle = degrees * (3.14159265358979323846 / 180.0);
    let c = cos(angle);
    let s = sin(angle);
    return vec3<f32>(
        dot(color, vec3<f32>(0.213 + c * 0.787 - s * 0.213,
                             0.715 - c * 0.715 - s * 0.715,
                             0.072 - c * 0.072 + s * 0.928)),
        dot(color, vec3<f32>(0.213 - c * 0.213 + s * 0.143,
                             0.715 + c * 0.285 + s * 0.140,
                             0.072 - c * 0.072 - s * 0.283)),
        dot(color, vec3<f32>(0.213 - c * 0.213 - s * 0.787,
                             0.715 - c * 0.715 + s * 0.715,
                             0.072 + c * 0.928 + s * 0.072))
    );
}

@compute @workgroup_size(8, 8)
fn paint_filter(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dimensions = textureDimensions(filter_source);
    if (gid.x >= dimensions.x || gid.y >= dimensions.y) {
        return;
    }
    let position = vec2<i32>(gid.xy);

    if (filter_params.kind == 0u) {
        let radius = min(i32(round(filter_params.params.x)), 32);
        var total = vec4<f32>(0.0);
        var count = 0u;
        for (var y = -radius; y <= radius; y = y + 1) {
            for (var x = -radius; x <= radius; x = x + 1) {
                let sample_position = position + vec2<i32>(x, y);
                if (sample_position.x >= 0 && sample_position.y >= 0 &&
                    sample_position.x < i32(dimensions.x) &&
                    sample_position.y < i32(dimensions.y)) {
                    total += textureLoad(filter_source, sample_position, 0);
                }
                count += 1u;
            }
        }
        textureStore(filter_destination, position, total / max(f32(count), 1.0));
        return;
    }

    let input = textureLoad(filter_source, position, 0);
    if (filter_params.kind == 1u) {
        let radius = min(i32(round(filter_params.params.w)), 32);
        let center = position - vec2<i32>(round(filter_params.params.xy));
        var alpha = 0.0;
        var count = 0u;
        for (var y = -radius; y <= radius; y = y + 1) {
            for (var x = -radius; x <= radius; x = x + 1) {
                let sample_position = center + vec2<i32>(x, y);
                if (sample_position.x >= 0 && sample_position.y >= 0 &&
                    sample_position.x < i32(dimensions.x) &&
                    sample_position.y < i32(dimensions.y)) {
                    alpha += textureLoad(filter_source, sample_position, 0).a;
                }
                count += 1u;
            }
        }
        alpha = alpha / max(f32(count), 1.0) * filter_params.color.a;
        let shadow = vec4<f32>(filter_params.color.rgb * alpha, alpha);
        textureStore(filter_destination, position, input + shadow * (1.0 - input.a));
        return;
    }

    var straight = straight_color(input);
    if (filter_params.kind == 2u) {
        let channels = straight;
        straight = vec4<f32>(
            dot(filter_params.matrix0, channels) + filter_params.bias.x,
            dot(filter_params.matrix1, channels) + filter_params.bias.y,
            dot(filter_params.matrix2, channels) + filter_params.bias.z,
            dot(filter_params.matrix3, channels) + filter_params.bias.w
        );
    } else if (filter_params.kind == 3u) {
        straight = vec4<f32>(straight.rgb * filter_params.params.x, straight.a);
    } else if (filter_params.kind == 4u) {
        straight = vec4<f32>((straight.rgb - vec3<f32>(0.5)) * filter_params.params.x + vec3<f32>(0.5), straight.a);
    } else if (filter_params.kind == 5u) {
        let luminance = dot(straight.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        straight = vec4<f32>(mix(vec3<f32>(luminance), straight.rgb, filter_params.params.x), straight.a);
    } else if (filter_params.kind == 6u) {
        straight = vec4<f32>(hue_rotated(straight.rgb, filter_params.params.x), straight.a);
    } else if (filter_params.kind == 7u) {
        straight = vec4<f32>(mix(straight.rgb, vec3<f32>(1.0) - straight.rgb, filter_params.params.x), straight.a);
    } else if (filter_params.kind == 8u) {
        straight = vec4<f32>(straight.rgb, straight.a * filter_params.params.x);
    }
    textureStore(filter_destination, position, premultiplied_color(clamp(straight, vec4<f32>(0.0), vec4<f32>(1.0))));
}

@group(0) @binding(0) var composite_source: texture_2d<f32>;
@group(0) @binding(1) var composite_backdrop: texture_2d<f32>;
@group(0) @binding(2) var composite_destination: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(3) var<uniform> composite: CompositeParams;

fn luminance(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.3, 0.59, 0.11));
}

fn saturation(color: vec3<f32>) -> f32 {
    return max(color.r, max(color.g, color.b)) - min(color.r, min(color.g, color.b));
}

fn clip_color(input: vec3<f32>) -> vec3<f32> {
    var color = input;
    let l = luminance(color);
    let minimum = min(color.r, min(color.g, color.b));
    let maximum = max(color.r, max(color.g, color.b));
    if (minimum < 0.0) {
        color = vec3<f32>(l) + ((color - vec3<f32>(l)) * l) / max(l - minimum, 0.000001);
    }
    if (maximum > 1.0) {
        color = vec3<f32>(l) + ((color - vec3<f32>(l)) * (1.0 - l)) / max(maximum - l, 0.000001);
    }
    return color;
}

fn set_luminance(color: vec3<f32>, value: f32) -> vec3<f32> {
    return clip_color(color + vec3<f32>(value - luminance(color)));
}

fn set_saturation(color: vec3<f32>, value: f32) -> vec3<f32> {
    let minimum = min(color.r, min(color.g, color.b));
    let maximum = max(color.r, max(color.g, color.b));
    if (maximum <= minimum) {
        return vec3<f32>(0.0);
    }
    return (color - vec3<f32>(minimum)) * (value / (maximum - minimum));
}

fn soft_light(backdrop: f32, source: f32) -> f32 {
    if (source <= 0.5) {
        return backdrop - (1.0 - 2.0 * source) * backdrop * (1.0 - backdrop);
    }
    var d = sqrt(backdrop);
    if (backdrop <= 0.25) {
        d = ((16.0 * backdrop - 12.0) * backdrop + 4.0) * backdrop;
    }
    return backdrop + (2.0 * source - 1.0) * (d - backdrop);
}

fn blend_rgb(mode: u32, backdrop: vec3<f32>, source: vec3<f32>) -> vec3<f32> {
    if (mode == 1u) { return backdrop * source; }
    if (mode == 2u) { return backdrop + source - backdrop * source; }
    if (mode == 3u) { return select(2.0 * backdrop * source, 1.0 - 2.0 * (1.0 - backdrop) * (1.0 - source), backdrop >= vec3<f32>(0.5)); }
    if (mode == 4u) { return min(backdrop, source); }
    if (mode == 5u) { return max(backdrop, source); }
    if (mode == 6u) { return select(backdrop / max(vec3<f32>(1.0) - source, vec3<f32>(0.000001)), vec3<f32>(1.0), source >= vec3<f32>(1.0)); }
    if (mode == 7u) { return select(vec3<f32>(1.0) - (vec3<f32>(1.0) - backdrop) / max(source, vec3<f32>(0.000001)), vec3<f32>(0.0), source <= vec3<f32>(0.0)); }
    if (mode == 8u) { return select(2.0 * backdrop * source, 1.0 - 2.0 * (1.0 - backdrop) * (1.0 - source), source >= vec3<f32>(0.5)); }
    if (mode == 9u) { return vec3<f32>(soft_light(backdrop.r, source.r), soft_light(backdrop.g, source.g), soft_light(backdrop.b, source.b)); }
    if (mode == 10u) { return abs(backdrop - source); }
    if (mode == 11u) { return backdrop + source - 2.0 * backdrop * source; }
    if (mode == 12u) { return set_luminance(set_saturation(source, saturation(backdrop)), luminance(backdrop)); }
    if (mode == 13u) { return set_luminance(set_saturation(backdrop, saturation(source)), luminance(backdrop)); }
    if (mode == 14u) { return set_luminance(source, luminance(backdrop)); }
    if (mode == 15u) { return set_luminance(backdrop, luminance(source)); }
    return source;
}

@compute @workgroup_size(8, 8)
fn paint_composite(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dimensions = textureDimensions(composite_source);
    if (gid.x >= dimensions.x || gid.y >= dimensions.y) {
        return;
    }
    let position = vec2<i32>(gid.xy);
    let source_pixel = textureLoad(composite_source, position, 0);
    let destination_pixel = textureLoad(composite_backdrop, position, 0);
    if (f32(gid.x) < composite.clip.x || f32(gid.y) < composite.clip.y ||
        f32(gid.x) >= composite.clip.x + composite.clip.z ||
        f32(gid.y) >= composite.clip.y + composite.clip.w) {
        textureStore(composite_destination, position, destination_pixel);
        return;
    }
    let source_alpha = clamp(source_pixel.a * composite.opacity, 0.0, 1.0);
    let destination_alpha = destination_pixel.a;
    var source_color = vec3<f32>(0.0);
    if (source_pixel.a > 0.000001) {
        source_color = source_pixel.rgb / source_pixel.a;
    }
    var destination_color = vec3<f32>(0.0);
    if (destination_alpha > 0.000001) {
        destination_color = destination_pixel.rgb / destination_alpha;
    }
    let blended = clamp(blend_rgb(composite.blend_mode, destination_color, source_color), vec3<f32>(0.0), vec3<f32>(1.0));
    let result = (1.0 - source_alpha) * destination_pixel.rgb
        + (1.0 - destination_alpha) * source_color * source_alpha
        + source_alpha * destination_alpha * blended;
    let alpha = source_alpha + destination_alpha * (1.0 - source_alpha);
    textureStore(composite_destination, position, vec4<f32>(clamp(result, vec3<f32>(0.0), vec3<f32>(1.0)), alpha));
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use paint_instructions::{
        BlendMode, GradientKind, GradientStop, ImageSrc, PaintBase, PaintGradient, PaintImage,
        PaintInstruction, PaintLayer, PaintRect, PaintText,
    };
    use paint_vm_runtime::{PaintBackendPreference, PaintBackendRegistry, PaintRenderOptions};

    #[test]
    fn exposes_tier1_descriptor() {
        let descriptor = descriptor();
        assert_eq!(descriptor.id, "paint-vm-wgpu");
        assert_eq!(descriptor.family, PaintBackendFamily::Wgpu);
        assert_eq!(descriptor.tier, PaintBackendTier::Tier1Smoke);
        assert_eq!(descriptor.capabilities.rect, SupportLevel::Supported);
        assert_eq!(descriptor.capabilities.image, SupportLevel::Supported);
        assert_eq!(
            descriptor.capabilities.linear_gradient,
            SupportLevel::Supported
        );
        assert_eq!(
            descriptor.capabilities.radial_gradient,
            SupportLevel::Supported
        );
    }

    #[test]
    fn exposes_wgpu_gpu_profile() {
        let profile = profile();
        assert_eq!(profile.id, "paint-vm-wgpu");
        assert_eq!(profile.family, GpuApiFamily::Wgpu);
        assert_eq!(profile.render_path, GpuRenderPath::GraphicsPipeline);
        assert_eq!(profile.readback, GpuReadbackStrategy::TextureCopyToBuffer);
        assert!(profile.supports_texture_sampling);
        assert!(profile.supports_linear_gradients);
        assert!(profile.supports_radial_gradients);
        assert!(profile.supports_isolated_layers);
        assert!(profile.supports_layer_filters);
        assert!(profile.supports_layer_blend_modes);
    }

    #[test]
    fn isolated_layer_plan_is_accepted_by_the_offscreen_executor_path() {
        let plan = GpuPaintPlan {
            width: 1,
            height: 1,
            background: paint_vm_gpu_core::GpuColor {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
            commands: vec![
                GpuCommand::BeginLayer(paint_vm_gpu_core::GpuLayer {
                    opacity: 0.5,
                    blend_mode: paint_vm_gpu_core::GpuBlendMode::Multiply,
                    filters: vec![],
                }),
                GpuCommand::EndLayer,
            ],
            meshes: vec![],
            images: vec![],
            diagnostics: vec![],
        };

        validate_plan(&plan).expect("WGPU executes isolated layers without flattening them");
    }

    #[test]
    fn shared_layer_filters_keep_stable_wgsl_discriminants() {
        let matrix = [
            1.0, 0.0, 0.0, 0.0, 0.1, 0.0, 1.0, 0.0, 0.0, 0.2, 0.0, 0.0, 1.0, 0.0, 0.3, 0.0, 0.0,
            0.0, 1.0, 0.4,
        ];
        let filters = [
            GpuFilter::Blur { radius: 2.0 },
            GpuFilter::DropShadow {
                dx: 1.0,
                dy: 2.0,
                blur: 3.0,
                color: GpuColor {
                    r: 0.1,
                    g: 0.2,
                    b: 0.3,
                    a: 0.4,
                },
            },
            GpuFilter::ColorMatrix { matrix },
            GpuFilter::Brightness { amount: 1.1 },
            GpuFilter::Contrast { amount: 1.2 },
            GpuFilter::Saturate { amount: 1.3 },
            GpuFilter::HueRotate {
                angle_degrees: 45.0,
            },
            GpuFilter::Invert { amount: 0.5 },
            GpuFilter::Opacity { amount: 0.6 },
        ];
        for (expected, filter) in filters.iter().enumerate() {
            assert_eq!(filter_params(filter).kind, expected as u32);
        }
        let encoded = filter_params(&GpuFilter::ColorMatrix { matrix });
        assert_eq!(encoded.matrix[0], [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(encoded.bias, [0.1, 0.2, 0.3, 0.4]);
        assert_eq!(std::mem::size_of::<FilterParams>(), 128);
        assert_eq!(std::mem::size_of::<CompositeParams>(), 48);
    }

    #[test]
    fn shared_blend_modes_keep_stable_wgsl_discriminants() {
        let modes = [
            GpuBlendMode::Normal,
            GpuBlendMode::Multiply,
            GpuBlendMode::Screen,
            GpuBlendMode::Overlay,
            GpuBlendMode::Darken,
            GpuBlendMode::Lighten,
            GpuBlendMode::ColorDodge,
            GpuBlendMode::ColorBurn,
            GpuBlendMode::HardLight,
            GpuBlendMode::SoftLight,
            GpuBlendMode::Difference,
            GpuBlendMode::Exclusion,
            GpuBlendMode::Hue,
            GpuBlendMode::Saturation,
            GpuBlendMode::Color,
            GpuBlendMode::Luminosity,
        ];
        for (expected, mode) in modes.into_iter().enumerate() {
            assert_eq!(blend_mode_index(mode), expected as u32);
        }
    }

    #[test]
    fn isolated_layers_match_the_shared_visual_oracle_when_an_adapter_is_available() {
        let scene = venture_browser_visual_fixtures::isolated_gpu_layer_scene();
        let pixels = match render(&scene) {
            Ok(pixels) => pixels,
            Err(PaintRenderError::BackendUnavailable { .. }) => return,
            Err(error) => panic!("WGPU isolated layer rendering failed: {error:?}"),
        };
        venture_browser_visual_fixtures::assert_isolated_gpu_layer_pixels(&pixels)
            .expect("WGPU isolated layer fixture");
    }

    #[test]
    fn nested_layers_apply_each_scope_opacity_once_when_an_adapter_is_available() {
        let inner = PaintLayer {
            base: PaintBase::default(),
            children: vec![PaintInstruction::Rect(PaintRect::filled(
                0.0, 0.0, 2.0, 2.0, "#ff0000",
            ))],
            filters: None,
            blend_mode: Some(BlendMode::Normal),
            opacity: Some(0.5),
            transform: None,
        };
        let outer = PaintLayer {
            base: PaintBase::default(),
            children: vec![PaintInstruction::Layer(inner)],
            filters: None,
            blend_mode: Some(BlendMode::Normal),
            opacity: Some(0.5),
            transform: None,
        };
        let mut scene = PaintScene::new(2.0, 2.0);
        scene
            .instructions
            .push(PaintInstruction::Rect(PaintRect::filled(
                0.0, 0.0, 2.0, 2.0, "#808080",
            )));
        scene.instructions.push(PaintInstruction::Layer(outer));

        let pixels = match render(&scene) {
            Ok(pixels) => pixels,
            Err(PaintRenderError::BackendUnavailable { .. }) => return,
            Err(error) => panic!("WGPU nested layer rendering failed: {error:?}"),
        };
        let (red, green, blue, alpha) = pixels.pixel_at(1, 1);
        assert!((red as i16 - 160).abs() <= 2, "red channel: {red}");
        assert!((green as i16 - 96).abs() <= 2, "green channel: {green}");
        assert!((blue as i16 - 96).abs() <= 2, "blue channel: {blue}");
        assert_eq!(alpha, 255);
    }

    #[test]
    fn runtime_selects_wgpu_for_solid_rect_scene() {
        let backend = renderer();
        let mut registry = PaintBackendRegistry::new();
        registry.register(&backend);
        let mut scene = PaintScene::new(8.0, 8.0);
        scene
            .instructions
            .push(PaintInstruction::Rect(PaintRect::filled(
                1.0, 1.0, 4.0, 4.0, "#000000",
            )));

        let selected = registry
            .select(
                &scene,
                PaintRenderOptions {
                    preference: PaintBackendPreference::Named("paint-vm-wgpu".to_string()),
                    ..PaintRenderOptions::default()
                },
            )
            .unwrap();
        assert_eq!(selected.descriptor().id, "paint-vm-wgpu");
    }

    #[test]
    fn runtime_selects_wgpu_for_pixel_image_scene() {
        let backend = renderer();
        let mut registry = PaintBackendRegistry::new();
        registry.register(&backend);
        let mut pixels = PixelContainer::new(1, 1);
        pixels.set_pixel(0, 0, 0, 128, 255, 255);
        let mut scene = PaintScene::new(8.0, 8.0);
        scene.instructions.push(PaintInstruction::Image(PaintImage {
            base: PaintBase::default(),
            x: 1.0,
            y: 1.0,
            width: 4.0,
            height: 4.0,
            src: ImageSrc::Pixels(pixels),
            opacity: None,
        }));

        let selected = registry
            .select(
                &scene,
                PaintRenderOptions {
                    preference: PaintBackendPreference::Named("paint-vm-wgpu".to_string()),
                    ..PaintRenderOptions::default()
                },
            )
            .unwrap();
        assert_eq!(selected.descriptor().id, "paint-vm-wgpu");
    }

    #[test]
    fn runtime_selects_wgpu_for_linear_gradient_scene() {
        let backend = renderer();
        let mut registry = PaintBackendRegistry::new();
        registry.register(&backend);
        let mut scene = PaintScene::new(8.0, 2.0);
        scene
            .instructions
            .push(PaintInstruction::Gradient(PaintGradient {
                base: PaintBase {
                    id: Some("fade".to_string()),
                    metadata: None,
                },
                kind: GradientKind::Linear {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 8.0,
                    y2: 0.0,
                },
                stops: vec![
                    GradientStop {
                        offset: 0.0,
                        color: "#000000".to_string(),
                    },
                    GradientStop {
                        offset: 1.0,
                        color: "#ffffff".to_string(),
                    },
                ],
            }));
        scene.instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: 0.0,
            y: 0.0,
            width: 8.0,
            height: 2.0,
            fill: Some("url(#fade)".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let selected = registry
            .select(
                &scene,
                PaintRenderOptions {
                    preference: PaintBackendPreference::Named("paint-vm-wgpu".to_string()),
                    ..PaintRenderOptions::default()
                },
            )
            .unwrap();
        assert_eq!(selected.descriptor().id, "paint-vm-wgpu");
    }

    #[test]
    fn runtime_selects_wgpu_for_radial_gradient_scene() {
        let backend = renderer();
        let mut registry = PaintBackendRegistry::new();
        registry.register(&backend);
        let scene = radial_gradient_scene(8.0, 8.0, 4.0);

        let selected = registry
            .select(
                &scene,
                PaintRenderOptions {
                    preference: PaintBackendPreference::Named("paint-vm-wgpu".to_string()),
                    ..PaintRenderOptions::default()
                },
            )
            .unwrap();
        assert_eq!(selected.descriptor().id, "paint-vm-wgpu");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn renders_solid_rect_when_adapter_is_available() {
        let mut scene = PaintScene::new(8.0, 8.0);
        scene
            .instructions
            .push(PaintInstruction::Rect(PaintRect::filled(
                2.0, 2.0, 4.0, 4.0, "#ff0000",
            )));

        let pixels = match render(&scene) {
            Ok(pixels) => pixels,
            Err(PaintRenderError::BackendUnavailable { .. }) => return,
            Err(err) => panic!("unexpected WGPU render failure: {err:?}"),
        };

        assert_eq!((pixels.width, pixels.height), (8, 8));
        assert_eq!(pixels.pixel_at(0, 0), (255, 255, 255, 255));
        let center = pixels.pixel_at(3, 3);
        assert!(
            center.0 > 240 && center.1 < 20 && center.2 < 20 && center.3 == 255,
            "expected center pixel to be opaque red, got {center:?}"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn renders_pixel_image_when_adapter_is_available() {
        let mut image = PixelContainer::new(2, 2);
        image.set_pixel(0, 0, 255, 0, 0, 255);
        image.set_pixel(1, 0, 0, 255, 0, 255);
        image.set_pixel(0, 1, 0, 0, 255, 255);
        image.set_pixel(1, 1, 255, 255, 0, 255);
        let mut scene = PaintScene::new(4.0, 4.0);
        scene.instructions.push(PaintInstruction::Image(PaintImage {
            base: PaintBase::default(),
            x: 1.0,
            y: 1.0,
            width: 2.0,
            height: 2.0,
            src: ImageSrc::Pixels(image),
            opacity: None,
        }));

        let pixels = match render(&scene) {
            Ok(pixels) => pixels,
            Err(PaintRenderError::BackendUnavailable { .. }) => return,
            Err(err) => panic!("unexpected WGPU render failure: {err:?}"),
        };

        assert_eq!(pixels.pixel_at(0, 0), (255, 255, 255, 255));
        assert_eq!(pixels.pixel_at(1, 1), (255, 0, 0, 255));
        assert_eq!(pixels.pixel_at(2, 1), (0, 255, 0, 255));
        assert_eq!(pixels.pixel_at(1, 2), (0, 0, 255, 255));
        assert_eq!(pixels.pixel_at(2, 2), (255, 255, 0, 255));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn renders_linear_gradient_when_adapter_is_available() {
        let mut scene = PaintScene::new(8.0, 2.0);
        scene
            .instructions
            .push(PaintInstruction::Gradient(PaintGradient {
                base: PaintBase {
                    id: Some("fade".to_string()),
                    metadata: None,
                },
                kind: GradientKind::Linear {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 8.0,
                    y2: 0.0,
                },
                stops: vec![
                    GradientStop {
                        offset: 0.0,
                        color: "#000000".to_string(),
                    },
                    GradientStop {
                        offset: 1.0,
                        color: "#ffffff".to_string(),
                    },
                ],
            }));
        scene.instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: 0.0,
            y: 0.0,
            width: 8.0,
            height: 2.0,
            fill: Some("url(#fade)".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let pixels = match render(&scene) {
            Ok(pixels) => pixels,
            Err(PaintRenderError::BackendUnavailable { .. }) => return,
            Err(err) => panic!("unexpected WGPU render failure: {err:?}"),
        };
        let left = pixels.pixel_at(0, 0);
        let right = pixels.pixel_at(7, 0);

        assert!(
            left.0 < 80,
            "expected dark left gradient edge, got {left:?}"
        );
        assert!(
            right.0 > 170,
            "expected bright right gradient edge, got {right:?}"
        );
        assert!(right.0 > left.0);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn renders_radial_gradient_when_adapter_is_available() {
        let scene = radial_gradient_scene(16.0, 16.0, 8.0);

        let pixels = match render(&scene) {
            Ok(pixels) => pixels,
            Err(PaintRenderError::BackendUnavailable { .. }) => return,
            Err(err) => panic!("unexpected WGPU render failure: {err:?}"),
        };
        let center = pixels.pixel_at(8, 8);
        let corner = pixels.pixel_at(0, 0);

        assert!(
            center.0 < 80,
            "expected dark radial gradient center, got {center:?}"
        );
        assert!(
            corner.0 > 170,
            "expected bright radial gradient edge, got {corner:?}"
        );
        assert!(corner.0 > center.0);
    }

    #[test]
    fn runtime_rejects_text_without_exact_glyph_atlas_support() {
        let backend = renderer();
        let mut registry = PaintBackendRegistry::new();
        registry.register(&backend);
        let mut scene = PaintScene::new(80.0, 40.0);
        scene.instructions.push(PaintInstruction::Text(PaintText {
            base: Default::default(),
            x: 4.0,
            y: 20.0,
            text: "not yet".to_string(),
            font_ref: None,
            font_size: 16.0,
            fill: Some("#000000".to_string()),
            text_align: None,
        }));

        assert!(registry
            .select(
                &scene,
                PaintRenderOptions {
                    preference: PaintBackendPreference::Named("paint-vm-wgpu".to_string()),
                    ..PaintRenderOptions::default()
                },
            )
            .is_err());
    }

    fn radial_gradient_scene(width: f64, height: f64, radius: f64) -> PaintScene {
        let mut scene = PaintScene::new(width, height);
        scene
            .instructions
            .push(PaintInstruction::Gradient(PaintGradient {
                base: PaintBase {
                    id: Some("fade".to_string()),
                    metadata: None,
                },
                kind: GradientKind::Radial {
                    cx: width / 2.0,
                    cy: height / 2.0,
                    r: radius,
                },
                stops: vec![
                    GradientStop {
                        offset: 0.0,
                        color: "#000000".to_string(),
                    },
                    GradientStop {
                        offset: 1.0,
                        color: "#ffffff".to_string(),
                    },
                ],
            }));
        scene.instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: 0.0,
            y: 0.0,
            width,
            height,
            fill: Some("url(#fade)".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));
        scene
    }
}
