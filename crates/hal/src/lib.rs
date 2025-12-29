#![cfg_attr(not(test), no_std)]

/// Physical address type.
pub type PhysAddr = u64;

/// Virtual address type.
pub type VirtAddr = u64;

/// Root page table pointer for an address space.
pub type PagingRoot = PhysAddr;

/// Common error codes used by kernel interfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Errno {
    InvalidArg,
    NoMem,
    NoPerm,
    NotFound,
    QueueFull,
    QueueEmpty,
    Unimplemented,
}

/// Page table mapping flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageFlags(u32);

impl PageFlags {
    pub const NONE: Self = Self(0);
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const EXECUTE: Self = Self(1 << 2);
    pub const USER: Self = Self(1 << 3);

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Architecture-specific paging operations.
pub trait PagingOps {
    /// Map a virtual page to a physical page in the given address space.
    fn map(
        &mut self,
        root: PagingRoot,
        va: VirtAddr,
        pa: PhysAddr,
        flags: PageFlags,
    ) -> Result<(), Errno>;

    /// Unmap a virtual page in the given address space.
    fn unmap(&mut self, root: PagingRoot, va: VirtAddr) -> Result<(), Errno>;

    /// Switch to the given address space root.
    fn switch_as(&self, root: PagingRoot);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_flags_union_and_contains() {
        let flags = PageFlags::READ.union(PageFlags::WRITE);
        assert!(flags.contains(PageFlags::READ));
        assert!(flags.contains(PageFlags::WRITE));
        assert!(!flags.contains(PageFlags::EXECUTE));
    }
}
