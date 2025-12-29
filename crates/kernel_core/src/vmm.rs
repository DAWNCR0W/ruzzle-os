use hal::{Errno, PageFlags, PagingOps, PhysAddr, VirtAddr};

use crate::process::AddressSpace;

/// Virtual memory manager facade over architecture paging operations.
#[derive(Debug)]
pub struct Vmm<P: PagingOps> {
    paging: P,
}

impl<P: PagingOps> Vmm<P> {
    /// Creates a new virtual memory manager.
    pub fn new(paging: P) -> Self {
        Self { paging }
    }

    /// Maps a virtual address to a physical address in the address space.
    pub fn map(
        &mut self,
        space: &AddressSpace,
        va: VirtAddr,
        pa: PhysAddr,
        flags: PageFlags,
    ) -> Result<(), Errno> {
        self.paging.map(space.root, va, pa, flags)
    }

    /// Unmaps a virtual address from the address space.
    pub fn unmap(&mut self, space: &AddressSpace, va: VirtAddr) -> Result<(), Errno> {
        self.paging.unmap(space.root, va)
    }

    /// Switches to the provided address space.
    pub fn switch_as(&self, space: &AddressSpace) {
        self.paging.switch_as(space.root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::Cell;
    use hal::PagingRoot;

    #[derive(Debug, Default)]
    struct MockPaging {
        last_map: Option<(PagingRoot, VirtAddr, PhysAddr, PageFlags)>,
        last_unmap: Option<(PagingRoot, VirtAddr)>,
        switch_root: Cell<Option<PagingRoot>>,
    }

    impl PagingOps for MockPaging {
        fn map(
            &mut self,
            root: PagingRoot,
            va: VirtAddr,
            pa: PhysAddr,
            flags: PageFlags,
        ) -> Result<(), Errno> {
            self.last_map = Some((root, va, pa, flags));
            Ok(())
        }

        fn unmap(&mut self, root: PagingRoot, va: VirtAddr) -> Result<(), Errno> {
            self.last_unmap = Some((root, va));
            Ok(())
        }

        fn switch_as(&self, _root: PagingRoot) {
            self.switch_root.set(Some(_root));
        }
    }

    #[test]
    fn vmm_maps_and_unmaps() {
        let paging = MockPaging::default();
        let mut vmm = Vmm::new(paging);
        let space = AddressSpace { root: 0x1000 };

        vmm.map(&space, 0x2000, 0x3000, PageFlags::READ)
            .expect("map should succeed");
        assert_eq!(
            vmm.paging.last_map,
            Some((0x1000, 0x2000, 0x3000, PageFlags::READ))
        );

        vmm.unmap(&space, 0x2000).expect("unmap should succeed");
        assert_eq!(vmm.paging.last_unmap, Some((0x1000, 0x2000)));
    }

    #[test]
    fn vmm_switch_as_records_root() {
        let paging = MockPaging::default();
        let vmm = Vmm::new(paging);
        let space = AddressSpace { root: 0xBEEF };
        vmm.switch_as(&space);
        assert_eq!(vmm.paging.switch_root.get(), Some(0xBEEF));
    }
}
