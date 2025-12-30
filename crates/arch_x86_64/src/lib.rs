#![no_std]
#![feature(abi_x86_interrupt)]

extern crate alloc;

use core::sync::atomic::{AtomicU64, Ordering};

use pic8259::ChainedPics;
use spin::{Mutex, Once};
use x86_64::instructions::interrupts;
use x86_64::instructions::port::Port;
use x86_64::instructions::segmentation::Segment;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

mod keyboard;
mod usb_input;
mod virtio_input;
mod vga;

/// Primary 8259 PIC offset for hardware interrupts.
pub const PIC_1_OFFSET: u8 = 32;
/// Secondary 8259 PIC offset for hardware interrupts.
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

const DOUBLE_FAULT_IST_INDEX: u16 = 0;
const SERIAL_PORT: u16 = 0x3F8;
const STACK_SIZE: usize = 4096 * 5;
const PIT_COMMAND_PORT: u16 = 0x43;
const PIT_CHANNEL0_PORT: u16 = 0x40;
const PIT_BASE_FREQUENCY: u32 = 1_193_182;

static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });
static TICKS: AtomicU64 = AtomicU64::new(0);
static HHDM_OFFSET: AtomicU64 = AtomicU64::new(0);
static KERNEL_VIRT_BASE: AtomicU64 = AtomicU64::new(0);
static KERNEL_PHYS_BASE: AtomicU64 = AtomicU64::new(0);

static TSS: Once<TaskStateSegment> = Once::new();
static GDT: Once<(GlobalDescriptorTable, Selectors)> = Once::new();
static IDT: Once<InterruptDescriptorTable> = Once::new();

pub use keyboard::{keyboard_has_data, keyboard_init, keyboard_read_byte};
pub use usb_input::{usb_input_has_data, usb_input_init, usb_input_read_byte};
pub use virtio_input::{virtio_input_has_data, virtio_input_init, virtio_input_read_byte};
pub use vga::{vga_init, vga_write_str};

/// Stores memory offsets used for MMIO and DMA translations.
pub fn set_memory_offsets(hhdm_offset: u64, kernel_virtual_base: u64, kernel_physical_base: u64) {
    HHDM_OFFSET.store(hhdm_offset, Ordering::Relaxed);
    KERNEL_VIRT_BASE.store(kernel_virtual_base, Ordering::Relaxed);
    KERNEL_PHYS_BASE.store(kernel_physical_base, Ordering::Relaxed);
}

/// Converts a physical address to a higher-half direct map virtual address.
pub fn phys_to_virt(phys: u64) -> *mut u8 {
    let offset = HHDM_OFFSET.load(Ordering::Relaxed);
    (phys + offset) as *mut u8
}

/// Converts a kernel virtual address to a physical address.
pub fn virt_to_phys(ptr: *const u8) -> u64 {
    let virt = ptr as u64;
    let virt_base = KERNEL_VIRT_BASE.load(Ordering::Relaxed);
    let phys_base = KERNEL_PHYS_BASE.load(Ordering::Relaxed);
    virt.saturating_sub(virt_base).saturating_add(phys_base)
}

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

/// Initializes CPU structures, interrupts, and timers for x86_64.
pub fn init() {
    init_gdt();
    init_idt();
    init_pic();
    init_pit(100);
}

/// Enables hardware interrupts.
pub fn enable_interrupts() {
    interrupts::enable();
}

/// Busy-loop with the `hlt` instruction.
pub fn halt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

/// Writes a byte to the legacy serial port.
pub fn serial_write_byte(byte: u8) {
    unsafe {
        while !serial_transmit_empty() {}
        Port::new(SERIAL_PORT).write(byte);
    }
}

