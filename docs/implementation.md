# Ruzzle OS Implementation Guide (v0.1)

This document describes a concrete implementation plan for Ruzzle OS, including repository structure, ABI, memory layout, and minimal algorithms needed to reach v0.1.

---

## 1. Repository Layout (Rust Workspace)

```
ruzzle/
Cargo.toml
rust-toolchain.toml
.cargo/config.toml

crates/
kernel/                       # kernel binary (frame assembler)
kernel_core/                  # arch-independent kernel logic
hal/                          # shared traits + types
arch_x86_64/                  # CPU-specific entry/trap/syscall/paging
arch_aarch64/
platform_qemu_x86_64/         # UART, timer, IRQ, boot info adapter
platform_qemu_aarch64_virt/
user_init/                    # module manager
user_console_service/         # logging service
user_tui_shell/               # default UI
user_fs_service/              # in-memory filesystem service (v0.1)
user_net_service/             # network stub service
user_user_service/            # user database and roles
user_settings_service/        # hostname/locale/timezone/keyboard
user_session_service/         # login state
user_setup_wizard/            # first boot wizard
user_sysinfo_service/         # system status text
user_file_manager/            # ls/cd/mkdir/rm helpers
user_text_editor/             # simple text editing
user_puzzle_board/            # slot registry
user_rust_toolchain/          # host toolchain metadata + build plans
user_container_service/       # Docker-style container lifecycle
user_server_stack/            # HTTP/TLS/metrics orchestration
user_net_manager/             # network profiles/policies
user_device_manager/          # device inventory + driver bindings
user_input_service/           # USB/virtio/PS2 input aggregation
user_gpu_service/             # GPU compute primitives
user_ml_runtime/              # ML inference runtime


tools/
build_iso_x86.sh
build_iso_arm.sh
run_qemu_x86.sh
run_qemu_arm.sh
mk_initramfs.py
doctor.sh
module_lint.py
slot_lint.py
market_scan.py
generate_slot_docs.py
pack_external_module.sh
pack_module.py
new_piece.sh
rpiece_build.sh

modules/
  README.md

slot_contracts/


docs/
spec.md
implementation.md
slot_contracts.md
protocols.md

```

### Key rule
`kernel_core` must never import arch/platform crates directly.  
It talks only to `hal` traits and types.

---

## 2. Build Strategy

### 2.1 Targets
- x86_64: `x86_64-unknown-none`
- aarch64: `aarch64-unknown-none`

### 2.2 Feature composition
The `kernel` crate selects the correct arch/platform:

```rust
#[cfg(feature="x86_64")] use arch_x86_64 as arch;
#[cfg(feature="aarch64")] use arch_aarch64 as arch;

#[cfg(feature="qemu_x86_64")] use platform_qemu_x86_64 as platform;
#[cfg(feature="qemu_virt")] use platform_qemu_aarch64_virt as platform;
```

Recommended builds:

* x86_64 QEMU: `--features x86_64,qemu_x86_64`
* aarch64 QEMU: `--features aarch64,qemu_virt`

---

## 2.3 AArch64 Boot (DTB path)

The AArch64 kernel uses a minimal `-kernel` + DTB boot path:

- `_start` is provided by a tiny assembly stub
- if QEMU starts at EL2, we drop to EL1h before calling Rust
- the DTB is parsed for:
  - `/memory` `reg`
  - `/chosen` `linux,initrd-start/end`

This is enough to build a BootInfo and load the initramfs on QEMU `virt`.
UEFI boot for x86_64 is supported via Limine hybrid ISO; AArch64 UEFI remains planned
(see `docs/boot.md`).

---

## 3. BootInfo Contract

### 3.1 BootInfo

Platform code must produce a `BootInfo` passed to the kernel.

```rust
pub struct BootInfo<'a> {
    pub memory_map: &'a [MemoryRegion],
    pub kernel_start: PhysAddr,
    pub kernel_end: PhysAddr,
    pub initramfs: Option<(PhysAddr, PhysAddr)>,
    pub dtb_ptr: Option<PhysAddr>,
    pub framebuffer: Option<FramebufferInfo>,
}
```

`PhysAddr` is a `u64` physical address type (defined in `hal`).

### 3.2 Memory Map

```rust
pub struct MemoryRegion {
    pub start: PhysAddr,
    pub end: PhysAddr,
    pub kind: MemoryKind,
}

pub enum MemoryKind {
    Usable,
    Reserved,
    Mmio,
}
```

### 3.3 UEFI BootInfo (x86_64 active, AArch64 planned)

The x86_64 hybrid ISO already uses Limine UEFI to populate `BootInfo` (including
framebuffer details). The AArch64 UEFI flow will mirror the DTB contract by
constructing a `BootInfo` from the UEFI memory map and initramfs on the EFI
System Partition:

