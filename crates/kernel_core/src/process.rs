use hal::PagingRoot;

use crate::caps::{CapSet, Capability};
use crate::ipc::EndpointTable;

/// Process execution states in the scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcState {
    Ready,
    Running,
    Blocked,
    Exited,
}

/// Address space metadata for a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressSpace {
    pub root: PagingRoot,
}

/// Kernel stack metadata for a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelStack {
    pub top: usize,
}

/// Saved CPU context for context switching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Context {
    pub pc: usize,
    pub sp: usize,
}

/// Process control block containing runtime state.
#[derive(Debug)]
pub struct Process {
    pub pid: u32,
    pub state: ProcState,
    pub caps: CapSet,
    pub space: AddressSpace,
    pub kstack: KernelStack,
    pub ctx: Context,
    pub endpoints: EndpointTable,
    pub pending_cap: Option<Capability>,
}

impl Process {
    /// Creates a new process control block with default runtime state.
    pub fn new(pid: u32, root: PagingRoot) -> Self {
        Self {
            pid,
            state: ProcState::Ready,
            caps: CapSet::empty(),
            space: AddressSpace { root },
            kstack: KernelStack { top: 0 },
            ctx: Context { pc: 0, sp: 0 },
            endpoints: EndpointTable::new(),
            pending_cap: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_starts_ready_with_empty_caps() {
        let process = Process::new(1, 0x1000);
        assert_eq!(process.pid, 1);
        assert_eq!(process.state, ProcState::Ready);
        assert!(process.caps.is_empty());
        assert_eq!(process.space.root, 0x1000);
        assert!(process.pending_cap.is_none());
    }
}
