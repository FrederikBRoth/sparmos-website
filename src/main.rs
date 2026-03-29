use sparmos_engine::prelude::run_game;
use sparmos_website::{
    app::{EventContainer, WasmEvent},
    gameloop::Website,
};

fn main() {
    run_game::<WasmEvent, _, Website>(
        EventContainer {},
        Website {
            score: 0,
            ..Default::default()
        },
    )
    .unwrap();
}