- Iterate `EFI_MEMORY_DESCRIPTOR` entries and keep usable ranges.
- Convert them into `MemoryRegion { start, end, kind: Usable }`.
- Load `initramfs.img` into memory and pass `(start, end)`.
- Set `dtb_ptr` to `None` (DTB optional in UEFI flow).
- Populate the optional `framebuffer` field when GOP is available.

---

## 4. Address Space Layout (Common Design)

### 4.1 High-level rule

* User space: low virtual addresses
* Kernel space: high-half mapping (shared across all processes, supervisor-only)

### 4.2 Suggested layout (48-bit VA systems)

* User: `0x0000_0000_0000_0000` .. `0x0000_7FFF_FFFF_FFFF`
* Kernel: `0xFFFF_8000_0000_0000` .. `0xFFFF_FFFF_FFFF_FFFF`

### 4.3 W^X (recommended)

* executable pages should not be writable
* writable pages should not be executable (NX/XN)

---

## 5. Memory Management

### 5.1 PMM (Physical Memory Manager)

Implement a simple frame allocator:

* frame size: 4 KiB
* data structure: free list (simplest), bitmap (more compact)

API:

```rust
fn alloc_frame() -> Option<PhysFrame>;
fn free_frame(frame: PhysFrame);
```

### 5.2 VMM (Virtual Memory Manager)

Arch-specific paging is hidden behind `hal::PagingOps`.

Common API:

```rust
fn map(va: VirtAddr, pa: PhysAddr, flags: PageFlags) -> Result<(), Errno>;
fn unmap(va: VirtAddr) -> Result<(), Errno>;
fn switch_as(space: &AddressSpace);
```

#### Per-process address space

Each process has its own page table root.
Kernel pages are mapped as supervisor-only in every process.

### 5.3 Kernel heap

* reserve a fixed virtual region for heap
* map it with frames
* attach an allocator:

  * v0.1: bump allocator
  * optional: linked_list_allocator

---

## 6. Process Model

### 6.1 Process struct (core concept)

```rust
pub struct Process {
    pub pid: u32,
    pub state: ProcState,
    pub caps: CapSet,
    pub space: AddressSpace,
    pub kstack: KernelStack,
    pub ctx: Context,
    pub endpoints: EndpointTable,
}
```

### 6.2 States

* Ready
* Running
* Blocked (sleep/recv/wait)
* Exited

---

## 7. Scheduler

### 7.1 Policy

* Preemptive, tick-based
* Round-robin ready queue
* v0.1: single-core only

### 7.2 Timer interrupt flow

1. trap entry
2. acknowledge interrupt controller
3. update time
4. scheduler decision
5. context switch
6. return to next process

---

## 8. Trap/Exception Handling

### 8.1 Required handlers

* page fault / data abort (must kill offending user process)
* illegal instruction
* breakpoint (optional for debug)
* timer interrupt
* syscall trap

### 8.2 User protection rule

Any user process that:

* executes privileged instructions
* accesses kernel VA
* violates permissions (R/W/X)
  must be terminated, not panic the kernel.

---

## 9. User Mode Transition

### 9.1 x86_64

* setup GDT + TSS (kernel stack switching)
* enter user mode via prepared iretq path or `sysretq` return path
* syscall entry via `syscall`

#### Syscall ABI (x86_64)

* nr: `rax`
* args: `rdi, rsi, rdx, r10, r8, r9`
* ret: `rax`
* errors: `rax = -errno`

### 9.2 AArch64

* kernel in EL1, user in EL0
* syscall via `svc #0`
* return with exception return sequence

#### Syscall ABI (AArch64)

* nr: `x8`
* args: `x0..x5`
* ret: `x0`
* errors: `x0 = -errno`

---

## 10. Syscall Dispatch

Implement a single entrypoint:

* validate user pointers (when needed)
* check capabilities
* call into kernel_core services

Recommended structure:

* `kernel_core::syscall::dispatch(nr, args, current_proc)`

---

## 11. IPC Implementation

### 11.1 Endpoint object

* endpoints are kernel-managed objects referenced by handles
* each endpoint contains a bounded ring buffer queue of messages

Constraints (v0.1):

* message size <= 4096 bytes
* queue length fixed (e.g., 64 messages)

### 11.2 send/recv semantics

* `send`: copy user buffer into kernel message, enqueue on receiver
* `recv`: dequeue message, copy into user buffer

### 11.3 Endpoint names

Implement a simple name service in init or kernel:

* `endpoint_connect("ruzzle.console") -> handle`

v0.1 recommendation:

* endpoint registry in `init` module
* kernel only provides raw endpoint handles

---

## 12. Shared Memory

### 12.1 Kernel objects

* shared memory is a kernel object referencing a set of frames
* a handle identifies it
* mapping requires permission

### 12.2 syscalls

