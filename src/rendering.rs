use std::marker::PhantomData;

use bytemuck::{Pod, Zeroable};
use cgmath::{SquareMatrix, Transform};
use wgpu::{util::DeviceExt, vertex_attr_array};

use crate::math::{FMat4, FVec2, FVec3};

#[derive(Clone, Debug)]
pub struct DrawState {
    pub view_matrix: FMat4,
}

impl DrawState {
    pub fn new() -> DrawState {
        Self {
            view_matrix: FMat4::identity(),
        }
    }

    pub fn update_view_matrix(
        &mut self,
        window_width: f32,
        window_height: f32,
        tilemap_width: f32,
        tilemap_height: f32,
    ) {
        let width_ratio = window_width / tilemap_width;
        let height_ratio = window_height / tilemap_height;
        let ratio = f32::min(width_ratio, height_ratio);
        let scaled_width = tilemap_width * ratio;
        let scaled_height = tilemap_height * ratio;

        self.view_matrix = FMat4::from_translation(FVec3::new(-1.0, -1.0, 0.0))
            * FMat4::from_nonuniform_scale(
                (ratio / window_width) * 2.0,
                (ratio / window_height) * 2.0,
                1.0,
            );
    }
}

pub struct UniformBuffer<T>
where
    T: Clone + bytemuck::Pod + bytemuck::Zeroable,
{
    buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    phantom: PhantomData<T>,
}

impl<T: bytemuck::Pod> UniformBuffer<T> {
    pub fn new(device: &wgpu::Device, label: &str) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{label}_uniform_buffer")),
            size: std::mem::size_of::<T>() as _,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some(&format!("{label}_bind_group_layout")),
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some(&format!("{label}_bind_group")),
        });

        Self {
            buffer,
            bind_group_layout,
            bind_group,
            phantom: PhantomData,
        }
    }

    pub fn write_with_queue(&self, queue: &wgpu::Queue, data: T) {
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&data));
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    position: FVec2,
}

impl Vertex {
    pub fn new(x: f32, y: f32) -> Self {
        Vertex {
            position: FVec2::new(x, y),
        }
    }

    const ATTR: &'static [wgpu::VertexAttribute] = &vertex_attr_array![0 => Float32x2];

    pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Vertex::ATTR,
        }
    }
}

pub fn create_pipeline_descriptor<'a>(
    label: Option<&'a str>,
    shader: &'a wgpu::ShaderModule,
    layout: Option<&'a wgpu::PipelineLayout>,
    buffer_layouts: &'a [wgpu::VertexBufferLayout<'a>],
) -> wgpu::RenderPipelineDescriptor<'a> {
    wgpu::RenderPipelineDescriptor {
        layout,
        vertex: wgpu::VertexState {
            buffers: buffer_layouts,
            module: &shader,
            entry_point: "vs_main",
        },
        fragment: Some(wgpu::FragmentState {
            targets: &[wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }],
            module: &shader,
            entry_point: "fs_main",
        }),
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
        label,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    }
}

pub fn create_vertex_buffer<T: bytemuck::Pod>(
    device: &wgpu::Device,
    label: Option<&str>,
    contents: &[T],
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label,
        contents: bytemuck::cast_slice(&contents),
        usage: wgpu::BufferUsages::VERTEX,
    })
}
