use std::ops::Range;

use super::gfx_config::Resolution;

pub mod texture_assets;
pub(crate) mod texture_loader;

pub(crate) struct Texture {
    pub texture: wgpu::Texture,
    pub texture_view: wgpu::TextureView,
    #[allow(dead_code)]
    pub texture_format: wgpu::TextureFormat,
}

impl Texture {
    pub fn new_from_empty(
        device: &wgpu::Device,
        resolution: Resolution,
        format: wgpu::TextureFormat,
        mipmap_count: u32,
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: resolution.width,
            height: resolution.height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: mipmap_count,
            sample_count: 1,
            view_formats: &[format],
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            base_mip_level: 0,
            mip_level_count: Some(1),
            ..wgpu::TextureViewDescriptor::default()
        });

        Self {
            texture,
            texture_view: view,
            texture_format: format,
        }
    }

    pub fn new_from_raw_data(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        raw_pixel_data: &[u8],
        dimensions: (u32, u32),
        mipmap_count: u32,
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: mipmap_count,
            sample_count: 1,
            view_formats: &[format],
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            raw_pixel_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            texture_view: view,
            texture_format: format,
        }
    }
}

// ---------------------------------------------------------- //
// ------------- Supporting types for textures -------------- //
// ---------------------------------------------------------- //

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub(crate) struct TextureId {
    pub tex_coords: [f32; 2],
    pub tex_coords_sizes: [f32; 2],
    pub tex_sheet_indices: [u32; 2],
    pub layout: TextureLayout,
    pub frame_count: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub(crate) enum TextureLayout {
    Square = 0,
    Isometric = 1,
    IsometricHex = 2,
    Overlay = 3,
}

impl TextureLayout {
    #[allow(dead_code)]
    pub fn sprite_vertex_count(&self) -> u16 {
        match self {
            TextureLayout::Square => 4,
            TextureLayout::Isometric => 4,
            TextureLayout::IsometricHex => 6,
            TextureLayout::Overlay => 4,
        }
    }

    #[allow(dead_code)]
    pub fn sprite_index_count(&self) -> u16 {
        match self {
            TextureLayout::Square => 6,
            TextureLayout::Isometric => 6,
            TextureLayout::IsometricHex => 12,
            TextureLayout::Overlay => 6,
        }
    }

    pub fn draw_range(&self) -> Range<u32> {
        match self {
            TextureLayout::Square => 0..6,
            TextureLayout::Isometric => 6..12,
            TextureLayout::IsometricHex => 12..24,
            TextureLayout::Overlay => 24..30,
        }
    }

    pub fn build_indices(&self, start_vertex_i: u16) -> Vec<u16> {
        let vertex_count = self.sprite_vertex_count();
        match self {
            TextureLayout::Square | TextureLayout::Isometric | TextureLayout::Overlay => (0..(vertex_count / 4))
                .flat_map(move |sprite_i| {
                    [0, 1, 2, 0, 2, 3]
                        .into_iter()
                        .map(move |index_i| (sprite_i * 4 + index_i) as u16 + start_vertex_i)
                })
                .collect(),
            TextureLayout::IsometricHex => (0..(vertex_count / 6))
                .flat_map(move |sprite_i| {
                    [0, 1, 2, 1, 3, 2, 2, 3, 5, 3, 4, 5]
                        .into_iter()
                        .map(move |index_i| (sprite_i * 6 + index_i) as u16 + start_vertex_i)
                })
                .collect(),
        }
    }
}
