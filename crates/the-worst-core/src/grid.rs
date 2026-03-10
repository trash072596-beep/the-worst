use std::collections::VecDeque;
use crate::cell::{Attrs, Cell, TermColor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EraseMode {
    ToEnd,
    ToStart,
    All,
}

pub struct TerminalGrid {
    pub cells: Vec<Cell>,
    pub cols: usize,
    pub rows: usize,

    // Cursor
    pub cursor_col: usize,
    pub cursor_row: usize,
    pub cursor_visible: bool,
    pub pending_wrap: bool,

    // Current drawing pen
    pub pen: Cell,

    // Scroll region (inclusive, 0-indexed)
    pub scroll_top: usize,
    pub scroll_bottom: usize,

    // DECAWM: auto-wrap
    pub auto_wrap: bool,

    // Alternate screen
    pub alt_cells: Option<Vec<Cell>>,
    pub alt_cursor: Option<(usize, usize)>,
    pub in_alt_screen: bool,

    // Saved cursor (DECSC/DECRC)
    pub saved_cursor: Option<(usize, usize, Cell)>,
    pub saved_cursor_alt: Option<(usize, usize, Cell)>,

    // Scrollback buffer
    pub scrollback: VecDeque<Vec<Cell>>,
    pub scrollback_limit: usize,
    pub scroll_offset: usize, // how many lines scrolled back (0 = bottom)

    // Tab stops
    pub tab_stops: Vec<bool>,

    // Window title
    pub title: String,

    // Origin mode (DECOM)
    pub origin_mode: bool,

    // Insert mode
    pub insert_mode: bool,
}

impl TerminalGrid {
    pub fn new(cols: usize, rows: usize) -> Self {
        let mut tab_stops = vec![false; cols];
        for i in (0..cols).step_by(8) {
            tab_stops[i] = true;
        }
        Self {
            cells: vec![Cell::default(); cols * rows],
            cols,
            rows,
            cursor_col: 0,
            cursor_row: 0,
            cursor_visible: true,
            pending_wrap: false,
            pen: Cell::default(),
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            auto_wrap: true,
            alt_cells: None,
            alt_cursor: None,
            in_alt_screen: false,
            saved_cursor: None,
            saved_cursor_alt: None,
            scrollback: VecDeque::new(),
            scrollback_limit: 10_000,
            scroll_offset: 0,
            tab_stops,
            title: "The-Worst".to_string(),
            origin_mode: false,
            insert_mode: false,
        }
    }

    pub fn resize(&mut self, new_cols: usize, new_rows: usize) {
        if new_cols == self.cols && new_rows == self.rows {
            return;
        }
        // Reflow: rebuild grid
        let mut new_cells = vec![Cell::default(); new_cols * new_rows];
        let copy_rows = self.rows.min(new_rows);
        let copy_cols = self.cols.min(new_cols);
        for r in 0..copy_rows {
            for c in 0..copy_cols {
                new_cells[r * new_cols + c] = self.cells[r * self.cols + c].clone();
            }
        }
        self.cells = new_cells;
        self.cols = new_cols;
        self.rows = new_rows;
        self.cursor_col = self.cursor_col.min(new_cols.saturating_sub(1));
        self.cursor_row = self.cursor_row.min(new_rows.saturating_sub(1));
        self.scroll_top = 0;
        self.scroll_bottom = new_rows.saturating_sub(1);
        self.pending_wrap = false;

        // Rebuild tab stops
        self.tab_stops = vec![false; new_cols];
        for i in (0..new_cols).step_by(8) {
            self.tab_stops[i] = true;
        }
    }

    pub fn cell(&self, row: usize, col: usize) -> &Cell {
        &self.cells[row * self.cols + col]
    }

    pub fn cell_mut(&mut self, row: usize, col: usize) -> &mut Cell {
        &mut self.cells[row * self.cols + col]
    }

    pub fn put_char(&mut self, ch: char) {
        use unicode_width::UnicodeWidthChar;
        let width = UnicodeWidthChar::width(ch).unwrap_or(1);

        if self.pending_wrap && self.auto_wrap {
            self.cursor_col = 0;
            self.cursor_row += 1;
            if self.cursor_row > self.scroll_bottom {
                self.cursor_row = self.scroll_bottom;
                self.scroll_up(1);
            }
            self.pending_wrap = false;
        }

        if self.cursor_col >= self.cols {
            self.cursor_col = self.cols - 1;
        }

        if self.insert_mode {
            // Shift cells right
            let row = self.cursor_row;
            let col = self.cursor_col;
            let cols = self.cols;
            for c in (col..cols.saturating_sub(1)).rev() {
                let src = self.cells[row * cols + c].clone();
                self.cells[row * cols + c + 1] = src;
            }
        }

        let mut cell = Cell {
            ch,
            fg: self.pen.fg,
            bg: self.pen.bg,
            attrs: self.pen.attrs,
        };

        if width == 2 {
            cell.attrs.set(Attrs::WIDE, true);
        }

        let row = self.cursor_row;
        let col = self.cursor_col;
        if row < self.rows && col < self.cols {
            self.cells[row * self.cols + col] = cell;
        }

        if width == 2 && col + 1 < self.cols {
            let mut cont = Cell::default();
            cont.attrs.set(Attrs::WIDE_CONT, true);
            cont.fg = self.pen.fg;
            cont.bg = self.pen.bg;
            self.cells[row * self.cols + col + 1] = cont;
        }

        let advance = width.max(1);
        if self.cursor_col + advance >= self.cols {
            self.pending_wrap = true;
        } else {
            self.cursor_col += advance;
        }
    }

    /// LF / IND: scroll up if at scroll bottom
    pub fn index(&mut self) {
        if self.cursor_row == self.scroll_bottom {
            self.scroll_up(1);
        } else if self.cursor_row < self.rows - 1 {
            self.cursor_row += 1;
        }
        self.pending_wrap = false;
    }

    /// RI: scroll down
    pub fn reverse_index(&mut self) {
        if self.cursor_row == self.scroll_top {
            self.scroll_down(1);
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
        }
        self.pending_wrap = false;
    }

    /// Scroll region up by n lines (content moves up, blank lines at bottom)
    pub fn scroll_up(&mut self, n: usize) {
        let top = self.scroll_top;
        let bot = self.scroll_bottom;
        let cols = self.cols;

        for _ in 0..n {
            if !self.in_alt_screen && top == 0 {
                // Push top row into scrollback
                let row: Vec<Cell> = self.cells[..cols].to_vec();
                self.scrollback.push_back(row);
                if self.scrollback.len() > self.scrollback_limit {
                    self.scrollback.pop_front();
                }
            }
            // Shift rows up
            for r in top..bot {
                for c in 0..cols {
                    self.cells[r * cols + c] = self.cells[(r + 1) * cols + c].clone();
                }
            }
            // Clear bottom row
            for c in 0..cols {
                self.cells[bot * cols + c] = Cell {
                    fg: self.pen.fg,
                    bg: self.pen.bg,
                    ..Cell::default()
                };
            }
        }
    }

    /// Scroll region down by n lines (content moves down, blank lines at top)
    pub fn scroll_down(&mut self, n: usize) {
        let top = self.scroll_top;
        let bot = self.scroll_bottom;
        let cols = self.cols;

        for _ in 0..n {
            for r in (top..bot).rev() {
                for c in 0..cols {
                    self.cells[(r + 1) * cols + c] = self.cells[r * cols + c].clone();
                }
            }
            for c in 0..cols {
                self.cells[top * cols + c] = Cell {
                    fg: self.pen.fg,
                    bg: self.pen.bg,
                    ..Cell::default()
                };
            }
        }
    }

    pub fn erase_in_display(&mut self, mode: EraseMode) {
        let (start, end) = match mode {
            EraseMode::ToEnd => {
                let s = self.cursor_row * self.cols + self.cursor_col;
                let e = self.rows * self.cols;
                (s, e)
            }
            EraseMode::ToStart => {
                let e = self.cursor_row * self.cols + self.cursor_col + 1;
                (0, e)
            }
            EraseMode::All => (0, self.rows * self.cols),
        };
        for i in start..end {
            self.cells[i] = Cell {
                fg: self.pen.fg,
                bg: self.pen.bg,
                ..Cell::default()
            };
        }
    }

    pub fn erase_in_line(&mut self, mode: EraseMode) {
        let row = self.cursor_row;
        let (start, end) = match mode {
            EraseMode::ToEnd => (self.cursor_col, self.cols),
            EraseMode::ToStart => (0, self.cursor_col + 1),
            EraseMode::All => (0, self.cols),
        };
        for c in start..end.min(self.cols) {
            self.cells[row * self.cols + c] = Cell {
                fg: self.pen.fg,
                bg: self.pen.bg,
                ..Cell::default()
            };
        }
    }

    pub fn delete_chars(&mut self, n: usize) {
        let row = self.cursor_row;
        let col = self.cursor_col;
        let cols = self.cols;
        let n = n.min(cols - col);
        for c in col..cols - n {
            self.cells[row * cols + c] = self.cells[row * cols + c + n].clone();
        }
        for c in cols - n..cols {
            self.cells[row * cols + c] = Cell {
                fg: self.pen.fg,
                bg: self.pen.bg,
                ..Cell::default()
            };
        }
    }

    pub fn insert_chars(&mut self, n: usize) {
        let row = self.cursor_row;
        let col = self.cursor_col;
        let cols = self.cols;
        let n = n.min(cols - col);
        for c in (col..cols - n).rev() {
            self.cells[row * cols + c + n] = self.cells[row * cols + c].clone();
        }
        for c in col..col + n {
            self.cells[row * cols + c] = Cell {
                fg: self.pen.fg,
                bg: self.pen.bg,
                ..Cell::default()
            };
        }
    }

    pub fn insert_lines(&mut self, n: usize) {
        let top = self.cursor_row;
        let bot = self.scroll_bottom;
        let cols = self.cols;
        let n = n.min(bot - top + 1);
        for _ in 0..n {
            for r in (top..bot).rev() {
                for c in 0..cols {
                    self.cells[(r + 1) * cols + c] = self.cells[r * cols + c].clone();
                }
            }
            for c in 0..cols {
                self.cells[top * cols + c] = Cell {
                    fg: self.pen.fg,
                    bg: self.pen.bg,
                    ..Cell::default()
                };
            }
        }
    }

    pub fn delete_lines(&mut self, n: usize) {
        let top = self.cursor_row;
        let bot = self.scroll_bottom;
        let cols = self.cols;
        let n = n.min(bot - top + 1);
        for _ in 0..n {
            for r in top..bot {
                for c in 0..cols {
                    self.cells[r * cols + c] = self.cells[(r + 1) * cols + c].clone();
                }
            }
            for c in 0..cols {
                self.cells[bot * cols + c] = Cell {
                    fg: self.pen.fg,
                    bg: self.pen.bg,
                    ..Cell::default()
                };
            }
        }
    }

    pub fn move_cursor(&mut self, row: usize, col: usize) {
        let top = if self.origin_mode { self.scroll_top } else { 0 };
        self.cursor_row = (top + row).min(self.rows - 1);
        self.cursor_col = col.min(self.cols - 1);
        self.pending_wrap = false;
    }

    pub fn set_pen_color_fg(&mut self, color: TermColor) {
        self.pen.fg = color;
    }
    pub fn set_pen_color_bg(&mut self, color: TermColor) {
        self.pen.bg = color;
    }

    pub fn enter_alt_screen(&mut self) {
        if !self.in_alt_screen {
            self.alt_cells = Some(self.cells.clone());
            self.alt_cursor = Some((self.cursor_row, self.cursor_col));
            self.cells = vec![Cell::default(); self.cols * self.rows];
            self.cursor_row = 0;
            self.cursor_col = 0;
            self.pending_wrap = false;
            self.in_alt_screen = true;
        }
    }

    pub fn exit_alt_screen(&mut self) {
        if self.in_alt_screen {
            if let Some(cells) = self.alt_cells.take() {
                self.cells = cells;
            }
            if let Some((r, c)) = self.alt_cursor.take() {
                self.cursor_row = r;
                self.cursor_col = c;
            }
            self.in_alt_screen = false;
            self.pending_wrap = false;
        }
    }

    pub fn save_cursor(&mut self) {
        if self.in_alt_screen {
            self.saved_cursor_alt = Some((self.cursor_row, self.cursor_col, self.pen.clone()));
        } else {
            self.saved_cursor = Some((self.cursor_row, self.cursor_col, self.pen.clone()));
        }
    }

    pub fn restore_cursor(&mut self) {
        let saved = if self.in_alt_screen {
            self.saved_cursor_alt.take()
        } else {
            self.saved_cursor.take()
        };
        if let Some((r, c, pen)) = saved {
            self.cursor_row = r.min(self.rows - 1);
            self.cursor_col = c.min(self.cols - 1);
            self.pen = pen;
            self.pending_wrap = false;
        }
    }

    pub fn advance_tab(&mut self) {
        let mut col = self.cursor_col + 1;
        while col < self.cols && !self.tab_stops[col] {
            col += 1;
        }
        self.cursor_col = col.min(self.cols - 1);
    }

    /// Get the visible cells (accounting for scrollback scroll offset).
    /// Returns cells either from scrollback+screen or just screen.
    pub fn visible_cells(&self) -> Vec<Cell> {
        if self.scroll_offset == 0 {
            return self.cells.clone();
        }
        let offset = self.scroll_offset.min(self.scrollback.len());
        let sb_start = self.scrollback.len().saturating_sub(offset);
        let sb_rows = offset.min(self.rows);
        let screen_rows = self.rows - sb_rows;

        let mut out = vec![Cell::default(); self.cols * self.rows];
        // Fill from scrollback
        for (i, r) in (sb_start..self.scrollback.len()).enumerate() {
            if i >= sb_rows { break; }
            let row = &self.scrollback[r];
            for c in 0..self.cols.min(row.len()) {
                out[i * self.cols + c] = row[c].clone();
            }
        }
        // Fill remaining from screen top
        for r in 0..screen_rows {
            for c in 0..self.cols {
                out[(sb_rows + r) * self.cols + c] = self.cells[r * self.cols + c].clone();
            }
        }
        out
    }
}
