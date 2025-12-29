#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod boot;
pub mod caps;
pub mod elf;
pub mod initramfs;
pub mod ipc;
pub mod module;
pub mod module_bundle;
pub mod pmm;
pub mod process;
pub mod protection;
pub mod runtime;
pub mod scheduler;
pub mod syscall;
pub mod vmm;

pub use boot::{BootInfo, MemoryKind, MemoryRegion};
pub use caps::{CapSet, Capability};
pub use elf::{load_elf, parse_elf, ElfLoader, LoadSegment, LoadedElf};
pub use initramfs::{build_initramfs, parse_initramfs, InitramfsEntry};
pub use ipc::{Endpoint, EndpointHandle, EndpointTable, RecvResult, IPC_MAX_MESSAGE_SIZE, IPC_QUEUE_LEN};
pub use module::{parse_module_manifest, ModuleManifest};
pub use module_bundle::{build_module_bundle, parse_module_bundle, ModuleBundle};
pub use hal::Errno;
pub use hal::PageFlags;
pub use pmm::{FrameAllocator, PhysFrame, FRAME_SIZE};
pub use process::{AddressSpace, Context, KernelStack, ProcState, Process};
pub use protection::{is_user_address, validate_user_buffer, KERNEL_VIRT_BASE};
pub use runtime::{cap_transfer, endpoint_create, recv as ipc_recv, send as ipc_send};
pub use scheduler::Scheduler;
pub use syscall::{Syscall, SyscallResult};
