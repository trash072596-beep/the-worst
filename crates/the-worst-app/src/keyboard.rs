use egui::{Key, Modifiers};

pub fn key_to_bytes(key: Key, mods: Modifiers) -> Option<Vec<u8>> {
    // Ctrl+letter
    if mods.ctrl && !mods.alt {
        if let Some(b) = ctrl_byte(key) {
            return Some(vec![b]);
        }
    }

    Some(match key {
        Key::Enter     => b"\r".to_vec(),
        Key::Backspace => b"\x7f".to_vec(),
        Key::Tab       => {
            if mods.shift {
                b"\x1b[Z".to_vec()
            } else {
                b"\t".to_vec()
            }
        }
        Key::Escape    => b"\x1b".to_vec(),
        Key::Delete    => b"\x1b[3~".to_vec(),
        Key::Home      => {
            if mods.ctrl { b"\x1b[1;5H".to_vec() } else { b"\x1b[H".to_vec() }
        }
        Key::End       => {
            if mods.ctrl { b"\x1b[1;5F".to_vec() } else { b"\x1b[F".to_vec() }
        }
        Key::PageUp    => b"\x1b[5~".to_vec(),
        Key::PageDown  => b"\x1b[6~".to_vec(),
        Key::ArrowUp    => cursor_seq(b'A', mods),
        Key::ArrowDown  => cursor_seq(b'B', mods),
        Key::ArrowRight => cursor_seq(b'C', mods),
        Key::ArrowLeft  => cursor_seq(b'D', mods),
        Key::F1  => b"\x1bOP".to_vec(),
        Key::F2  => b"\x1bOQ".to_vec(),
        Key::F3  => b"\x1bOR".to_vec(),
        Key::F4  => b"\x1bOS".to_vec(),
        Key::F5  => b"\x1b[15~".to_vec(),
        Key::F6  => b"\x1b[17~".to_vec(),
        Key::F7  => b"\x1b[18~".to_vec(),
        Key::F8  => b"\x1b[19~".to_vec(),
        Key::F9  => b"\x1b[20~".to_vec(),
        Key::F10 => b"\x1b[21~".to_vec(),
        Key::F11 => b"\x1b[23~".to_vec(),
        Key::F12 => b"\x1b[24~".to_vec(),
        _ => return None,
    })
}

fn cursor_seq(letter: u8, mods: Modifiers) -> Vec<u8> {
    let modifier = modifier_code(mods);
    if modifier == 1 {
        format!("\x1b[{}", letter as char).into_bytes()
    } else {
        format!("\x1b[1;{}{}", modifier, letter as char).into_bytes()
    }
}

fn modifier_code(mods: Modifiers) -> u8 {
    let mut v = 1u8;
    if mods.shift { v += 1; }
    if mods.alt   { v += 2; }
    if mods.ctrl  { v += 4; }
    v
}

fn ctrl_byte(key: Key) -> Option<u8> {
    match key {
        Key::A => Some(0x01), Key::B => Some(0x02), Key::C => Some(0x03),
        Key::D => Some(0x04), Key::E => Some(0x05), Key::F => Some(0x06),
        Key::G => Some(0x07), Key::H => Some(0x08), Key::I => Some(0x09),
        Key::J => Some(0x0A), Key::K => Some(0x0B), Key::L => Some(0x0C),
        Key::M => Some(0x0D), Key::N => Some(0x0E), Key::O => Some(0x0F),
        Key::P => Some(0x10), Key::Q => Some(0x11), Key::R => Some(0x12),
        Key::S => Some(0x13), Key::T => Some(0x14), Key::U => Some(0x15),
        Key::V => Some(0x16), Key::W => Some(0x17), Key::X => Some(0x18),
        Key::Y => Some(0x19), Key::Z => Some(0x1A),
        _ => None,
    }
}
