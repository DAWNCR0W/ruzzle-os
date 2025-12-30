#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
  cat <<'USAGE'
Usage: new_piece.sh <piece-name> [dest-dir]

Example:
  tools/new_piece.sh note-piece
  tools/new_piece.sh log-viewer /tmp/log_viewer
USAGE
}

if [ $# -lt 1 ]; then
  usage >&2
  exit 1
fi

PIECE_NAME="$1"
DEST_DIR="${2:-}"

if [[ ! "${PIECE_NAME}" =~ ^[a-z0-9]+(-[a-z0-9]+)*$ ]]; then
  echo "Invalid piece name: ${PIECE_NAME}" >&2
  echo "Use kebab-case (e.g., note-piece)." >&2
  exit 1
fi

PKG_NAME="${PIECE_NAME//-/_}"
if [ -z "${DEST_DIR}" ]; then
  DEST_DIR="${ROOT_DIR}/external/${PKG_NAME}"
fi

if [ -e "${DEST_DIR}" ]; then
  echo "Destination already exists: ${DEST_DIR}" >&2
  exit 1
fi

mkdir -p "${DEST_DIR}/src" "${DEST_DIR}/.cargo" "${DEST_DIR}/crates/kernel"
cp "${ROOT_DIR}/crates/kernel/linker.ld" "${DEST_DIR}/crates/kernel/linker.ld"

cat > "${DEST_DIR}/Cargo.toml" <<EOF
[package]
name = "${PKG_NAME}"
version = "0.1.0"
edition = "2021"

[dependencies]

[lib]
name = "${PKG_NAME}"
path = "src/lib.rs"

[[bin]]
name = "${PIECE_NAME}"
path = "src/main.rs"
test = false
bench = false

[workspace]
EOF

cat > "${DEST_DIR}/module.toml" <<EOF
name = "${PIECE_NAME}"
version = "0.1.0"
provides = ["ruzzle.${PIECE_NAME}"]
slots = ["ruzzle.slot.editor@1"]
requires_caps = []
depends = []
EOF

cat > "${DEST_DIR}/.cargo/config.toml" <<'EOF'
[target.x86_64-unknown-none]
rustflags = [
  "-C", "link-arg=-Tcrates/kernel/linker.ld",
  "-C", "relocation-model=static",
  "-C", "code-model=kernel",
  "-C", "no-redzone=yes",
  "-C", "panic=abort",
]
EOF

cat > "${DEST_DIR}/src/main.rs" <<'EOF'
#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
EOF

cat > "${DEST_DIR}/src/lib.rs" <<'EOF'
#![cfg_attr(not(test), no_std)]

/// Small toggle helper for pieces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Toggle {
    enabled: bool,
}

/// Errors returned by the toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleError {
    AlreadyOn,
    AlreadyOff,
}

impl Toggle {
    /// Creates a new toggle (disabled).
    pub const fn new() -> Self {
        Self { enabled: false }
    }

    /// Enables the toggle.
    pub fn enable(&mut self) -> Result<(), ToggleError> {
        if self.enabled {
            return Err(ToggleError::AlreadyOn);
        }
        self.enabled = true;
        Ok(())
    }

    /// Disables the toggle.
    pub fn disable(&mut self) -> Result<(), ToggleError> {
        if !self.enabled {
            return Err(ToggleError::AlreadyOff);
        }
        self.enabled = false;
        Ok(())
    }

    /// Returns the current state.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_starts_off() {
        let toggle = Toggle::new();
        assert!(!toggle.is_enabled());
    }

    #[test]
    fn enable_disable_roundtrip() {
        let mut toggle = Toggle::new();
        assert_eq!(toggle.enable(), Ok(()));
        assert!(toggle.is_enabled());
        assert_eq!(toggle.disable(), Ok(()));
        assert!(!toggle.is_enabled());
    }

    #[test]
    fn enable_rejects_when_already_on() {
        let mut toggle = Toggle::new();
        toggle.enable().unwrap();
        assert_eq!(toggle.enable(), Err(ToggleError::AlreadyOn));
    }

    #[test]
    fn disable_rejects_when_already_off() {
        let mut toggle = Toggle::new();
        assert_eq!(toggle.disable(), Err(ToggleError::AlreadyOff));
    }
}
EOF

echo "New piece created at: ${DEST_DIR}"
echo "Next:"
echo "  cargo test --manifest-path ${DEST_DIR}/Cargo.toml"
echo "  cargo build --manifest-path ${DEST_DIR}/Cargo.toml --release --target x86_64-unknown-none"
echo "  tools/pack_external_module.sh ${DEST_DIR}/module.toml ${DEST_DIR}/target/x86_64-unknown-none/release/${PIECE_NAME}"
