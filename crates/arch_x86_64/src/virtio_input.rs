use alloc::boxed::Box;
use alloc::vec::Vec;
use core::alloc::Layout;
use core::cmp::min;
use core::mem::{align_of, size_of};
use core::ptr::{read_volatile, write_volatile};

use spin::Mutex;
use x86_64::instructions::port::Port;

use crate::virt_to_phys;

const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

const VIRTIO_VENDOR_ID: u16 = 0x1AF4;
const VIRTIO_DEVICE_ID_LEGACY_INPUT: u16 = 0x1012;

const VIRTIO_PCI_HOST_FEATURES: u16 = 0x00;
const VIRTIO_PCI_GUEST_FEATURES: u16 = 0x04;
const VIRTIO_PCI_QUEUE_PFN: u16 = 0x08;
const VIRTIO_PCI_QUEUE_NUM: u16 = 0x0C;
const VIRTIO_PCI_QUEUE_SEL: u16 = 0x0E;
const VIRTIO_PCI_QUEUE_NOTIFY: u16 = 0x10;
const VIRTIO_PCI_STATUS: u16 = 0x12;

const VIRTIO_STATUS_ACKNOWLEDGE: u8 = 0x01;
const VIRTIO_STATUS_DRIVER: u8 = 0x02;
const VIRTIO_STATUS_FEATURES_OK: u8 = 0x08;
const VIRTIO_STATUS_DRIVER_OK: u8 = 0x04;

const VIRTQ_DESC_F_WRITE: u16 = 0x2;

const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;

const KEY_LEFTSHIFT: u16 = 42;
const KEY_RIGHTSHIFT: u16 = 54;

static INPUT_STATE: Mutex<InputState> = Mutex::new(InputState::new());

struct InputState {
    virtio: Option<VirtioInput>,
    buffer: RingBuffer,
}

