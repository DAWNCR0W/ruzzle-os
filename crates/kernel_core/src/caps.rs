use core::fmt;

/// Capabilities that gate privileged kernel actions.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    ConsoleWrite = 0,
    EndpointCreate = 1,
    ShmCreate = 2,
    ProcessSpawn = 3,
    Timer = 4,
    FsRoot = 5,
    WindowServer = 6,
    InputDevice = 7,
    GpuDevice = 8,
}

impl Capability {
    const fn bit(self) -> u32 {
        1u32 << (self as u32)
    }
}

/// A compact bitset for process capabilities.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CapSet {
    bits: u32,
}

impl CapSet {
    /// Creates an empty capability set.
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    /// Creates a capability set containing every defined capability.
    pub const fn all() -> Self {
        let bits = (1u32 << (Capability::GpuDevice as u32 + 1)) - 1;
        Self { bits }
    }

    /// Returns true when the capability is present.
    pub const fn contains(self, cap: Capability) -> bool {
        (self.bits & cap.bit()) != 0
    }

    /// Inserts a capability into the set.
    pub fn insert(&mut self, cap: Capability) {
        self.bits |= cap.bit();
    }

    /// Removes a capability from the set.
    pub fn remove(&mut self, cap: Capability) {
        self.bits &= !cap.bit();
    }

    /// Returns true when the set is empty.
    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }
}

impl fmt::Debug for CapSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CapSet")
            .field("bits", &self.bits)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capset_starts_empty() {
        let caps = CapSet::empty();
        assert!(caps.is_empty());
        assert!(!caps.contains(Capability::ConsoleWrite));
    }

    #[test]
    fn capset_insert_and_remove() {
        let mut caps = CapSet::empty();
        caps.insert(Capability::ConsoleWrite);
        assert!(caps.contains(Capability::ConsoleWrite));
        caps.remove(Capability::ConsoleWrite);
        assert!(!caps.contains(Capability::ConsoleWrite));
    }

    #[test]
    fn capset_all_contains_everything() {
        let caps = CapSet::all();
        assert!(caps.contains(Capability::ConsoleWrite));
        assert!(caps.contains(Capability::EndpointCreate));
        assert!(caps.contains(Capability::ShmCreate));
        assert!(caps.contains(Capability::ProcessSpawn));
        assert!(caps.contains(Capability::Timer));
        assert!(caps.contains(Capability::FsRoot));
        assert!(caps.contains(Capability::WindowServer));
        assert!(caps.contains(Capability::InputDevice));
        assert!(caps.contains(Capability::GpuDevice));
    }

    #[test]
    fn capset_debug_includes_bits() {
        let caps = CapSet::empty();
        let text = format!("{:?}", caps);
        assert!(text.contains("CapSet"));
        assert!(text.contains("bits"));
    }
}
