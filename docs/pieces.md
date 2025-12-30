# Ruzzle Pieces (External Modules)

Ruzzle OS treats external functionality as **pieces** that snap into **slots**.
This doc defines naming, packaging, and the recommended workflow for building and using pieces.

---

## Naming & Extensions

- **Concept name**: *Ruzzle Piece* (short: *piece*)
- **Bundle extension**: `.rpiece`

Slot contracts live in `slot_contracts/*.toml` and are documented in
`docs/slot_contracts.md`. Validate contracts with:

```bash
tools/slot_lint.py slot_contracts
tools/generate_slot_docs.py
```

---

## Anatomy of a Piece

A piece consists of:

1) **`module.toml`** manifest  
2) **ELF64 static binary** (no_std, v0.1)

Example `module.toml`:

```toml
name = "note-piece"
version = "0.1.0"
provides = ["ruzzle.notes"]
slots = ["ruzzle.slot.editor@1"]
requires_caps = []
depends = []
```

Naming rules:
- module names: `kebab-case` (e.g. `note-piece`)
- services: `ruzzle.*` (e.g. `ruzzle.notes`)
- slots: `ruzzle.slot.*@<version>` (e.g. `ruzzle.slot.editor@1`)

---

## Build & Pack (x86_64)

1) **Create the project**

```bash
# or use the helper:
tools/new_piece.sh note-piece
```

2) **Make it no_std**

```rust
#![no_std]
#![no_main]
```

3) **Linker script**

Either:
- copy `crates/kernel/linker.ld` into your piece repo at `crates/kernel/linker.ld`, or
- point `RUSTFLAGS` at the absolute path

Example `.cargo/config.toml`:

```toml
[target.x86_64-unknown-none]
rustflags = [
  "-C", "link-arg=-Tcrates/kernel/linker.ld",
  "-C", "relocation-model=static",
  "-C", "code-model=kernel",
  "-C", "no-redzone=yes",
  "-C", "panic=abort",
]
```

4) **Build the binary**

```bash
cargo build --release --target x86_64-unknown-none
```

5) **Lint + pack**

```bash
tools/module_lint.py path/to/module.toml
tools/pack_external_module.sh path/to/module.toml path/to/elf
```

The bundle lands in `modules/<name>.rpiece`.

Signed bundles are required for the local marketplace. Override the signing key
with `RUZZLE_MARKETPLACE_KEY` if you want to test a custom marketplace key.

### Host toolchain helper

Ruzzle ships a helper to build and pack pieces on the host:

```bash
tools/rpiece_build.sh path/to/piece-dir x86_64-unknown-none
```

It will compile the piece and emit a `.rpiece` bundle into `modules/`.

6) **Rebuild ISO**

```bash
tools/build_iso_x86.sh
```

---

## Using a Piece in Ruzzle

Inside the shell:

```
catalog
catalog --verified
catalog --slot ruzzle.slot.editor@1
piece check <piece-name>
market scan
install <piece-name>
start <piece-name>
slots
```

The catalog marks bundles as `[verified]` or `[unsigned]`. Unsigned bundles
cannot be installed.

`piece check <name>` reports signature status, dependency health, and slot
compatibility with a dependency graph.

`market scan` rebuilds the local catalog from initramfs bundles.

Installs print a manifest summary (version, slots, caps, dependencies).

`slots` and `graph` render as ASCII puzzle boards for quick scanning.

---

## Built-in Pieces (v0.1)

Core:
- `init`, `console-service`, `tui-shell`
- `fs-service`, `user-service`, `session-service`, `settings-service`
- `sysinfo-service`, `file-manager`, `text-editor`, `setup-wizard`

Connectivity & devices:
- `net-service`, `net-manager`
- `input-service`, `device-manager`

Compute & tooling:
- `rust-toolchain`
- `gpu-service`, `ml-runtime`
- `docker-service` (container lifecycle)
- `server-stack` (HTTP/TLS/metrics)

### Quick usage snippets

```text
# containers
catalog
install docker-service
start docker-service
plug ruzzle.slot.container@1 docker-service

# server stack
install server-stack
start server-stack
plug ruzzle.slot.server@1 server-stack

# gpu + ml
install gpu-service
start gpu-service
plug ruzzle.slot.gpu@1 gpu-service
install ml-runtime
start ml-runtime
plug ruzzle.slot.ml@1 ml-runtime
```

### Help topics

```
help slot
help market
```

Optional manual wiring:

```
plug <slot> <piece-name>
unplug <slot>
```

Dry-run a hot swap:

```
plug --dry-run ruzzle.slot.editor@1 vim-piece
```

Execute the swap (with rollback on failure):

```
plug --swap ruzzle.slot.editor@1 vim-piece
```

### Local market index

The local marketplace is indexed into `modules/index.toml`. It is regenerated
automatically when you pack a piece or build an ISO:

```
tools/market_scan.py
```

`catalog` reads bundles from the local market and supports filtering:

```
catalog --verified
catalog --slot ruzzle.slot.editor@1
```

### Vim-style editor flow

The shell provides a built-in, vim-like line editor that is only enabled when the
`ruzzle.slot.editor@1` slot is filled by a piece.

Example (using the bundled `vim-piece`):

```
catalog
install vim-piece
start vim-piece
plug ruzzle.slot.editor@1 vim-piece
edit /home/<user>/notes.txt
```

You can also use the built-in `text-editor` module in the catalog.

Editor commands:

```
:w    save
:q    quit
:wq   save + quit
p     print buffer
a <text>       append line
i <n> <text>   insert line n (1-based)
r <n> <text>   replace line n
d <n>          delete line n
```

---

## Example Pieces

This repo ships example pieces under:

```
external/note_piece
external/vim_piece
external/net_panel
external/file_browser
```

Build + pack a piece:

```bash
cargo test --manifest-path external/note_piece/Cargo.toml
cargo llvm-cov --summary-only --manifest-path external/note_piece/Cargo.toml
cargo build --manifest-path external/note_piece/Cargo.toml --release --target x86_64-unknown-none
tools/pack_external_module.sh external/note_piece/module.toml \
  external/note_piece/target/x86_64-unknown-none/release/note-piece
tools/build_iso_x86.sh
```

Boot and verify:

```
catalog
install note-piece
start note-piece
slots
```

You should see `note-piece` fill `ruzzle.slot.editor@1`.

Repeat with `vim-piece`, `net-panel`, or `file-browser` as needed.

---

## AArch64 Notes

Use `tools/build_bundle_arm.sh` to generate a kernel + initramfs bundle under `build/aarch64/`.
The AArch64 boot path is stubbed; `tools/run_qemu_arm.sh --force` attempts boot anyway.

---

## Common Pitfalls

- **Missing linker script**: build fails with “cannot find linker script”.
- **Non-ASCII names**: slot/service names must be lowercase ASCII.
- **Wrong target**: piece binaries must target `x86_64-unknown-none` for now.
- **No manifest**: `module.toml` is required for catalog visibility.
