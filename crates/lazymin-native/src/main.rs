use std::error::Error;
use std::io::Cursor;
use std::num::{NonZeroU16, NonZeroU32};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use lazymin_core::app::App;
use lazymin_core::audio::{BACKGROUND_LOOP_OPUS, DING_OPUS};
use lazymin_core::game::save;
use lazymin_core::input::InputEvent;
use lazymin_core::ui;
use rodio::buffer::SamplesBuffer;
use rodio::mixer::Mixer;
use rodio::{DeviceSinkBuilder, Source};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CodecRegistry, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia_adapter_libopus::OpusDecoder;

#[cfg(feature = "dev-presets")]
use lazymin_core::game::dev_presets::{dev_game_state, DevTier};

type AppResult<T> = Result<T, Box<dyn Error>>;

#[derive(Clone)]
struct MuteableSource<S> {
    source: S,
    muted: Arc<AtomicBool>,
}

impl<S> Iterator for MuteableSource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.muted.load(Ordering::Relaxed) {
            return Some(0.0);
        }
        self.source.next()
    }
}

impl<S> Source for MuteableSource<S>
where
    S: Source<Item = f32>,
{
    fn current_span_len(&self) -> Option<usize> {
        self.source.current_span_len()
    }

    fn channels(&self) -> NonZeroU16 {
        self.source.channels()
    }

    fn sample_rate(&self) -> NonZeroU32 {
        self.source.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }
}

fn decode_opus_to_samples(bytes: &[u8]) -> Result<SamplesBuffer, SymphoniaError> {
    let cursor = Cursor::new(bytes.to_vec());
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

    let mut hint = Hint::new();
    // the underlying container is typically ogg even if the file extension is opus/ogx.
    hint.with_extension("ogg");
    hint.with_extension("opus");

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;
    let mut format = probed.format;

    let track = format
        .default_track()
        .ok_or(SymphoniaError::Unsupported("no default track"))?;
    let track_id = track.id;
    let codec_params = track.codec_params.clone();
    let channels: u16 = codec_params
        .channels
        .ok_or(SymphoniaError::Unsupported("missing channel count"))?
        .count() as u16;
    let sample_rate: u32 = codec_params
        .sample_rate
        .ok_or(SymphoniaError::Unsupported("missing sample rate"))?;

    let mut codec_registry = CodecRegistry::new();
    codec_registry.register_all::<OpusDecoder>();
    let mut decoder = codec_registry.make(&codec_params, &DecoderOptions::default())?;

    let mut samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(_)) => break,
            Err(e) => return Err(e),
        };
        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymphoniaError::IoError(_)) => break,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(e),
        };

        let mut buf =
            SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        buf.copy_interleaved_ref(decoded);
        samples.extend_from_slice(buf.samples());
    }

    let channels = NonZeroU16::new(channels).ok_or(SymphoniaError::Unsupported("zero channels"))?;
    let sample_rate =
        NonZeroU32::new(sample_rate).ok_or(SymphoniaError::Unsupported("zero sample rate"))?;
    Ok(SamplesBuffer::new(channels, sample_rate, samples))
}

fn app_from_disk_or_new() -> App {
    match save::load() {
        Ok(Some(mut state)) => {
            save::append_restore_log_line(&mut state);
            state.sound_muted = false;
            App::with_game_state(state)
        }
        Ok(None) => App::new(),
        Err(e) => {
            eprintln!("warning: could not load save: {e}");
            App::new()
        }
    }
}

fn maybe_play_ding(mixer: &Mixer, app: &mut App, muted: &Arc<AtomicBool>) {
    if app.game.sound_muted {
        return;
    }
    if !app.poll_input_became_ready() {
        return;
    }
    if DING_OPUS.is_empty() {
        return;
    }
    if let Ok(buf) = decode_opus_to_samples(DING_OPUS) {
        mixer.add(MuteableSource {
            source: buf,
            muted: Arc::clone(muted),
        });
    }
}

fn main() -> AppResult<()> {
    #[cfg(feature = "dev-presets")]
    let mut app = match parse_dev_tier_from_env_args() {
        Ok(Some(tier)) => {
            eprintln!("dev preset active: {}", tier.as_str());
            App::with_game_state(dev_game_state(tier))
        }
        Ok(None) => app_from_disk_or_new(),
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(1);
        }
    };
    #[cfg(not(feature = "dev-presets"))]
    let mut app = app_from_disk_or_new();

    let sound_muted = Arc::new(AtomicBool::new(app.game.sound_muted));

    let mut sink_handle = DeviceSinkBuilder::open_default_sink()
        .map_err(|e| format!("failed to open audio output: {e}"))?;
    sink_handle.log_on_drop(false);
    let mixer = sink_handle.mixer();

    if !BACKGROUND_LOOP_OPUS.is_empty() {
        if let Ok(buf) = decode_opus_to_samples(BACKGROUND_LOOP_OPUS) {
            let source = buf.repeat_infinite();
            mixer.add(MuteableSource {
                source,
                muted: Arc::clone(&sound_muted),
            });
        }
    }

    let mut terminal = ratatui::init();
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        let mut events = Vec::new();
        if event::poll(Duration::from_millis(16))? {
            loop {
                let maybe_event = match event::read()? {
                    Event::Key(key) => map_key(key),
                    _ => None,
                };
                if let Some(input_event) = maybe_event {
                    events.push(input_event);
                }
                if !event::poll(Duration::from_millis(0))? {
                    break;
                }
            }
        }

        if !events.is_empty() {
            app.update(&events);
            sound_muted.store(app.game.sound_muted, Ordering::Relaxed);
            maybe_play_ding(&mixer, &mut app, &sound_muted);
        }

        let now = Instant::now();
        let delta_secs = now.duration_since(last_tick).as_secs_f64();
        last_tick = now;
        app.tick(delta_secs);
        sound_muted.store(app.game.sound_muted, Ordering::Relaxed);
        maybe_play_ding(&mixer, &mut app, &sound_muted);

        if app.should_quit {
            break;
        }
    }

    ratatui::restore();
    Ok(())
}

fn map_key(key: KeyEvent) -> Option<InputEvent> {
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputEvent::CtrlC)
        }
        KeyCode::Char(c) => Some(InputEvent::Char(c)),
        KeyCode::Backspace => Some(InputEvent::Backspace),
        KeyCode::Enter => Some(InputEvent::Enter),
        KeyCode::Up => Some(InputEvent::Up),
        KeyCode::Down => Some(InputEvent::Down),
        _ => None,
    }
}

#[cfg(feature = "dev-presets")]
fn parse_dev_tier_from_env_args() -> Result<Option<DevTier>, String> {
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == "--dev-tier" {
            let name = args
                .next()
                .ok_or_else(|| "missing value for --dev-tier".to_string())?;
            let tier = DevTier::from_str(&name).ok_or_else(|| {
                format!(
                    "unknown dev tier {name:?}. valid: {}",
                    DevTier::valid_names_csv()
                )
            })?;
            return Ok(Some(tier));
        }
    }
    Ok(None)
}
