use alloc::vec::Vec;
use core::alloc::Layout;
use core::ptr::{read_volatile, write_volatile};

use spin::Mutex;
use x86_64::instructions::port::Port;

use crate::{phys_to_virt, virt_to_phys};

const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

const USB_CLASS_SERIAL: u8 = 0x0C;
const USB_SUBCLASS_USB: u8 = 0x03;
const USB_PROG_IF_XHCI: u8 = 0x30;

const XHCI_TRB_TYPE_NORMAL: u32 = 1;
const XHCI_TRB_TYPE_SETUP: u32 = 2;
const XHCI_TRB_TYPE_DATA: u32 = 3;
const XHCI_TRB_TYPE_STATUS: u32 = 4;
const XHCI_TRB_TYPE_LINK: u32 = 6;
const XHCI_TRB_TYPE_ENABLE_SLOT: u32 = 9;
const XHCI_TRB_TYPE_ADDRESS_DEVICE: u32 = 11;
const XHCI_TRB_TYPE_CONFIGURE_ENDPOINT: u32 = 12;
const XHCI_TRB_TYPE_TRANSFER_EVENT: u32 = 32;
const XHCI_TRB_TYPE_CMD_COMPLETION: u32 = 33;

const XHCI_COMPLETION_SUCCESS: u32 = 1;

const EP_TYPE_CONTROL: u32 = 4;
const EP_TYPE_INTERRUPT_IN: u32 = 7;

const EP0_DCI: u8 = 1;
const EP1_IN_DCI: u8 = 3;

const SETUP_STAGE_IN: u32 = 1;
const SETUP_STAGE_OUT: u32 = 0;

const USB_REQ_GET_DESCRIPTOR: u8 = 0x06;
const USB_REQ_SET_CONFIGURATION: u8 = 0x09;
const USB_REQ_SET_PROTOCOL: u8 = 0x0B;

const USB_DESC_DEVICE: u8 = 1;
const USB_DESC_CONFIG: u8 = 2;

const HID_PROTOCOL_BOOT: u16 = 0;

const KEYBOARD_REPORT_LEN: usize = 8;

static USB_STATE: Mutex<UsbState> = Mutex::new(UsbState::new());

struct UsbState {
    controller: Option<XhciController>,
    buffer: RingBuffer,
    keyboard: HidKeyboard,
}

impl UsbState {
    const fn new() -> Self {
        Self {
            controller: None,
            buffer: RingBuffer::new(),
            keyboard: HidKeyboard::new(),
        }
    }
}

pub fn usb_input_init() {
    let mut state = USB_STATE.lock();
    if state.controller.is_some() {
        return;
    }
    let Some(dev) = find_xhci_device() else {
        return;
    };
    if let Some(controller) = XhciController::new(dev) {
        state.controller = Some(controller);
    }
}

pub fn usb_input_has_data() -> bool {
    let mut state = USB_STATE.lock();
    let controller_ptr = state
        .controller
        .as_mut()
        .map(|controller| controller as *mut XhciController);
    if let Some(controller_ptr) = controller_ptr {
        let keyboard_ptr = &mut state.keyboard as *mut HidKeyboard;
        let buffer_ptr = &mut state.buffer as *mut RingBuffer;
        unsafe {
            (*controller_ptr).poll(&mut *keyboard_ptr, &mut *buffer_ptr);
        }
    }
    !state.buffer.is_empty()
}

pub fn usb_input_read_byte() -> Option<u8> {
    let mut state = USB_STATE.lock();
    let controller_ptr = state
        .controller
        .as_mut()
        .map(|controller| controller as *mut XhciController);
    if let Some(controller_ptr) = controller_ptr {
        let keyboard_ptr = &mut state.keyboard as *mut HidKeyboard;
        let buffer_ptr = &mut state.buffer as *mut RingBuffer;
        unsafe {
            (*controller_ptr).poll(&mut *keyboard_ptr, &mut *buffer_ptr);
        }
    }
    state.buffer.pop()
}

#[derive(Clone, Copy)]
struct PciDevice {
    mmio_base: u64,
}

