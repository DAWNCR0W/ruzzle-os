# Ruzzle OS Identity Roadmap (v0.1+)

This roadmap reinforces the **"puzzle-frame" identity** by making module composition explicit, testable, and easy to extend.

## Identity Pillars
- **Kernel is the frame**: minimal, stable, and security-focused.
- **Puzzle Contract**: modules interact only via syscalls, IPC endpoints, shared memory, and capabilities.
- **Replaceable pieces**: services are swappable without kernel changes.
- **Explicit privilege**: capabilities are the only way to gain authority.

## Implementation Order (1–7)

1) **Protocol Canonicalization (DONE)**  
   - Lock down TLV message formats and service naming.  
   - Ensure all module-facing protocols are documented and versioned.  

2) **Service Registry Contract (DONE)**  
   - Define registry requests/responses and error semantics.  
   - Keep service discovery stable across module swaps.  

3) **Puzzle Slots + Manifest Schema (DONE)**  
   - Add `slots` to `module.toml` and enforce validation.  
   - Make slot compatibility explicit for every module.  

4) **Puzzle Board UX (DONE)**  
   - Expose `slots`, `plug`, `unplug` in the shell.  
   - Show required vs optional slots and current providers.  

5) **First Boot Baseline (DONE)**  
   - Setup wizard creates users, settings, and base directories.  
   - Baseline tools (file manager, text editor) are bundled as modules.  

6) **Module Lint (DONE) + Puzzle Contract Tests (NEXT)**  
   - CLI lint tool for module authors (`tools/module_lint.py`).  
   - Add a test crate that asserts IPC-only communication and capability usage.  

7) **Reference Module Kit + Capability Workflows (NEXT)**  
   - Template module with build scripts, manifest, and IPC helpers.  
   - Standardize capability handoff recipes (init → console-service, etc.).  

## Definition of Done (Roadmap)
- Every module can be added/removed without touching the kernel.
- All module contracts are stable, versioned, and verified by tests.
- New module authors can build a working module with the reference kit.
