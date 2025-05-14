use ion_engine::core::world::UiDataType;

pub mod ui_debug;
pub mod ui_init;
pub mod ui_pause;
pub mod ui_tips;

#[derive(Debug, Clone)]
pub struct UiData {
    pub debug: Option<UiDebugData>,
}

impl UiDataType for UiData {}

#[derive(Debug, Clone)]
pub struct UiDebugData {
    pub lighting_sun: f32,
    pub lighting_ambient: f32,
}
