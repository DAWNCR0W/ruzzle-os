#![no_std]

#[cfg(test)]
extern crate std;

use core::ptr::{read_volatile, write_volatile};

use kernel_core::{BootInfo, MemoryKind, MemoryRegion};

const UART_BASE: usize = 0x0900_0000;
const UART_DR: usize = UART_BASE + 0x00;
const UART_FR: usize = UART_BASE + 0x18;
const UART_ICR: usize = UART_BASE + 0x44;

const FDT_MAGIC: u32 = 0xd00d_feed;
const FDT_BEGIN_NODE: u32 = 0x1;
const FDT_END_NODE: u32 = 0x2;
const FDT_PROP: u32 = 0x3;
const FDT_NOP: u32 = 0x4;
const FDT_END: u32 = 0x9;
const FDT_HEADER_SIZE: usize = 40;
const MAX_DEPTH: usize = 8;
const MAX_MEMORY_REGIONS: usize = 1;

static mut MEMORY_REGIONS: [MemoryRegion; MAX_MEMORY_REGIONS] = [MemoryRegion {
    start: 0,
    end: 0,
    kind: MemoryKind::Reserved,
}; MAX_MEMORY_REGIONS];
static mut MEMORY_REGION_COUNT: usize = 0;

/// Initializes platform devices such as the UART.
pub fn init() {
    uart_init();
}

/// Returns a BootInfo constructed from the device tree.
pub fn boot_info_from_dtb(
    dtb_ptr: usize,
    kernel_start: usize,
    kernel_end: usize,
) -> BootInfo<'static> {
    let info = parse_dtb(dtb_ptr).unwrap_or_default();
    let mut count = 0usize;
    if let Some((start, size)) = info.memory {
        if size > 0 {
            unsafe {
                MEMORY_REGIONS[0] = MemoryRegion {
                    start,
                    end: start.saturating_add(size),
                    kind: MemoryKind::Usable,
                };
            }
            count = 1;
        }
    }
    unsafe {
        MEMORY_REGION_COUNT = count;
        BootInfo {
            memory_map: core::slice::from_raw_parts(
                core::ptr::addr_of!(MEMORY_REGIONS) as *const MemoryRegion,
                MEMORY_REGION_COUNT,
            ),
            kernel_start: kernel_start as u64,
            kernel_end: kernel_end as u64,
            initramfs: info.initrd.map(|(start, end)| (start, end)),
            dtb_ptr: Some(dtb_ptr as u64),
        }
    }
}

/// Writes a byte to the PL011 UART.
pub fn uart_write(byte: u8) {
    unsafe {
        while read_volatile(UART_FR as *const u32) & (1 << 5) != 0 {}
        write_volatile(UART_DR as *mut u32, byte as u32);
    }
}

/// Returns true if UART receive data is available.
pub fn uart_has_data() -> bool {
    unsafe { read_volatile(UART_FR as *const u32) & (1 << 4) == 0 }
}

/// Reads a byte from the UART.
pub fn uart_read_byte() -> u8 {
    unsafe {
        while !uart_has_data() {}
        (read_volatile(UART_DR as *const u32) & 0xFF) as u8
    }
}

/// Placeholder timer tick handler for QEMU AArch64.
pub fn timer_tick() {
    // Timer handling will be implemented later.
}

/// Placeholder IRQ acknowledgement routine.
pub fn acknowledge_irq(_irq: u32) {
    // Interrupt controller integration will be implemented later.
}

