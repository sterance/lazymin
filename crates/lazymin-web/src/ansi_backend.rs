use std::fmt::Display;
use std::io::{self, Write as _};
use std::mem;

use anes::{ResetAttributes, SetAttribute, SetBackgroundColor, SetForegroundColor};
use ratatui::backend::{Backend, ClearType, WindowSize};
use ratatui::buffer::Cell;
use ratatui::layout::{Position, Size};
use ratatui::style::{Color, Modifier};
use wasm_bindgen::JsValue;

pub struct AnsiBackendOptions {
    pub get_size: js_sys::Function,
    pub write: js_sys::Function,
}

pub struct AnsiBackend {
    get_size: js_sys::Function,
    write: js_sys::Function,
    pos: Option<Position>,
    buf: Vec<u8>,
}

impl AnsiBackend {
    pub fn new(options: AnsiBackendOptions) -> Self {
        let AnsiBackendOptions { get_size, write } = options;
        Self {
            get_size,
            write,
            pos: None,
            buf: Vec::new(),
        }
    }

    pub fn exclusive(&mut self) -> io::Result<()> {
        self.push(anes::SwitchBufferToAlternate)?;
        self.set_cursor_position(Position { x: 0, y: 0 })?;
        self.clear()
    }

    #[allow(dead_code)]
    pub fn normal(&mut self) -> io::Result<()> {
        self.set_cursor_position(Position { x: 0, y: 0 })?;
        self.clear()?;
        self.push(anes::SwitchBufferToNormal)
    }

    fn apply_modifiers(&mut self, old: &mut Option<Modifier>, new: &Modifier) -> io::Result<()> {
        if let Some(prev) = old {
            if prev == new {
                return Ok(());
            }
        }

        let prev = match old {
            Some(prev) => prev.clone(),
            None => {
                self.push(ResetAttributes)?;
                Modifier::empty()
            }
        };

        let to_set = *new - prev;
        let to_del = prev - *new;

        use anes::Attribute as AA;
        use SetAttribute as Set;

        if to_set.contains(Modifier::BOLD) {
            self.push(Set(AA::Bold))?;
        } else if to_set.contains(Modifier::DIM) {
            self.push(Set(AA::Faint))?;
        } else if to_del.contains(Modifier::DIM) || to_del.contains(Modifier::BOLD) {
            self.push(Set(AA::Normal))?;
        }

        if to_set.contains(Modifier::CROSSED_OUT) {
            self.push(Set(AA::Crossed))?;
        } else if to_del.contains(Modifier::CROSSED_OUT) {
            self.push(Set(AA::CrossedOff))?;
        }

        if to_set.contains(Modifier::HIDDEN) {
            self.push(Set(AA::Conceal))?;
        } else if to_del.contains(Modifier::HIDDEN) {
            self.push(Set(AA::ConcealOff))?;
        }

        if to_set.contains(Modifier::ITALIC) {
            self.push(Set(AA::Italic))?;
        } else if to_del.contains(Modifier::ITALIC) {
            self.push(Set(AA::ItalicOff))?;
        }

        if to_set.contains(Modifier::RAPID_BLINK) || to_set.contains(Modifier::SLOW_BLINK) {
            self.push(Set(AA::Blink))?;
        } else if to_del.contains(Modifier::RAPID_BLINK) || to_del.contains(Modifier::SLOW_BLINK) {
            self.push(Set(AA::BlinkOff))?;
        }

        if to_set.contains(Modifier::REVERSED) {
            self.push(Set(AA::Reverse))?;
        } else if to_del.contains(Modifier::REVERSED) {
            self.push(Set(AA::ReverseOff))?;
        }

        if to_set.contains(Modifier::UNDERLINED) {
            self.push(Set(AA::Underline))?;
        } else if to_del.contains(Modifier::UNDERLINED) {
            self.push(Set(AA::UnderlineOff))?;
        }

        *old = Some(*new);
        Ok(())
    }

    fn push(&mut self, ansi: impl Display) -> io::Result<()> {
        write!(self.buf, "{}", ansi)
    }
}

impl Backend for AnsiBackend {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, content: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        let mut prev_pos: Option<Position> = None;
        let mut prev_mod: Option<Modifier> = None;
        let mut prev_fg: Option<Color> = None;
        let mut prev_bg: Option<Color> = None;

