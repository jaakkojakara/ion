use std::{num::NonZeroU32, ops::Range};

use crate::{
    WASM_COMPATIBLE_RENDERING,
    gfx::{
        GfxBundle, GfxRef, Sprite, SpriteTypeId,
        renderer::{
            gpu_data_types::{InstanceLight, InstanceSprite},
            render_camera::RenderCamera,
        },
    },
};

use super::{Texture, TextureLayout, texture_loader::TextureLoader};

/// A collection of texture assets.
/// Any sprites that need to be rendered need to be included here and then loaded to the GPU.
/// This is done by first calling the `include_sprite_bundle` function for all the assets,
/// and then submitting this struct to the `load_texture_assets` function of the `Renderer`.
pub struct TextureAssets {
    bind_group_layout: Option<wgpu::BindGroupLayout>,
    bind_group: Option<wgpu::BindGroup>,
    bind_groups_wasm: Option<Vec<wgpu::BindGroup>>,

    gfx_bundles: Vec<GfxBundle>,
    texture_sheets: Vec<Texture>,

    assets_ready: bool,
}

impl TextureAssets {
    pub fn new() -> Self {
        Self {
            bind_group_layout: None,
            bind_group: None,
            bind_groups_wasm: None,

            gfx_bundles: Vec::new(),
            texture_sheets: Vec::new(),
            assets_ready: false,
        }
    }

    /// Add a sprite bundle to the texture assets.
    /// Returns the reference to the sprite bundle.
    pub fn include_gfx_bundle(&mut self, sprite_bundle: GfxBundle) -> u32 {
        self.gfx_bundles.push(sprite_bundle);
        self.gfx_bundles.len() as u32 - 1
    }

