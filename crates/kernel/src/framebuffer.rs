use core::ptr;

use kernel_core::FramebufferInfo;

use crate::font::FONT;

const FONT_WIDTH: usize = 8;
const FONT_HEIGHT: usize = 16;

/// Simple framebuffer text console using a built-in 8x16 bitmap font.
pub struct FramebufferConsole {
    info: FramebufferInfo,
    cols: usize,
    rows: usize,
    col: usize,
    row: usize,
    fg: u32,
    bg: u32,
    bytes_per_pixel: usize,
}

impl FramebufferConsole {
    /// Creates a framebuffer console if the format is supported.
    pub fn new(info: FramebufferInfo) -> Option<Self> {
        let bytes_per_pixel = (info.bpp / 8) as usize;
        if bytes_per_pixel < 3 {
            return None;
        }
        let cols = info.width as usize / FONT_WIDTH;
        let rows = info.height as usize / FONT_HEIGHT;
        if cols == 0 || rows == 0 {
            return None;
        }
        let fg = pack_rgb(&info, 0xdd, 0xdd, 0xdd);
        let bg = pack_rgb(&info, 0x12, 0x12, 0x12);
        Some(Self {
            info,
            cols,
            rows,
            col: 0,
            row: 0,
            fg,
            bg,
            bytes_per_pixel,
        })
    }

    /// Clears the framebuffer with the background color.
    pub fn clear(&mut self) {
        let total_rows = self.info.height as usize;
        let fb_ptr = self.info.addr as *mut u8;
        for y in 0..total_rows {
            for x in 0..(self.info.width as usize) {
                unsafe {
                    self.write_pixel_raw(fb_ptr, x, y, self.bg);
                }
            }
        }
    }

    /// Writes a string to the framebuffer console.
    pub fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }

    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                self.col = 0;
                self.row += 1;
            }
            b'\r' => {
                self.col = 0;
            }
            b'\t' => {
                let next = (self.col + 4) & !3;
                while self.col < next {
                    self.write_byte(b' ');
                }
                return;
            }
            0x20..=0x7e | 0x80..=0xff => {
                self.draw_glyph(byte);
                self.col += 1;
            }
            _ => {}
        }

        if self.col >= self.cols {
            self.col = 0;
            self.row += 1;
        }

        if self.row >= self.rows {
            self.scroll();
            self.row = self.rows.saturating_sub(1);
        }
    }

    fn draw_glyph(&self, byte: u8) {
        let glyph = &FONT[(byte as usize) * FONT_HEIGHT..][..FONT_HEIGHT];
        let x0 = self.col * FONT_WIDTH;
        let y0 = self.row * FONT_HEIGHT;
        let fb_ptr = self.info.addr as *mut u8;
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..FONT_WIDTH {
                let mask = 1u8 << (7 - col);
                let color = if bits & mask != 0 { self.fg } else { self.bg };
                unsafe {
                    self.write_pixel_raw(fb_ptr, x0 + col, y0 + row, color);
                }
            }
        }
    }

    fn scroll(&mut self) {
        let pitch = self.info.pitch as usize;
        let height = self.info.height as usize;
        let fb_ptr = self.info.addr as *mut u8;
        let scroll_rows = FONT_HEIGHT.min(height);
        let copy_rows = height.saturating_sub(scroll_rows);
        let copy_bytes = copy_rows.saturating_mul(pitch);
        unsafe {
            let src = fb_ptr.add(scroll_rows * pitch);
            ptr::copy(src, fb_ptr, copy_bytes);
        }
        self.clear_pixel_rows(height.saturating_sub(scroll_rows), scroll_rows);
    }

    fn clear_pixel_rows(&mut self, start_row: usize, rows: usize) {
        let fb_ptr = self.info.addr as *mut u8;
        for y in start_row..start_row.saturating_add(rows) {
            if y >= self.info.height as usize {
                break;
            }
            for x in 0..(self.info.width as usize) {
                unsafe {
                    self.write_pixel_raw(fb_ptr, x, y, self.bg);
                }
            }
        }
    }

    unsafe fn write_pixel_raw(&self, base: *mut u8, x: usize, y: usize, color: u32) {
        let pitch = self.info.pitch as usize;
        let offset = y * pitch + x * self.bytes_per_pixel;
        let ptr = base.add(offset);
        let bytes = color.to_le_bytes();
        for i in 0..self.bytes_per_pixel {
            ptr.add(i).write(bytes[i]);
        }
    }
}

fn pack_rgb(info: &FramebufferInfo, r: u8, g: u8, b: u8) -> u32 {
    pack_component(r, info.red_mask_size, info.red_mask_shift)
        | pack_component(g, info.green_mask_size, info.green_mask_shift)
        | pack_component(b, info.blue_mask_size, info.blue_mask_shift)
}

fn pack_component(value: u8, size: u8, shift: u8) -> u32 {
    if size == 0 {
        return 0;
    }
    let max = (1u32 << size) - 1;
    let scaled = (value as u32 * max + 127) / 255;
    scaled << shift
}
