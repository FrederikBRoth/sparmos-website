pub mod app;
pub mod circular_buffer;
pub mod easter_egg;
pub mod gameloop;
pub mod gui;
pub mod markers;
pub mod transition;
pub mod voxel_builder;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::gameloop::Website;

// use app; // Removed because there is no external crate or module named 'app'

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    use crate::{
        app::{EventContainer, WasmEvent},
        gameloop::Website,
    };
    use sparmos_engine::prelude::run_game;

    console_error_panic_hook::set_once();
    run_game::<WasmEvent, _, Website>(
        EventContainer {},
        Website {
            score: 0,
            ..Default::default()
        },
    )
    .unwrap_throw();
    Ok(())
}
