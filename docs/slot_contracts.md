# Slot Contracts

Auto-generated from `slot_contracts/*.toml`.

| Slot | Summary | Provides | Requires Caps |
| --- | --- | --- | --- |
| `ruzzle.slot.console@1` | Console output service for logs and diagnostics. | ruzzle.console | ConsoleWrite, EndpointCreate |
| `ruzzle.slot.container@1` | Container runtime orchestration and lifecycle control. | ruzzle.container | ProcessSpawn |
| `ruzzle.slot.device@1` | Device inventory and driver binding service. | ruzzle.device | - |
| `ruzzle.slot.editor@1` | Text editor service for the built-in edit/vim commands. | ruzzle.editor | - |
| `ruzzle.slot.filemgr@1` | File manager service for browsing and managing files. | ruzzle.filemgr | - |
| `ruzzle.slot.fs@1` | Filesystem service providing storage primitives. | ruzzle.fs | FsRoot |
| `ruzzle.slot.gpu@1` | GPU/accelerator service for rendering or compute. | ruzzle.gpu | GpuDevice |
| `ruzzle.slot.init@1` | Module manager and init process. | ruzzle.init | ProcessSpawn, EndpointCreate |
| `ruzzle.slot.input@1` | Input device aggregation (USB/virtio/PS2). | ruzzle.input | InputDevice |
| `ruzzle.slot.ml@1` | Machine learning runtime and model execution. | ruzzle.ml | - |
| `ruzzle.slot.net@1` | Network configuration and interface management service. | ruzzle.net | - |
| `ruzzle.slot.netmgr@1` | Network profile and policy manager. | ruzzle.netmgr | - |
| `ruzzle.slot.server@1` | Server stack orchestration (HTTP/TLS/metrics) module. | ruzzle.server | - |
| `ruzzle.slot.session@1` | Session service for logins and active user state. | ruzzle.session | - |
| `ruzzle.slot.settings@1` | System settings service for hostname, locale, and preferences. | ruzzle.settings | - |
| `ruzzle.slot.setup@1` | First-boot setup wizard service. | ruzzle.setup | - |
| `ruzzle.slot.shell@1` | Interactive shell service for command input and routing. | ruzzle.shell | EndpointCreate |
| `ruzzle.slot.sysinfo@1` | System information reporting service. | ruzzle.sysinfo | - |
| `ruzzle.slot.toolchain@1` | Rust toolchain integration for building and packaging pieces. | ruzzle.toolchain | - |
| `ruzzle.slot.user@1` | User management service for accounts and identities. | ruzzle.user | - |

## Maintenance

Regenerate:

```bash
tools/slot_lint.py slot_contracts
tools/generate_slot_docs.py
```
