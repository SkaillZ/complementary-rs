use bytemuck::{Pod, Zeroable};
use wgpu::vertex_attr_array;

use crate::math::FVec2;

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