/// Initializes the legacy serial port for early logging.
pub fn init_serial() {
    unsafe {
        Port::new(SERIAL_PORT + 1).write(0x00u8);
        Port::new(SERIAL_PORT + 3).write(0x80u8);
        Port::new(SERIAL_PORT + 0).write(0x03u8);
        Port::new(SERIAL_PORT + 1).write(0x00u8);
        Port::new(SERIAL_PORT + 3).write(0x03u8);
        Port::new(SERIAL_PORT + 2).write(0xC7u8);
        Port::new(SERIAL_PORT + 4).write(0x0Bu8);
    }
}

/// Writes a string to the legacy serial port.
pub fn serial_write_str(text: &str) {
    for byte in text.bytes() {
        if byte == b'\n' {
            serial_write_byte(b'\r');
        }
        serial_write_byte(byte);
    }
}

/// Returns true when a byte is available on the legacy serial port.
pub fn serial_has_data() -> bool {
    unsafe {
        let mut port = Port::new(SERIAL_PORT + 5);
        let value: u8 = port.read();
        value & 0x01 != 0
    }
}

/// Reads a byte from the legacy serial port (caller must ensure data is available).
pub fn serial_read_byte() -> u8 {
    unsafe {
        let mut port = Port::new(SERIAL_PORT);
        port.read()
    }
}

/// Returns the number of timer ticks since boot.
pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

/// Acknowledges the given IRQ line.
pub fn acknowledge_irq(irq: u8) {
    unsafe {
        PICS.lock().notify_end_of_interrupt(irq);
    }
}

fn init_gdt() {
    let (gdt, selectors) = GDT.call_once(|| {
        let tss = TSS.call_once(build_tss);
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        let tss_selector = gdt.append(Descriptor::tss_segment(tss));
        (gdt, Selectors { code_selector, tss_selector })
    });

    gdt.load();
    unsafe {
        x86_64::instructions::segmentation::CS::set_reg(selectors.code_selector);
        x86_64::instructions::tables::load_tss(selectors.tss_selector);
    }
}

fn init_idt() {
    let idt = IDT.call_once(|| {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(DOUBLE_FAULT_IST_INDEX);
        }
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.general_protection_fault
            .set_handler_fn(general_protection_handler);
        idt[InterruptIndex::Timer.as_u8()].set_handler_fn(timer_interrupt_handler);
        idt
    });
    idt.load();
}

fn init_pic() {
    unsafe {
        PICS.lock().initialize();
    }
}

fn init_pit(frequency_hz: u32) {
    let divisor = PIT_BASE_FREQUENCY / frequency_hz.max(1);
    unsafe {
        Port::new(PIT_COMMAND_PORT).write(0x36u8);
        Port::new(PIT_CHANNEL0_PORT).write((divisor & 0xFF) as u8);
        Port::new(PIT_CHANNEL0_PORT).write((divisor >> 8) as u8);
    }
}

fn serial_transmit_empty() -> bool {
    unsafe { Port::<u8>::new(SERIAL_PORT + 5).read() & 0x20 != 0 }
}

fn build_tss() -> TaskStateSegment {
    static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

    let mut tss = TaskStateSegment::new();
    let stack_start = VirtAddr::from_ptr(&raw const STACK);
    let stack_end = stack_start + STACK_SIZE as u64;
    tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = stack_end;
    tss
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum InterruptIndex {
    Timer = PIC_1_OFFSET,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }
}

extern "x86-interrupt" fn breakpoint_handler(_stack: InterruptStackFrame) {}

extern "x86-interrupt" fn double_fault_handler(
    _stack: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    halt_loop();
}

extern "x86-interrupt" fn page_fault_handler(
    _stack: InterruptStackFrame,
    _error_code: PageFaultErrorCode,
) {
    halt_loop();
}

extern "x86-interrupt" fn general_protection_handler(
    _stack: InterruptStackFrame,
    _error_code: u64,
) {
    halt_loop();
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack: InterruptStackFrame) {
    TICKS.fetch_add(1, Ordering::Relaxed);
    acknowledge_irq(InterruptIndex::Timer as u8);
}
