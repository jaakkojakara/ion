use derive_engine::Config;
use std::{collections::BTreeMap, time::Duration};
use winit::dpi::PhysicalSize;

// ---------------------------------------------------------- //
// --------------------- Render Config ---------------------- //
// ---------------------------------------------------------- //

#[derive(Debug, Clone, PartialEq, Config)]
pub struct GfxConfig {
    pub window_decorations: bool,
    pub window_transparent: bool,
    pub window_mode: WindowMode,
    pub frame_resolution: Resolution,
    pub frame_rate_cap: Option<u32>,
    pub vsync: VsyncOpts,
}

impl Default for GfxConfig {
    fn default() -> Self {
        Self {
            window_decorations: true,
            window_transparent: false,
            window_mode: WindowMode::Windowed,
            frame_resolution: Resolution {
                width: 800,
                height: 600,
            },
            frame_rate_cap: Some(60),
            vsync: VsyncOpts::On,
        }
    }
}

impl GfxConfig {
    pub fn frame_time_cap(&self) -> Option<Duration> {
        self.frame_rate_cap
            .map(|frame_rate_cap| Duration::from_nanos(1_000_000_000u64 / frame_rate_cap as u64))
    }
}

// ---------------------------------------------------------- //
// --------------- Individual Render options ---------------- //
// ---------------------------------------------------------- //

#[derive(Debug, Clone, Copy, PartialEq, Eq, Config)]
pub enum VsyncOpts {
    On,
    Off,
    OnFast,
    OnAdaptive,
}

impl From<VsyncOpts> for wgpu::PresentMode {
    fn from(value: VsyncOpts) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        match value {
            VsyncOpts::On => wgpu::PresentMode::Fifo,
            VsyncOpts::Off => wgpu::PresentMode::Immediate,
            VsyncOpts::OnFast => wgpu::PresentMode::Mailbox,
            VsyncOpts::OnAdaptive => wgpu::PresentMode::FifoRelaxed,
        }

        #[cfg(target_arch = "wasm32")]
        match value {
            VsyncOpts::On => wgpu::PresentMode::AutoVsync,
            VsyncOpts::Off => wgpu::PresentMode::AutoNoVsync,
            VsyncOpts::OnFast => wgpu::PresentMode::AutoVsync,
            VsyncOpts::OnAdaptive => wgpu::PresentMode::AutoVsync,
        }
    }
}

impl From<wgpu::PresentMode> for VsyncOpts {
    fn from(value: wgpu::PresentMode) -> Self {
        match value {
            wgpu::PresentMode::Fifo => VsyncOpts::On,
            wgpu::PresentMode::Immediate => VsyncOpts::Off,
            wgpu::PresentMode::Mailbox => VsyncOpts::OnFast,
            wgpu::PresentMode::FifoRelaxed => VsyncOpts::OnAdaptive,
            wgpu::PresentMode::AutoVsync => VsyncOpts::On,
            wgpu::PresentMode::AutoNoVsync => VsyncOpts::Off,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WindowMode {
    Windowed,
    BorderlessFullscreen,
    ExclusiveFullscreen(VideoMode),
}

impl Config for WindowMode {
    fn encode_kv_table(&self, name: &str, table: &mut BTreeMap<String, String>) {
        match self {
            WindowMode::Windowed => table.insert(name.to_string(), "Windowed".to_string()),
            WindowMode::BorderlessFullscreen => table.insert(name.to_string(), "BorderlessFullscreen".to_string()),
            WindowMode::ExclusiveFullscreen(mode) => table.insert(
                name.to_string(),
                format!(
                    "ExclusiveFullscreen({},{},{})",
                    mode.width, mode.height, mode.frame_rate
                ),
            ),
        };
    }

    fn decode_kv_table(name: &str, table: &BTreeMap<String, String>) -> Result<Self, ConfigParseError> {
        let value = table.get(name).ok_or(ConfigParseError::MissingData(name.to_string()))?;
        if value == "Windowed" {
            Ok(WindowMode::Windowed)
        } else if value == "BorderlessFullscreen" {
            Ok(WindowMode::BorderlessFullscreen)
        } else if value.starts_with("ExclusiveFullscreen(") {
            let parts = value.split(',').collect::<Vec<&str>>();
            Ok(WindowMode::ExclusiveFullscreen(VideoMode {
                width: parts[0].parse().unwrap(),
                height: parts[1].parse().unwrap(),
                frame_rate: parts[2].parse().unwrap(),
            }))
        } else {
            Err(ConfigParseError::InvalidFieldType(name.to_string()))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VideoMode {
    pub width: u32,
    pub height: u32,
    pub frame_rate: u32,
}

impl From<winit::monitor::VideoModeHandle> for VideoMode {
    fn from(value: winit::monitor::VideoModeHandle) -> Self {
        VideoMode {
            width: value.size().width,
            height: value.size().height,
            frame_rate: value.refresh_rate_millihertz() / 1000,
        }
    }
}

impl From<VideoMode> for PhysicalSize<u32> {
    fn from(value: VideoMode) -> Self {
        PhysicalSize {
            width: value.width,
            height: value.height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Config)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl From<Resolution> for PhysicalSize<u32> {
    fn from(value: Resolution) -> Self {
        PhysicalSize {
            width: value.width,
            height: value.height,
        }
    }
}

impl From<PhysicalSize<u32>> for Resolution {
    fn from(value: PhysicalSize<u32>) -> Self {
        Self {
            width: value.width,
            height: value.height,
        }
    }
}

impl Resolution {
    #[allow(dead_code)]
    pub(crate) fn as_u32(self) -> (u32, u32) {
        (self.width, self.height)
    }

    #[allow(dead_code)]
    pub(crate) fn as_fraction_u32(self, fraction: u32) -> (u32, u32) {
        (self.width / fraction, self.height / fraction)
    }
}
