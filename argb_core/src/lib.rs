//! Shared core for ArgbProMaster: settings schema, preset engine and the
//! animation renderer used by both the configurator GUI (live preview) and the
//! thermal daemon (real hardware output). Keeping the renderer here guarantees
//! the preview is pixel-identical to what the LEDs will do.

pub mod engine;
pub mod openrgb;
pub mod presets;
pub mod settings;
pub mod win;
pub mod zones;
