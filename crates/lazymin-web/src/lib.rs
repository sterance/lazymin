mod ansi_backend;
mod xterm_input;

use std::cell::RefCell;
use std::rc::Rc;

use lazymin_core::app::App;
use lazymin_core::audio::{BACKGROUND_LOOP_OPUS, DING_OPUS};
use lazymin_core::game::save;
use lazymin_core::input::InputEvent;
use lazymin_core::ui;
use ratatui::Terminal;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;

use ansi_backend::{AnsiBackend, AnsiBackendOptions};

thread_local! {
    static INPUT_QUEUE: RefCell<Vec<InputEvent>> = RefCell::new(Vec::new());
    static MUSIC: RefCell<MusicState> = RefCell::new(MusicState::new());
    static DING: RefCell<DingState> = RefCell::new(DingState::new());
}

#[derive(Default)]
struct MusicState {
    started: bool,
    attempt_in_flight: bool,
    object_url: Option<String>,
    html_audio: Option<web_sys::HtmlAudioElement>,
    audio_ctx: Option<web_sys::AudioContext>,
    webaudio_source: Option<web_sys::AudioBufferSourceNode>,
}

impl MusicState {
    fn new() -> Self {
        Self::default()
    }
}

#[derive(Default)]
struct DingState {
    attempt_in_flight: bool,
    pending_play: bool,
    object_url: Option<String>,
    html_audio: Option<web_sys::HtmlAudioElement>,
    decoded: Option<web_sys::AudioBuffer>,
}

impl DingState {
    fn new() -> Self {
        Self::default()
    }
}

fn sync_audio_mute(sound_muted: bool) {
    MUSIC.with(|slot| {
        let state = slot.borrow_mut();

        if let Some(ctx) = state.audio_ctx.as_ref() {
            if sound_muted {
                let _ = ctx.suspend();
            } else {
                let _ = ctx.resume();
            }
        }

        if let Some(audio) = state.html_audio.as_ref() {
            audio.set_muted(sound_muted);
        }
    });

    DING.with(|slot| {
        let state = slot.borrow_mut();
        if let Some(audio) = state.html_audio.as_ref() {
            audio.set_muted(sound_muted);
        }
    });

    if sound_muted {
        DING.with(|slot| slot.borrow_mut().pending_play = false);
    }
}

fn ensure_object_url(state: &mut MusicState) -> Result<String, JsValue> {
    if let Some(url) = state.object_url.clone() {
        return Ok(url);
    }

    let bytes = js_sys::Uint8Array::from(BACKGROUND_LOOP_OPUS);
    let parts = js_sys::Array::new();
    parts.push(&bytes.buffer());
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("audio/ogg; codecs=opus");
    let blob = web_sys::Blob::new_with_buffer_source_sequence_and_options(&parts, &opts)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;
    state.object_url = Some(url.clone());
    Ok(url)
}

fn ensure_ding_object_url(state: &mut DingState) -> Result<String, JsValue> {
    if let Some(url) = state.object_url.clone() {
        return Ok(url);
    }

    let bytes = js_sys::Uint8Array::from(DING_OPUS);
    let parts = js_sys::Array::new();
    parts.push(&bytes.buffer());
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("audio/ogg; codecs=opus");
    let blob = web_sys::Blob::new_with_buffer_source_sequence_and_options(&parts, &opts)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;
    state.object_url = Some(url.clone());
    Ok(url)
}

fn try_start_music() {
    MUSIC.with(|slot| {
        let mut state = slot.borrow_mut();
        if let Some(audio) = state.html_audio.as_ref() {
            if !audio.paused() {
                state.started = true;
                return;
            }
        }
        if state.started || state.attempt_in_flight {
            return;
        }
        if BACKGROUND_LOOP_OPUS.is_empty() {
            return;
        }

        state.attempt_in_flight = true;

        let object_url = match ensure_object_url(&mut state) {
            Ok(u) => u,
            Err(e) => {
                state.attempt_in_flight = false;
                web_sys::console::error_1(&e);
                return;
            }
        };

        // try Web Audio first; it provides the cleanest loop when it can decode the asset.
        // if decoding fails (common for opus in some browsers), fall back to an <audio> element.
        let audio_ctx = match web_sys::AudioContext::new() {
            Ok(ctx) => ctx,
            Err(e) => {
                state.attempt_in_flight = false;
                web_sys::console::error_1(&e);
                return;
            }
        };

        let bytes = js_sys::Uint8Array::from(BACKGROUND_LOOP_OPUS);
        let array_buffer = bytes.buffer();

        let ctx_for_ok = audio_ctx.clone();
        let ok = Closure::wrap(Box::new(move |buffer: web_sys::AudioBuffer| {
            MUSIC.with(|slot| {
                let mut state = slot.borrow_mut();
                let source = match ctx_for_ok.create_buffer_source() {
                    Ok(s) => s,
                    Err(e) => {
                        state.attempt_in_flight = false;
                        web_sys::console::error_1(&e);
                        return;
                    }
                };
                source.set_buffer(Some(&buffer));
                source.set_loop(true);
                source.connect_with_audio_node(&ctx_for_ok.destination()).ok();
                if let Err(e) = source.start() {
                    state.attempt_in_flight = false;
                    web_sys::console::error_1(&e);
                    return;
                }

                state.audio_ctx = Some(ctx_for_ok.clone());
                state.webaudio_source = Some(source);
                state.started = true;
                state.attempt_in_flight = false;
            });
        }) as Box<dyn FnMut(web_sys::AudioBuffer)>);

        let err = Closure::wrap(Box::new(move || {
            MUSIC.with(|slot| {
                let mut state = slot.borrow_mut();
                state.audio_ctx = None;
                state.webaudio_source = None;
                state.attempt_in_flight = false;

                // fallback to HTML audio; start is retried on subsequent user inputs
                // if autoplay is blocked.
                let audio = state
                    .html_audio
                    .get_or_insert_with(|| web_sys::HtmlAudioElement::new().unwrap());
                audio.set_src(&object_url);
                audio.set_loop(true);
                audio.set_preload("auto");

                if let Err(e) = audio.play() {
                    // expected if autoplay is still blocked; we'll retry on the next input.
                    web_sys::console::debug_1(&e);
                } else if !audio.paused() {
                    state.started = true;
                }
            });
        }) as Box<dyn FnMut()>);

        // use callback-based decode to avoid async/futures plumbing.
        // on success, we start Web Audio looping; on error, we fall back.
        audio_ctx
            .decode_audio_data_with_success_callback_and_error_callback(
                &array_buffer,
                ok.as_ref().unchecked_ref(),
                err.as_ref().unchecked_ref(),
            )
            .ok();

        state.audio_ctx = Some(audio_ctx);

        ok.forget();
        err.forget();
    });
}

