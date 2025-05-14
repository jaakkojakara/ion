use std::{collections::VecDeque, ffi::OsStr, fs::File, path::PathBuf};
use std::{
    io::{self, Read},
    sync::mpsc::{self, Receiver, Sender},
};

use image::{GenericImageView, RgbaImage};

use ion_common::{Map, log_info};

use crate::build_shader;
use crate::core::Constants;
use crate::files::file_helpers::{list_files, load_resource};
use crate::gfx::renderer::gpu_data_types::SHADER_SCALE;
use crate::util::concurrency::{JoinHandle, spawn_thread_with_handle};

use super::{Texture, TextureId, TextureLayout};

const DEBUG_SAVE_TEXTURES: bool = false;
const MIPMAP_COUNT: u32 = 10;

pub struct TextureLoader {
    texture_sheet_max_size: u32,
    texture_details: Option<VecDeque<SingleTextureDetails>>,

    loaded_textures: Vec<Texture>,
    loaded_texture_ids: Map<String, TextureId>,

    details_thread: Option<JoinHandle<VecDeque<SingleTextureDetails>>>,
    loader_thread: Option<JoinHandle<(Vec<RgbaImage>, Map<String, TextureId>, VecDeque<SingleTextureDetails>)>>,
    // Progress tracking
    total_textures: usize,
    progress: f32,
    progress_sender: Sender<f32>,
    progress_receiver: Receiver<f32>,
}

impl TextureLoader {
    pub(crate) fn new(constants: &Constants, textures: Vec<String>, texture_sheet_max_size: u32) -> Self {
        log_info!("Loading texture assets: {} textures", textures.len());
        log_info!("Using texture_sheet_max_size of: {}", texture_sheet_max_size);

        let constants = constants.clone();
        let asset_path = constants.gfx.asset_path.clone();
        let total_textures = textures.len();
        let details_thread = spawn_thread_with_handle(Some("gen_texture_details"), move || {
            Self::gen_texture_details(&constants, asset_path, &textures)
        });

        let (progress_sender, progress_receiver) = mpsc::channel();

        Self {
            texture_sheet_max_size,
            texture_details: None,

            loaded_textures: Vec::new(),
            loaded_texture_ids: Map::default(),

            details_thread: Some(details_thread),
            loader_thread: None,
            total_textures,
            progress: 0.0,
            progress_sender,
            progress_receiver,
        }
    }

    pub(crate) fn finish(self) -> (Vec<Texture>, Map<String, TextureId>) {
        assert!(self.details_thread.is_none(), "Details thread must be finished");
        assert!(self.loader_thread.is_none(), "Loader thread must be finished");

        (self.loaded_textures, self.loaded_texture_ids)
    }

    pub(crate) fn progress(&self) -> f32 {
        self.progress
    }

