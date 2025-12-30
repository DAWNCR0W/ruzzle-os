# Boot & ISO Build Guide

This guide explains how to build a bootable ISO and run it in QEMU or UTM.

## Requirements

- `cargo`
- `python3`
- `xorriso`
- `mtools` (for Limine UEFI CD image)
- `qemu-system-x86_64` (for QEMU)
- `nasm`, `make`, `curl`, `tar`
- `binutils` (for `readelf` on macOS)

Optional check:

```bash
tools/doctor.sh
```

## Build ISO (QEMU/UTM)

```bash
tools/build_iso_x86.sh
```

Output:
```
build/ruzzle-x86_64.iso
```

The ISO is now **hybrid (BIOS + UEFI)** using Limine.

## Run in QEMU

```bash
tools/run_qemu_x86.sh
```

Optional timeout:
```bash
QEMU_TIMEOUT=30s tools/run_qemu_x86.sh
```

Optional flags:
```bash
tools/run_qemu_x86.sh --no-rebuild
tools/run_qemu_x86.sh --gdb
```

## First Boot

On the first boot the shell starts a setup wizard that:
- creates an admin user
- writes `/etc/hostname`, `/etc/locale`, `/etc/timezone`, `/etc/keyboard`
- creates base directories and a home skeleton

After setup the base profile auto-installs and starts:
- `fs-service`, `user-service`, `session-service`, `settings-service`
- `sysinfo-service`, `file-manager`, `net-service`
- `setup-wizard` (kept available for reruns)
- the preferred editor (`vim-piece` if present, else `text-editor`)

You can re-run it manually with:
```
setup
```

## Shell Commands (baseline)

```
ps
lsmod
catalog
install <module>
remove <module>
start <module>
stop <module>
setup
login <user>
logout
whoami
users
useradd <user>
pwd
ls [path]
cd <path>
mkdir <path>
mkdir -p <path>
touch <path>
cat <path>
edit <path>
vim <path>
cp <src> <dst>
cp -r <src> <dst>
mv <src> <dst>
write <path> <text>
rm <path>
rm -r <path>
slots
plug [--dry-run|-n] <slot> <module>
unplug <slot>
graph
sysinfo
log tail
help [command]
```

## External Pieces

Place `.rpiece` bundles in `modules/` to include them in the initramfs:

```bash
tools/module_lint.py path/to/module.toml
tools/pack_external_module.sh path/to/module.toml path/to/elf
tools/build_iso_x86.sh
```

Signed bundles are required for catalog install. Set `RUZZLE_MARKETPLACE_KEY`
to override the dev signing key when packing pieces.

## Run in UTM (macOS)

1) Create a new **x86_64 (Emulated)** VM in UTM  
2) Choose **UEFI** boot  
3) Attach `build/ruzzle-x86_64.iso` as a CD/DVD  
4) Boot the VM  

UTM will display Limine and the framebuffer console.
Serial logging remains available for debugging.

## VGA Console Input

The x86_64 build now mirrors output to the VGA text buffer and accepts PS/2 keyboard input.
In UTM you can type directly into the VM display without opening a serial console.

If your VM does not expose a PS/2 keyboard, attach a serial device and use the
serial console as a fallback. The kernel will consume input from both sources.

## Framebuffer Console

Limine framebuffer output is enabled for UEFI/BIOS hybrid boots. When a
framebuffer is present, the kernel renders the TUI into it and continues to
mirror logs to the serial console.

## AArch64 (DTB boot path)

The AArch64 path uses a minimal DTB boot flow on QEMU `virt`:
- `_start` assembly stub
- EL2 → EL1h drop (if needed)
- DTB parsing for memory + initramfs

Build artifacts for QEMU/UTM:

```bash
tools/build_iso_arm.sh
tools/run_qemu_arm.sh
```

If the AArch64 target is missing:
```bash
rustup target add aarch64-unknown-none
```

Artifacts are emitted under:
```
build/aarch64/
```

To boot in QEMU:
```bash
tools/run_qemu_arm.sh --force
```

## AArch64 (UEFI path - design)

The UEFI path is planned as a parallel boot flow to DTB:
- UEFI firmware (EDK2) loads `BOOTAA64.EFI`.
- The UEFI stub loads the kernel ELF and initramfs from the EFI System Partition.
- The stub builds a `BootInfo` from the UEFI memory map and passes it to the kernel entry.
- Device tree support remains optional; UEFI-provided memory descriptors are the source of truth.

Planned artifacts:
- `build/aarch64/BOOTAA64.EFI` (UEFI stub)
- `build/aarch64/kernel-aarch64`
- `build/aarch64/initramfs.img`

This document will be updated when the UEFI stub is implemented.
