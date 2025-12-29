use core::ptr::{read_volatile, write_volatile};
use spin::Mutex;

const VGA_BUFFER: usize = 0xb8000;
const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;
const DEFAULT_COLOR: u8 = 0x0f;

static VGA_WRITER: Mutex<VgaWriter> = Mutex::new(VgaWriter::new());

pub fn vga_init() {
    VGA_WRITER.lock().clear();
}

pub fn vga_write_str(text: &str) {
    let mut writer = VGA_WRITER.lock();
    for byte in text.bytes() {
        writer.write_byte(byte);
    }
}

struct VgaWriter {
    row: usize,
    column: usize,
    color: u8,
}

impl VgaWriter {
    const fn new() -> Self {
        Self {
            row: 0,
            column: 0,
            color: DEFAULT_COLOR,
        }
    }

    fn clear(&mut self) {
        for row in 0..VGA_HEIGHT {
            self.clear_row(row);
        }
        self.row = 0;
        self.column = 0;
    }

    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            b'\r' => {}
            0x08 => self.backspace(),
            byte => {
                if self.column >= VGA_WIDTH {
                    self.new_line();
                }
                self.write_cell(self.row, self.column, byte);
                self.column += 1;
            }
        }
    }

    fn new_line(&mut self) {
        self.column = 0;
        if self.row + 1 >= VGA_HEIGHT {
            self.scroll();
        } else {
            self.row += 1;
        }
    }

    fn backspace(&mut self) {
        if self.column == 0 {
            if self.row == 0 {
                return;
            }
            self.row -= 1;
            self.column = VGA_WIDTH - 1;
        } else {
            self.column -= 1;
        }
        self.write_cell(self.row, self.column, b' ');
    }

    fn scroll(&mut self) {
        for row in 1..VGA_HEIGHT {
            for col in 0..VGA_WIDTH {
                let (ch, color) = self.read_cell(row, col);
                self.write_cell_raw(row - 1, col, ch, color);
            }
        }
        self.clear_row(VGA_HEIGHT - 1);
    }

    fn clear_row(&self, row: usize) {
        for col in 0..VGA_WIDTH {
            self.write_cell_raw(row, col, b' ', self.color);
        }
    }

    fn write_cell(&self, row: usize, col: usize, byte: u8) {
        self.write_cell_raw(row, col, byte, self.color);
    }

    fn write_cell_raw(&self, row: usize, col: usize, byte: u8, color: u8) {
        let idx = (row * VGA_WIDTH + col) * 2;
        let ptr = VGA_BUFFER as *mut u8;
        unsafe {
            write_volatile(ptr.add(idx), byte);
            write_volatile(ptr.add(idx + 1), color);
        }
    }

    fn read_cell(&self, row: usize, col: usize) -> (u8, u8) {
        let idx = (row * VGA_WIDTH + col) * 2;
        let ptr = VGA_BUFFER as *const u8;
        unsafe {
            let ch = read_volatile(ptr.add(idx));
            let color = read_volatile(ptr.add(idx + 1));
            (ch, color)
        }
    }
}
