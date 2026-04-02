use sparmos_engine::{
    application::{event_loop::AppLifecycle, state::State},
    entity::entities::cube::new,
    log,
    winit::event::DeviceEvent,
};

#[cfg(target_arch = "wasm32")]
use sparmos_engine::{application::event_loop::UserEvent, winit};

use crate::circular_buffer::CircularBuffer;

pub enum WasmEvent {
    ScrollPosition { x: f64, y: f64 },
    KeyboardButton { keypress: String },
}

pub struct EventContainer {}

impl AppLifecycle<WasmEvent> for EventContainer {
    fn on_user_event(&mut self, state: &mut State, event: WasmEvent) {
        match event {
            WasmEvent::ScrollPosition { x, y } => {
                state
                    .engine
                    .arguments
                    .args
                    .insert("scrolly".to_string(), Box::new(y));
                log::warn!("scroll x: {}, y: {}", x, y);
            }
            WasmEvent::KeyboardButton { keypress } => {
                let buffer = state
                    .engine
                    .arguments
                    .args
                    .entry("keypress".to_string())
                    .or_insert(Box::new(CircularBuffer::<String>::new(8)))
                    .downcast_mut::<CircularBuffer<String>>();
                if let Some(buffer) = buffer {
                    buffer.insert(keypress);
                    log::warn!("{:?}", buffer.to_string())
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn on_resumed(&mut self, proxy: &winit::event_loop::EventLoopProxy<UserEvent<WasmEvent>>) {
        use sparmos_engine::wgpu;
        use wasm_bindgen::{JsCast, prelude::Closure};

        let window = wgpu::web_sys::window().unwrap();
        let window_clone = window.clone();

        // Scroll listener
        let scroll_proxy = proxy.clone();
        let scroll_closure = Closure::<dyn FnMut(_)>::new(move |_event: wgpu::web_sys::Event| {
            let x = window_clone.scroll_x().unwrap_or(0.0);
            let y = window_clone.scroll_y().unwrap_or(0.0);

            let _ = scroll_proxy.send_event(UserEvent::Custom(WasmEvent::ScrollPosition { x, y }));
        });

        window
            .add_event_listener_with_callback("scroll", scroll_closure.as_ref().unchecked_ref())
            .unwrap();

        scroll_closure.forget();

        // Keyboard listener
        let key_proxy = proxy.clone();
        let key_closure = Closure::<dyn FnMut(_)>::new(move |event: wgpu::web_sys::Event| {
            use web_sys::KeyboardEvent;

            if let Some(kev) = event.dyn_ref::<KeyboardEvent>() {
                let _ = key_proxy.send_event(UserEvent::Custom(WasmEvent::KeyboardButton {
                    keypress: kev.key(),
                }));
            }
        });

        window
            .add_event_listener_with_callback("keypress", key_closure.as_ref().unchecked_ref())
            .unwrap();

        key_closure.forget();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn on_resumed(&mut self) {
        println!("Native resumed");
    }

    fn on_device_event(&mut self, event: DeviceEvent, _state: &mut State) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                log::debug!("Mouse delta: {:?}", delta);
            }
            _ => {}
        }
    }
}
