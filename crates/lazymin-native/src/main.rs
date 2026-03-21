use std::error::Error;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use lazymin_core::app::App;
use lazymin_core::input::InputEvent;
use lazymin_core::ui;

#[cfg(feature = "dev-presets")]
use lazymin_core::game::dev_presets::{dev_game_state, DevTier};

type AppResult<T> = Result<T, Box<dyn Error>>;

fn main() -> AppResult<()> {
    #[cfg(feature = "dev-presets")]
    let mut app = match parse_dev_tier_from_env_args() {
        Ok(Some(tier)) => {
            eprintln!("dev preset active: {}", tier.as_str());
            App::with_game_state(dev_game_state(tier))
        }
        Ok(None) => App::new(),
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(1);
        }
    };
    #[cfg(not(feature = "dev-presets"))]
    let mut app = App::new();

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
        }

        let now = Instant::now();
        let delta_secs = now.duration_since(last_tick).as_secs_f64();
        last_tick = now;
        app.tick(delta_secs);

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