    /// Returns `true` if the texture details are ready to be loaded.
    /// Should be polled from main thread until it returns `true`.
    /// Needs to be implemented by polling because on wasm writes to GPU are possible only in the main thread.
    pub(crate) fn poll_loading(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> bool {
        // If both threads are finished, we are done
        if self.details_thread.is_none() && self.loader_thread.is_none() {
            return true;
        }

        // Check if the details thread has finished
        if let Some(details_thread) = self.details_thread.as_mut() {
            if let Some(details) = details_thread.try_join() {
                self.details_thread = None;
                self.texture_details = Some(details);
                self.total_textures = self.texture_details.as_ref().unwrap().len();
            }
        }

        // Check if the loader thread has finished
        if let Some(loader_thread) = self.loader_thread.as_mut() {
            if let Some((image_sheets, texture_ids, texture_details)) = loader_thread.try_join() {
                image_sheets.into_iter().for_each(|image| {
                    let texture_name = format!("texture_sheet_{}", self.loaded_textures.len());
                    let texture = Texture::new_from_raw_data(
                        device,
                        queue,
                        wgpu::TextureFormat::Rgba8UnormSrgb,
                        &image,
                        image.dimensions(),
                        MIPMAP_COUNT,
                        &texture_name,
                    );

                    Self::generate_mipmaps(device, queue, &texture.texture, MIPMAP_COUNT);

                    self.loaded_textures.push(texture);

                    if DEBUG_SAVE_TEXTURES {
                        image
                            .save(format!("target/debug/textures/texture_{}.png", texture_name))
                            .unwrap();
                    }
                });

                self.loaded_texture_ids.extend(texture_ids);
                self.texture_details = Some(texture_details);
                self.loader_thread = None;
            }
        }

        while let Ok(progress) = self.progress_receiver.try_recv() {
            self.progress = progress;
        }

        // Check if there are still textures to load
        if self.loader_thread.is_none() && self.texture_details.is_some() {
            let texture_details = self.texture_details.take().unwrap();
            let texture_sheet_max_size = self.texture_sheet_max_size;
            let loaded_textures_len = self.loaded_textures.len() as u32;
            let total_textures = self.total_textures;
            let progress_sender = self.progress_sender.clone();

            if !texture_details.is_empty() {
                self.loader_thread = Some(spawn_thread_with_handle(Some("gen_texture_sheet"), move || {
                    Self::gen_texture_sheet(
                        texture_details,
                        texture_sheet_max_size,
                        loaded_textures_len,
                        total_textures,
                        progress_sender,
                    )
                }));
            }
        }

        false
    }

    fn gen_texture_details(
        constants: &Constants,
        asset_path: PathBuf,
        asset_names: &[String],
    ) -> VecDeque<SingleTextureDetails> {
        let mut separate_texture_paths: Vec<_> = Self::list_files_with_dimensions(&asset_path)
            .expect("Texture assets must be accessible")
            .into_iter()
            .filter(|(path, _)| {
                asset_names.iter().any(|texture| {
                    path.file_stem()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .contains(&format!("{}.", texture))
                })
            })
            .collect();

        log_info!("Loading {} textures", separate_texture_paths.len());

        separate_texture_paths.sort();

        let mut texture_sources: Map<String, (PathBuf, Option<PathBuf>, Option<PathBuf>, (u32, u32))> = Map::default();

        for (path, dimensions) in separate_texture_paths {
            let file_name = path.file_stem().unwrap().to_str().unwrap().to_owned();
            let file_name_parts: Vec<_> = file_name.split('.').collect();
            let name = file_name_parts[0].to_owned();
            let type_tag = file_name_parts[1];

            match type_tag {
                "c" => {
                    texture_sources.insert(name, (path, None, None, dimensions));
                }
                "n" => {
                    texture_sources.get_mut(&name).unwrap().1 = Some(path);
                }
                "h" => {
                    texture_sources.get_mut(&name).unwrap().2 = Some(path);
                }
                _ => {
                    panic!("Invalid texture type tag: {}", name);
                }
            }
        }

        let mut texture_details: Vec<SingleTextureDetails> = texture_sources
            .into_iter()
            .map(|(_, (path_c, path_n, path_h, dimensions))| {
                let file_name = path_c.file_stem().unwrap().to_str().unwrap().to_owned();
                let file_name_parts: Vec<_> = file_name.split('.').collect();

                let name = file_name_parts[0].to_owned();
                let is_anim = file_name_parts
                    .get(4)
                    .map(|anim_text| *anim_text == "anim")
                    .unwrap_or(false);
                let sub_images = file_name_parts.get(3).map(|dimension_text| {
                    let mut parts = dimension_text.split('x');
                    let x: u32 = parts.next().unwrap().parse().unwrap();
                    let y: u32 = parts.next().unwrap().parse().unwrap();
                    (x, y)
                });
                let layout = match file_name_parts[2] {
                    "ov" => TextureLayout::Overlay,
                    "sq" => TextureLayout::Square,
                    "iso" => {
                        let unit_height = ((dimensions.0 / sub_images.unwrap_or((1, 1)).0) as f32
                            * constants.gfx.camera_angle_deg.to_radians().cos())
                        .round() as u32;

                        #[allow(clippy::comparison_chain)]
                        if unit_height == dimensions.1 {
                            TextureLayout::Isometric
                        } else if unit_height < dimensions.1 {
                            TextureLayout::IsometricHex
                        } else {
                            panic!("Invalid texture dimensions (y too small) for {}", name)
                        }
                    }
                    _ => panic!("Invalid texture layout tag for {}", name),
                };
                SingleTextureDetails {
                    name,
                    path_c,
                    path_n,
                    path_h,
                    dimensions,
                    sub_images,
                    is_anim,
                    layout,
                }
            })
            .collect();

        texture_details.sort_by_key(|details| {
            let (x_count, _) = details.sub_images.unwrap_or((1, 1));
            details.dimensions.0 / x_count
        });
        texture_details.sort_by_key(|details| {
            let (_, y_count) = details.sub_images.unwrap_or((1, 1));
            details.dimensions.1 / y_count
        });

        VecDeque::from(texture_details)
    }

    fn gen_texture_sheet(
        mut texture_details: VecDeque<SingleTextureDetails>,
        texture_sheet_max_size: u32,
        texture_sheet_index: u32,
        total_textures: usize,
        progress_sender: Sender<f32>,
    ) -> (Vec<RgbaImage>, Map<String, TextureId>, VecDeque<SingleTextureDetails>) {
        let mut sheet_c = RgbaImage::new(texture_sheet_max_size, texture_sheet_max_size);
        let mut sheet_nh = RgbaImage::new(texture_sheet_max_size, texture_sheet_max_size);
        let mut texture_ids = Map::default();

        let max_size_f32 = texture_sheet_max_size as f32;
        let mut cur_x = 0;
        let mut cur_y = 0;

        let next_exists_and_fits = |texture_details: &VecDeque<SingleTextureDetails>, cur_x: u32, cur_y: u32| {
            if let Some(details) = texture_details.front() {
                let (x_count, y_count) = details.sub_images.unwrap_or((1, 1));
                let width = details.dimensions.0 / x_count;
                let height = details.dimensions.1 / y_count;
                let rows_available = (texture_sheet_max_size - cur_y) / (height + 1); // +1 for spacing between rows
                let cols_available = (texture_sheet_max_size - cur_x) / (width + 1); // +1 for spacing between columns
                let cur_row_cols_used = (cur_x + width) / (width + 1);

                x_count * y_count <= rows_available * cols_available - cur_row_cols_used
            } else {
                false
            }
        };

        while next_exists_and_fits(&texture_details, cur_x, cur_y) {
            let details = texture_details.pop_front().unwrap();
            let (x_sub, y_sub) = details.sub_images.unwrap_or((1, 1));
            let width = details.dimensions.0 / x_sub;
            let height = details.dimensions.1 / y_sub;
            let x_images = if details.is_anim { 1 } else { x_sub };

            let img_c = image::load_from_memory(&load_resource(&details.path_c).expect("Failed to load color texture"))
                .expect("Failed to parse color texture");
            let img_n = details.path_n.as_ref().map(|path| {
                image::load_from_memory(&load_resource(&path).expect("Failed to load normal texture"))
                    .expect("Failed to parse normal texture")
            });
            let img_h = details.path_h.as_ref().map(|path| {
                image::load_from_memory(&load_resource(&path).expect("Failed to load height texture"))
                    .expect("Failed to parse height texture")
            });

            let mut cur_src_x = 0;
            let mut cur_src_y = 0;
            for y in 0..y_sub {
                for x_image in 0..x_images {
                    if cur_x + width > texture_sheet_max_size {
                        cur_x = 0;
                        cur_y += height + 1;
                    }

                    let texture_id = TextureId {
                        tex_coords: [cur_x as f32 / max_size_f32, cur_y as f32 / max_size_f32],
                        tex_coords_sizes: [width as f32 / max_size_f32, height as f32 / max_size_f32],
                        tex_sheet_indices: [
                            texture_sheet_index,
                            if details.has_n_or_h() {
                                texture_sheet_index + 1
                            } else {
                                u32::MAX
                            },
                        ],
                        layout: details.layout,
                        frame_count: if details.is_anim { x_sub as u16 } else { 1 },
                    };

                    let texture_name = if x_images > 1 || y_sub > 1 {
                        format!("{}_{}", details.name, y * x_images + x_image)
                    } else {
                        details.name.clone()
                    };

                    texture_ids.insert(texture_name, texture_id);

                    let x_anim_frames = if details.is_anim { x_sub } else { 1 };
                    for _x in 0..x_anim_frames {
                        if cur_src_x + width > details.dimensions.0 {
                            cur_src_x = 0;
                            cur_src_y += height;
                        }
                        if cur_x + width > texture_sheet_max_size {
                            cur_x = 0;
                            cur_y += height + 1;
                        }

                        // Copy pixels
                        for y_pixel in 0..height {
                            for x_pixel in 0..width {
                                let src_x = cur_src_x + x_pixel;
                                let src_y = cur_src_y + y_pixel;
                                let dst_x = cur_x + x_pixel;
                                let dst_y = cur_y + y_pixel;
                                sheet_c.put_pixel(dst_x, dst_y, img_c.get_pixel(src_x, src_y));
                                if let Some(img_n) = img_n.as_ref() {
                                    let mut pixel = img_n.get_pixel(src_x, src_y);
                                    if let Some(img_h) = img_h.as_ref() {
                                        pixel[3] = img_h.get_pixel(src_x, src_y).0[0];
                                    }
                                    sheet_nh.put_pixel(dst_x, dst_y, pixel);
                                }
                            }
                        }

                        cur_src_x += width;
                        cur_x += width + 1;
                    }
                }
            }

            progress_sender
                .send(1.0 - texture_details.len() as f32 / total_textures as f32)
                .expect("Texture loader progress sender must be valid");
        }

        (vec![sheet_c, sheet_nh], texture_ids, texture_details)
    }

    /// Reads the first 24 bytes of a PNG and parses out (width, height)
    #[cfg(not(target_arch = "wasm32"))]
    fn read_png_dimensions(path: &PathBuf) -> io::Result<(u32, u32)> {
        let mut file = File::open(path)?;
        let mut header = [0u8; 24];
        file.read_exact(&mut header)?;

        // Bytes 16..20 = width, Bytes 20..24 = height, big-endian u32
        let width = u32::from_be_bytes([header[16], header[17], header[18], header[19]]);
        let height = u32::from_be_bytes([header[20], header[21], header[22], header[23]]);

        Ok((width, height))
    }

    fn list_files_with_dimensions(path: &PathBuf) -> io::Result<Vec<(PathBuf, (u32, u32))>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let texture_file_types = vec![OsStr::new("png")];
            let files = list_files(path, Some(texture_file_types.as_slice()))?;

            let mut results = Vec::new();
            for (file_path, _) in files {
                if let Ok(dimensions) = Self::read_png_dimensions(&file_path) {
                    results.push((file_path, dimensions));
                }
            }
            Ok(results)
        }

