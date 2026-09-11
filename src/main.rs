use sparmos_engine::{
    log::{self, logger},
    prelude::run_game,
};
use sparmos_website::{
    app::{EventContainer, WasmEvent},
    gameloop::Website,
};

fn main() {
    let state = run_game::<WasmEvent, _, Website>(
        EventContainer {},
        Website {
            score: 0,
            ..Default::default()
        },
    );

    if let Some(err) = state.err() {
        log::error!("\n{}", err.er_report());
    }
}