fn find_xhci_device() -> Option<PciDevice> {
    for bus in 0u8..=0xff {
        for device in 0u8..32 {
            let header = pci_config_read16(bus, device, 0, 0x0E);
            if header == 0xFFFF {
                continue;
            }
            let functions = if header & 0x80 != 0 { 8 } else { 1 };
            for function in 0u8..functions {
                let class = pci_config_read8(bus, device, function, 0x0B);
                let subclass = pci_config_read8(bus, device, function, 0x0A);
                let prog_if = pci_config_read8(bus, device, function, 0x09);
                if class != USB_CLASS_SERIAL || subclass != USB_SUBCLASS_USB || prog_if != USB_PROG_IF_XHCI {
                    continue;
                }
                let bar0 = pci_config_read32(bus, device, function, 0x10);
                let bar1 = pci_config_read32(bus, device, function, 0x14);
                let mmio = if bar0 & 0x4 != 0 {
                    ((bar1 as u64) << 32) | (bar0 as u64 & 0xFFFF_FFF0)
                } else {
                    (bar0 as u64) & 0xFFFF_FFF0
                };
                if mmio == 0 {
                    continue;
                }
                enable_pci_mmio_master(bus, device, function);
                return Some(PciDevice { mmio_base: mmio });
            }
        }
    }
    None
}

