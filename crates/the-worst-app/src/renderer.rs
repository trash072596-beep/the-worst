use egui::{Color32, FontId, Painter, Pos2, Rect, Stroke, Vec2};
use the_worst_core::cell::{Attrs, Cell};
use crate::colors;

#[derive(Clone)]
pub struct RenderMetrics {
    pub cell_w: f32,
    pub cell_h: f32,
    pub font_id: FontId,
}

impl RenderMetrics {
    pub fn compute(ctx: &egui::Context, font_size: f32) -> Self {
        let font_id = FontId::monospace(font_size);
        let cell_w = ctx.fonts(|f| f.glyph_width(&font_id, 'M'));
        let cell_h = ctx.fonts(|f| f.row_height(&font_id));
        Self { cell_w, cell_h, font_id }
    }
}

pub fn draw_grid(
    painter: &Painter,
    cells: &[Cell],
    cols: usize,
    rows: usize,
    metrics: &RenderMetrics,
    origin: Pos2,
    cursor_row: usize,
    cursor_col: usize,
    cursor_visible: bool,
    blink_on: bool,
) {
    let cw = metrics.cell_w;
    let ch = metrics.cell_h;

    for row in 0..rows {
        for col in 0..cols {
            let idx = row * cols + col;
            if idx >= cells.len() {
                break;
            }
            let cell = &cells[idx];

            if cell.attrs.get(Attrs::WIDE_CONT) {
                continue;
            }

            let cell_width = if cell.attrs.get(Attrs::WIDE) { cw * 2.0 } else { cw };
            let tl = origin + Vec2::new(col as f32 * cw, row as f32 * ch);
            let rect = Rect::from_min_size(tl, Vec2::new(cell_width, ch));

            let mut fg = colors::resolve(cell.fg, false);
            let mut bg = colors::resolve(cell.bg, true);

            if cell.attrs.get(Attrs::INVERSE) {
                std::mem::swap(&mut fg, &mut bg);
            }
            if cell.attrs.get(Attrs::DIM) {
                fg = dim(fg);
            }
            if cell.attrs.get(Attrs::BOLD) {
                fg = brighten(fg);
            }

            // Background
            if bg != colors::BG {
                painter.rect_filled(rect, 0.0, bg);
            }

            // Glyph
            if cell.ch != ' ' && cell.ch != '\0' {
                let mut job = egui::text::LayoutJob::default();
                let mut format = egui::TextFormat {
                    font_id: metrics.font_id.clone(),
                    color: fg,
                    ..Default::default()
                };
                if cell.attrs.get(Attrs::ITALIC) {
                    format.italics = true;
                }
                // egui doesn't support bold on monospace directly, but we can try
                job.append(&cell.ch.to_string(), 0.0, format);

                painter.add(egui::Shape::galley(
                    tl,
                    painter.ctx().fonts(|f| f.layout_job(job)),
                    Color32::WHITE,
                ));
            }

            // Underline
            if cell.attrs.get(Attrs::UNDERLINE) {
                let y = rect.bottom() - 1.0;
                painter.line_segment(
                    [
                        Pos2::new(rect.left(), y),
                        Pos2::new(rect.right(), y),
                    ],
                    Stroke::new(1.0, fg),
                );
            }

            // Strikeout
            if cell.attrs.get(Attrs::STRIKEOUT) {
                let y = rect.center().y;
                painter.line_segment(
                    [
                        Pos2::new(rect.left(), y),
                        Pos2::new(rect.right(), y),
                    ],
                    Stroke::new(1.0, fg),
                );
            }
        }
    }

    // Cursor — thin vertical beam, glows on blink
    if cursor_visible && cursor_row < rows && cursor_col < cols {
        let tl = origin + Vec2::new(cursor_col as f32 * cw, cursor_row as f32 * ch);
        let alpha = if blink_on { 255 } else { 80 };
        let cursor_color = Color32::from_rgba_premultiplied(
            (colors::CURSOR_COL.r() as u16 * alpha as u16 / 255) as u8,
            (colors::CURSOR_COL.g() as u16 * alpha as u16 / 255) as u8,
            (colors::CURSOR_COL.b() as u16 * alpha as u16 / 255) as u8,
            alpha,
        );
        // 2px beam
        let beam = Rect::from_min_size(tl, Vec2::new(2.0, ch));
        painter.rect_filled(beam, 1.0, cursor_color);
        // Subtle glow (wider, very transparent)
        if blink_on {
            let glow = Rect::from_min_size(
                tl - Vec2::new(2.0, 0.0),
                Vec2::new(6.0, ch),
            );
            painter.rect_filled(
                glow,
                1.0,
                Color32::from_rgba_premultiplied(
                    colors::CURSOR_COL.r() / 4,
                    colors::CURSOR_COL.g() / 4,
                    colors::CURSOR_COL.b() / 4,
                    40,
                ),
            );
        }
    }
}

fn dim(c: Color32) -> Color32 {
    Color32::from_rgba_premultiplied(
        (c.r() as f32 * 0.6) as u8,
        (c.g() as f32 * 0.6) as u8,
        (c.b() as f32 * 0.6) as u8,
        c.a(),
    )
}

fn brighten(c: Color32) -> Color32 {
    Color32::from_rgb(
        (c.r() as u16 + 40).min(255) as u8,
        (c.g() as u16 + 40).min(255) as u8,
        (c.b() as u16 + 40).min(255) as u8,
    )
}
