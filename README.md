# Ruzzle OS 🧩
**A Rust-based, ultra-lightweight, puzzle-frame operating system** where **everything is a puzzle piece**.

Ruzzle OS is designed as a **minimal kernel “frame”** that provides only the essential security boundaries and system primitives (user mode isolation, memory protection, scheduling, IPC, capabilities).  
Everything else—filesystems, networking, UI stacks, drivers, runtimes—can be **added, replaced, or removed** as **external pieces (modules)** that “fit into” the frame.

> **Core idea:** The OS is not a monolith you install.  
> It’s a **frame** you assemble.

---

## What makes Ruzzle OS different?

Most operating systems are “feature-complete products.”  
Ruzzle OS is a **composition platform**:

- The kernel is intentionally small and stable.
- Modules are first-class citizens.
- You can start with a TUI-only system and later “plug in” GUI or other capabilities—**without changing the kernel architecture**.
- The system’s identity is defined by the **set of puzzle pieces you assemble**, not by the kernel.

---

## Key Principles

- **Kernel as Frame:** The kernel provides slots and rules, not a full feature set.
- **Everything is a Puzzle Piece:** Filesystems, UI servers, and even device drivers can be modules.
- **Small Trusted Computing Base (TCB):** Keep kernel complexity minimal.
- **Capability-First Security:** Privileges are explicit and transferable.
- **Composable UX:** Your environment evolves by plugging in modules.
- **Signed Pieces:** Marketplace bundles are signed before install.

---

## Puzzle Slots (the “tabs”)

Modules declare which **slots** they can fill inside `module.toml`:

```toml
name = "console-service"
version = "0.1.0"
provides = ["ruzzle.console"]
slots = ["ruzzle.slot.console@1"]
requires_caps = ["ConsoleWrite"]
depends = []
```

The shell exposes this puzzle board:

```
slots
plug <slot> <module>
unplug <slot>
```

This makes the **shape** of the OS visible: which slots are required, which are filled, and which pieces can snap in.

Slot contracts are versioned (`@1`) so the shape can evolve without breaking existing pieces.

---

## Target Platforms

- **Architectures:** x86_64 + AArch64
- **Primary Platform:** QEMU (for fast iteration and reproducibility)
- **User Mode:** Required (memory protection & isolation are non-negotiable)

---

## Ruzzle OS at a glance

### Kernel provides (minimal, stable primitives)
- User/kernel memory isolation (MMU)
- Address spaces & process model
- Preemptive scheduling (round-robin)
- Syscalls (small set)
- IPC endpoints (message passing)
- Capabilities (permission model)
- Shared memory (recommended for high-bandwidth modules)
- Minimal drivers: console, timer, IRQ controller

### Modules provide (everything else)
- Init / module manager
- Console/logging service
- TUI shell
- Filesystem service (initramfs read-only first)
- Settings/session/user services (first boot + login)
- File manager + text editor (baseline tools)
- Sysinfo service
- Window service (compositor, optional)
- Input service (optional)
- GPU service (optional)
- Network service (optional)
- Network manager (profiles/policies)
- Device manager (drivers/bindings)
- Toolchain service (host build integration)
- Container service (Docker-style lifecycle)
- Server stack (HTTP/TLS/metrics)
- ML runtime (model execution)
- Any runtime or application you want

---

## Status

- **Design-first**: Ruzzle OS is specified as a stable “puzzle-frame” platform.
- **v0.1 goal**: boot → spawn init → run modules → user mode protection proof.
- **First boot UX**: a wizard creates the first user and base directories, then logs you in.

## Protocols

- See `docs/protocols.md` for the TLV-based IPC contract shared by modules.
- See `docs/identity_roadmap.md` for the puzzle-frame identity roadmap.

---

## Boot & ISO

Build an ISO and boot in QEMU/UTM:

```bash
tools/build_iso_x86.sh
tools/run_qemu_x86.sh
```

On first boot the shell runs `setup` automatically.
After setup, use:

```
login <user>
ls /home
slots
install <module>
start <module>
```

The base profile auto-installs and starts essential services (fs/user/session/settings/net/sysinfo
and an editor piece). See `docs/boot.md`.

### External pieces

Drop prebuilt `.rpiece` bundles into `modules/` and rebuild the ISO:

```bash
tools/module_lint.py path/to/module.toml
tools/pack_external_module.sh path/to/module.toml path/to/elf
tools/build_iso_x86.sh
```

See `docs/boot.md` and `docs/pieces.md` for details.

Sample pieces shipped in `modules/`:
`note-piece`, `vim-piece`, `net-panel`, `file-browser`.

---

## Repository Structure (planned)

```
ruzzle/
README.md
docs/
  spec.md
  implementation.md
  protocols.md
crates/
  kernel/
  kernel_core/
  hal/
  arch_x86_64/
  arch_aarch64/
  platform_qemu_x86_64/
  platform_qemu_aarch64_virt/
  user_init/
  user_console_service/
  user_tui_shell/
  user_fs_service/
  user_net_service/
  user_user_service/
  user_file_manager/
  user_text_editor/
  user_settings_service/
  user_session_service/
  user_setup_wizard/
  user_sysinfo_service/
  user_puzzle_board/
tools/
  run_qemu_x86.sh
  run_qemu_arm.sh
  mk_initramfs.py
```

---

## Release Criteria (v0.1)

Ruzzle OS v0.1 is considered complete when:

1) **Boots to a TUI-only system** using modules (init + console-service + tui-shell)  
2) **User-mode processes run** and can use syscalls + IPC  
3) **Capability model is enforced** (no privileged actions without caps)  
4) **Memory protection proof**: user process accessing kernel memory is killed, kernel survives  

---

## License
Apache-2.0. See `LICENSE`.

---

## Why “Ruzzle”?  
Rust + Puzzle = **Ruzzle**.  
A puzzle-frame OS assembled from puzzle pieces.
