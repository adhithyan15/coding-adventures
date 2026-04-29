//! WGPU backend for the Paint VM runtime.
//!
//! This backend consumes the shared `paint-vm-gpu-core` render plan and draws
//! solid meshes into an offscreen WGPU texture. It is the first concrete GPU
//! consumer of the shared tessellation layer; glyph atlases, exact gradients,
//! and filters remain deliberately outside this Tier 1 slice.

use std::sync::mpsc;

use paint_instructions::{PaintScene, PixelContainer};
use paint_vm_gpu_core::{
    plan_scene, GpuColor, GpuCommand, GpuImageUpload, GpuMesh, GpuPaintPlan, GpuPlanSeverity,
    GpuRect,
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
            layer_filters: SupportLevel::Unsupported,
            layer_blend_modes: SupportLevel::Unsupported,
            linear_gradient: SupportLevel::Degraded,
            radial_gradient: SupportLevel::Degraded,
            antialiasing: SupportLevel::Unsupported,
            offscreen_pixels: SupportLevel::Supported,
        },
        priority: 55,
    }
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
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("paint-vm-wgpu-nearest-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..wgpu::SamplerDescriptor::default()
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

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("paint-vm-wgpu-target"),
        size: wgpu::Extent3d {
            width: plan.width,
            height: plan.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: TARGET_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let prepared_meshes = prepare_meshes(&device, &plan.meshes);
    let (white_texture, prepared_textures) = prepare_textures(
        &device,
        &queue,
        &texture_bind_group_layout,
        &sampler,
        &plan.images,
    );
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
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("paint-vm-wgpu-render-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(to_wgpu_color(plan.background)),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &viewport_bind_group, &[]);
        pass.set_scissor_rect(0, 0, plan.width, plan.height);
        let mut clip_stack = vec![GpuRect {
            x: 0.0,
            y: 0.0,
            width: plan.width as f32,
            height: plan.height as f32,
        }];
        for command in &plan.commands {
            match command {
                GpuCommand::DrawMesh { mesh_id } => {
                    if let Some(mesh) = prepared_meshes.get(*mesh_id) {
                        let texture = mesh
                            .texture_id
                            .and_then(|texture_id| prepared_textures.get(texture_id))
                            .unwrap_or(&white_texture);
                        pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                        pass.set_index_buffer(
                            mesh.index_buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        pass.set_bind_group(1, &texture.bind_group, &[]);
                        pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                    }
                }
                GpuCommand::PushClip { rect } => {
                    let current = *clip_stack.last().unwrap();
                    let clipped = intersect_rect(current, *rect);
                    clip_stack.push(clipped);
                    set_scissor(&mut pass, clipped, plan.width, plan.height);
                }
                GpuCommand::PopClip => {
                    if clip_stack.len() > 1 {
                        clip_stack.pop();
                    }
                    set_scissor(
                        &mut pass,
                        *clip_stack.last().unwrap(),
                        plan.width,
                        plan.height,
                    );
                }
                GpuCommand::DrawText(_) | GpuCommand::DrawGlyphRun(_) => {}
            }
        }
    }
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &target,
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
    sampler: &wgpu::Sampler,
    images: &[GpuImageUpload],
) -> (PreparedTexture, Vec<PreparedTexture>) {
    let white_texture = prepare_texture(
        device,
        queue,
        layout,
        sampler,
        "paint-vm-wgpu-white-texture",
        1,
        1,
        &[255, 255, 255, 255],
    );
    let textures = images
        .iter()
        .enumerate()
        .map(|(index, image)| {
            prepare_texture(
                device,
                queue,
                layout,
                sampler,
                texture_label(index),
                image.width,
                image.height,
                &image.data,
            )
        })
        .collect();
    (white_texture, textures)
}

fn prepare_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    label: &'static str,
    width: u32,
    height: u32,
    data: &[u8],
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
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    PreparedTexture {
        bind_group,
        _texture: texture,
        _view: view,
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
    ((value + alignment - 1) / alignment) * alignment
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

#[cfg(test)]
mod tests {
    use super::*;
    use paint_instructions::{
        ImageSrc, PaintBase, PaintImage, PaintInstruction, PaintRect, PaintText,
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
}
