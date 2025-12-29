use alloc::vec::Vec;

use hal::PhysAddr;

/// Size of a physical frame in bytes.
pub const FRAME_SIZE: u64 = 4096;

/// Represents a single physical frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysFrame {
    pub addr: PhysAddr,
}

/// Simple free-list physical frame allocator.
#[derive(Debug, Default)]
pub struct FrameAllocator {
    free: Vec<PhysFrame>,
}

impl FrameAllocator {
    /// Creates an empty frame allocator.
    pub fn new() -> Self {
        Self { free: Vec::new() }
    }

    /// Adds usable frames from the given physical range.
    pub fn init_from_region(&mut self, start: PhysAddr, end: PhysAddr) {
        let mut addr = align_up(start, FRAME_SIZE);
        let end = align_down(end, FRAME_SIZE);
        while addr + FRAME_SIZE <= end {
            self.free.push(PhysFrame { addr });
            addr += FRAME_SIZE;
        }
    }

    /// Allocates a single frame from the free list.
    pub fn alloc_frame(&mut self) -> Option<PhysFrame> {
        self.free.pop()
    }

    /// Returns a frame back to the allocator.
    pub fn free_frame(&mut self, frame: PhysFrame) {
        self.free.push(frame);
    }

    /// Returns the number of available frames.
    pub fn free_count(&self) -> usize {
        self.free.len()
    }
}

const fn align_up(value: PhysAddr, align: u64) -> PhysAddr {
    if value % align == 0 {
        value
    } else {
        value + (align - (value % align))
    }
}

const fn align_down(value: PhysAddr, align: u64) -> PhysAddr {
    value - (value % align)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_allocator_initializes_and_allocates() {
        let mut allocator = FrameAllocator::new();
        allocator.init_from_region(0x1000, 0x9000);
        assert_eq!(allocator.free_count(), 8);

        let mut frames = Vec::new();
        for _ in 0..8 {
            frames.push(allocator.alloc_frame().expect("frame should be available"));
        }
        assert_eq!(allocator.alloc_frame(), None);

        allocator.free_frame(frames[0]);
        assert_eq!(allocator.free_count(), 1);
    }

    #[test]
    fn frame_allocator_aligns_regions() {
        let mut allocator = FrameAllocator::new();
        allocator.init_from_region(0x1003, 0x3005);
        assert_eq!(allocator.free_count(), 1);

        allocator.init_from_region(0x1003, 0x5005);
        assert_eq!(allocator.free_count(), 4);
        let frame = allocator.alloc_frame().expect("frame should be available");
        assert_eq!(frame.addr, 0x4000);
    }

    #[test]
    fn frame_allocator_ignores_too_small_region() {
        let mut allocator = FrameAllocator::new();
        allocator.init_from_region(0x1000, 0x1800);
        assert_eq!(allocator.free_count(), 0);
    }
}