        #[cfg(target_arch = "wasm32")]
        {
            use ion_common::web_sys::XmlHttpRequest;

            let xhr = XmlHttpRequest::new()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to create XHR: {:?}", e)))?;

            // Configure XHR
            xhr.set_response_type(ion_common::web_sys::XmlHttpRequestResponseType::Text);
            xhr.open_with_async("GET", "/api/textures", false)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to open XHR: {:?}", e)))?;

            // Send the request
            xhr.send()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to send XHR: {:?}", e)))?;

            // Check if the request was successful
            if xhr.status().unwrap_or(0) != 200 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("HTTP error: {}", xhr.status().unwrap_or(0)),
                ));
            }

            // Parse the response
            let response_text = xhr
                .response_text()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get response text: {:?}", e)))?;

            let mut results = Vec::new();
            if let Some(text) = response_text {
                for line in text.lines() {
                    let parts: Vec<&str> = line.split('\t').collect();
                    if parts.len() != 2 {
                        continue;
                    }

                    let path_str = parts[0];
                    let dimensions_str = parts[1];
                    let dim_parts: Vec<&str> = dimensions_str.split('x').collect();

                    if dim_parts.len() != 2 {
                        continue;
                    }

                    if let (Ok(width), Ok(height)) = (dim_parts[0].parse::<u32>(), dim_parts[1].parse::<u32>()) {
                        results.push((PathBuf::from(path_str), (width, height)));
                    }
                }
            }

            Ok(results)
        }
    }

    fn generate_mipmaps(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture, mipmap_count: u32) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("mipmap_encoder"),
        });

        let shader = build_shader!(device, SHADER_SCALE);
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mipmap_blit_pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::TextureFormat::Rgba8UnormSrgb.into())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let bind_group_layout = pipeline.get_bind_group_layout(0);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mipmap_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let views = (0..mipmap_count)
            .map(|mip| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some(format!("mipmap_view_{}", mip).as_str()),
                    format: None,
                    dimension: None,
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: mip,
                    mip_level_count: Some(1),
                    base_array_layer: 0,
                    array_layer_count: None,
                    usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING),
                })
            })
            .collect::<Vec<_>>();

        for target_mip in 1..mipmap_count as usize {
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&views[target_mip - 1]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
                label: None,
            });

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &views[target_mip],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            render_pass.set_pipeline(&pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));
    }
}

#[derive(Debug)]
struct SingleTextureDetails {
    name: String,
    path_c: PathBuf,
    path_n: Option<PathBuf>,
    path_h: Option<PathBuf>,
    dimensions: (u32, u32),
    sub_images: Option<(u32, u32)>,
    is_anim: bool,
    layout: TextureLayout,
}

impl SingleTextureDetails {
    fn has_n_or_h(&self) -> bool {
        self.path_n.is_some() || self.path_h.is_some()
    }
}
