use egui::Color32;
use the_worst_core::cell::TermColor;

// ─── FUTURISTIC PALETTE ───────────────────────────────────────────────────────
// Deep space black, cold cyan, electric blue, soft violet

pub const BG:         Color32 = Color32::from_rgb(6,   8,  14);   // deep space
pub const FG:         Color32 = Color32::from_rgb(200, 220, 240); // cold white

pub const CURSOR_COL: Color32 = Color32::from_rgb(0,   200, 255); // electric cyan
pub const SELECT_COL: Color32 = Color32::from_rgb(40,  80,  160); // selection blue

pub const TAB_BG:     Color32 = Color32::from_rgb(8,   10,  18);  // panel bg
pub const TAB_ACTIVE: Color32 = Color32::from_rgb(0,   180, 240); // active accent
pub const TAB_DIM:    Color32 = Color32::from_rgb(55,  65,  90);  // inactive text
pub const ACCENT:     Color32 = Color32::from_rgb(0,   180, 240); // ui accent cyan
pub const ACCENT2:    Color32 = Color32::from_rgb(100, 60,  220); // violet accent

// ANSI 16 — cold sci-fi tuned
const ANSI: [Color32; 16] = [
    Color32::from_rgb(12,  14,  22),  // Black         → void
    Color32::from_rgb(210, 50,  80),  // Red           → alert red
    Color32::from_rgb(0,   200, 140), // Green         → hologram green
    Color32::from_rgb(200, 170, 0),   // Yellow        → caution amber
    Color32::from_rgb(40,  100, 220), // Blue          → electric blue
    Color32::from_rgb(140, 60,  220), // Magenta       → deep violet
    Color32::from_rgb(0,   190, 210), // Cyan          → cool cyan
    Color32::from_rgb(160, 180, 200), // White         → steel grey
    // Bright variants
    Color32::from_rgb(40,  50,  70),  // Bright Black  → dark panel
    Color32::from_rgb(255, 80,  110), // Bright Red    → hot alert
    Color32::from_rgb(0,   255, 180), // Bright Green  → neon hologram
    Color32::from_rgb(255, 220, 0),   // Bright Yellow → pure amber
    Color32::from_rgb(80,  150, 255), // Bright Blue   → sky blue
    Color32::from_rgb(180, 100, 255), // Bright Magenta→ neon violet
    Color32::from_rgb(0,   240, 255), // Bright Cyan   → laser cyan
    Color32::from_rgb(220, 235, 255), // Bright White  → ice white
];

pub fn resolve(color: TermColor, is_bg: bool) -> Color32 {
    match color {
        TermColor::Default => if is_bg { BG } else { FG },
        TermColor::Named(c) => ANSI[c as usize],
        TermColor::Indexed(i) => xterm256(i),
        TermColor::Rgb(r, g, b) => Color32::from_rgb(r, g, b),
    }
}

pub fn xterm256(i: u8) -> Color32 {
    match i {
        0..=15 => ANSI[i as usize],
        16..=231 => {
            let i = i - 16;
            let r = (i / 36) * 51;
            let g = ((i % 36) / 6) * 51;
            let b = (i % 6) * 51;
            Color32::from_rgb(r, g, b)
        }
        232..=255 => {
            let v = 8 + (i - 232) * 10;
            Color32::from_rgb(v, v, v)
        }
    }
}
