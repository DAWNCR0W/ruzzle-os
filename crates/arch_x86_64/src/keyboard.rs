use spin::Mutex;
use x86_64::instructions::port::Port;

const DATA_PORT: u16 = 0x60;
const STATUS_PORT: u16 = 0x64;
const EXTENDED_PREFIX: u8 = 0xE0;
const LEFT_SHIFT: u8 = 0x2A;
const RIGHT_SHIFT: u8 = 0x36;

static KEYBOARD_STATE: Mutex<KeyboardState> = Mutex::new(KeyboardState::new());

pub fn keyboard_init() {
    init_controller();
    KEYBOARD_STATE.lock().reset();
}

pub fn keyboard_has_data() -> bool {
    unsafe {
        let mut port = Port::new(STATUS_PORT);
        let status: u8 = port.read();
        status & 0x01 != 0
    }
}

pub fn keyboard_read_byte() -> Option<u8> {
    let scancode = unsafe {
        let mut port = Port::new(DATA_PORT);
        port.read()
    };
    let mut state = KEYBOARD_STATE.lock();
    apply_scancode(scancode, &mut state)
}

fn init_controller() {
    if !wait_input_empty() {
        return;
    }
    unsafe {
        let mut cmd = Port::new(STATUS_PORT);
        cmd.write(0xADu8);
        cmd.write(0xA7u8);
    }
    flush_output();

    if !wait_input_empty() {
        return;
    }
    unsafe {
        let mut cmd = Port::new(STATUS_PORT);
        cmd.write(0x20u8);
    }
    let mut config = read_data().unwrap_or(0);
    config |= 0x01;
    config &= !0x20;
    if !wait_input_empty() {
        return;
    }
    unsafe {
        let mut cmd = Port::new(STATUS_PORT);
        cmd.write(0x60u8);
        let mut data = Port::new(DATA_PORT);
        data.write(config);
    }
    if !wait_input_empty() {
        return;
    }
    unsafe {
        let mut cmd = Port::new(STATUS_PORT);
        cmd.write(0xAEu8);
    }
    if !wait_input_empty() {
        return;
    }
    unsafe {
        let mut data = Port::new(DATA_PORT);
        data.write(0xF4u8);
    }
    let _ = read_data();
}

fn flush_output() {
    while keyboard_has_data() {
        let _ = unsafe {
            let mut data: Port<u8> = Port::new(DATA_PORT);
            data.read()
        };
    }
}

fn read_data() -> Option<u8> {
    if !wait_output_full() {
        return None;
    }
    unsafe {
        let mut data = Port::new(DATA_PORT);
        Some(data.read())
    }
}

fn wait_input_empty() -> bool {
    for _ in 0..10000 {
        let status = unsafe {
            let mut port: Port<u8> = Port::new(STATUS_PORT);
            port.read()
        };
        if status & 0x02 == 0 {
            return true;
        }
    }
    false
}

fn wait_output_full() -> bool {
    for _ in 0..10000 {
        let status = unsafe {
            let mut port: Port<u8> = Port::new(STATUS_PORT);
            port.read()
        };
        if status & 0x01 != 0 {
            return true;
        }
    }
    false
}

fn apply_scancode(scancode: u8, state: &mut KeyboardState) -> Option<u8> {
    if scancode == EXTENDED_PREFIX {
        return None;
    }
    let released = scancode & 0x80 != 0;
    let code = scancode & 0x7F;
    match code {
        LEFT_SHIFT => {
            state.shift_left = !released;
            return None;
        }
        RIGHT_SHIFT => {
            state.shift_right = !released;
            return None;
        }
        _ => {}
    }
    if released {
        return None;
    }
    map_scancode(code, state.shift_active())
}

