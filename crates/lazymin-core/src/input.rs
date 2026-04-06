#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    Char(char),
    Backspace,
    Delete,
    Enter,
    Up,
    Down,
    Left,
    Right,
    CtrlA,
    CtrlC,
    ScrollUp { column: u16, row: u16 },
    ScrollDown { column: u16, row: u16 },
}
