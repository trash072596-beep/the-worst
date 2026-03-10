/// Pixelated "The_Worst" logo with sine wave animation.
/// Each glyph is a 5×7 bitmap (row-major, MSB left).

use egui::{Color32, Painter, Pos2, Rect, Vec2};

// ── Pixel bitmaps (5 wide × 7 tall) ─────────────────────────────────────────
// Each u8 row: bit4=leftmost, bit0=rightmost

const T: [[u8; 5]; 7] = [
    [1,1,1,1,1],
    [0,0,1,0,0],
    [0,0,1,0,0],
    [0,0,1,0,0],
    [0,0,1,0,0],
    [0,0,1,0,0],
    [0,0,1,0,0],
];
const H: [[u8; 5]; 7] = [
    [1,0,0,0,1],
    [1,0,0,0,1],
    [1,0,0,0,1],
    [1,1,1,1,1],
    [1,0,0,0,1],
    [1,0,0,0,1],
    [1,0,0,0,1],
];
const E: [[u8; 5]; 7] = [
    [1,1,1,1,1],
    [1,0,0,0,0],
    [1,0,0,0,0],
    [1,1,1,1,0],
    [1,0,0,0,0],
    [1,0,0,0,0],
    [1,1,1,1,1],
];
const UNDER: [[u8; 5]; 7] = [
    [0,0,0,0,0],
    [0,0,0,0,0],
    [0,0,0,0,0],
    [0,0,0,0,0],
    [0,0,0,0,0],
    [0,0,0,0,0],
    [1,1,1,1,1],
];
const W: [[u8; 5]; 7] = [
    [1,0,0,0,1],
    [1,0,0,0,1],
    [1,0,0,0,1],
    [1,0,1,0,1],
    [1,0,1,0,1],
    [1,1,0,1,1],
    [1,0,0,0,1],
];
const O: [[u8; 5]; 7] = [
    [0,1,1,1,0],
    [1,0,0,0,1],
    [1,0,0,0,1],
    [1,0,0,0,1],
    [1,0,0,0,1],
    [1,0,0,0,1],
    [0,1,1,1,0],
];
const R: [[u8; 5]; 7] = [
    [1,1,1,1,0],
    [1,0,0,0,1],
    [1,0,0,0,1],
    [1,1,1,1,0],
    [1,0,1,0,0],
    [1,0,0,1,0],
    [1,0,0,0,1],
];
const S: [[u8; 5]; 7] = [
    [0,1,1,1,1],
    [1,0,0,0,0],
    [1,0,0,0,0],
    [0,1,1,1,0],
    [0,0,0,0,1],
    [0,0,0,0,1],
    [1,1,1,1,0],
];
const TT: [[u8; 5]; 7] = [
    [0,1,1,1,0],
    [0,0,1,0,0],
    [0,0,1,0,0],
    [0,0,1,0,0],
    [0,0,1,0,0],
    [0,0,1,0,0],
    [0,1,1,1,0],
];

const GLYPHS: [[[u8; 5]; 7]; 9] = [T, H, E, UNDER, W, O, R, S, TT];
const GAP: f32 = 1.0; // px gap between chars

pub fn draw(painter: &Painter, origin: Pos2, px: f32, t: f32) {
    // Neon gradient: cycle through cyan→violet per glyph column
    let char_w = 5.0 * px + GAP;
    let total_cols: usize = GLYPHS.len() * 5 + (GLYPHS.len() - 1); // 5 px + 1 gap per char

    let mut x = origin.x;
    for (gi, glyph) in GLYPHS.iter().enumerate() {
        for col in 0..5usize {
            // Global column index for wave phase
            let global_col = gi * 6 + col;
            let phase = t * 3.0 + global_col as f32 * 0.4;
            let wave_y = phase.sin() * 3.5 * px;

            // Color: shift hue across the logo
            let hue_t = global_col as f32 / (total_cols as f32);
            let color = logo_color(hue_t, phase);

            for row in 0..7usize {
                if glyph[row][col] == 1 {
                    let px_x = x + col as f32 * px;
                    let px_y = origin.y + row as f32 * px + wave_y;
                    painter.rect_filled(
                        Rect::from_min_size(Pos2::new(px_x, px_y), Vec2::splat(px - 0.5)),
                        0.0,
                        color,
                    );
                }
            }
        }
        x += char_w;
    }
}

/// Width in pixels of the full logo
pub fn width(px: f32) -> f32 {
    let char_w = 5.0 * px + GAP;
    char_w * GLYPHS.len() as f32 - GAP
}

/// Height including max wave displacement
pub fn height(px: f32) -> f32 {
    7.0 * px + 7.0 // 7px char + wave room
}

fn logo_color(hue_t: f32, phase: f32) -> Color32 {
    // Interpolate cyan (#00d4ff) → violet (#cc44ff) across width
    // plus a subtle brightness pulse from the wave
    let brightness = 0.85 + 0.15 * phase.sin();

    let r = lerp(0.0,   204.0, hue_t) * brightness;
    let g = lerp(212.0,  68.0, hue_t) * brightness;
    let b = lerp(255.0, 255.0, hue_t) * brightness;

    Color32::from_rgb(r as u8, g as u8, b as u8)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}
