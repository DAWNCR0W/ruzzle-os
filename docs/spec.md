# Ruzzle OS Specification (v0.1)
**Rust-based ultra-lightweight puzzle-frame operating system**  
Architecture: **x86_64 + AArch64**  
Primary platform: **QEMU**  
User mode: **mandatory**

---

## 1. Overview

### 1.1 Definition
**Ruzzle OS is a minimal kernel frame that enables a composable operating system assembled from modules (“puzzle pieces”).**  
The kernel only provides fundamental primitives (protection, scheduling, IPC, capabilities).  
All higher-level functionality is implemented as user-space modules.

### 1.2 Goals
- Provide a **small and stable kernel frame** (minimal TCB).
- Support **user-mode isolation** with MMU-based memory protection.
- Support **module-based composition**: services/apps/drivers can be added/removed/replaced.
- Enable multiple system personalities by swapping modules:
  - TUI-only
  - GUI desktop
  - appliance-style OS
  - specialized runtime environments
- Provide a **first-boot experience** that creates the initial user and base system layout.

### 1.3 Non-goals
- Full POSIX compliance
- Broad hardware support (initially QEMU targets only)
- SMP / multi-core scheduling (v0.1)
- Complex filesystems (disk-based)
- Kernel-integrated GUI/network stacks

---

## 2. System Architecture

### 2.1 Layer Model

**Kernel Frame (trusted minimal core)**
- process + scheduler
- address spaces + MMU protection
- syscalls
- IPC endpoints
- capability enforcement
- shared memory primitives (recommended)
- minimal drivers: console, timer, interrupt controller

**User-space Modules (puzzle pieces)**
- init/module manager
- console/logging service
- shells and UI stacks
- filesystem service
- networking service
- device drivers (where feasible)
- runtimes and applications

> Modules define the OS experience.  
> The kernel defines the rules and safety boundaries.

### 2.2 Kernel Style
**Hybrid microkernel-ish**
- Kernel includes only what is required to:
  1) isolate and schedule processes  
  2) move data between modules  
  3) enforce permissions  
  4) interface with minimal hardware

Everything else is module territory.

---

## 3. Supported Platforms

### 3.1 Architectures
- **x86_64**
- **AArch64**

### 3.2 Virtualization targets (v0.1)
- x86_64: QEMU (UEFI/bootloader based)
- AArch64: QEMU `virt` machine (DTB-based or UEFI-based)

---

## 4. Kernel Requirements (v0.1)

### 4.1 MUST (required)
- Boot + early init for x86_64 and AArch64
- Trap/exception handling (page faults / data aborts)
- IRQ handling (timer interrupt at minimum)
- Physical memory manager (PMM): 4KiB frames
- Virtual memory manager (VMM): page tables per process
- User/kernel isolation:
  - kernel mapped supervisor-only
  - user memory mapped user-accessible
  - NX/XN support recommended
- Process model + preemptive scheduling (round-robin)
- Syscall dispatcher
- IPC endpoints (message passing)
- Capability model enforcement
- Minimal device support:
  - UART console
  - timer
  - interrupt controller

### 4.2 MUST NOT (prohibited in kernel)
- filesystem logic (beyond initramfs mapping)
- network stack
- GUI compositor / window manager
- large driver stacks (USB, complex GPU stacks)
- POSIX layer

---

## 5. Capability Security Model

### 5.1 Principles
- Default deny: processes start with **no privileges**
- All privileged actions require a capability
- Capabilities can be transferred only through IPC
- Kernel validates:
  - syscalls requiring caps
  - endpoint access requiring caps

### 5.2 Capability Types (v0.1 baseline)
- `ConsoleWrite`
- `EndpointCreate`
- `ShmCreate`
- `ProcessSpawn` (init-only)
- `Timer`
- `FsRoot` (filesystem service)
- `WindowServer`
- `InputDevice`
- `GpuDevice`

