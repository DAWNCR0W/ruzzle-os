# Ruzzle Pieces (External Modules)

Ruzzle OS treats external functionality as **pieces** that snap into **slots**.
This doc defines naming, packaging, and the recommended workflow for building and using pieces.

---

## Naming & Extensions

- **Concept name**: *Ruzzle Piece* (short: *piece*)
- **Bundle extension**: `.rpiece`

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
slots = ["ruzzle.slot.editor"]
requires_caps = []
depends = []
```

Naming rules:
- module names: `kebab-case` (e.g. `note-piece`)
- services: `ruzzle.*` (e.g. `ruzzle.notes`)
- slots: `ruzzle.slot.*` (e.g. `ruzzle.slot.editor`)

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

6) **Rebuild ISO**

```bash
tools/build_iso_x86.sh
```

---

## Using a Piece in Ruzzle

Inside the shell:

```
catalog
install <piece-name>
start <piece-name>
slots
```

Optional manual wiring:

```
plug <slot> <piece-name>
unplug <slot>
```

### Vim-style editor flow

The shell provides a built-in, vim-like line editor that is only enabled when the
`ruzzle.slot.editor` slot is filled by a piece.

Example (using the bundled `vim-piece`):

```
catalog
install vim-piece
start vim-piece
plug ruzzle.slot.editor vim-piece
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

You should see `note-piece` fill `ruzzle.slot.editor`.

Repeat with `vim-piece`, `net-panel`, or `file-browser` as needed.

---

## AArch64 Notes

Use `tools/build_iso_arm.sh` to generate a kernel + initramfs bundle under `build/aarch64/`.
The AArch64 boot path is stubbed; `tools/run_qemu_arm.sh --force` attempts boot anyway.

---

## Common Pitfalls

- **Missing linker script**: build fails with “cannot find linker script”.
- **Non-ASCII names**: slot/service names must be lowercase ASCII.
- **Wrong target**: piece binaries must target `x86_64-unknown-none` for now.
- **No manifest**: `module.toml` is required for catalog visibility.