        for (x, y, cell) in content {
            if cell.skip {
                continue;
            }

            let mut new_pos = Position { x, y };
            if let Some(prev) = prev_pos {
                if new_pos != prev {
                    self.set_cursor_position(new_pos)?;
                }
            } else {
                self.set_cursor_position(new_pos)?;
            }

            self.apply_modifiers(&mut prev_mod, &cell.modifier)?;

            if prev_bg != Some(cell.bg) {
                self.push(SetBackgroundColor(ansi_color(cell.bg)))?;
                prev_bg = Some(cell.bg);
            }

            if prev_fg != Some(cell.fg) {
                self.push(SetForegroundColor(ansi_color(cell.fg)))?;
                prev_fg = Some(cell.fg);
            }

            self.buf.extend_from_slice(cell.symbol().as_bytes());
            let width = 1u16;
            new_pos.x = new_pos.x.saturating_add(width);
            prev_pos = Some(new_pos);
        }
        self.flush()?;
        if let Some(pos) = prev_pos {
            self.pos = Some(pos);
        }

        Ok(())
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        write!(self.buf, "{}", anes::HideCursor)
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        write!(self.buf, "{}", anes::ShowCursor)
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        let pos = match self.pos {
            Some(pos) => pos,
            None => {
                let new_pos = Position { x: 0, y: 0 };
                self.set_cursor_position(new_pos)?;
                self.pos = Some(new_pos);
                new_pos
            }
        };
        Ok(pos)
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, new_pos_into: P) -> io::Result<()> {
        let new_pos: Position = new_pos_into.into();
        if Some(new_pos) == self.pos {
            return Ok(());
        }
        self.push(anes::MoveCursorTo(new_pos.x + 1, new_pos.y + 1))?;
        self.pos = Some(new_pos);
        Ok(())
    }

    fn clear(&mut self) -> io::Result<()> {
        self.push(ResetAttributes)?;
        self.push(anes::ClearBuffer::All)
    }

    fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
        use anes::{ClearBuffer, ClearLine};
        match clear_type {
            ClearType::All => self.clear(),
            ClearType::AfterCursor => self.push(ClearBuffer::Below),
            ClearType::BeforeCursor => self.push(ClearBuffer::Above),
            ClearType::CurrentLine => self.push(ClearLine::All),
            ClearType::UntilNewLine => self.push(ClearLine::Right),
        }
    }

    fn size(&self) -> io::Result<Size> {
        let v = self
            .get_size
            .call0(&JsValue::NULL)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "terminal size callback failed"))?;
        size_from_js_value(v)
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        Ok(WindowSize {
            columns_rows: self.size()?,
            pixels: Size::default(),
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.buf.is_empty() {
            return Ok(());
        }
        let bytes = mem::take(&mut self.buf);
        let arr = js_sys::Uint8Array::from(bytes.as_slice());
        self.write
            .call1(&JsValue::NULL, &arr.into())
            .map_err(|err| {
                web_sys::console::error_1(&err);
                io::Error::new(io::ErrorKind::Other, "stdout writer failed")
            })?;
        Ok(())
    }
}

fn size_from_js_value(v: JsValue) -> io::Result<Size> {
    let cols = js_sys::Reflect::get(&v, &JsValue::from_str("columns"))
        .ok()
        .and_then(|c| c.as_f64())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "columns"))?
        as u16;
    let rows = js_sys::Reflect::get(&v, &JsValue::from_str("rows"))
        .ok()
        .and_then(|r| r.as_f64())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "rows"))? as u16;
    Ok(Size {
        width: cols,
        height: rows,
    })
}

fn ansi_color(color: Color) -> anes::Color {
    use anes::Color as AColor;
    use Color as RColor;

    match color {
        RColor::Reset => AColor::Default,
        RColor::Black => AColor::Black,
        RColor::Red => AColor::DarkRed,
        RColor::Green => AColor::DarkGreen,
        RColor::Yellow => AColor::DarkYellow,
        RColor::Blue => AColor::DarkBlue,
        RColor::Magenta => AColor::DarkMagenta,
        RColor::Cyan => AColor::DarkCyan,
        RColor::Gray => AColor::DarkGray,
        RColor::DarkGray => AColor::DarkGray,
        RColor::LightRed => AColor::Red,
        RColor::LightGreen => AColor::Green,
        RColor::LightYellow => AColor::Yellow,
        RColor::LightBlue => AColor::Blue,
        RColor::LightMagenta => AColor::Magenta,
        RColor::LightCyan => AColor::Cyan,
        RColor::White => AColor::White,
        RColor::Rgb(r, g, b) => AColor::Rgb(r, g, b),
        RColor::Indexed(code) => AColor::Ansi(code),
    }
}
