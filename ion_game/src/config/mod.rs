use ion_engine::{
    core::{Constants, GfxConstants},
    gfx::gfx_config::{GfxConfig, Resolution, VsyncOpts, WindowMode},
};
use std::path::PathBuf;

use crate::state::Props;

pub mod bindings;

pub fn constants() -> Constants {
    Constants {
        app_name: "ION",
        gfx: GfxConstants {
            asset_path: PathBuf::from("ion_game/assets/textures"),
            camera_angle_deg: 28.995,
            pixels_per_unit: 128.0,
            height_units_total: 8.0,
            height_scaled_zero: 64.0 / 255.0,
        },
        net: None,
    }
}

pub fn splash_screen_gfx_config(props: &Props) -> GfxConfig {
    let dpi_factor = props.renderer.dpi_factor();
    GfxConfig {
        window_decorations: false,
        window_transparent: false,
        window_mode: WindowMode::Windowed,
        frame_resolution: Resolution {
            width: (480.0 * dpi_factor) as u32,
            height: (240.0 * dpi_factor) as u32,
        },
        frame_rate_cap: Some(60),
        vsync: VsyncOpts::Off,
    }
}

pub fn default_gfx_config() -> GfxConfig {
    GfxConfig {
        window_decorations: true,
        window_transparent: false,
        window_mode: WindowMode::Windowed,
        frame_resolution: Resolution {
            width: 1920,
            height: 1080,
        },
        frame_rate_cap: Some(120),
        vsync: VsyncOpts::On,
    }
}
