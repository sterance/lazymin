use lazymin_core::input::InputEvent;

pub fn parse_xterm_data(s: &str) -> Vec<InputEvent> {
    let mut out = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            match chars.peek() {
                Some('[') => {
                    chars.next();
                    match chars.next() {
                        Some('A') => out.push(InputEvent::Up),
                        Some('B') => out.push(InputEvent::Down),
                        Some('C') | Some('D') => {}
                        Some('3') => if chars.next() == Some('~') {},
                        _ => {}
                    }
                }
                _ => {}
            }
            continue;
        }
        match c {
            '\r' | '\n' => out.push(InputEvent::Enter),
            '\u{7f}' | '\u{8}' => out.push(InputEvent::Backspace),
            '\u{3}' => out.push(InputEvent::CtrlC),
            c if !c.is_control() => out.push(InputEvent::Char(c)),
            _ => {}
        }
    }
    out
}
