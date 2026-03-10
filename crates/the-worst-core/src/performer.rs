use crate::cell::{AnsiColor, Attrs, TermColor};
use crate::grid::{EraseMode, TerminalGrid};

pub struct Performer<'a> {
    pub grid: &'a mut TerminalGrid,
    pub title_buf: &'a mut String,
    pub title_changed: &'a mut bool,
}

impl<'a> vte::Perform for Performer<'a> {
    fn print(&mut self, c: char) {
        self.grid.put_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x07 => {} // BEL
            0x08 => {
                // BS
                if self.grid.cursor_col > 0 {
                    self.grid.cursor_col -= 1;
                }
                self.grid.pending_wrap = false;
            }
            0x09 => {
                // HT
                self.grid.advance_tab();
            }
            0x0A | 0x0B | 0x0C => {
                // LF / VT / FF
                self.grid.index();
            }
            0x0D => {
                // CR
                self.grid.cursor_col = 0;
                self.grid.pending_wrap = false;
            }
            0x0E | 0x0F => {} // SO/SI charset — ignored
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let ps: Vec<u16> = params.iter().map(|s| s[0]).collect();
        let p = |i: usize| ps.get(i).copied().unwrap_or(0) as usize;
        let p1 = |i: usize| ps.get(i).copied().unwrap_or(1).max(1) as usize;

        match (intermediates, action) {
            // CUU — cursor up
            (b"", 'A') => {
                self.grid.cursor_row = self.grid.cursor_row.saturating_sub(p1(0));
                self.grid.pending_wrap = false;
            }
            // CUD — cursor down
            (b"", 'B') => {
                self.grid.cursor_row =
                    (self.grid.cursor_row + p1(0)).min(self.grid.rows - 1);
                self.grid.pending_wrap = false;
            }
            // CUF — cursor forward
            (b"", 'C') => {
                self.grid.cursor_col =
                    (self.grid.cursor_col + p1(0)).min(self.grid.cols - 1);
                self.grid.pending_wrap = false;
            }
            // CUB — cursor back
            (b"", 'D') => {
                self.grid.cursor_col = self.grid.cursor_col.saturating_sub(p1(0));
                self.grid.pending_wrap = false;
            }
            // CNL — cursor next line
            (b"", 'E') => {
                self.grid.cursor_row =
                    (self.grid.cursor_row + p1(0)).min(self.grid.rows - 1);
                self.grid.cursor_col = 0;
                self.grid.pending_wrap = false;
            }
            // CPL — cursor preceding line
            (b"", 'F') => {
                self.grid.cursor_row = self.grid.cursor_row.saturating_sub(p1(0));
                self.grid.cursor_col = 0;
                self.grid.pending_wrap = false;
            }
            // CHA — cursor horizontal absolute
            (b"", 'G') => {
                self.grid.cursor_col = p1(0).saturating_sub(1).min(self.grid.cols - 1);
                self.grid.pending_wrap = false;
            }
            // CUP / HVP — cursor position
            (b"", 'H') | (b"", 'f') => {
                let row = p1(0).saturating_sub(1);
                let col = p1(1).saturating_sub(1);
                self.grid.move_cursor(row, col);
            }
            // ED — erase in display
            (b"", 'J') => {
                let mode = match p(0) {
                    0 => EraseMode::ToEnd,
                    1 => EraseMode::ToStart,
                    2 | 3 => EraseMode::All,
                    _ => EraseMode::ToEnd,
                };
                self.grid.erase_in_display(mode);
            }
            // EL — erase in line
            (b"", 'K') => {
                let mode = match p(0) {
                    0 => EraseMode::ToEnd,
                    1 => EraseMode::ToStart,
                    2 => EraseMode::All,
                    _ => EraseMode::ToEnd,
                };
                self.grid.erase_in_line(mode);
            }
            // IL — insert lines
            (b"", 'L') => self.grid.insert_lines(p1(0)),
            // DL — delete lines
            (b"", 'M') => self.grid.delete_lines(p1(0)),
            // DCH — delete characters
            (b"", 'P') => self.grid.delete_chars(p1(0)),
            // SU — scroll up
            (b"", 'S') => self.grid.scroll_up(p1(0)),
            // SD — scroll down
            (b"", 'T') => self.grid.scroll_down(p1(0)),
            // ECH — erase characters
            (b"", 'X') => {
                let n = p1(0);
                let row = self.grid.cursor_row;
                let col = self.grid.cursor_col;
                let end = (col + n).min(self.grid.cols);
                let cols = self.grid.cols;
                let (fg, bg) = (self.grid.pen.fg, self.grid.pen.bg);
                for c in col..end {
                    self.grid.cells[row * cols + c] = crate::cell::Cell {
                        fg,
                        bg,
                        ..Default::default()
                    };
                }
            }
            // ICH — insert characters
            (b"", '@') => self.grid.insert_chars(p1(0)),
            // VPA — vertical line position absolute
            (b"", 'd') => {
                let row = p1(0).saturating_sub(1);
                self.grid.cursor_row = row.min(self.grid.rows - 1);
                self.grid.pending_wrap = false;
            }
            // TBC — tab clear
            (b"", 'g') => {
                if p(0) == 3 {
                    self.grid.tab_stops.iter_mut().for_each(|t| *t = false);
                } else {
                    let c = self.grid.cursor_col;
                    if c < self.grid.tab_stops.len() {
                        self.grid.tab_stops[c] = false;
                    }
                }
            }
            // SM / RM — set/reset mode
            (b"", 'h') => {
                for &p in &ps {
                    match p {
                        4 => self.grid.insert_mode = true,
                        _ => {}
                    }
                }
            }
            (b"", 'l') => {
                for &p in &ps {
                    match p {
                        4 => self.grid.insert_mode = false,
                        _ => {}
                    }
                }
            }
            // DECSET — private modes set
            (b"?", 'h') => {
                for &p in &ps {
                    match p {
                        1 => {}    // DECCKM: app cursor keys — TODO
                        7 => self.grid.auto_wrap = true,
                        12 => {}   // cursor blink — handled in renderer
                        25 => self.grid.cursor_visible = true,
                        47 | 1047 => self.grid.enter_alt_screen(),
                        1049 => {
                            self.grid.save_cursor();
                            self.grid.enter_alt_screen();
                        }
                        _ => {}
                    }
                }
            }
            // DECRST — private modes reset
            (b"?", 'l') => {
                for &p in &ps {
                    match p {
                        1 => {}    // DECCKM
                        7 => self.grid.auto_wrap = false,
                        25 => self.grid.cursor_visible = false,
                        47 | 1047 => self.grid.exit_alt_screen(),
                        1049 => {
                            self.grid.exit_alt_screen();
                            self.grid.restore_cursor();
                        }
                        _ => {}
                    }
                }
            }
            // SGR — select graphic rendition
            (b"", 'm') => {
                if ps.is_empty() {
                    // Reset all
                    self.grid.pen.fg = TermColor::Default;
                    self.grid.pen.bg = TermColor::Default;
                    self.grid.pen.attrs.reset();
                    return;
                }
                let mut i = 0;
                while i < ps.len() {
                    match ps[i] {
                        0 => {
                            self.grid.pen.fg = TermColor::Default;
                            self.grid.pen.bg = TermColor::Default;
                            self.grid.pen.attrs.reset();
                        }
                        1 => self.grid.pen.attrs.set(Attrs::BOLD, true),
                        2 => self.grid.pen.attrs.set(Attrs::DIM, true),
                        3 => self.grid.pen.attrs.set(Attrs::ITALIC, true),
                        4 => self.grid.pen.attrs.set(Attrs::UNDERLINE, true),
                        5 | 6 => self.grid.pen.attrs.set(Attrs::BLINK, true),
                        7 => self.grid.pen.attrs.set(Attrs::INVERSE, true),
                        8 => {} // conceal
                        9 => self.grid.pen.attrs.set(Attrs::STRIKEOUT, true),
                        22 => {
                            self.grid.pen.attrs.set(Attrs::BOLD, false);
                            self.grid.pen.attrs.set(Attrs::DIM, false);
                        }
                        23 => self.grid.pen.attrs.set(Attrs::ITALIC, false),
                        24 => self.grid.pen.attrs.set(Attrs::UNDERLINE, false),
                        25 => self.grid.pen.attrs.set(Attrs::BLINK, false),
                        27 => self.grid.pen.attrs.set(Attrs::INVERSE, false),
                        29 => self.grid.pen.attrs.set(Attrs::STRIKEOUT, false),
                        30..=37 => {
                            self.grid.pen.fg = TermColor::Named(idx_to_ansi(ps[i] - 30));
                        }
                        38 => {
                            if let Some(c) = parse_extended_color(&ps, &mut i) {
                                self.grid.pen.fg = c;
                            }
                        }
                        39 => self.grid.pen.fg = TermColor::Default,
                        40..=47 => {
                            self.grid.pen.bg = TermColor::Named(idx_to_ansi(ps[i] - 40));
                        }
                        48 => {
                            if let Some(c) = parse_extended_color(&ps, &mut i) {
                                self.grid.pen.bg = c;
                            }
                        }
                        49 => self.grid.pen.bg = TermColor::Default,
                        90..=97 => {
                            self.grid.pen.fg = TermColor::Named(idx_to_ansi(ps[i] - 90 + 8));
                        }
                        100..=107 => {
                            self.grid.pen.bg = TermColor::Named(idx_to_ansi(ps[i] - 100 + 8));
                        }
                        _ => {}
                    }
                    i += 1;
                }
            }
            // DECSTBM — set scroll region
            (b"", 'r') => {
                let top = p1(0).saturating_sub(1);
                let bot = if ps.get(1).copied().unwrap_or(0) == 0 {
                    self.grid.rows - 1
                } else {
                    (ps[1] as usize).saturating_sub(1).min(self.grid.rows - 1)
                };
                if top < bot {
                    self.grid.scroll_top = top;
                    self.grid.scroll_bottom = bot;
                    self.grid.move_cursor(0, 0);
                }
            }
            // DECCUSR — cursor shape (ignore)
            (b" ", 'q') => {}
            _ => {
                log::trace!(
                    "unhandled CSI: {:?} {:?} {:?}",
                    intermediates,
                    ps,
                    action
                );
            }
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        match (intermediates, byte) {
            (b"", b'7') => self.grid.save_cursor(),
            (b"", b'8') => self.grid.restore_cursor(),
            (b"", b'M') => self.grid.reverse_index(),
            (b"", b'D') => self.grid.index(),
            (b"", b'E') => {
                self.grid.index();
                self.grid.cursor_col = 0;
            }
            (b"", b'H') => {
                // HTS — set tab stop
                let c = self.grid.cursor_col;
                if c < self.grid.tab_stops.len() {
                    self.grid.tab_stops[c] = true;
                }
            }
            (b"", b'c') => {
                // RIS — reset to initial state
                let (cols, rows) = (self.grid.cols, self.grid.rows);
                *self.grid = TerminalGrid::new(cols, rows);
            }
            (b"(", _) | (b")", _) => {} // charset designation — ignore
            _ => {
                log::trace!("unhandled ESC: {:?} {:?}", intermediates, byte as char);
            }
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        match params {
            [b"0", title] | [b"2", title] => {
                if let Ok(s) = std::str::from_utf8(title) {
                    *self.title_buf = s.to_string();
                    *self.title_changed = true;
                }
            }
            _ => {}
        }
    }

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
}

fn idx_to_ansi(i: u16) -> AnsiColor {
    match i {
        0 => AnsiColor::Black,
        1 => AnsiColor::Red,
        2 => AnsiColor::Green,
        3 => AnsiColor::Yellow,
        4 => AnsiColor::Blue,
        5 => AnsiColor::Magenta,
        6 => AnsiColor::Cyan,
        7 => AnsiColor::White,
        8 => AnsiColor::BrightBlack,
        9 => AnsiColor::BrightRed,
        10 => AnsiColor::BrightGreen,
        11 => AnsiColor::BrightYellow,
        12 => AnsiColor::BrightBlue,
        13 => AnsiColor::BrightMagenta,
        14 => AnsiColor::BrightCyan,
        15 => AnsiColor::BrightWhite,
        _ => AnsiColor::White,
    }
}

fn parse_extended_color(ps: &[u16], i: &mut usize) -> Option<TermColor> {
    match ps.get(*i + 1).copied() {
        Some(2) => {
            // 24-bit: 38;2;r;g;b
            let r = ps.get(*i + 2).copied()? as u8;
            let g = ps.get(*i + 3).copied()? as u8;
            let b = ps.get(*i + 4).copied()? as u8;
            *i += 4;
            Some(TermColor::Rgb(r, g, b))
        }
        Some(5) => {
            // 256-color: 38;5;n
            let n = ps.get(*i + 2).copied()? as u8;
            *i += 2;
            Some(TermColor::Indexed(n))
        }
        _ => None,
    }
}
