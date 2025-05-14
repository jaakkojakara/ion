use std::cell::RefCell;

use crate::{
    WASM_COMPATIBLE_RENDERING, build_shader,
    core::coordinates::ChunkLocation,
    gfx::{
        renderer::{
            RenderGraph,
            gpu_data_types::{InstanceSprite, SHADER_SHADOW, SHADER_SHADOW_WASM, Vertex},
            render_camera::RenderCamera,
            render_globals::RenderGlobals,
            render_helpers::{build_render_pipeline, build_tex_bind_group_layout},
        },
        textures::texture_assets::TextureAssets,
    },
};

pub struct RenderPassShadow {
    render_pipeline: wgpu::RenderPipeline,

    source_tex_bind_group_layout: wgpu::BindGroupLayout,
    source_tex_bind_group: RefCell<Option<wgpu::BindGroup>>,
}

impl RenderPassShadow {
    pub fn new(
        device: &wgpu::Device,
        render_camera: &RenderCamera,
        render_globals: &RenderGlobals,
        asset_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let source_tex_bind_group_layout =
            build_tex_bind_group_layout(device, 1, false, "source_tex_bind_group_layout_shadow");

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_pipeline_layout_shadow"),
            bind_group_layouts: &[
                &render_globals.globals_bind_group_layout,
                &render_camera.camera_bind_group_layout,
                asset_bind_group_layout,
                &source_tex_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let shader = if WASM_COMPATIBLE_RENDERING {
            build_shader!(device, SHADER_SHADOW_WASM)
        } else {
            build_shader!(device, SHADER_SHADOW)
        };

        let render_pipeline = build_render_pipeline(
            device,
            &render_pipeline_layout,
            &shader,
            &[Vertex::buffer_layout(), InstanceSprite::buffer_layout()],
            &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Max,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Max,
                    },
                }),
                write_mask: wgpu::ColorWrites::ALPHA,
            })],
            None,
            "render_pipeline_shadow",
        );

        Self {
            render_pipeline,

            source_tex_bind_group_layout,
            source_tex_bind_group: RefCell::new(None),
        }
    }

    pub fn set_render_graph(&self, device: &wgpu::Device, render_graph: &RenderGraph) {
        *self.source_tex_bind_group.borrow_mut() = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.source_tex_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &render_graph.target_height_id.as_ref().unwrap().texture_view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&render_graph.nearest_sampler),
                },
            ],
            label: Some("render_targets_bind_group_shadow"),
        }));
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        render_camera: &RenderCamera,
        render_globals: &RenderGlobals,
        render_graph: &RenderGraph,
        texture_assets: &TextureAssets,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_pass_shadow"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &render_graph.target_light_shadow.as_ref().unwrap().texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &render_globals.globals_bind_group, &[]);
        render_pass.set_bind_group(1, &render_camera.camera_bind_group, &[]);
        render_pass.set_bind_group(3, self.source_tex_bind_group.borrow().as_ref().unwrap(), &[]);

        render_pass.set_vertex_buffer(0, render_graph.vertex_buffer.slice(..));
        render_pass.set_index_buffer(render_graph.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        if WASM_COMPATIBLE_RENDERING {
            self.execute_draw_calls_wasm(&mut render_pass, render_graph, texture_assets);
        } else {
            render_pass.set_bind_group(2, texture_assets.bind_group(), &[]);
            self.execute_draw_calls_native(&mut render_pass, render_graph);
        }
    }

    /// Executes draw calls for the native renderer.
    /// Assumes that all render pass bindings are set, except for the instance buffer.
    fn execute_draw_calls_native(&self, render_pass: &mut wgpu::RenderPass, render_graph: &RenderGraph) {
        // ------------------ Chunked sprite rendering ------------------ //

        let mut all_chunked_draw_calls = render_graph
            .chunk_buffers
            .iter()
            .flat_map(|(chunk, buffers)| {
                buffers
                    .shadow_draw_calls
                    .iter()
                    .map(move |draw_call| (chunk, draw_call))
            })
            .collect::<Vec<_>>();

        all_chunked_draw_calls.sort_by_key(|(chunk, draw_call)| (draw_call.layer, *chunk));

        let mut prev_chunk: Option<ChunkLocation> = None;
        for (chunk, draw_call) in all_chunked_draw_calls {
            if prev_chunk.is_none() || prev_chunk.unwrap() != *chunk {
                prev_chunk = Some(*chunk);
                let buffers = render_graph.chunk_buffers.get(chunk).unwrap();
                render_pass.set_vertex_buffer(1, buffers.shadow_buf.slice(..));
            }

            render_pass.draw_indexed(draw_call.layout.draw_range(), 0, draw_call.draw_range.clone());
        }

        // ------------------ Dynamic sprite rendering ------------------ //
        render_pass.set_vertex_buffer(1, render_graph.dynamic_buffers.shadow_buf.slice(..));

        for draw_call in render_graph.dynamic_buffers.shadow_draw_calls.iter() {
            render_pass.draw_indexed(draw_call.layout.draw_range(), 0, draw_call.draw_range.clone());
        }
    }

    /// Executes draw calls for the wasm renderer.
    /// Assumes that all render pass bindings are set, except for the instance buffer and the texture sheet.
    fn execute_draw_calls_wasm(
        &self,
        render_pass: &mut wgpu::RenderPass,
        render_graph: &RenderGraph,
        texture_assets: &TextureAssets,
    ) {
        // ------------------ Chunked sprite rendering ------------------ //

        let mut all_chunked_draw_calls = render_graph
            .chunk_buffers
            .iter()
            .flat_map(|(chunk, buffers)| {
                buffers
                    .shadow_draw_calls_wasm
                    .iter()
                    .map(move |draw_call| (chunk, draw_call))
            })
            .collect::<Vec<_>>();

        all_chunked_draw_calls
            .sort_by_key(|(chunk, draw_call)| (draw_call.layer, draw_call.texture_sheet_index, *chunk));

        let mut prev_chunk: Option<ChunkLocation> = None;
        let mut prev_texture_sheet_index: Option<usize> = None;
        for (chunk, draw_call) in all_chunked_draw_calls {
            if prev_chunk.is_none() || prev_chunk.unwrap() != *chunk {
                prev_chunk = Some(*chunk);
                let buffers = render_graph.chunk_buffers.get(chunk).unwrap();
                render_pass.set_vertex_buffer(1, buffers.shadow_buf.slice(..));
            }

            if prev_texture_sheet_index.unwrap_or(usize::MAX) != draw_call.texture_sheet_index {
                prev_texture_sheet_index = Some(draw_call.texture_sheet_index);
                let sheet = &texture_assets.bind_groups_wasm()[draw_call.texture_sheet_index / 2];
                render_pass.set_bind_group(2, sheet, &[]);
            }

            render_pass.draw_indexed(draw_call.layout.draw_range(), 0, draw_call.draw_range.clone());
        }

        // ------------------ Dynamic sprite rendering ------------------ //
        render_pass.set_vertex_buffer(1, render_graph.dynamic_buffers.shadow_buf.slice(..));

        for draw_call in render_graph.dynamic_buffers.shadow_draw_calls_wasm.iter() {
            if prev_texture_sheet_index.unwrap_or(usize::MAX) != draw_call.texture_sheet_index {
                prev_texture_sheet_index = Some(draw_call.texture_sheet_index);
                let sheet = &texture_assets.bind_groups_wasm()[draw_call.texture_sheet_index / 2];
                render_pass.set_bind_group(2, sheet, &[]);
            }
            render_pass.draw_indexed(draw_call.layout.draw_range(), 0, draw_call.draw_range.clone());
        }
    }
}
