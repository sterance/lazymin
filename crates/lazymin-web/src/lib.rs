mod ansi_backend;
mod xterm_input;

use std::cell::RefCell;
use std::rc::Rc;

use lazymin_core::app::App;
use lazymin_core::game::save;
use lazymin_core::input::InputEvent;
use lazymin_core::ui;
use ratatui::Terminal;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;

use ansi_backend::{AnsiBackend, AnsiBackendOptions};

thread_local! {
    static INPUT_QUEUE: RefCell<Vec<InputEvent>> = RefCell::new(Vec::new());
}

#[wasm_bindgen]
pub fn on_terminal_data(chunk: String) {
    INPUT_QUEUE.with(|q| {
        let mut q = q.borrow_mut();
        q.extend(xterm_input::parse_xterm_data(&chunk));
    });
}

#[wasm_bindgen]
pub fn run_game(write: &js_sys::Function, get_size: &js_sys::Function) -> Result<(), JsValue> {
    let app = match save::load() {
        Ok(Some(mut state)) => {
            save::append_restore_log_line(&mut state);
            App::with_game_state(state)
        }
        Ok(None) => App::new(),
        Err(_) => App::new(),
    };

    let mut backend = AnsiBackend::new(AnsiBackendOptions {
        get_size: get_size.clone(),
        write: write.clone(),
    });
    backend
        .exclusive()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let terminal = Terminal::new(backend).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let app = Rc::new(RefCell::new(app));
    let terminal = Rc::new(RefCell::new(terminal));

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let performance = window
        .performance()
        .ok_or_else(|| JsValue::from_str("no performance"))?;

    let last_tick_ms = Rc::new(RefCell::new(performance.now()));

    let closure_slot: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let closure_slot_for_cb = Rc::clone(&closure_slot);

    let app_for_cb = Rc::clone(&app);
    let terminal_for_cb = Rc::clone(&terminal);
    let perf_for_cb = performance.clone();
    let last_for_cb = Rc::clone(&last_tick_ms);

    let game_frame = Closure::wrap(Box::new(move || {
        let events: Vec<InputEvent> = INPUT_QUEUE.with(|q| q.borrow_mut().drain(..).collect());

        {
            let mut a = app_for_cb.borrow_mut();
            if !events.is_empty() {
                a.update(&events);
            }
            let now = perf_for_cb.now();
            let delta_secs = {
                let mut last = last_for_cb.borrow_mut();
                let d = (now - *last) / 1000.0;
                *last = now;
                d
            };
            a.tick(delta_secs);
        }

        {
            let a = app_for_cb.borrow();
            let mut t = terminal_for_cb.borrow_mut();
            if let Err(e) = t.draw(|f| ui::draw(f, &*a)) {
                web_sys::console::error_1(&JsValue::from_str(&format!("draw failed: {e}")));
            }
        }

        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };
        let slot = closure_slot_for_cb.borrow();
        let Some(closure) = slot.as_ref() else {
            return;
        };
        let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
    }) as Box<dyn FnMut()>);

    *closure_slot.borrow_mut() = Some(game_frame);

    {
        let slot = closure_slot.borrow();
        let first = slot
            .as_ref()
            .ok_or_else(|| JsValue::from_str("closure missing"))?;

        window.request_animation_frame(first.as_ref().unchecked_ref())?;
    }

    std::mem::forget(closure_slot);
    Ok(())
}
