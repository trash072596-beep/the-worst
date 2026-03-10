use std::time::{Duration, Instant};
use crossbeam_channel::{Sender, Receiver};
use egui::{Color32, Context, FontId, Key, Pos2, Sense, Stroke, Vec2};
use the_worst_core::pty::PtyEvent;

use crate::colors;
use crate::keyboard;
use crate::renderer::{self, RenderMetrics};
use crate::tab::Tab;

const FONT_SIZE: f32 = 14.5;
const BLINK_PERIOD: Duration = Duration::from_millis(530);
const MENU_H: f32 = 22.0;
const TAB_H: f32 = 28.0;
const STATUS_H: f32 = 18.0;

pub struct TheWorstApp {
    tabs: Vec<Tab>,
    active: usize,

    metrics: Option<RenderMetrics>,
    cols: usize,
    rows: usize,

    font_size: f32,

    blink_start: Instant,
    blink_on: bool,

    repaint_tx: Sender<PtyEvent>,
    repaint_rx: Receiver<PtyEvent>,
    egui_ctx: Option<Context>,
}

impl TheWorstApp {
    pub fn new() -> Self {
        let (repaint_tx, repaint_rx) = crossbeam_channel::bounded::<PtyEvent>(64);
        let tab = Tab::new(220, 50, repaint_tx.clone());
        Self {
            tabs: vec![tab],
            active: 0,
            metrics: None,
            cols: 220,
            rows: 50,
            font_size: FONT_SIZE,
            blink_start: Instant::now(),
            blink_on: true,
            repaint_tx,
            repaint_rx,
            egui_ctx: None,
        }
    }

    fn add_tab(&mut self) {
        let tab = Tab::new(self.cols as u16, self.rows as u16, self.repaint_tx.clone());
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
    }

    fn close_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.tabs.remove(self.active);
            if self.active >= self.tabs.len() {
                self.active = self.tabs.len() - 1;
            }
        }
    }

    fn send_active(&self, bytes: Vec<u8>) {
        if let Some(tab) = self.tabs.get(self.active) {
            tab.grid().lock().scroll_offset = 0;
            tab.send_input(bytes);
        }
    }

    fn reset_active(&self) {
        if let Some(tab) = self.tabs.get(self.active) {
            // Send 'reset' command
            tab.send_input(b"reset\r".to_vec());
        }
    }

    fn clear_active(&self) {
        // Ctrl+L
        self.send_active(vec![0x0C]);
    }

    fn copy_selection(&self, ctx: &Context) {
        // TODO: selection not yet implemented — placeholder
        let _ = ctx;
    }

    fn paste_clipboard(&self, ctx: &Context) {
        ctx.output_mut(|o| {
            // egui clipboard read not directly available; send via input
        });
        // Use arboard or just send Ctrl+Shift+V to the shell
        // For now send bracketed paste via xdotool / xclip isn't available here
        // We'll just signal the user
    }

    fn zoom_in(&mut self) {
        self.font_size = (self.font_size + 1.0).min(32.0);
        self.metrics = None; // recompute next frame
    }

    fn zoom_out(&mut self) {
        self.font_size = (self.font_size - 1.0).max(8.0);
        self.metrics = None;
    }

    fn zoom_reset(&mut self) {
        self.font_size = FONT_SIZE;
        self.metrics = None;
    }
}

impl eframe::App for TheWorstApp {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        // Repaint relay thread (once)
        if self.egui_ctx.is_none() {
            self.egui_ctx = Some(ctx.clone());
            let rx = self.repaint_rx.clone();
            let ctx2 = ctx.clone();
            std::thread::Builder::new()
                .name("repaint-relay".into())
                .spawn(move || {
                    loop {
                        match rx.recv() {
                            Ok(_) => ctx2.request_repaint(),
                            Err(_) => break,
                        }
                    }
                })
                .ok();
        }

        if self.metrics.is_none() {
            self.metrics = Some(RenderMetrics::compute(ctx, self.font_size));
        }

        // Poll tabs
        for tab in &mut self.tabs { tab.poll(); }

        // Cursor blink
        let blink_phase = (self.blink_start.elapsed().as_millis() / BLINK_PERIOD.as_millis()) % 2;
        self.blink_on = blink_phase == 0;
        ctx.request_repaint_after(Duration::from_millis(33)); // ~30fps for wave