fn play_ding() {
    if DING_OPUS.is_empty() {
        return;
    }

    DING.with(|slot| {
        let mut state = slot.borrow_mut();
        if let Some(ctx) = MUSIC.with(|m| m.borrow().audio_ctx.clone()) {
            if let Some(buffer) = state.decoded.as_ref() {
                if let Ok(source) = ctx.create_buffer_source() {
                    source.set_buffer(Some(buffer));
                    source.set_loop(false);
                    source.connect_with_audio_node(&ctx.destination()).ok();
                    source.start().ok();
                }
                state.pending_play = false;
                return;
            }

            if state.attempt_in_flight {
                state.pending_play = true;
                return;
            }

            state.attempt_in_flight = true;
            state.pending_play = true;

            let bytes = js_sys::Uint8Array::from(DING_OPUS);
            let array_buffer = bytes.buffer();

            let ctx_for_ok = ctx.clone();
            let ok = Closure::wrap(Box::new(move |buffer: web_sys::AudioBuffer| {
                DING.with(|slot| {
                    let mut state = slot.borrow_mut();
                    state.decoded = Some(buffer.clone());
                    state.attempt_in_flight = false;

                    if state.pending_play {
                        if let Ok(source) = ctx_for_ok.create_buffer_source() {
                            source.set_buffer(Some(&buffer));
                            source.set_loop(false);
                            source
                                .connect_with_audio_node(&ctx_for_ok.destination())
                                .ok();
                            source.start().ok();
                        }
                        state.pending_play = false;
                    }
                });
            }) as Box<dyn FnMut(web_sys::AudioBuffer)>);

            let err = Closure::wrap(Box::new(move || {
                DING.with(|slot| {
                    let mut state = slot.borrow_mut();
                    state.decoded = None;
                    state.attempt_in_flight = false;
                    state.pending_play = false;

                    let object_url = match ensure_ding_object_url(&mut state) {
                        Ok(u) => u,
                        Err(e) => {
                            web_sys::console::error_1(&e);
                            return;
                        }
                    };

                    let audio = state
                        .html_audio
                        .get_or_insert_with(|| web_sys::HtmlAudioElement::new().unwrap());
                    audio.set_src(&object_url);
                    audio.set_loop(false);
                    audio.set_preload("auto");
                    let _ = audio.set_current_time(0.0);
                    audio.play().ok();
                });
            }) as Box<dyn FnMut()>);

            ctx.decode_audio_data_with_success_callback_and_error_callback(
                &array_buffer,
                ok.as_ref().unchecked_ref(),
                err.as_ref().unchecked_ref(),
            )
            .ok();

            ok.forget();
            err.forget();
            return;
        }

        let object_url = match ensure_ding_object_url(&mut state) {
            Ok(u) => u,
            Err(e) => {
                web_sys::console::error_1(&e);
                return;
            }
        };

        let audio = state
            .html_audio
            .get_or_insert_with(|| web_sys::HtmlAudioElement::new().unwrap());
        audio.set_src(&object_url);
        audio.set_loop(false);
        audio.set_preload("auto");
        let _ = audio.set_current_time(0.0);
        audio.play().ok();
    });
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
            state.sound_muted = false;
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

        let (ding_now, sound_muted_now) = {
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
            let ding = a.poll_input_became_ready();
            (ding, a.game.sound_muted)
        };

        sync_audio_mute(sound_muted_now);
        if !sound_muted_now && ding_now {
            play_ding();
        }
        if !sound_muted_now {
            try_start_music();
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
