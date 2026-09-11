pub mod app;
pub mod circular_buffer;
pub mod easter_egg;
pub mod gameloop;
pub mod gui;
pub mod markers;
pub mod transition;
pub mod voxel_builder;
use er::Er;
#[cfg(target_arch = "wasm32")]
use er::ErResult;
use sparmos_engine::log;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// use app; // Removed because there is no external crate or module named 'app'

#[derive(Er)]
pub struct WasmError;
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    use crate::{
        app::{EventContainer, WasmEvent},
        gameloop::Website,
    };
    use sparmos_engine::prelude::run_game;

    console_error_panic_hook::set_once();
    match run_game::<WasmEvent, _, Website>(
        EventContainer {},
        Website {
            score: 0,
            ..Default::default()
        },
    ) {
        Ok(_) => {}
        Err(err) => {
            log::error!("{}", err.er_report())
        }
    }
    Ok(())
}
