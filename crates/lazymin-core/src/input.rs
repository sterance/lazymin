#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    Char(char),
    Backspace,
    Enter,
    Up,
    Down,
    CtrlC,
    ScrollUp { column: u16, row: u16 },
    ScrollDown { column: u16, row: u16 },
}