fn enable_pci_mmio_master(bus: u8, device: u8, function: u8) {
    let mut cmd = pci_config_read16(bus, device, function, 0x04);
    cmd |= 0x2 | 0x4;
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

fn pci_config_read8(bus: u8, device: u8, function: u8, offset: u8) -> u8 {
    let value = pci_config_read32(bus, device, function, offset);
    let shift = (offset & 3) * 8;
    ((value >> shift) & 0xFF) as u8
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

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Trb {
    parameter: u64,
    status: u32,
    control: u32,
}

impl Trb {
    fn new(parameter: u64, status: u32, control: u32) -> Self {
        Self {
            parameter,
            status,
            control,
        }
    }

    fn trb_type(&self) -> u32 {
        (self.control >> 10) & 0x3F
    }

    fn completion_code(&self) -> u32 {
        (self.status >> 24) & 0xFF
    }

    fn slot_id(&self) -> u8 {
        ((self.control >> 24) & 0xFF) as u8
    }
}

struct Ring {
    trbs: *mut Trb,
    phys: u64,
    size: u16,
    index: u16,
    cycle: bool,
}

impl Ring {
    fn new(size: u16) -> Option<Self> {
        let bytes = size_of_trb() * size as usize;
        let layout = Layout::from_size_align(bytes, 64).ok()?;
        let mem = unsafe { alloc::alloc::alloc_zeroed(layout) };
        if mem.is_null() {
            return None;
        }
        let trbs = mem as *mut Trb;
        let phys = virt_to_phys(mem);
        let mut ring = Self {
            trbs,
            phys,
            size,
            index: 0,
            cycle: true,
        };
        ring.setup_link_trb();
        Some(ring)
    }

    fn setup_link_trb(&mut self) {
        let link_index = self.size - 1;
        let link_ptr = self.phys;
        let control = build_trb_control(XHCI_TRB_TYPE_LINK, self.cycle)
            | (1 << 1); // toggle cycle
        unsafe {
            write_volatile(self.trbs.add(link_index as usize), Trb::new(link_ptr, 0, control));
        }
    }

    fn push(&mut self, mut trb: Trb) -> u16 {
        trb.control |= cycle_bit(self.cycle);
        let index = self.index;
        unsafe {
            write_volatile(self.trbs.add(index as usize), trb);
        }
        self.index += 1;
        if self.index >= self.size - 1 {
            self.setup_link_trb();
            self.index = 0;
            self.cycle = !self.cycle;
        }
        index
    }

    fn phys_addr(&self) -> u64 {
        self.phys
    }
}

struct EventRing {
    trbs: *mut Trb,
    phys: u64,
    size: u16,
    index: u16,
    cycle: bool,
    erst_phys: u64,
}

impl EventRing {
    fn new(size: u16) -> Option<Self> {
        let bytes = size_of_trb() * size as usize;
        let layout = Layout::from_size_align(bytes, 64).ok()?;
        let mem = unsafe { alloc::alloc::alloc_zeroed(layout) };
        if mem.is_null() {
            return None;
        }
        let trbs = mem as *mut Trb;
        let phys = virt_to_phys(mem);

        let erst_layout = Layout::from_size_align(size_of::<ErstEntry>(), 64).ok()?;
        let erst_mem = unsafe { alloc::alloc::alloc_zeroed(erst_layout) };
        if erst_mem.is_null() {
            return None;
        }
        let erst = erst_mem as *mut ErstEntry;
        let erst_phys = virt_to_phys(erst_mem);
        unsafe {
            write_volatile(
                erst,
                ErstEntry {
                    segment_base: phys,
                    segment_size: size as u32,
                    _rsvd: 0,
                },
            );
        }

        Some(Self {
            trbs,
            phys,
            size,
            index: 0,
            cycle: true,
            erst_phys,
        })
    }

    fn pop(&mut self) -> Option<Trb> {
        let trb = unsafe { read_volatile(self.trbs.add(self.index as usize)) };
        let trb_cycle = (trb.control & 1) != 0;
        if trb_cycle != self.cycle {
            return None;
        }
        self.index = (self.index + 1) % self.size;
        if self.index == 0 {
            self.cycle = !self.cycle;
        }
        Some(trb)
    }

    fn phys_addr(&self) -> u64 {
        self.phys
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ErstEntry {
    segment_base: u64,
    segment_size: u32,
    _rsvd: u32,
}

struct XhciController {
    op_base: *mut u8,
    rt_base: *mut u8,
    db_base: *mut u8,
    max_ports: u8,
    max_slots: u8,
    context_size: usize,
    cmd_ring: Ring,
    event_ring: EventRing,
    dcbaa: *mut u64,
    dcbaa_phys: u64,
    input_ctx: *mut u32,
    input_ctx_phys: u64,
    device_ctx: *mut u32,
    device_ctx_phys: u64,
    ep0_ring: Ring,
    intr_ring: Ring,
    slot_id: u8,
    port_id: u8,
    report_buffers: Vec<*mut u8>,
}

unsafe impl Send for XhciController {}

impl XhciController {
    fn new(dev: PciDevice) -> Option<Self> {
        let mmio = phys_to_virt(dev.mmio_base);
        if mmio.is_null() {
            return None;
        }
        let cap_len = unsafe { read_volatile(mmio as *const u8) } as usize;
        let hcsparams1 = unsafe { mmio_read32(mmio, 0x04) };
        let hccparams1 = unsafe { mmio_read32(mmio, 0x10) };
        let dboff = unsafe { mmio_read32(mmio, 0x14) } as usize;
        let rtsoff = unsafe { mmio_read32(mmio, 0x18) } as usize;
        let max_slots = (hcsparams1 & 0xFF) as u8;
        let max_ports = ((hcsparams1 >> 24) & 0xFF) as u8;
        let context_size = if (hccparams1 >> 2) & 1 == 1 { 64 } else { 32 };

        let op_base = unsafe { mmio.add(cap_len) };
        let rt_base = unsafe { mmio.add(rtsoff) };
        let db_base = unsafe { mmio.add(dboff) };

        let cmd_ring = Ring::new(32)?;
        let event_ring = EventRing::new(32)?;
        let (dcbaa, dcbaa_phys) = alloc_zeroed_u64((max_slots as usize) + 1)?;

        let (input_ctx, input_ctx_phys) = alloc_zeroed_ctx(context_size, 33)?;
        let (device_ctx, device_ctx_phys) = alloc_zeroed_ctx(context_size, 33)?;
        let ep0_ring = Ring::new(32)?;
        let intr_ring = Ring::new(32)?;

        let mut controller = Self {
            op_base,
            rt_base,
            db_base,
            max_ports,
            max_slots,
            context_size,
            cmd_ring,
            event_ring,
            dcbaa,
            dcbaa_phys,
            input_ctx,
            input_ctx_phys,
            device_ctx,
            device_ctx_phys,
            ep0_ring,
            intr_ring,
            slot_id: 0,
            port_id: 0,
            report_buffers: Vec::new(),
        };

        controller.reset();
        controller.configure();
        controller.start();
        controller.init_device()?;
        Some(controller)
    }

    fn reset(&mut self) {
        unsafe {
            let usbcmd = mmio_read32(self.op_base, 0x00);
            mmio_write32(self.op_base, 0x00, usbcmd | (1 << 1));
            for _ in 0..1_000_000 {
                let cmd = mmio_read32(self.op_base, 0x00);
                if cmd & (1 << 1) == 0 {
                    break;
                }
            }
        }
    }

    fn configure(&mut self) {
        unsafe {
            mmio_write32(self.op_base, 0x38, self.max_slots as u32);
            mmio_write64(self.op_base, 0x30, self.dcbaa_phys);
            mmio_write64(self.op_base, 0x18, self.cmd_ring.phys_addr() | 1);
            self.init_event_ring();
        }
    }

    fn start(&mut self) {
        unsafe {
            let usbcmd = mmio_read32(self.op_base, 0x00);
            mmio_write32(self.op_base, 0x00, usbcmd | 0x1);
            for _ in 0..1_000_000 {
                let status = mmio_read32(self.op_base, 0x04);
                if status & (1 << 0) == 0 {
                    break;
                }
            }
        }
    }

    fn init_event_ring(&mut self) {
        unsafe {
            let iman = self.rt_base.add(0x20);
            mmio_write32(iman, 0x00, 0x2);
            let erstsz = self.rt_base.add(0x28);
            mmio_write32(erstsz, 0x00, 1);
            let erstba = self.rt_base.add(0x30);
            mmio_write64(erstba, 0x00, self.event_ring.erst_phys);
            let erdp = self.rt_base.add(0x38);
            mmio_write64(erdp, 0x00, self.event_ring.phys_addr());
        }
    }

    fn init_device(&mut self) -> Option<()> {
        let port = self.find_port()?;
        self.port_id = port;
        self.reset_port(port)?;
        self.enable_slot()?;
        self.address_device()?;
        self.configure_keyboard()?;
        Some(())
    }

    fn find_port(&self) -> Option<u8> {
        for port in 1..=self.max_ports {
            let portsc = self.portsc(port);
            if portsc & 0x1 != 0 {
                return Some(port);
            }
        }
        None
    }

    fn reset_port(&self, port: u8) -> Option<()> {
        let portsc = self.portsc(port);
        unsafe {
            mmio_write32(self.portsc_ptr(port), 0x0, portsc | (1 << 4));
        }
        for _ in 0..1_000_000 {
            let status = self.portsc(port);
            if status & (1 << 4) == 0 && status & (1 << 1) != 0 {
                return Some(());
            }
        }
        None
    }

    fn enable_slot(&mut self) -> Option<()> {
        let trb = Trb::new(0, 0, build_trb_control(XHCI_TRB_TYPE_ENABLE_SLOT, self.cmd_ring.cycle));
        self.cmd_ring.push(trb);
        self.ring_doorbell(0, 0);
        let event = self.wait_for_event(XHCI_TRB_TYPE_CMD_COMPLETION)?;
        if event.completion_code() != XHCI_COMPLETION_SUCCESS {
            return None;
        }
        self.slot_id = event.slot_id();
        Some(())
    }

    fn address_device(&mut self) -> Option<()> {
        unsafe {
            write_volatile(self.dcbaa.add(self.slot_id as usize), self.device_ctx_phys);
        }
        let mps = 8u16;
        let speed = self.port_speed(self.port_id);
        let ctx_size = self.context_size / 4;
        clear_context(self.input_ctx, ctx_size * 34);
        clear_context(self.device_ctx, ctx_size * 34);

        let add_flags = (1u32 << 0) | (1u32 << EP0_DCI);
        unsafe {
            write_volatile(self.input_ctx.add(1), add_flags);
        }
        let slot_ctx = unsafe { self.input_ctx.add(ctx_size) };
        write_slot_context(slot_ctx, speed, self.port_id, EP0_DCI);
        let ep0_ctx = unsafe { self.input_ctx.add(ctx_size * 2) };
        write_endpoint_context(
            ep0_ctx,
            EP_TYPE_CONTROL,
            mps,
            self.ep0_ring.phys_addr(),
            true,
            0,
            8,
        );

        let trb = Trb::new(
            self.input_ctx_phys,
            0,
            build_command_control(XHCI_TRB_TYPE_ADDRESS_DEVICE, self.cmd_ring.cycle, self.slot_id),
        );
        self.cmd_ring.push(trb);
        self.ring_doorbell(0, 0);
        let event = self.wait_for_event(XHCI_TRB_TYPE_CMD_COMPLETION)?;
        if event.completion_code() != XHCI_COMPLETION_SUCCESS {
            return None;
        }
        Some(())
    }

    fn configure_keyboard(&mut self) -> Option<()> {
        let mut device_desc = [0u8; 18];
        self.control_in(
            setup_get_descriptor(USB_DESC_DEVICE, 0, device_desc.len() as u16),
            &mut device_desc,
        )?;

        let mut config_header = [0u8; 9];
        self.control_in(
            setup_get_descriptor(USB_DESC_CONFIG, 0, config_header.len() as u16),
            &mut config_header,
        )?;

        self.control_out(setup_set_configuration(1))?;
        self.control_out(setup_set_protocol(0, HID_PROTOCOL_BOOT))?;

        let ctx_size = self.context_size / 4;
        clear_context(self.input_ctx, ctx_size * 34);
        let add_flags = (1u32 << 0) | (1u32 << EP1_IN_DCI);
        unsafe {
            write_volatile(self.input_ctx.add(1), add_flags);
        }
        let slot_ctx = unsafe { self.input_ctx.add(ctx_size) };
        write_slot_context(slot_ctx, self.port_speed(self.port_id), self.port_id, EP1_IN_DCI);
        let ep1_ctx = unsafe { self.input_ctx.add(ctx_size * (EP1_IN_DCI as usize + 1)) };
        write_endpoint_context(
            ep1_ctx,
            EP_TYPE_INTERRUPT_IN,
            KEYBOARD_REPORT_LEN as u16,
            self.intr_ring.phys_addr(),
            true,
            4,
            8,
        );

        let trb = Trb::new(
            self.input_ctx_phys,
            0,
            build_command_control(XHCI_TRB_TYPE_CONFIGURE_ENDPOINT, self.cmd_ring.cycle, self.slot_id),
        );
        self.cmd_ring.push(trb);
        self.ring_doorbell(0, 0);
        let event = self.wait_for_event(XHCI_TRB_TYPE_CMD_COMPLETION)?;
        if event.completion_code() != XHCI_COMPLETION_SUCCESS {
            return None;
        }

        self.queue_reports();
        Some(())
    }

    fn queue_reports(&mut self) {
        self.report_buffers.clear();
        for _ in 0..4 {
            if let Some(buf) = alloc_zeroed_bytes(KEYBOARD_REPORT_LEN, 16) {
                let trb = Trb::new(
                    virt_to_phys(buf),
                    KEYBOARD_REPORT_LEN as u32,
                    build_trb_control(XHCI_TRB_TYPE_NORMAL, self.intr_ring.cycle) | (1 << 5),
                );
                self.intr_ring.push(trb);
                self.report_buffers.push(buf);
            }
        }
        self.ring_doorbell(self.slot_id, EP1_IN_DCI);
    }

    fn poll(&mut self, keyboard: &mut HidKeyboard, buffer: &mut RingBuffer) {
        while let Some(event) = self.event_ring.pop() {
            if event.trb_type() == XHCI_TRB_TYPE_TRANSFER_EVENT {
                let trb_ptr = event.parameter as *mut Trb;
                let report = self.read_report_for_trb(trb_ptr);
                keyboard.process_report(&report, buffer);
                self.requeue_report(trb_ptr);
            }
        }
    }

    fn read_report_for_trb(&self, trb_ptr: *mut Trb) -> [u8; KEYBOARD_REPORT_LEN] {
        let mut report = [0u8; KEYBOARD_REPORT_LEN];
        for &buf in &self.report_buffers {
            let ptr = buf as *const u8;
            if virt_to_phys(ptr) == unsafe { read_volatile(trb_ptr).parameter } {
                unsafe {
                    for i in 0..KEYBOARD_REPORT_LEN {
                        report[i] = read_volatile(ptr.add(i));
                    }
                }
                break;
            }
        }
        report
    }

    fn requeue_report(&mut self, trb_ptr: *mut Trb) {
        for &buf in &self.report_buffers {
            let ptr = buf as *const u8;
            if virt_to_phys(ptr) == unsafe { read_volatile(trb_ptr).parameter } {
                let trb = Trb::new(
                    virt_to_phys(buf),
                    KEYBOARD_REPORT_LEN as u32,
                    build_trb_control(XHCI_TRB_TYPE_NORMAL, self.intr_ring.cycle) | (1 << 5),
                );
                self.intr_ring.push(trb);
                self.ring_doorbell(self.slot_id, EP1_IN_DCI);
                return;
            }
        }
    }

    fn control_in(&mut self, setup: SetupPacket, data: &mut [u8]) -> Option<()> {
        let setup_phys = setup.as_phys();
        let setup_trb = Trb::new(
            setup_phys,
            (size_of::<SetupPacket>() as u32) << 16,
            build_trb_control(XHCI_TRB_TYPE_SETUP, self.ep0_ring.cycle)
                | (SETUP_STAGE_IN << 16)
                | (1 << 6),
        );
        self.ep0_ring.push(setup_trb);

        let data_trb = Trb::new(
            virt_to_phys(data.as_ptr()),
            data.len() as u32,
            build_trb_control(XHCI_TRB_TYPE_DATA, self.ep0_ring.cycle) | (1 << 16),
        );
        self.ep0_ring.push(data_trb);

        let status_trb = Trb::new(
            0,
            0,
            build_trb_control(XHCI_TRB_TYPE_STATUS, self.ep0_ring.cycle) | (1 << 5),
        );
        self.ep0_ring.push(status_trb);

        self.ring_doorbell(self.slot_id, EP0_DCI);
        self.wait_for_event(XHCI_TRB_TYPE_TRANSFER_EVENT)?;
        Some(())
    }

    fn control_out(&mut self, setup: SetupPacket) -> Option<()> {
        let setup_trb = Trb::new(
            setup.as_phys(),
            (size_of::<SetupPacket>() as u32) << 16,
            build_trb_control(XHCI_TRB_TYPE_SETUP, self.ep0_ring.cycle)
                | (SETUP_STAGE_OUT << 16)
                | (1 << 6),
        );
        self.ep0_ring.push(setup_trb);

        let status_trb = Trb::new(
            0,
            0,
            build_trb_control(XHCI_TRB_TYPE_STATUS, self.ep0_ring.cycle) | (1 << 5) | (1 << 16),
        );
        self.ep0_ring.push(status_trb);

        self.ring_doorbell(self.slot_id, EP0_DCI);
        self.wait_for_event(XHCI_TRB_TYPE_TRANSFER_EVENT)?;
        Some(())
    }

    fn wait_for_event(&mut self, trb_type: u32) -> Option<Trb> {
        for _ in 0..1_000_000 {
            if let Some(event) = self.event_ring.pop() {
                if event.trb_type() == trb_type {
                    return Some(event);
                }
            }
        }
        None
    }

    fn ring_doorbell(&self, slot: u8, target: u8) {
        unsafe {
            let doorbell = self.db_base.add(slot as usize * 4);
            mmio_write32(doorbell, 0x0, target as u32);
        }
    }

    fn portsc_ptr(&self, port: u8) -> *mut u8 {
        unsafe { self.op_base.add(0x400 + (port as usize - 1) * 0x10) }
    }

    fn portsc(&self, port: u8) -> u32 {
        unsafe { mmio_read32(self.portsc_ptr(port), 0x0) }
    }

    fn port_speed(&self, port: u8) -> u8 {
        ((self.portsc(port) >> 10) & 0x0F) as u8
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SetupPacket {
    request_type: u8,
    request: u8,
    value: u16,
    index: u16,
    length: u16,
}

impl SetupPacket {
    fn as_phys(&self) -> u64 {
        virt_to_phys(self as *const _ as *const u8)
    }
}

fn setup_get_descriptor(desc_type: u8, index: u8, length: u16) -> SetupPacket {
    SetupPacket {
        request_type: 0x80,
        request: USB_REQ_GET_DESCRIPTOR,
        value: ((desc_type as u16) << 8) | index as u16,
        index: 0,
        length,
    }
}

fn setup_set_configuration(config: u16) -> SetupPacket {
    SetupPacket {
        request_type: 0x00,
        request: USB_REQ_SET_CONFIGURATION,
        value: config,
        index: 0,
        length: 0,
    }
}

fn setup_set_protocol(interface: u16, protocol: u16) -> SetupPacket {
    SetupPacket {
        request_type: 0x21,
        request: USB_REQ_SET_PROTOCOL,
        value: protocol,
        index: interface,
        length: 0,
    }
}

fn write_slot_context(ctx: *mut u32, speed: u8, port: u8, last_dci: u8) {
    unsafe {
        let entries = last_dci.max(1) as u32;
        write_volatile(ctx, (speed as u32) << 20 | (entries << 27));
        write_volatile(ctx.add(1), (port as u32) << 16);
        write_volatile(ctx.add(2), 0);
        write_volatile(ctx.add(3), 0);
    }
}

fn write_endpoint_context(
    ctx: *mut u32,
    ep_type: u32,
    max_packet: u16,
    ring_phys: u64,
    dcs: bool,
    interval: u8,
    avg_trb: u16,
) {
    unsafe {
        let dw0 = (interval as u32) << 16;
        let dw1 = (max_packet as u32) << 16 | (ep_type << 3) | (3 << 1);
        write_volatile(ctx, dw0);
        write_volatile(ctx.add(1), dw1);
        let ptr = ring_phys & !0xF;
        let dcs_bit = if dcs { 1 } else { 0 };
        write_volatile(ctx.add(2), (ptr as u32) | dcs_bit);
        write_volatile(ctx.add(3), (ptr >> 32) as u32);
        write_volatile(ctx.add(4), avg_trb as u32);
    }
}

fn build_trb_control(trb_type: u32, cycle: bool) -> u32 {
    let mut control = trb_type << 10;
    if cycle {
        control |= 1;
    }
    control
}

fn build_command_control(trb_type: u32, cycle: bool, slot: u8) -> u32 {
    build_trb_control(trb_type, cycle) | ((slot as u32) << 24)
}

fn cycle_bit(cycle: bool) -> u32 {
    if cycle { 1 } else { 0 }
}

fn size_of_trb() -> usize {
    core::mem::size_of::<Trb>()
}

fn alloc_zeroed_u64(entries: usize) -> Option<(*mut u64, u64)> {
    let bytes = entries * core::mem::size_of::<u64>();
    let layout = Layout::from_size_align(bytes, 64).ok()?;
    let mem = unsafe { alloc::alloc::alloc_zeroed(layout) };
    if mem.is_null() {
        return None;
    }
    let phys = virt_to_phys(mem);
    Some((mem as *mut u64, phys))
}

fn alloc_zeroed_ctx(ctx_size: usize, count: usize) -> Option<(*mut u32, u64)> {
    let bytes = ctx_size * count;
    let layout = Layout::from_size_align(bytes, 64).ok()?;
    let mem = unsafe { alloc::alloc::alloc_zeroed(layout) };
    if mem.is_null() {
        return None;
    }
    let phys = virt_to_phys(mem);
    Some((mem as *mut u32, phys))
}

fn alloc_zeroed_bytes(size: usize, align: usize) -> Option<*mut u8> {
    let layout = Layout::from_size_align(size, align).ok()?;
    let mem = unsafe { alloc::alloc::alloc_zeroed(layout) };
    if mem.is_null() {
        None
    } else {
        Some(mem)
    }
}

fn clear_context(ctx: *mut u32, count: usize) {
    unsafe {
        for i in 0..count {
            write_volatile(ctx.add(i), 0);
        }
    }
}

unsafe fn mmio_read32(base: *mut u8, offset: usize) -> u32 {
    read_volatile(base.add(offset) as *const u32)
}

unsafe fn mmio_write32(base: *mut u8, offset: usize, value: u32) {
    write_volatile(base.add(offset) as *mut u32, value);
}

unsafe fn mmio_write64(base: *mut u8, offset: usize, value: u64) {
    write_volatile(base.add(offset) as *mut u64, value);
}

struct HidKeyboard {
    last_keys: [u8; 6],
}

impl HidKeyboard {
    const fn new() -> Self {
        Self { last_keys: [0; 6] }
    }

    fn process_report(&mut self, report: &[u8; KEYBOARD_REPORT_LEN], buffer: &mut RingBuffer) {
        let modifiers = report[0];
        let shift = modifiers & 0x22 != 0;
        let keys = &report[2..8];
        for &key in keys {
            if key == 0 || self.contains_key(key) {
                continue;
            }
            if let Some(ch) = hid_key_to_ascii(key, shift) {
                buffer.push(ch);
            }
        }
        self.last_keys.copy_from_slice(keys);
    }

    fn contains_key(&self, key: u8) -> bool {
        self.last_keys.iter().any(|&k| k == key)
    }
}

fn hid_key_to_ascii(key: u8, shift: bool) -> Option<u8> {
    match key {
        4..=29 => {
            let base = if shift { b'A' } else { b'a' };
            Some(base + (key - 4))
        }
        30 => Some(if shift { b'!' } else { b'1' }),
        31 => Some(if shift { b'@' } else { b'2' }),
        32 => Some(if shift { b'#' } else { b'3' }),
        33 => Some(if shift { b'$' } else { b'4' }),
        34 => Some(if shift { b'%' } else { b'5' }),
        35 => Some(if shift { b'^' } else { b'6' }),
        36 => Some(if shift { b'&' } else { b'7' }),
        37 => Some(if shift { b'*' } else { b'8' }),
        38 => Some(if shift { b'(' } else { b'9' }),
        39 => Some(if shift { b')' } else { b'0' }),
        40 => Some(b'\n'),
        42 => Some(0x08),
        43 => Some(b'\t'),
        44 => Some(b' '),
        45 => Some(if shift { b'_' } else { b'-' }),
        46 => Some(if shift { b'+' } else { b'=' }),
        47 => Some(if shift { b'{' } else { b'[' }),
        48 => Some(if shift { b'}' } else { b']' }),
        49 => Some(if shift { b'|' } else { b'\\' }),
        51 => Some(if shift { b':' } else { b';' }),
        52 => Some(if shift { b'"' } else { b'\'' }),
        53 => Some(if shift { b'~' } else { b'`' }),
        54 => Some(if shift { b'<' } else { b',' }),
        55 => Some(if shift { b'>' } else { b'.' }),
        56 => Some(if shift { b'?' } else { b'/' }),
        _ => None,
    }
}

struct RingBuffer {
    data: [u8; 256],
    head: usize,
    tail: usize,
    len: usize,
}

impl RingBuffer {
    const fn new() -> Self {
        Self {
            data: [0; 256],
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hid_maps_letters() {
        assert_eq!(hid_key_to_ascii(4, false), Some(b'a'));
        assert_eq!(hid_key_to_ascii(4, true), Some(b'A'));
        assert_eq!(hid_key_to_ascii(29, false), Some(b'z'));
    }

    #[test]
    fn hid_maps_numbers() {
        assert_eq!(hid_key_to_ascii(30, false), Some(b'1'));
        assert_eq!(hid_key_to_ascii(30, true), Some(b'!'));
        assert_eq!(hid_key_to_ascii(39, false), Some(b'0'));
    }

    #[test]
    fn hid_ignores_unknown() {
        assert_eq!(hid_key_to_ascii(0, false), None);
        assert_eq!(hid_key_to_ascii(100, false), None);
    }

    #[test]
    fn keyboard_reports_push_once() {
        let mut buffer = RingBuffer::new();
        let mut kb = HidKeyboard::new();
        let report = [0, 0, 4, 0, 0, 0, 0, 0];
        kb.process_report(&report, &mut buffer);
        assert_eq!(buffer.pop(), Some(b'a'));
        let report_repeat = [0, 0, 4, 0, 0, 0, 0, 0];
        kb.process_report(&report_repeat, &mut buffer);
        assert_eq!(buffer.pop(), None);
    }
}
