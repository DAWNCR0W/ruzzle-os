#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[cfg(feature = "x86_64")]
use limine::request::{
    ExecutableAddressRequest, ExecutableFileRequest, MemoryMapRequest, ModuleRequest,
    RequestsEndMarker, RequestsStartMarker,
};
#[cfg(feature = "x86_64")]
use limine::BaseRevision;

use kernel::kprintln;

#[cfg(feature = "x86_64")]
use kernel::boot::build_boot_info;

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
#[link_section = ".limine_requests.end"]
static LIMINE_END: RequestsEndMarker = RequestsEndMarker::new();

#[cfg(feature = "x86_64")]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    kernel::console::init();
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

    let boot_info = build_boot_info(memory_map, kernel_start, kernel_end, initramfs);
    kernel::entry(boot_info)
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kprintln!("panic: {}", info);
    loop {}
}