fn uart_init() {
    unsafe {
        write_volatile(UART_ICR as *mut u32, 0x7ff);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DtbInfo {
    pub memory: Option<(u64, u64)>,
    pub initrd: Option<(u64, u64)>,
}

impl Default for DtbInfo {
    fn default() -> Self {
        Self {
            memory: None,
            initrd: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DtbError {
    BadMagic,
    Truncated,
    InvalidOffset,
    InvalidToken,
    DepthOverflow,
}

pub fn parse_dtb(dtb_ptr: usize) -> Result<DtbInfo, DtbError> {
    let header = unsafe { core::slice::from_raw_parts(dtb_ptr as *const u8, FDT_HEADER_SIZE) };
    let totalsize = read_be_u32(header, 4)? as usize;
    let data = unsafe { core::slice::from_raw_parts(dtb_ptr as *const u8, totalsize) };
    parse_dtb_bytes(data)
}

fn parse_dtb_bytes(data: &[u8]) -> Result<DtbInfo, DtbError> {
    if data.len() < FDT_HEADER_SIZE {
        return Err(DtbError::Truncated);
    }
    let magic = read_be_u32(data, 0)?;
    if magic != FDT_MAGIC {
        return Err(DtbError::BadMagic);
    }
    let totalsize = read_be_u32(data, 4)? as usize;
    if totalsize > data.len() {
        return Err(DtbError::Truncated);
    }
    let off_struct = read_be_u32(data, 8)? as usize;
    let off_strings = read_be_u32(data, 12)? as usize;
    let size_strings = read_be_u32(data, 32)? as usize;
    let size_struct = read_be_u32(data, 36)? as usize;

    let struct_end = off_struct.checked_add(size_struct).ok_or(DtbError::InvalidOffset)?;
    let strings_end =
        off_strings.checked_add(size_strings).ok_or(DtbError::InvalidOffset)?;
    if struct_end > totalsize || strings_end > totalsize {
        return Err(DtbError::InvalidOffset);
    }

    let struct_block = &data[off_struct..struct_end];
    let strings_block = &data[off_strings..strings_end];

    let mut cursor = 0usize;
    let mut depth = 0usize;
    let mut stack = [NodeKind::Other; MAX_DEPTH];
    let mut info = DtbInfo::default();
    let mut initrd_start: Option<u64> = None;
    let mut initrd_end: Option<u64> = None;

    loop {
        if cursor + 4 > struct_block.len() {
            return Err(DtbError::Truncated);
        }
        let token = read_be_u32(struct_block, cursor)?;
        cursor += 4;
        match token {
            FDT_BEGIN_NODE => {
                let (name, next) = read_cstr(struct_block, cursor)?;
                cursor = align4(next);
                if depth >= MAX_DEPTH {
                    return Err(DtbError::DepthOverflow);
                }
                let kind = if depth == 0 && name == b"memory" {
                    NodeKind::Memory
                } else if depth == 0 && name == b"chosen" {
                    NodeKind::Chosen
                } else if depth == 1 && name == b"memory" {
                    NodeKind::Memory
                } else if depth == 1 && name == b"chosen" {
                    NodeKind::Chosen
                } else {
                    NodeKind::Other
                };
                stack[depth] = kind;
                depth += 1;
            }
            FDT_END_NODE => {
                if depth == 0 {
                    return Err(DtbError::InvalidToken);
                }
                depth -= 1;
            }
            FDT_PROP => {
                let len = read_be_u32(struct_block, cursor)? as usize;
                let nameoff = read_be_u32(struct_block, cursor + 4)? as usize;
                cursor += 8;
                let value_end = cursor.checked_add(len).ok_or(DtbError::Truncated)?;
                if value_end > struct_block.len() {
                    return Err(DtbError::Truncated);
                }
                if nameoff >= strings_block.len() {
                    return Err(DtbError::InvalidOffset);
                }
                let (name, _) = read_cstr(strings_block, nameoff)?;
                let value = &struct_block[cursor..value_end];
                cursor = align4(value_end);

                if depth == 0 {
                    continue;
                }
                match stack[depth - 1] {
                    NodeKind::Memory => {
                        if name == b"reg" {
                            if let Some(pair) = parse_reg(value) {
                                info.memory = Some(pair);
                            }
                        }
                    }
                    NodeKind::Chosen => {
                        if name == b"linux,initrd-start" {
                            initrd_start = parse_u64(value);
                        } else if name == b"linux,initrd-end" {
                            initrd_end = parse_u64(value);
                        }
                    }
                    NodeKind::Other => {}
                }
            }
            FDT_NOP => {}
            FDT_END => break,
            _ => return Err(DtbError::InvalidToken),
        }
    }

    if let (Some(start), Some(end)) = (initrd_start, initrd_end) {
        if end > start {
            info.initrd = Some((start, end));
        }
    }

    Ok(info)
}

fn read_be_u32(data: &[u8], offset: usize) -> Result<u32, DtbError> {
    if offset + 4 > data.len() {
        return Err(DtbError::Truncated);
    }
    Ok(u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

fn read_cstr(data: &[u8], offset: usize) -> Result<(&[u8], usize), DtbError> {
    if offset >= data.len() {
        return Err(DtbError::Truncated);
    }
    let mut end = offset;
    while end < data.len() {
        if data[end] == 0 {
            return Ok((&data[offset..end], end + 1));
        }
        end += 1;
    }
    Err(DtbError::Truncated)
}

fn align4(value: usize) -> usize {
    (value + 3) & !3
}

fn parse_reg(value: &[u8]) -> Option<(u64, u64)> {
    if value.len() >= 16 {
        let start = parse_u64(&value[0..8])?;
        let size = parse_u64(&value[8..16])?;
        return Some((start, size));
    }
    if value.len() >= 8 {
        let start = parse_u32(&value[0..4])? as u64;
        let size = parse_u32(&value[4..8])? as u64;
        return Some((start, size));
    }
    None
}

fn parse_u64(value: &[u8]) -> Option<u64> {
    if value.len() >= 8 {
        Some(u64::from_be_bytes([
            value[0], value[1], value[2], value[3], value[4], value[5], value[6], value[7],
        ]))
    } else if value.len() >= 4 {
        Some(parse_u32(&value[0..4])? as u64)
    } else {
        None
    }
}

fn parse_u32(value: &[u8]) -> Option<u32> {
    if value.len() < 4 {
        None
    } else {
        Some(u32::from_be_bytes([value[0], value[1], value[2], value[3]]))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeKind {
    Other,
    Memory,
    Chosen,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;
    use std::vec::Vec;

    fn push_be_u32(buf: &mut Vec<u8>, value: u32) {
        buf.extend_from_slice(&value.to_be_bytes());
    }

    fn push_be_u64(buf: &mut Vec<u8>, value: u64) {
        buf.extend_from_slice(&value.to_be_bytes());
    }

    fn push_str(buf: &mut Vec<u8>, text: &str) -> usize {
        let offset = buf.len();
        buf.extend_from_slice(text.as_bytes());
        buf.push(0);
        offset
    }

    fn align4_vec(buf: &mut Vec<u8>) {
        while buf.len() % 4 != 0 {
            buf.push(0);
        }
    }

    fn push_node(struct_block: &mut Vec<u8>, name: &str) {
        push_be_u32(struct_block, FDT_BEGIN_NODE);
        struct_block.extend_from_slice(name.as_bytes());
        struct_block.push(0);
        align4_vec(struct_block);
    }

    fn push_end_node(struct_block: &mut Vec<u8>) {
        push_be_u32(struct_block, FDT_END_NODE);
    }

    fn push_prop(struct_block: &mut Vec<u8>, nameoff: u32, value: &[u8]) {
        push_be_u32(struct_block, FDT_PROP);
        push_be_u32(struct_block, value.len() as u32);
        push_be_u32(struct_block, nameoff);
        struct_block.extend_from_slice(value);
        align4_vec(struct_block);
    }

    fn build_dtb(struct_block: Vec<u8>, strings: Vec<u8>) -> Vec<u8> {
        let mut dtb = Vec::new();
        let mut mem_rsv = Vec::new();
        push_be_u64(&mut mem_rsv, 0);
        push_be_u64(&mut mem_rsv, 0);

        let off_struct = FDT_HEADER_SIZE + mem_rsv.len();
        let off_strings = off_struct + struct_block.len();
        let totalsize = off_strings + strings.len();

        push_be_u32(&mut dtb, FDT_MAGIC);
        push_be_u32(&mut dtb, totalsize as u32);
        push_be_u32(&mut dtb, off_struct as u32);
        push_be_u32(&mut dtb, off_strings as u32);
        push_be_u32(&mut dtb, FDT_HEADER_SIZE as u32);
        push_be_u32(&mut dtb, 17);
        push_be_u32(&mut dtb, 16);
        push_be_u32(&mut dtb, 0);
        push_be_u32(&mut dtb, strings.len() as u32);
        push_be_u32(&mut dtb, struct_block.len() as u32);

        dtb.extend_from_slice(&mem_rsv);
        dtb.extend_from_slice(&struct_block);
        dtb.extend_from_slice(&strings);
        dtb
    }

    fn build_sample_dtb(
        include_initrd: bool,
        use_32bit: bool,
    ) -> Vec<u8> {
        let mut strings = Vec::new();
        let reg_off = push_str(&mut strings, "reg") as u32;
        let initrd_start_off = push_str(&mut strings, "linux,initrd-start") as u32;
        let initrd_end_off = push_str(&mut strings, "linux,initrd-end") as u32;

        let mut struct_block = Vec::new();
        push_node(&mut struct_block, "");
        push_node(&mut struct_block, "chosen");
        if include_initrd {
            if use_32bit {
                let mut value = Vec::new();
                push_be_u32(&mut value, 0x42000000);
                push_prop(&mut struct_block, initrd_start_off, &value);
                value.clear();
                push_be_u32(&mut value, 0x42100000);
                push_prop(&mut struct_block, initrd_end_off, &value);
            } else {
                let mut value = Vec::new();
                push_be_u64(&mut value, 0x42000000);
                push_prop(&mut struct_block, initrd_start_off, &value);
                value.clear();
                push_be_u64(&mut value, 0x42100000);
                push_prop(&mut struct_block, initrd_end_off, &value);
            }
        }
        push_end_node(&mut struct_block);
        push_node(&mut struct_block, "memory");
        if use_32bit {
            let mut value = Vec::new();
            push_be_u32(&mut value, 0x40000000);
            push_be_u32(&mut value, 0x01000000);
            push_prop(&mut struct_block, reg_off, &value);
        } else {
            let mut value = Vec::new();
            push_be_u64(&mut value, 0x40000000);
            push_be_u64(&mut value, 0x01000000);
            push_prop(&mut struct_block, reg_off, &value);
        }
        push_end_node(&mut struct_block);
        push_end_node(&mut struct_block);
        push_be_u32(&mut struct_block, FDT_END);

        build_dtb(struct_block, strings)
    }

    #[test]
    fn parse_valid_dtb_with_initrd() {
        let dtb = build_sample_dtb(true, false);
        let info = parse_dtb_bytes(&dtb).expect("dtb should parse");
        assert_eq!(info.memory, Some((0x40000000, 0x01000000)));
        assert_eq!(info.initrd, Some((0x42000000, 0x42100000)));
    }

    #[test]
    fn parse_valid_dtb_with_32bit_values() {
        let dtb = build_sample_dtb(true, true);
        let info = parse_dtb_bytes(&dtb).expect("dtb should parse");
        assert_eq!(info.memory, Some((0x40000000, 0x01000000)));
        assert_eq!(info.initrd, Some((0x42000000, 0x42100000)));
    }

    #[test]
    fn parse_dtb_without_initrd() {
        let dtb = build_sample_dtb(false, false);
        let info = parse_dtb_bytes(&dtb).expect("dtb should parse");
        assert_eq!(info.memory, Some((0x40000000, 0x01000000)));
        assert_eq!(info.initrd, None);
    }

    #[test]
    fn parse_dtb_rejects_bad_magic() {
        let mut dtb = build_sample_dtb(true, false);
        dtb[0] = 0;
        assert_eq!(parse_dtb_bytes(&dtb), Err(DtbError::BadMagic));
    }

    #[test]
    fn parse_dtb_rejects_truncated_header() {
        let dtb = vec![0u8; 8];
        assert_eq!(parse_dtb_bytes(&dtb), Err(DtbError::Truncated));
    }

    #[test]
    fn parse_dtb_rejects_invalid_offsets() {
        let mut dtb = build_sample_dtb(true, false);
        dtb[8] = 0xFF;
        assert_eq!(parse_dtb_bytes(&dtb), Err(DtbError::InvalidOffset));
    }

    #[test]
    fn parse_dtb_rejects_invalid_token() {
        let mut dtb = build_sample_dtb(true, false);
        let struct_offset = read_be_u32(&dtb, 8).unwrap() as usize;
        dtb[struct_offset + 3] = 0x99;
        assert_eq!(parse_dtb_bytes(&dtb), Err(DtbError::InvalidToken));
    }

    #[test]
    fn parse_dtb_rejects_depth_overflow() {
        let mut strings = Vec::new();
        push_str(&mut strings, "reg");
        let mut struct_block = Vec::new();
        push_node(&mut struct_block, "");
        for _ in 0..(MAX_DEPTH + 1) {
            push_node(&mut struct_block, "memory");
        }
        push_be_u32(&mut struct_block, FDT_END);
        let dtb = build_dtb(struct_block, strings);
        assert_eq!(parse_dtb_bytes(&dtb), Err(DtbError::DepthOverflow));
    }

    #[test]
    fn boot_info_from_dtb_populates_memory() {
        let dtb = build_sample_dtb(true, false);
        let info = boot_info_from_dtb(dtb.as_ptr() as usize, 0x1000, 0x2000);
        assert_eq!(info.kernel_start, 0x1000);
        assert_eq!(info.kernel_end, 0x2000);
        assert_eq!(info.initramfs, Some((0x42000000, 0x42100000)));
        assert_eq!(info.memory_map.len(), 1);
    }
}