Implementation is intentionally simple:
- per-process capability bitset + handle table

---

## 6. IPC Specification

### 6.1 IPC Goals
- simple and correct first
- supports service composition
- later optimization possible without changing high-level model

### 6.2 IPC Types

#### 6.2.1 Message IPC (required)
- message size limit: **4 KiB**
- operations:
  - `send(endpoint, bytes)`
  - `recv(endpoint, buffer)`
- copy-based IPC (v0.1)

#### 6.2.2 Shared Memory (recommended)
Shared memory enables high-bandwidth composition (UI, graphics, streaming pipelines).
- operations:
  - `shm_create(size)`
  - `shm_map(handle, addr_hint)`
  - `shm_share(handle, pid)`

---

## 7. Syscall Specification

### 7.1 Design Principles
- <= 20 syscalls (v0.1)
- no POSIX emulation in kernel
- I/O functionality is generally provided via IPC services
- error convention: negative return codes (`-errno`) recommended

### 7.2 Syscall List (v0.1, recommended 18)

**Process/Scheduler**
- `spawn(image_handle, argv_ptr, caps_ptr) -> pid`
- `exit(code) -> !`
- `wait(pid) -> exit_code`
- `yield()`
- `sleep(ms)`

**Memory**
- `mmap(addr_hint, size, prot, flags) -> addr`
- `munmap(addr, size)`
- `shm_create(size) -> shm_handle`
- `shm_map(shm_handle, addr_hint) -> addr`
- `shm_share(shm_handle, pid)`

**IPC**
- `endpoint_create() -> ep_handle`
- `endpoint_connect(name_ptr) -> ep_handle`
- `send(ep, buf, len) -> len`
- `recv(ep, buf, len) -> len`
- `cap_transfer(ep, cap)`

**Debug/Time**
- `debug_log(buf, len)`
- `time_now_ns() -> u64`

---

## 8. Modules (“Puzzle Pieces”)

### 8.1 Definition
A module is:
- an **ELF64 user-space program**
- with an optional `module.toml` manifest
- declaring the **slots** it can fill

### 8.2 Manifest Schema (v0.1)
- name, version
- provided services (endpoint names)
- **slots** (puzzle compatibility)
- required capabilities
- dependencies

Example:
```toml
name = "console-service"
version = "0.1.0"
provides = ["ruzzle.console"]
slots = ["ruzzle.slot.console"]
requires_caps = ["ConsoleWrite", "EndpointCreate"]
depends = []
```

### 8.3 Baseline Modules (required + recommended)

**Required**
- `init` (module manager)
- `console-service`
- `tui-shell`

**Recommended baseline pieces**
- `user-service` (users/roles)
- `settings-service` (hostname/locale/timezone)
- `session-service` (login state)
- `setup-wizard` (first boot)
- `file-manager` (ls/cd/mkdir/rm)
- `text-editor` (simple editing)
- `sysinfo-service` (system summary)

### 8.4 Optional Modules (examples)
- `fs-service` (initramfs read-only)
- `input-service`
- `gpu-service`
- `window-service`
- `net-service`
- `any-runtime`

> The OS is defined by the selected module set.

---

## 9. Composability Requirements (the “Puzzle Contract”)

A system is considered Ruzzle-compatible if:

1. Modules interact only through:

   * syscalls
   * IPC endpoints
   * shared memory (optional)
2. Privileges are acquired only through capabilities.
3. Services announce stable endpoint names.
4. **Modules declare which slots they fill** via `module.toml`.
5. The kernel remains unchanged when swapping modules.

---

## 10. Acceptance Tests (v0.1)

A v0.1-compliant Ruzzle OS build must demonstrate:

* Boot → init module runs
* init spawns console-service + tui-shell
* tui-shell can start/stop modules
* user process attempts to access kernel memory → process killed, kernel survives
* capability enforcement proof: a process without `ConsoleWrite` cannot write directly to device
* first boot wizard creates an initial user and base directories

