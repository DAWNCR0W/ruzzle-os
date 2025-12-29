# External Pieces

Drop prebuilt `.rpiece` bundles into this folder to include them in the initramfs.

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
plug ruzzle.slot.editor vim-piece
edit /home/<user>/notes.txt
```