        // Hotkeys
        let new_tab     = ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(Key::T));
        let close_tab   = ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(Key::W));
        let switch_next = ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::PageDown));
        let switch_prev = ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::PageUp));
        let zoom_in     = ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Plus));
        let zoom_out    = ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Minus));
        let zoom_reset  = ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Num0));

        if new_tab    { self.add_tab(); }
        if close_tab  { self.close_tab(); }
        if switch_next && self.tabs.len() > 1 { self.active = (self.active + 1) % self.tabs.len(); }
        if switch_prev && self.tabs.len() > 1 { self.active = (self.active + self.tabs.len() - 1) % self.tabs.len(); }
        if zoom_in    { self.zoom_in(); }
        if zoom_out   { self.zoom_out(); }
        if zoom_reset { self.zoom_reset(); }

        // Scrollback
        let scroll_up    = ctx.input(|i| i.modifiers.shift && i.key_pressed(Key::PageUp));
        let scroll_down  = ctx.input(|i| i.modifiers.shift && i.key_pressed(Key::PageDown));
        let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);

        if let Some(tab) = self.tabs.get(self.active) {
            let arc = tab.grid();
            let mut g = arc.lock();
            if scroll_up   { g.scroll_offset = (g.scroll_offset + self.rows / 2).min(g.scrollback.len()); }
            if scroll_down { g.scroll_offset = g.scroll_offset.saturating_sub(self.rows / 2); }
            if scroll_delta.abs() > 1.0 {
                let lines = (scroll_delta.abs() / 20.0).round() as usize + 1;
                if scroll_delta > 0.0 { g.scroll_offset = (g.scroll_offset + lines).min(g.scrollback.len()); }
                else { g.scroll_offset = g.scroll_offset.saturating_sub(lines); }
            }
        }

        // Keyboard input
        let input_bytes: Vec<Vec<u8>> = ctx.input(|i| {
            let mut out = vec![];
            for ev in &i.events {
                match ev {
                    egui::Event::Text(s) if !s.is_empty() => out.push(s.as_bytes().to_vec()),
                    egui::Event::Key { key, pressed: true, modifiers, .. } => {
                        if let Some(b) = keyboard::key_to_bytes(*key, *modifiers) { out.push(b); }
                    }
                    _ => {}
                }
            }
            out
        });
        for bytes in input_bytes { self.send_active(bytes); }

        // ── Menu bar ─────────────────────────────────────────────────────────
        // Collect actions from menus before borrowing self mutably below
        let mut do_new_tab    = false;
        let mut do_close_tab  = false;
        let mut do_quit       = false;
        let mut do_clear      = false;
        let mut do_reset      = false;
        let mut do_zoom_in    = false;
        let mut do_zoom_out   = false;
        let mut do_zoom_reset = false;
        let mut do_scroll_top = false;
        let mut do_scroll_bot = false;

        egui::TopBottomPanel::top("menu_bar")
            .exact_height(MENU_H)
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(5, 6, 12))
                    .inner_margin(egui::Margin::symmetric(0, 0)),
            )
            .show(ctx, |ui| {
                // Bottom hairline
                let r = ui.max_rect();
                ui.painter().line_segment(
                    [Pos2::new(r.left(), r.bottom()), Pos2::new(r.right(), r.bottom())],
                    Stroke::new(1.0, Color32::from_rgba_premultiplied(0, 180, 240, 25)),
                );

                egui::menu::bar(ui, |ui| {
                    ui.visuals_mut().widgets.inactive.bg_fill = Color32::TRANSPARENT;
                    ui.visuals_mut().widgets.hovered.bg_fill =
                        Color32::from_rgba_premultiplied(0, 180, 240, 18);
                    ui.visuals_mut().widgets.active.bg_fill =
                        Color32::from_rgba_premultiplied(0, 180, 240, 30);

                    let label = |text: &str| {
                        egui::RichText::new(text)
                            .font(FontId::proportional(11.5))
                            .color(Color32::from_rgb(140, 160, 190))
                    };
                    let item = |text: &str, hint: &str| -> egui::RichText {
                        egui::RichText::new(format!("{:<22}{}", text, hint))
                            .font(FontId::monospace(11.0))
                            .color(Color32::from_rgb(180, 200, 220))
                    };

                    // ── File ──
                    ui.menu_button(label("File"), |ui| {
                        ui.set_min_width(220.0);
                        if ui.button(item("New Tab", "Ctrl+Shift+T")).clicked() {
                            do_new_tab = true; ui.close_menu();
                        }
                        if ui.button(item("Close Tab", "Ctrl+Shift+W")).clicked() {
                            do_close_tab = true; ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(item("Quit", "Ctrl+Q")).clicked() {
                            do_quit = true; ui.close_menu();
                        }
                    });

                    // ── Edit ──
                    ui.menu_button(label("Edit"), |ui| {
                        ui.set_min_width(220.0);
                        if ui.button(item("Copy", "Ctrl+Shift+C")).clicked() {
                            ui.close_menu();
                        }
                        if ui.button(item("Paste", "Ctrl+Shift+V")).clicked() {
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(item("Clear", "Ctrl+L")).clicked() {
                            do_clear = true; ui.close_menu();
                        }
                        if ui.button(item("Reset Terminal", "")).clicked() {
                            do_reset = true; ui.close_menu();
                        }
                    });

                    // ── View ──
                    ui.menu_button(label("View"), |ui| {
                        ui.set_min_width(220.0);
                        if ui.button(item("Zoom In", "Ctrl++")).clicked() {
                            do_zoom_in = true; ui.close_menu();
                        }
                        if ui.button(item("Zoom Out", "Ctrl+-")).clicked() {
                            do_zoom_out = true; ui.close_menu();
                        }
                        if ui.button(item("Reset Zoom", "Ctrl+0")).clicked() {
                            do_zoom_reset = true; ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(item("Scroll to Top", "")).clicked() {
                            do_scroll_top = true; ui.close_menu();
                        }
                        if ui.button(item("Scroll to Bottom", "")).clicked() {
                            do_scroll_bot = true; ui.close_menu();
                        }
                    });

                    // ── Tabs ──
                    ui.menu_button(label("Tabs"), |ui| {
                        ui.set_min_width(180.0);
                        if ui.button(item("New Tab", "Ctrl+Shift+T")).clicked() {
                            do_new_tab = true; ui.close_menu();
                        }
                        if ui.button(item("Next Tab", "Ctrl+PgDn")).clicked() {
                            if self.tabs.len() > 1 { self.active = (self.active + 1) % self.tabs.len(); }
                            ui.close_menu();
                        }
                        if ui.button(item("Prev Tab", "Ctrl+PgUp")).clicked() {
                            if self.tabs.len() > 1 { self.active = (self.active + self.tabs.len() - 1) % self.tabs.len(); }
                            ui.close_menu();
                        }
                        ui.separator();
                        for i in 0..self.tabs.len() {
                            let t = self.tabs[i].title.clone();
                            let label_t = egui::RichText::new(format!("{}  {}", i + 1, t))
                                .font(FontId::monospace(11.0))
                                .color(if i == self.active {
                                    colors::ACCENT
                                } else {
                                    Color32::from_rgb(140, 160, 180)
                                });
                            if ui.button(label_t).clicked() {
                                self.active = i; ui.close_menu();
                            }
                        }
                    });

                    // ── Help ──
                    ui.menu_button(label("Help"), |ui| {
                        ui.set_min_width(220.0);
                        ui.label(
                            egui::RichText::new("The_Worst Terminal")
                                .font(FontId::proportional(11.0))
                                .color(colors::ACCENT),
                        );
                        ui.label(
                            egui::RichText::new("Pure Rust · egui · portable-pty · vte")
                                .font(FontId::proportional(10.0))
                                .color(Color32::from_rgb(80, 100, 130)),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new(
                                "Ctrl+Shift+T  New tab\n\
                                 Ctrl+Shift+W  Close tab\n\
                                 Ctrl+PgUp/Dn  Switch tabs\n\
                                 Shift+PgUp/Dn Scrollback\n\
                                 Ctrl++/-/0    Zoom"
                            )
                            .font(FontId::monospace(10.0))
                            .color(Color32::from_rgb(100, 120, 150)),
                        );
                    });

                    // Font size indicator on the right
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        if self.font_size != FONT_SIZE {
                            ui.label(
                                egui::RichText::new(format!("{:.0}px", self.font_size))
                                    .font(FontId::monospace(10.0))
                                    .color(Color32::from_rgb(60, 120, 160)),
                            );
                        }
                    });
                });
            });

        // Apply menu actions
        if do_new_tab    { self.add_tab(); }
        if do_close_tab  { self.close_tab(); }
        if do_quit       { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
        if do_clear      { self.clear_active(); }
        if do_reset      { self.reset_active(); }
        if do_zoom_in    { self.zoom_in(); }
        if do_zoom_out   { self.zoom_out(); }
        if do_zoom_reset { self.zoom_reset(); }
        if do_scroll_top {
            if let Some(tab) = self.tabs.get(self.active) {
                let arc = tab.grid(); arc.lock().scroll_offset = arc.lock().scrollback.len();
            }
        }
        if do_scroll_bot {
            if let Some(tab) = self.tabs.get(self.active) {
                tab.grid().lock().scroll_offset = 0;
            }
        }

        // ── Tab bar ──────────────────────────────────────────────────────────
        egui::TopBottomPanel::top("tab_bar")
            .exact_height(TAB_H)
            .frame(egui::Frame::none().fill(colors::TAB_BG))
            .show(ctx, |ui| {
                let r = ui.max_rect();
                ui.painter().line_segment(
                    [Pos2::new(r.left(), r.bottom() - 1.0), Pos2::new(r.right(), r.bottom() - 1.0)],
                    Stroke::new(1.0, Color32::from_rgba_premultiplied(0, 180, 240, 40)),
                );

                ui.horizontal_centered(|ui| {
                    ui.add_space(8.0);

                    // Animated pixel logo
                    let px = 2.0_f32;
                    let logo_w = crate::pixel_logo::width(px);
                    let logo_h = crate::pixel_logo::height(px);
                    let (logo_rect, _) = ui.allocate_exact_size(
                        Vec2::new(logo_w, TAB_H),
                        Sense::focusable_noninteractive(),
                    );
                    let t = self.blink_start.elapsed().as_secs_f32();
                    let origin = Pos2::new(
                        logo_rect.left(),
                        logo_rect.center().y - logo_h * 0.5 + 1.0,
                    );
                    crate::pixel_logo::draw(&ui.painter_at(logo_rect), origin, px, t);

                    ui.add_space(10.0);

                    let tab_count = self.tabs.len();
                    for i in 0..tab_count {
                        let title = self.tabs[i].title.clone();
                        let exited = self.tabs[i].exited;
                        let is_active = i == self.active;

                        let text_color = if is_active {
                            Color32::from_rgb(220, 235, 255)
                        } else if exited {
                            Color32::from_rgb(100, 60, 60)
                        } else {
                            colors::TAB_DIM
                        };

                        let display = if exited { format!("{} ✕", i + 1) } else { format!("{}", i + 1) };

                        let response = ui.add(
                            egui::Button::new(
                                egui::RichText::new(&display)
                                    .font(FontId::proportional(12.5))
                                    .color(text_color),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE)
                            .min_size(Vec2::new(0.0, TAB_H - 4.0)),
                        );

                        if is_active {
                            let rb = response.rect;
                            ui.painter().line_segment(
                                [Pos2::new(rb.left() + 2.0, rb.bottom() + 1.0),
                                 Pos2::new(rb.right() - 2.0, rb.bottom() + 1.0)],
                                Stroke::new(2.0, colors::ACCENT),
                            );
                        }

                        if response.clicked() { self.active = i; }
                        ui.add_space(2.0);
                    }

                    ui.add_space(4.0);
                    let plus = ui.add(
                        egui::Button::new(
                            egui::RichText::new("+").font(FontId::proportional(15.0)).color(colors::TAB_DIM),
                        )
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE)
                        .min_size(Vec2::new(24.0, TAB_H - 4.0)),
                    );
                    if plus.hovered() {
                        ui.painter().rect_filled(plus.rect, 3.0, Color32::from_rgba_premultiplied(0, 180, 240, 18));
                    }
                    if plus.clicked() { self.add_tab(); }
                });
            });

        // ── Status bar ───────────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(STATUS_H)
            .frame(egui::Frame::none().fill(colors::TAB_BG))
            .show(ctx, |ui| {
                let r = ui.max_rect();
                ui.painter().line_segment(
                    [Pos2::new(r.left(), r.top()), Pos2::new(r.right(), r.top())],
                    Stroke::new(1.0, Color32::from_rgba_premultiplied(0, 180, 240, 30)),
                );

                ui.horizontal_centered(|ui| {
                    ui.add_space(8.0);

                    let (cur_row, cur_col, scroll_off, title) =
                        if let Some(tab) = self.tabs.get(self.active) {
                            let arc = tab.grid();
                            let g = arc.lock();
                            (g.cursor_row + 1, g.cursor_col + 1, g.scroll_offset, tab.title.clone())
                        } else { (0, 0, 0, String::new()) };

                    ui.label(
                        egui::RichText::new(&title)
                            .font(FontId::proportional(10.5))
                            .color(Color32::from_rgb(80, 100, 140)),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "{}×{}  {}:{}{}",
                                self.cols, self.rows, cur_row, cur_col,
                                if scroll_off > 0 { format!("  ↑{}", scroll_off) } else { String::new() }
                            ))
                            .font(FontId::monospace(10.0))
                            .color(Color32::from_rgb(50, 75, 110)),
                        );
                    });
                });
            });

        // ── Terminal area ────────────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(colors::BG))
            .show(ctx, |ui| {
                let metrics = self.metrics.clone().unwrap();
                let avail = ui.available_size();

                let new_cols = (avail.x / metrics.cell_w).floor() as usize;
                let new_rows = (avail.y / metrics.cell_h).floor() as usize;

                if new_cols > 0 && new_rows > 0 && (new_cols != self.cols || new_rows != self.rows) {
                    self.cols = new_cols;
                    self.rows = new_rows;
                    for tab in &self.tabs {
                        tab.grid().lock().resize(new_cols, new_rows);
                        tab.send_resize(new_cols as u16, new_rows as u16);
                    }
                }

                let cols = self.cols;
                let rows = self.rows;

                let (rect, resp) = ui.allocate_exact_size(
                    Vec2::new(cols as f32 * metrics.cell_w, rows as f32 * metrics.cell_h),
                    Sense::click(),
                );

                ui.memory_mut(|m| m.request_focus(ui.id()));

                // ── Right-click context menu ──────────────────────────────
                resp.context_menu(|ui| {
                    ui.set_min_width(180.0);

                    // Style the popup
                    ui.visuals_mut().widgets.inactive.bg_fill = Color32::TRANSPARENT;
                    ui.visuals_mut().widgets.hovered.bg_fill =
                        Color32::from_rgba_premultiplied(0, 180, 240, 22);

                    let item = |text: &str, hint: &str| {
                        egui::RichText::new(format!("{:<18}{}", text, hint))
                            .font(FontId::monospace(11.5))
                            .color(Color32::from_rgb(180, 205, 230))
                    };

                    if ui.button(item("Copy", "Ctrl+Shift+C")).clicked() {
                        // Ctrl+Shift+C — let xterm handle it, or future selection
                        ui.close_menu();
                    }
                    if ui.button(item("Paste", "Ctrl+Shift+V")).clicked() {
                        // Read system clipboard and send to PTY
                        if let Ok(mut cb) = arboard::Clipboard::new() {
                            if let Ok(text) = cb.get_text() {
                                self.send_active(text.into_bytes());
                            }
                        }
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button(item("Clear", "Ctrl+L")).clicked() {
                        self.clear_active();
                        ui.close_menu();
                    }
                    if ui.button(item("Reset Terminal", "")).clicked() {
                        self.reset_active();
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button(item("New Tab", "Ctrl+Shift+T")).clicked() {
                        self.add_tab();
                        ui.close_menu();
                    }
                    if ui.button(item("Close Tab", "Ctrl+Shift+W")).clicked() {
                        self.close_tab();
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button(item("Zoom In", "Ctrl++")).clicked() {
                        self.zoom_in(); ui.close_menu();
                    }
                    if ui.button(item("Zoom Out", "Ctrl+-")).clicked() {
                        self.zoom_out(); ui.close_menu();
                    }
                    if ui.button(item("Reset Zoom", "Ctrl+0")).clicked() {
                        self.zoom_reset(); ui.close_menu();
                    }

                    ui.separator();

                    if ui.button(item("Scroll to Top", "")).clicked() {
                        if let Some(tab) = self.tabs.get(self.active) {
                            let arc = tab.grid();
                            let len = arc.lock().scrollback.len();
                            arc.lock().scroll_offset = len;
                        }
                        ui.close_menu();
                    }
                    if ui.button(item("Scroll to Bottom", "")).clicked() {
                        if let Some(tab) = self.tabs.get(self.active) {
                            tab.grid().lock().scroll_offset = 0;
                        }
                        ui.close_menu();
                    }
                });

                if let Some(tab) = self.tabs.get(self.active) {
                    let (cells, cursor_row, cursor_col, cursor_visible) = {
                        let arc = tab.grid();
                        let g = arc.lock();
                        (g.visible_cells(), g.cursor_row, g.cursor_col, g.cursor_visible)
                    };

                    renderer::draw_grid(
                        &ui.painter_at(rect),
                        &cells, cols, rows, &metrics, rect.min,
                        cursor_row, cursor_col, cursor_visible, self.blink_on,
                    );
                }
            });
    }
}