fn map_scancode(code: u8, shift: bool) -> Option<u8> {
    let byte = match code {
        0x02 => if shift { b'!' } else { b'1' },
        0x03 => if shift { b'@' } else { b'2' },
        0x04 => if shift { b'#' } else { b'3' },
        0x05 => if shift { b'$' } else { b'4' },
        0x06 => if shift { b'%' } else { b'5' },
        0x07 => if shift { b'^' } else { b'6' },
        0x08 => if shift { b'&' } else { b'7' },
        0x09 => if shift { b'*' } else { b'8' },
        0x0A => if shift { b'(' } else { b'9' },
        0x0B => if shift { b')' } else { b'0' },
        0x0C => if shift { b'_' } else { b'-' },
        0x0D => if shift { b'+' } else { b'=' },
        0x0E => 0x08,
        0x0F => b'\t',
        0x10 => if shift { b'Q' } else { b'q' },
        0x11 => if shift { b'W' } else { b'w' },
        0x12 => if shift { b'E' } else { b'e' },
        0x13 => if shift { b'R' } else { b'r' },
        0x14 => if shift { b'T' } else { b't' },
        0x15 => if shift { b'Y' } else { b'y' },
        0x16 => if shift { b'U' } else { b'u' },
        0x17 => if shift { b'I' } else { b'i' },
        0x18 => if shift { b'O' } else { b'o' },
        0x19 => if shift { b'P' } else { b'p' },
        0x1A => if shift { b'{' } else { b'[' },
        0x1B => if shift { b'}' } else { b']' },
        0x1C => b'\n',
        0x1E => if shift { b'A' } else { b'a' },
        0x1F => if shift { b'S' } else { b's' },
        0x20 => if shift { b'D' } else { b'd' },
        0x21 => if shift { b'F' } else { b'f' },
        0x22 => if shift { b'G' } else { b'g' },
        0x23 => if shift { b'H' } else { b'h' },
        0x24 => if shift { b'J' } else { b'j' },
        0x25 => if shift { b'K' } else { b'k' },
        0x26 => if shift { b'L' } else { b'l' },
        0x27 => if shift { b':' } else { b';' },
        0x28 => if shift { b'"' } else { b'\'' },
        0x29 => if shift { b'~' } else { b'`' },
        0x2B => if shift { b'|' } else { b'\\' },
        0x2C => if shift { b'Z' } else { b'z' },
        0x2D => if shift { b'X' } else { b'x' },
        0x2E => if shift { b'C' } else { b'c' },
        0x2F => if shift { b'V' } else { b'v' },
        0x30 => if shift { b'B' } else { b'b' },
        0x31 => if shift { b'N' } else { b'n' },
        0x32 => if shift { b'M' } else { b'm' },
        0x33 => if shift { b'<' } else { b',' },
        0x34 => if shift { b'>' } else { b'.' },
        0x35 => if shift { b'?' } else { b'/' },
        0x39 => b' ',
        _ => 0,
    };
    if byte == 0 {
        None
    } else {
        Some(byte)
    }
}

#[derive(Debug, Clone, Copy)]
struct KeyboardState {
    shift_left: bool,
    shift_right: bool,
}

impl KeyboardState {
    const fn new() -> Self {
        Self {
            shift_left: false,
            shift_right: false,
        }
    }

    fn reset(&mut self) {
        self.shift_left = false;
        self.shift_right = false;
    }

    fn shift_active(&self) -> bool {
        self.shift_left || self.shift_right
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_letters_with_shift() {
        let mut state = KeyboardState::new();
        assert_eq!(apply_scancode(0x1E, &mut state), Some(b'a'));
        assert_eq!(apply_scancode(LEFT_SHIFT, &mut state), None);
        assert_eq!(apply_scancode(0x1E, &mut state), Some(b'A'));
        assert_eq!(apply_scancode(LEFT_SHIFT | 0x80, &mut state), None);
        assert_eq!(apply_scancode(0x1E, &mut state), Some(b'a'));
    }

    #[test]
    fn map_numbers_with_shift() {
        let mut state = KeyboardState::new();
        assert_eq!(apply_scancode(0x02, &mut state), Some(b'1'));
        assert_eq!(apply_scancode(RIGHT_SHIFT, &mut state), None);
        assert_eq!(apply_scancode(0x02, &mut state), Some(b'!'));
        assert_eq!(apply_scancode(RIGHT_SHIFT | 0x80, &mut state), None);
    }

    #[test]
    fn shift_state_tracks_both_keys() {
        let mut state = KeyboardState::new();
        assert_eq!(apply_scancode(LEFT_SHIFT, &mut state), None);
        assert_eq!(apply_scancode(RIGHT_SHIFT, &mut state), None);
        assert_eq!(apply_scancode(LEFT_SHIFT | 0x80, &mut state), None);
        assert_eq!(apply_scancode(0x10, &mut state), Some(b'Q'));
        assert_eq!(apply_scancode(RIGHT_SHIFT | 0x80, &mut state), None);
        assert_eq!(apply_scancode(0x10, &mut state), Some(b'q'));
    }

    #[test]
    fn handles_control_keys() {
        let mut state = KeyboardState::new();
        assert_eq!(apply_scancode(0x0E, &mut state), Some(0x08));
        assert_eq!(apply_scancode(0x1C, &mut state), Some(b'\n'));
        assert_eq!(apply_scancode(0x0F, &mut state), Some(b'\t'));
        assert_eq!(apply_scancode(0x39, &mut state), Some(b' '));
    }

    #[test]
    fn ignores_releases_and_unknown_codes() {
        let mut state = KeyboardState::new();
        assert_eq!(apply_scancode(0x1E | 0x80, &mut state), None);
        assert_eq!(apply_scancode(EXTENDED_PREFIX, &mut state), None);
        assert_eq!(apply_scancode(0x5E, &mut state), None);
    }
}
