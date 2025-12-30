# External Pieces

Drop prebuilt `.rpiece` bundles into this folder to include them in the initramfs.

Bundles are signed for the local marketplace. Override the default signing key
by setting `RUZZLE_MARKETPLACE_KEY` when packing pieces.

## Included Samples

- `note-piece.rpiece` (editor slot example)
- `vim-piece.rpiece` (vim-style editor slot example)
- `net-panel.rpiece` (network slot example)
- `file-browser.rpiece` (file manager slot example)

## Build + Pack

```bash
# generate a new piece template
tools/new_piece.sh note-piece

# lint manifest
tools/module_lint.py path/to/module.toml

# pack an ELF + manifest into modules/<name>.rpiece
tools/pack_external_module.sh path/to/module.toml path/to/elf

# host toolchain helper (build + pack)
tools/rpiece_build.sh path/to/piece-dir x86_64-unknown-none
```

## Build ISO

```bash
tools/build_iso_x86.sh
```

The ISO build copies every `.rpiece` from `modules/` into the initramfs store.

## Try in Shell

```text
catalog
install vim-piece
start vim-piece
plug ruzzle.slot.editor@1 vim-piece
edit /home/<user>/notes.txt
```