    pub(crate) fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        self.bind_group_layout
            .as_ref()
            .expect("Texture assets must be loaded before accessing bind group layout")
    }

    pub(crate) fn bind_group(&self) -> &wgpu::BindGroup {
        self.bind_group
            .as_ref()
            .expect("Texture assets must be loaded before accessing bind group")
    }

    pub(crate) fn bind_groups_wasm(&self) -> &[wgpu::BindGroup] {
        self.bind_groups_wasm
            .as_ref()
            .expect("Texture assets must be loaded before accessing bind groups")
    }

    pub(crate) fn assets_ready(&self) -> bool {
        self.assets_ready
    }

    /// Convert a list of sprite references to a list of renderable instances.
    /// Instances are sorted and grouped by layer and then by texture layout.
    /// Returns a vec of instances for writing to GPU and a vec of ranges for `render_pass.draw_indexed` call.
    pub(crate) fn refs_to_draw_calls(
        &self,
        gfx_refs: &[GfxRef],
        camera: &RenderCamera,
    ) -> (
        (Vec<InstanceSprite>, Vec<DrawCall>),
        (Vec<InstanceSprite>, Vec<DrawCall>),
        (Vec<InstanceLight>, Vec<DrawCall>),
    ) {
        let mut instances_color = Vec::new();
        let mut instances_shadow = Vec::new();
        let mut instances_light = Vec::new();
        let mut draw_calls_color = Vec::new();
        let mut draw_calls_shadow = Vec::new();
        let mut draw_calls_light = Vec::new();

        // TODO: If any of the sprites contain transparency, this needs to be handled differently

        for gfx_ref in gfx_refs {
            let sprite_bundle = self.gfx_bundles.get(gfx_ref.id as usize).unwrap();
            sprite_bundle.extract_for_render(
                Some(camera),
                gfx_ref,
                &mut instances_color,
                &mut instances_shadow,
                &mut instances_light,
            );
        }

        // Sort by layer first, then by layout
        instances_color.sort_by_key(|(_, layout, layer)| (*layer, *layout));

        // No layers here, just sort by layout
        instances_shadow.sort_by_key(|(_, layout)| *layout);
        instances_light.sort_by_key(|(_, layout)| *layout);

        if !instances_color.is_empty() {
            let mut current_layer = instances_color[0].2;
            let mut current_layout = instances_color[0].1;
            let mut start_index = 0;

            for (i, (_, layout, layer)) in instances_color.iter().enumerate().skip(1) {
                if *layer != current_layer || *layout != current_layout {
                    draw_calls_color.push(DrawCall {
                        layer: current_layer,
                        layout: current_layout,
                        draw_range: start_index..i as u32,
                    });
                    current_layer = *layer;
                    current_layout = *layout;
                    start_index = i as u32;
                }
            }

            draw_calls_color.push(DrawCall {
                layer: current_layer,
                layout: current_layout,
                draw_range: start_index..instances_color.len() as u32,
            });
        }

        // Build draw calls for shadows (no layers, only layout)
        if !instances_shadow.is_empty() {
            let mut current_layout = instances_shadow[0].1;
            let mut start_index = 0;

            for (i, (_, layout)) in instances_shadow.iter().enumerate().skip(1) {
                if *layout != current_layout {
                    draw_calls_shadow.push(DrawCall {
                        layer: 0, // Shadows don't use layers
                        layout: current_layout,
                        draw_range: start_index..i as u32,
                    });
                    current_layout = *layout;
                    start_index = i as u32;
                }
            }

            draw_calls_shadow.push(DrawCall {
                layer: 0, // Shadows don't use layers
                layout: current_layout,
                draw_range: start_index..instances_shadow.len() as u32,
            });
        }

        // Build draw calls for lights (no layers, only layout)
        if !instances_light.is_empty() {
            let mut current_layout = instances_light[0].1;
            let mut start_index = 0;

            for (i, (_, layout)) in instances_light.iter().enumerate().skip(1) {
                if *layout != current_layout {
                    draw_calls_light.push(DrawCall {
                        layer: 0, // Lights don't use layers
                        layout: current_layout,
                        draw_range: start_index..i as u32,
                    });
                    current_layout = *layout;
                    start_index = i as u32;
                }
            }

            draw_calls_light.push(DrawCall {
                layer: 0, // Lights don't use layers
                layout: current_layout,
                draw_range: start_index..instances_light.len() as u32,
            });
        }

        (
            (
                instances_color.into_iter().map(|(instance, _, _)| instance).collect(),
                draw_calls_color,
            ),
            (
                instances_shadow.into_iter().map(|(instance, _)| instance).collect(),
                draw_calls_shadow,
            ),
            (
                instances_light.into_iter().map(|(instance, _)| instance).collect(),
                draw_calls_light,
            ),
        )
    }

    /// Convert a list of sprite references to a list of renderable instances.
    /// Instances are sorted and grouped by layer, then by texture sheet index, and finally by texture layout.
    /// Returns a vec of instances for writing to GPU and a map of texture sheet indices to their layout ranges.
    pub(crate) fn refs_to_draw_calls_wasm(
        &self,
        gfx_refs: &[GfxRef],
    ) -> (
        (Vec<InstanceSprite>, Vec<DrawCallWasm>),
        (Vec<InstanceSprite>, Vec<DrawCallWasm>),
        (Vec<InstanceLight>, Vec<DrawCallWasm>),
    ) {
        let mut instances_color = Vec::new();
        let mut instances_shadow = Vec::new();
        let mut instances_light = Vec::new();
        let mut draw_ranges_color = Vec::new();
        let mut draw_ranges_shadow = Vec::new();
        let mut draw_ranges_light = Vec::new();

        // Collect instances
        for gfx_ref in gfx_refs {
            let sprite_bundle = self.gfx_bundles.get(gfx_ref.id as usize).unwrap();
            sprite_bundle.extract_for_render(
                None,
                gfx_ref,
                &mut instances_color,
                &mut instances_shadow,
                &mut instances_light,
            );
        }

        // Sort by layer first, then by texture sheet index, and finally by layout
        instances_color.sort_by_key(|(instance, layout, layer)| {
            // Extract first texture sheet index from tex_sheet_indices
            let sheet_idx = (instance.tex_sheet_indices & 0xFF) as usize;
            (*layer, sheet_idx, *layout)
        });

        // Sort shadows by texture sheet index, then by layout (no layers)
        instances_shadow.sort_by_key(|(instance, layout)| {
            let sheet_idx = (instance.tex_sheet_indices & 0xFF) as usize;
            (sheet_idx, *layout)
        });

        // Sort lights by texture sheet index, then by layout (no layers)
        instances_light.sort_by_key(|(instance, layout)| {
            let sheet_idx = (instance.tex_sheet_index) as usize;
            (sheet_idx, *layout)
        });

        if !instances_color.is_empty() {
            let mut current_layer = instances_color[0].2;
            let mut current_sheet_idx = (instances_color[0].0.tex_sheet_indices & 0xFF) as usize;
            let mut current_layout = instances_color[0].1;
            let mut start_index = 0;

            for (i, (instance, layout, layer)) in instances_color.iter().enumerate().skip(1) {
                let sheet_idx = (instance.tex_sheet_indices & 0xFF) as usize;
                if *layer != current_layer || sheet_idx != current_sheet_idx || *layout != current_layout {
                    // Store the range for the previous layer/sheet/layout combination
                    draw_ranges_color.push(DrawCallWasm {
                        texture_sheet_index: current_sheet_idx,
                        layer: current_layer,
                        layout: current_layout,
                        draw_range: start_index..i as u32,
                    });

                    current_layer = *layer;
                    current_sheet_idx = sheet_idx;
                    current_layout = *layout;
                    start_index = i as u32;
                }
            }

            // Store the final range
            draw_ranges_color.push(DrawCallWasm {
                texture_sheet_index: current_sheet_idx,
                layer: current_layer,
                layout: current_layout,
                draw_range: start_index..instances_color.len() as u32,
            });
        }

        // Build draw calls for shadows (no layers, grouped by sheet index then layout)
        if !instances_shadow.is_empty() {
            let mut current_sheet_idx = (instances_shadow[0].0.tex_sheet_indices & 0xFF) as usize;
            let mut current_layout = instances_shadow[0].1;
            let mut start_index = 0;

            for (i, (instance, layout)) in instances_shadow.iter().enumerate().skip(1) {
                let sheet_idx = (instance.tex_sheet_indices & 0xFF) as usize;
                if sheet_idx != current_sheet_idx || *layout != current_layout {
                    draw_ranges_shadow.push(DrawCallWasm {
                        texture_sheet_index: current_sheet_idx,
                        layer: 0, // Shadows don't use layers
                        layout: current_layout,
                        draw_range: start_index..i as u32,
                    });
                    current_sheet_idx = sheet_idx;
                    current_layout = *layout;
                    start_index = i as u32;
                }
            }

            draw_ranges_shadow.push(DrawCallWasm {
                texture_sheet_index: current_sheet_idx,
                layer: 0, // Shadows don't use layers
                layout: current_layout,
                draw_range: start_index..instances_shadow.len() as u32,
            });
        }

        // Build draw calls for lights (no layers, grouped by sheet index then layout)
        if !instances_light.is_empty() {
            let mut current_sheet_idx = instances_light[0].0.tex_sheet_index as usize;
            let mut current_layout = instances_light[0].1;
            let mut start_index = 0;

            for (i, (instance, layout)) in instances_light.iter().enumerate().skip(1) {
                let sheet_idx = instance.tex_sheet_index as usize;
                if sheet_idx != current_sheet_idx || *layout != current_layout {
                    draw_ranges_light.push(DrawCallWasm {
                        texture_sheet_index: current_sheet_idx,
                        layer: 0, // Lights don't use layers
                        layout: current_layout,
                        draw_range: start_index..i as u32,
                    });
                    current_sheet_idx = sheet_idx;
                    current_layout = *layout;
                    start_index = i as u32;
                }
            }

            draw_ranges_light.push(DrawCallWasm {
                texture_sheet_index: current_sheet_idx,
                layer: 0, // Lights don't use layers
                layout: current_layout,
                draw_range: start_index..instances_light.len() as u32,
            });
        }

        (
            (
                instances_color.into_iter().map(|(instance, _, _)| instance).collect(),
                draw_ranges_color,
            ),
            (
                instances_shadow.into_iter().map(|(instance, _)| instance).collect(),
                draw_ranges_shadow,
            ),
            (
                instances_light.into_iter().map(|(instance, _)| instance).collect(),
                draw_ranges_light,
            ),
        )
    }

    pub(crate) fn take_finished_loader(&mut self, device: &wgpu::Device, texture_loader: TextureLoader) {
        let (texture_sheets, texture_ids) = texture_loader.finish();
        self.texture_sheets.extend(texture_sheets);

        let fill_ids = |sprite: &mut Sprite| {
            if let Some(texture_id) = texture_ids.get(&sprite.texture) {
                sprite.texture_id = Some(*texture_id);
            } else {
                sprite.type_id = SpriteTypeId::Missing;
            }

            if let Some(mask_name) = &sprite.texture_mask {
                if let Some(texture_id) = texture_ids.get(mask_name) {
                    sprite.texture_mask_id = Some(*texture_id);
                } else {
                    sprite.type_id = SpriteTypeId::Missing;
                }
            }
        };

        // Register texture IDs for all sprites in all bundles
        for bundle in &mut self.gfx_bundles {
            for sprite in &mut bundle.sprites {
                fill_ids(sprite);
            }

            // Register texture IDs for shadows
            for shadow in &mut bundle.shadows {
                fill_ids(shadow);
            }

            // Register texture IDs for lights
            for light in &mut bundle.lights {
                fill_ids(&mut light.sprite);
            }
        }

        let sampler_descriptor = wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        };

        let sampler = device.create_sampler(&sampler_descriptor);

        if WASM_COMPATIBLE_RENDERING {
            let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_assets_bind_group_layout_wasm"),
            });

            let bind_groups_wasm = (&self.texture_sheets)
                .chunks(2)
                .enumerate()
                .map(|(i, sheets)| {
                    device.create_bind_group(&wgpu::BindGroupDescriptor {
                        layout: &bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&sheets[0].texture_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::TextureView(&sheets[1].texture_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::Sampler(&sampler),
                            },
                        ],
                        label: Some(&format!("texture_assets_bind_group_wasm_{}", i / 2)),
                    })
                })
                .collect::<Vec<_>>();

            self.bind_group_layout = Some(bind_group_layout);
            self.bind_groups_wasm = Some(bind_groups_wasm);
        } else {
            let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: Some(NonZeroU32::new(self.texture_sheets.len() as u32).unwrap()),
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_assets_bind_group_layout"),
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureViewArray(
                            self.texture_sheets
                                .iter()
                                .map(|sheet| &sheet.texture_view)
                                .collect::<Vec<&wgpu::TextureView>>()
                                .as_slice(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
                label: Some("texture_assets_bind_group"),
            });

            self.bind_group_layout = Some(bind_group_layout);
            self.bind_group = Some(bind_group);
        }

        self.assets_ready = true;
    }

    pub(crate) fn required_textures(&self) -> Vec<String> {
        let mut textures: Vec<_> = self
            .gfx_bundles
            .iter()
            .flat_map(|bundle| {
                bundle
                    .sprites
                    .iter()
                    .map(|sprite| sprite.texture.clone())
                    .chain(bundle.sprites.iter().filter_map(|sprite| sprite.texture_mask.clone()))
                    .chain(bundle.shadows.iter().map(|shadow| shadow.texture.clone()))
                    .chain(bundle.shadows.iter().filter_map(|shadow| shadow.texture_mask.clone()))
                    .chain(bundle.lights.iter().map(|light| light.sprite.texture.clone()))
                    .chain(
                        bundle
                            .lights
                            .iter()
                            .filter_map(|light| light.sprite.texture_mask.clone()),
                    )
            })
            .collect();

        textures = textures
            .iter()
            .flat_map(|texture| {
                if let Some(last_underscore) = texture.rfind('_')
                    && let Ok(_) = texture[last_underscore + 1..].parse::<u32>()
                {
                    vec![texture[..last_underscore].to_string(), texture.to_string()]
                } else {
                    vec![texture.to_string()]
                }
            })
            .collect();

        textures.sort();
        textures.dedup();
        textures
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DrawCall {
    pub layer: u8,
    pub layout: TextureLayout,
    pub draw_range: Range<u32>,
}

#[derive(Debug, Clone)]
pub(crate) struct DrawCallWasm {
    pub texture_sheet_index: usize,
    pub layer: u8,
    pub layout: TextureLayout,
    pub draw_range: Range<u32>,
}
