#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[cfg(feature = "x86_64")]
use limine::request::{
    ExecutableAddressRequest, ExecutableFileRequest, FramebufferRequest, MemoryMapRequest,
    ModuleRequest, MpRequest, RequestsEndMarker, RequestsStartMarker,
};
#[cfg(feature = "x86_64")]
use limine::BaseRevision;
#[cfg(feature = "x86_64")]
use limine::framebuffer::MemoryModel;

use kernel::kprintln;

#[cfg(feature = "x86_64")]
use kernel::boot::build_boot_info;
#[cfg(feature = "x86_64")]
use kernel_core::FramebufferInfo;

#[cfg(feature = "aarch64")]
mod aarch64_entry;

#[cfg(feature = "x86_64")]
#[used]
#[link_section = ".limine_requests.start"]
static LIMINE_START: RequestsStartMarker = RequestsStartMarker::new();

#[cfg(feature = "x86_64")]
#[used]
#[link_section = ".limine_requests"]
static BASE_REVISION: BaseRevision = BaseRevision::with_revision(0);

#[cfg(feature = "x86_64")]
#[used]
#[link_section = ".limine_requests"]
static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

#[cfg(feature = "x86_64")]
#[used]
#[link_section = ".limine_requests"]
static EXECUTABLE_FILE_REQUEST: ExecutableFileRequest = ExecutableFileRequest::new();

#[cfg(feature = "x86_64")]
#[used]
#[link_section = ".limine_requests"]
static EXECUTABLE_ADDRESS_REQUEST: ExecutableAddressRequest = ExecutableAddressRequest::new();

#[cfg(feature = "x86_64")]
#[used]
#[link_section = ".limine_requests"]
static MODULE_REQUEST: ModuleRequest = ModuleRequest::new();

#[cfg(feature = "x86_64")]
#[used]
#[link_section = ".limine_requests"]
static MP_REQUEST: MpRequest = MpRequest::new();

#[cfg(feature = "x86_64")]
#[used]
#[link_section = ".limine_requests"]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[cfg(feature = "x86_64")]
#[used]
#[link_section = ".limine_requests.end"]
static LIMINE_END: RequestsEndMarker = RequestsEndMarker::new();

#[cfg(feature = "x86_64")]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    kernel::console::init_early();
    kprintln!("Ruzzle OS: limine entry starting");
    if !BASE_REVISION.is_supported() {
        kprintln!("Ruzzle OS: unsupported limine base revision");
        loop {}
    }

    let memory_map = match MEMORY_MAP_REQUEST.get_response() {
        Some(response) => response,
        None => {
            kprintln!(
                "Ruzzle OS: limine memory map response missing (valid={}, supported={})",
                BASE_REVISION.is_valid(),
                BASE_REVISION.is_supported()
            );
            loop {}
        }
    };
    let exec_file = match EXECUTABLE_FILE_REQUEST.get_response() {
        Some(response) => response,
        None => {
            kprintln!("Ruzzle OS: limine executable file response missing");
            loop {}
        }
    };
    let exec_addr = match EXECUTABLE_ADDRESS_REQUEST.get_response() {
        Some(response) => response,
        None => {
            kprintln!("Ruzzle OS: limine executable address response missing");
            loop {}
        }
    };

    let kernel_start = exec_addr.physical_base();
    let kernel_end = kernel_start + exec_file.file().size();

    let initramfs = MODULE_REQUEST
        .get_response()
        .and_then(|response| response.modules().first().copied())
        .map(|module| {
            let start = module.addr() as u64;
            let end = start + module.size();
            (start, end)
        });

    let framebuffer = FRAMEBUFFER_REQUEST
        .get_response()
        .and_then(|response| response.framebuffers().next())
        .and_then(|fb| {
            if fb.bpp() < 24 || fb.memory_model() != MemoryModel::RGB {
                return None;
            }
            Some(FramebufferInfo {
                addr: fb.addr() as u64,
                width: fb.width() as u32,
                height: fb.height() as u32,
                pitch: fb.pitch() as u32,
                bpp: fb.bpp(),
                red_mask_size: fb.red_mask_size(),
                red_mask_shift: fb.red_mask_shift(),
                green_mask_size: fb.green_mask_size(),
                green_mask_shift: fb.green_mask_shift(),
                blue_mask_size: fb.blue_mask_size(),
                blue_mask_shift: fb.blue_mask_shift(),
            })
        });

    let cpu_count = MP_REQUEST
        .get_response()
        .map(|response| response.cpus().len())
        .unwrap_or(1);
    kernel::smp::init(cpu_count);

    let boot_info = build_boot_info(memory_map, kernel_start, kernel_end, initramfs, framebuffer);
    kernel::entry(boot_info)
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kprintln!("panic: {}", info);
    loop {}
}