impl InputState {
    const fn new() -> Self {
        Self {
            virtio: None,
            buffer: RingBuffer::new(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct VirtioInputEvent {
    type_: u16,
    code: u16,
    value: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct VirtqDesc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct VirtqUsedElem {
    id: u32,
    len: u32,
}

struct VirtioInput {
    base_port: u16,
    queue_size: u16,
    desc: *mut VirtqDesc,
    avail: *mut u8,
    used: *mut u8,
    events: Box<[VirtioInputEvent]>,
    avail_idx: u16,
    used_idx: u16,
    shift: bool,
}

unsafe impl Send for VirtioInput {}

pub fn virtio_input_init() {
    let mut state = INPUT_STATE.lock();
    if state.virtio.is_some() {
        return;
    }
    let Some(dev) = find_virtio_input_device() else {
        return;
    };
    if let Some(input) = VirtioInput::new(dev) {
        state.virtio = Some(input);
    }
}

pub fn virtio_input_has_data() -> bool {
    let mut state = INPUT_STATE.lock();
    let buffer_ptr: *mut RingBuffer = &mut state.buffer;
    if let Some(input) = state.virtio.as_mut() {
        unsafe {
            input.poll(&mut *buffer_ptr);
        }
    }
    !state.buffer.is_empty()
}

pub fn virtio_input_read_byte() -> Option<u8> {
    let mut state = INPUT_STATE.lock();
    let buffer_ptr: *mut RingBuffer = &mut state.buffer;
    if let Some(input) = state.virtio.as_mut() {
        unsafe {
            input.poll(&mut *buffer_ptr);
        }
    }
    state.buffer.pop()
}

#[derive(Clone, Copy)]
struct PciDevice {
    io_base: u16,
}

fn find_virtio_input_device() -> Option<PciDevice> {
    for bus in 0u8..=0xff {
        for device in 0u8..32 {
            let header = pci_config_read16(bus, device, 0, 0x0E);
            if header == 0xFFFF {
                continue;
            }
            let functions = if header & 0x80 != 0 { 8 } else { 1 };
            for function in 0u8..functions {
                let vendor = pci_config_read16(bus, device, function, 0x00);
                if vendor == 0xFFFF {
                    continue;
                }
                if vendor != VIRTIO_VENDOR_ID {
                    continue;
                }
                let device_id = pci_config_read16(bus, device, function, 0x02);
                if device_id != VIRTIO_DEVICE_ID_LEGACY_INPUT {
                    continue;
                }
                let bar0 = pci_config_read32(bus, device, function, 0x10);
                if bar0 & 0x1 == 0 {
                    continue;
                }
                let io_base = (bar0 & 0xFFFC) as u16;
                enable_pci_io_master(bus, device, function);
                return Some(PciDevice { io_base });
            }
        }
    }
    None
}

fn enable_pci_io_master(bus: u8, device: u8, function: u8) {
    let mut cmd = pci_config_read16(bus, device, function, 0x04);
    cmd |= 0x1 | 0x4;
    pci_config_write16(bus, device, function, 0x04, cmd);
}

fn pci_config_read32(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let address = 0x8000_0000u32
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | (offset as u32 & 0xFC);
    unsafe {
        let mut addr_port: Port<u32> = Port::new(PCI_CONFIG_ADDRESS);
        addr_port.write(address);
        let mut data_port: Port<u32> = Port::new(PCI_CONFIG_DATA);
        data_port.read()
    }
}

fn pci_config_read16(bus: u8, device: u8, function: u8, offset: u8) -> u16 {
    let value = pci_config_read32(bus, device, function, offset);
    let shift = (offset & 2) * 8;
    ((value >> shift) & 0xFFFF) as u16
}

fn pci_config_write16(bus: u8, device: u8, function: u8, offset: u8, value: u16) {
    let mut current = pci_config_read32(bus, device, function, offset);
    let shift = (offset & 2) * 8;
    current &= !(0xFFFFu32 << shift);
    current |= (value as u32) << shift;
    pci_config_write32(bus, device, function, offset, current);
}

fn pci_config_write32(bus: u8, device: u8, function: u8, offset: u8, value: u32) {
    let address = 0x8000_0000u32
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | (offset as u32 & 0xFC);
    unsafe {
        let mut addr_port: Port<u32> = Port::new(PCI_CONFIG_ADDRESS);
        addr_port.write(address);
        let mut data_port: Port<u32> = Port::new(PCI_CONFIG_DATA);
        data_port.write(value);
    }
}

impl VirtioInput {
    fn new(dev: PciDevice) -> Option<Self> {
        let base_port = dev.io_base;
        write_port_u8(base_port + VIRTIO_PCI_STATUS, 0);

        let mut status = VIRTIO_STATUS_ACKNOWLEDGE;
        write_port_u8(base_port + VIRTIO_PCI_STATUS, status);
        status |= VIRTIO_STATUS_DRIVER;
        write_port_u8(base_port + VIRTIO_PCI_STATUS, status);

        let _features = read_port_u32(base_port + VIRTIO_PCI_HOST_FEATURES);
        write_port_u32(base_port + VIRTIO_PCI_GUEST_FEATURES, 0);

        status |= VIRTIO_STATUS_FEATURES_OK;
        write_port_u8(base_port + VIRTIO_PCI_STATUS, status);

        write_port_u16(base_port + VIRTIO_PCI_QUEUE_SEL, 0);
        let queue_size = read_port_u16(base_port + VIRTIO_PCI_QUEUE_NUM);
        if queue_size == 0 {
            return None;
        }

        let queue_size = min(queue_size, 64);
        let (queue_mem, desc, avail, used) = alloc_queue(queue_size)?;
        let events = alloc_events(queue_size as usize);

        let queue_pfn = virt_to_phys(queue_mem as *const u8) >> 12;
        write_port_u32(base_port + VIRTIO_PCI_QUEUE_PFN, queue_pfn as u32);

        let mut input = Self {
            base_port,
            queue_size,
            desc,
            avail,
            used,
            events,
            avail_idx: 0,
            used_idx: 0,
            shift: false,
        };
        input.fill_queue();
        status |= VIRTIO_STATUS_DRIVER_OK;
        write_port_u8(base_port + VIRTIO_PCI_STATUS, status);
        input.notify_queue();
        Some(input)
    }

    fn fill_queue(&mut self) {
        let ring_ptr = self.avail_ring_ptr();
        for i in 0..self.queue_size {
            let desc = unsafe { &mut *self.desc.add(i as usize) };
            let event_ptr = &self.events[i as usize] as *const VirtioInputEvent as *const u8;
            desc.addr = virt_to_phys(event_ptr);
            desc.len = size_of::<VirtioInputEvent>() as u32;
            desc.flags = VIRTQ_DESC_F_WRITE;
            desc.next = 0;
            unsafe {
                write_volatile(ring_ptr.add(i as usize), i);
            }
        }
        self.avail_idx = self.queue_size;
        unsafe {
            write_volatile(self.avail_idx_ptr(), self.avail_idx);
        }
    }

    fn poll(&mut self, buffer: &mut RingBuffer) {
        let used_idx = unsafe { read_volatile(self.used_idx_ptr()) };
        while self.used_idx != used_idx {
            let ring_index = (self.used_idx % self.queue_size) as usize;
            let elem = unsafe { read_volatile(self.used_ring_ptr().add(ring_index)) };
            let desc_index = elem.id as usize;
            if desc_index < self.events.len() {
                let event = self.events[desc_index];
                self.handle_event(event, buffer);
            }
            self.requeue(desc_index as u16);
            self.used_idx = self.used_idx.wrapping_add(1);
        }
    }

    fn handle_event(&mut self, event: VirtioInputEvent, buffer: &mut RingBuffer) {
        if event.type_ == EV_SYN {
            return;
        }
        if event.type_ != EV_KEY {
            return;
        }
        let pressed = event.value == 1 || event.value == 2;
        if event.code == KEY_LEFTSHIFT || event.code == KEY_RIGHTSHIFT {
            self.shift = pressed;
            return;
        }
        if !pressed {
            return;
        }
        if let Some(byte) = keycode_to_ascii(event.code, self.shift) {
            buffer.push(byte);
        }
    }

    fn requeue(&mut self, desc_index: u16) {
        let ring_ptr = self.avail_ring_ptr();
        let idx = self.avail_idx;
        unsafe {
            write_volatile(ring_ptr.add((idx % self.queue_size) as usize), desc_index);
        }
        self.avail_idx = idx.wrapping_add(1);
        unsafe {
            write_volatile(self.avail_idx_ptr(), self.avail_idx);
        }
        self.notify_queue();
    }

    fn notify_queue(&self) {
        write_port_u16(self.base_port + VIRTIO_PCI_QUEUE_NOTIFY, 0);
    }

    fn avail_idx_ptr(&self) -> *mut u16 {
        unsafe { (self.avail as *mut u16).add(1) }
    }

    fn avail_ring_ptr(&self) -> *mut u16 {
        unsafe { (self.avail as *mut u16).add(2) }
    }

    fn used_idx_ptr(&self) -> *mut u16 {
        unsafe { (self.used as *mut u16).add(1) }
    }

    fn used_ring_ptr(&self) -> *mut VirtqUsedElem {
        unsafe { (self.used as *mut u16).add(2) as *mut VirtqUsedElem }
    }
}

fn alloc_events(count: usize) -> Box<[VirtioInputEvent]> {
    let mut events = Vec::with_capacity(count);
    events.resize_with(count, VirtioInputEvent::default);
    events.into_boxed_slice()
}

fn alloc_queue(queue_size: u16) -> Option<(*mut u8, *mut VirtqDesc, *mut u8, *mut u8)> {
    let desc_size = size_of::<VirtqDesc>() * queue_size as usize;
    let avail_size = 4 + 2 * queue_size as usize + 2;
    let used_align = 4usize.max(align_of::<VirtqUsedElem>());
    let used_offset = align_up(desc_size + avail_size, used_align);
    let used_size = 4 + size_of::<VirtqUsedElem>() * queue_size as usize + 2;
    let total = align_up(used_offset + used_size, 4096);
    let layout = Layout::from_size_align(total, 4096).ok()?;
    let mem = unsafe { alloc::alloc::alloc_zeroed(layout) };
    if mem.is_null() {
        return None;
    }
    let desc = mem as *mut VirtqDesc;
    let avail = unsafe { mem.add(desc_size) };
    let used = unsafe { mem.add(used_offset) };
    Some((mem, desc, avail, used))
}

fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

fn read_port_u32(port: u16) -> u32 {
    unsafe {
        let mut p: Port<u32> = Port::new(port);
        p.read()
    }
}

fn read_port_u16(port: u16) -> u16 {
    unsafe {
        let mut p: Port<u16> = Port::new(port);
        p.read()
    }
}

fn write_port_u32(port: u16, value: u32) {
    unsafe {
        let mut p: Port<u32> = Port::new(port);
        p.write(value);
    }
}

fn write_port_u16(port: u16, value: u16) {
    unsafe {
        let mut p: Port<u16> = Port::new(port);
        p.write(value);
    }
}

fn write_port_u8(port: u16, value: u8) {
    unsafe {
        let mut p: Port<u8> = Port::new(port);
        p.write(value);
    }
}

fn keycode_to_ascii(code: u16, shift: bool) -> Option<u8> {
    match code {
        2 => Some(if shift { b'!' } else { b'1' }),
        3 => Some(if shift { b'@' } else { b'2' }),
        4 => Some(if shift { b'#' } else { b'3' }),
        5 => Some(if shift { b'$' } else { b'4' }),
        6 => Some(if shift { b'%' } else { b'5' }),
        7 => Some(if shift { b'^' } else { b'6' }),
        8 => Some(if shift { b'&' } else { b'7' }),
        9 => Some(if shift { b'*' } else { b'8' }),
        10 => Some(if shift { b'(' } else { b'9' }),
        11 => Some(if shift { b')' } else { b'0' }),
        12 => Some(if shift { b'_' } else { b'-' }),
        13 => Some(if shift { b'+' } else { b'=' }),
        14 => Some(0x08),
        15 => Some(b'\t'),
        16 => Some(if shift { b'Q' } else { b'q' }),
        17 => Some(if shift { b'W' } else { b'w' }),
        18 => Some(if shift { b'E' } else { b'e' }),
        19 => Some(if shift { b'R' } else { b'r' }),
        20 => Some(if shift { b'T' } else { b't' }),
        21 => Some(if shift { b'Y' } else { b'y' }),
        22 => Some(if shift { b'U' } else { b'u' }),
        23 => Some(if shift { b'I' } else { b'i' }),
        24 => Some(if shift { b'O' } else { b'o' }),
        25 => Some(if shift { b'P' } else { b'p' }),
        26 => Some(if shift { b'{' } else { b'[' }),
        27 => Some(if shift { b'}' } else { b']' }),
        28 => Some(b'\n'),
        30 => Some(if shift { b'A' } else { b'a' }),
        31 => Some(if shift { b'S' } else { b's' }),
        32 => Some(if shift { b'D' } else { b'd' }),
        33 => Some(if shift { b'F' } else { b'f' }),
        34 => Some(if shift { b'G' } else { b'g' }),
        35 => Some(if shift { b'H' } else { b'h' }),
        36 => Some(if shift { b'J' } else { b'j' }),
        37 => Some(if shift { b'K' } else { b'k' }),
        38 => Some(if shift { b'L' } else { b'l' }),
        39 => Some(if shift { b':' } else { b';' }),
        40 => Some(if shift { b'"' } else { b'\'' }),
        41 => Some(if shift { b'~' } else { b'`' }),
        43 => Some(if shift { b'|' } else { b'\\' }),
        44 => Some(if shift { b'Z' } else { b'z' }),
        45 => Some(if shift { b'X' } else { b'x' }),
        46 => Some(if shift { b'C' } else { b'c' }),
        47 => Some(if shift { b'V' } else { b'v' }),
        48 => Some(if shift { b'B' } else { b'b' }),
        49 => Some(if shift { b'N' } else { b'n' }),
        50 => Some(if shift { b'M' } else { b'm' }),
        51 => Some(if shift { b'<' } else { b',' }),
        52 => Some(if shift { b'>' } else { b'.' }),
        53 => Some(if shift { b'?' } else { b'/' }),
        57 => Some(b' '),
        _ => None,
    }
}

struct RingBuffer {
    data: [u8; 128],
    head: usize,
    tail: usize,
    len: usize,
}

impl RingBuffer {
    const fn new() -> Self {
        Self {
            data: [0; 128],
            head: 0,
            tail: 0,
            len: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn push(&mut self, value: u8) {
        if self.len == self.data.len() {
            return;
        }
        self.data[self.tail] = value;
        self.tail = (self.tail + 1) % self.data.len();
        self.len += 1;
    }

    fn pop(&mut self) -> Option<u8> {
        if self.len == 0 {
            return None;
        }
        let value = self.data[self.head];
        self.head = (self.head + 1) % self.data.len();
        self.len -= 1;
        Some(value)
    }
}
