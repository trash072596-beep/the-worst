/// ANSI named colors (indices 0–15)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AnsiColor {
    Black = 0,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermColor {
    Default,
    Named(AnsiColor),
    Indexed(u8),
    Rgb(u8, u8, u8),
}

impl Default for TermColor {
    fn default() -> Self {
        TermColor::Default
    }
}

/// SGR attribute flags packed into u16
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Attrs(pub u16);

impl Attrs {
    pub const BOLD: u16      = 1 << 0;
    pub const DIM: u16       = 1 << 1;
    pub const ITALIC: u16    = 1 << 2;
    pub const UNDERLINE: u16 = 1 << 3;
    pub const BLINK: u16     = 1 << 4;
    pub const INVERSE: u16   = 1 << 5;
    pub const STRIKEOUT: u16 = 1 << 6;
    pub const WIDE: u16      = 1 << 7;
    pub const WIDE_CONT: u16 = 1 << 8;

    pub fn set(&mut self, flag: u16, on: bool) {
        if on { self.0 |= flag; } else { self.0 &= !flag; }
    }
    pub fn get(self, flag: u16) -> bool {
        self.0 & flag != 0
    }
    pub fn reset(&mut self) {
        self.0 = 0;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: TermColor,
    pub bg: TermColor,
    pub attrs: Attrs,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: TermColor::Default,
            bg: TermColor::Default,
            attrs: Attrs::default(),
        }
    }
}

impl Cell {
    pub fn reset(&mut self) {
        *self = Cell::default();
    }
}
