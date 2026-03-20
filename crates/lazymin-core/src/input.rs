#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    Char(char),
    Backspace,
    Enter,
    Up,
    Down,
    Tab,
    CtrlC,
    CtrlL,
}
