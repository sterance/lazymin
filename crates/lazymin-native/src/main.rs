use std::error::Error;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use lazymin_core::app::App;
use lazymin_core::input::InputEvent;
use lazymin_core::ui;

type AppResult<T> = Result<T, Box<dyn Error>>;

fn main() -> AppResult<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new();

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
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(InputEvent::CtrlL)
        }
        KeyCode::Char(c) => Some(InputEvent::Char(c)),
        KeyCode::Backspace => Some(InputEvent::Backspace),
        KeyCode::Enter => Some(InputEvent::Enter),
        KeyCode::Up => Some(InputEvent::Up),
        KeyCode::Down => Some(InputEvent::Down),
        KeyCode::Tab => Some(InputEvent::Tab),
        _ => None,
    }
}