* `shm_create(size) -> handle`
* `shm_map(handle, addr_hint) -> addr`
* `shm_share(handle, pid)`

### 12.3 Use cases

* UI surfaces
* framebuffers
* high-bandwidth data pipelines
* zero-copy IPC for large payloads

---

## 13. Capability Enforcement

### 13.1 Representation

* per-process `CapSet` (bitset)
* optional: per-capability handle table (for resource caps)

### 13.2 Policy examples

* only `init` has `ProcessSpawn`
* only `console-service` has `ConsoleWrite`
* user apps log by sending to `console-service`, not by writing device directly

### 13.3 Transfer

`cap_transfer(ep, cap)` attaches a capability to a message transfer slot.
Receiver obtains it with the next `recv`.

---

## 14. Puzzle Slots & Manifests

### 14.1 Slot declarations
Modules declare which slots they fill via `module.toml`:

```toml
slots = ["ruzzle.slot.console@1"]
```

Slots are the “tabs” that make module compatibility visible.
The shell exposes `slots`, `plug` (with optional `--dry-run`), `unplug`, and `graph`
commands to manage the board and dependencies.

### 14.2 Board behavior
The puzzle board tracks:
- required vs optional slots
- which module currently provides each slot

A running module with matching slots automatically fills the board.

---

## 15. ELF Loader (User Modules)

### 15.1 Minimal support

* ELF64 little-endian
* supports only `PT_LOAD`
* static binaries only (v0.1)

### 15.2 Loading steps

1. validate ELF header
2. iterate program headers
3. for each PT_LOAD:

   * allocate frames
   * map into process address space with appropriate flags
4. set entry point
5. setup user stack
6. mark process runnable

### 15.3 Module bundle format (`.rpiece`)

Bundles are signed for the local marketplace.

```
RMOD | version(u16) | manifest_len(u32) | payload_len(u32) | manifest | payload | signature(32)
```

* version `2` uses HMAC-SHA256 over `manifest || payload`
* version `1` is unsigned (legacy; shown as unsigned in the catalog)
* the signing key defaults to `ruzzle-dev-key` and can be overridden when packing

---

## 16. Init Module (Module Manager)

### 16.1 Responsibilities

* load modules from initramfs (or built-in table in early versions)
* resolve dependencies
* spawn modules
* provide endpoint registry (“service discovery”)
* distribute capabilities

### 16.2 Boot sequence (v0.1)

1. kernel spawns init with `ProcessSpawn`
2. init spawns:

   * console-service (granted ConsoleWrite)
   * tui-shell
3. tui-shell can start optional modules (fs-service, etc.)

---

## 17. First Boot Wizard

The setup wizard establishes a usable baseline:

1. create initial admin user
2. write `/etc/hostname`, `/etc/locale`, `/etc/timezone`, `/etc/keyboard`
3. create base directories (`/system`, `/etc`, `/var`, `/home`, `/usr`, ...)
4. create user home skeleton (`docs`, `bin`, `.config`, `downloads`)
5. log in as the new user

The initial implementation runs from the shell and uses in-kernel state,
with module versions bundled for future swap-in.

---

## 18. Baseline Modules

### 18.1 console-service

* provides endpoint: `ruzzle.console`
* accepts messages:

  * `log(level, text)`
* writes to UART
* supports prefixing by pid/module name

### 18.2 tui-shell

* provides endpoint: `ruzzle.shell`
* supports commands:

  * `ps`
  * `lsmod`
  * `catalog`
  * `install <module>`
  * `start <module>`
  * `stop <module>`
  * `setup`
  * `login <user>` / `logout`
  * `whoami` / `users` / `useradd <user>`
  * `pwd` / `ls [path]` / `cd <path>`
  * `mkdir <path>` / `touch <path>` / `rm <path>`
  * `cat <path>` / `write <path> <text>`
  * `slots` / `plug [--dry-run|-n] <slot> <module>` / `unplug <slot>`
  * `graph`
  * `sysinfo`

---

## 19. Testing & Debugging

### 19.1 QEMU

* serial console as the primary debug channel
* GDB stub support scripts per architecture

### 19.2 Mandatory tests

* user attempts to read kernel memory:

  * process is terminated
  * kernel remains alive
* capability denial:

  * process without cap cannot perform privileged syscall
* IPC stress:

  * queue bounds are enforced
  * deadlocks avoided in simple scenarios
* first boot wizard:

  * user created, base directories present

---

## 20. v0.1 Implementation Milestones (functional, not time-based)

1. UART logging works on both arch targets
2. PMM + VMM + user/kernel isolation works
3. user mode entry works (Hello from user process)
4. syscalls: spawn/exit/write (via console-service)
5. IPC endpoints + service discovery (init registry)
6. capability enforcement
7. TUI shell module to control module composition
8. puzzle slot board and first-boot setup wizard
