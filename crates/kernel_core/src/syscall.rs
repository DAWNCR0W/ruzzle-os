use hal::Errno;

use crate::caps::{CapSet, Capability};

/// Supported syscalls for the v0.1 interface surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syscall {
    Spawn,
    Exit,
    Wait,
    Yield,
    Sleep,
    Mmap,
    Munmap,
    ShmCreate,
    ShmMap,
    ShmShare,
    EndpointCreate,
    EndpointConnect,
    Send,
    Recv,
    CapTransfer,
    DebugLog,
    TimeNowNs,
}

/// Result returned from syscall dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallResult {
    Unit,
    Value(u64),
}

impl Syscall {
    fn required_cap(self) -> Option<Capability> {
        match self {
            Syscall::Spawn => Some(Capability::ProcessSpawn),
            Syscall::EndpointCreate => Some(Capability::EndpointCreate),
            Syscall::ShmCreate => Some(Capability::ShmCreate),
            Syscall::Sleep | Syscall::TimeNowNs => Some(Capability::Timer),
            Syscall::DebugLog => Some(Capability::ConsoleWrite),
            _ => None,
        }
    }
}

/// Dispatches a syscall with capability validation.
///
/// This implementation only validates permissions and does not execute
/// architecture-specific side effects.
pub fn dispatch(
    syscall: Syscall,
    caps: CapSet,
    transfer_cap: Option<Capability>,
) -> Result<SyscallResult, Errno> {
    if let Some(required) = syscall.required_cap() {
        if !caps.contains(required) {
            return Err(Errno::NoPerm);
        }
    }

    if syscall == Syscall::CapTransfer {
        let cap = transfer_cap.ok_or(Errno::InvalidArg)?;
        if !caps.contains(cap) {
            return Err(Errno::NoPerm);
        }
    }

    match syscall {
        Syscall::TimeNowNs => Ok(SyscallResult::Value(0)),
        _ => Ok(SyscallResult::Unit),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_rejects_missing_capabilities() {
        let caps = CapSet::empty();
        let result = dispatch(Syscall::Spawn, caps, None);
        assert_eq!(result, Err(Errno::NoPerm));
    }

    #[test]
    fn dispatch_allows_when_capabilities_present() {
        let mut caps = CapSet::empty();
        caps.insert(Capability::ProcessSpawn);
        let result = dispatch(Syscall::Spawn, caps, None);
        assert_eq!(result, Ok(SyscallResult::Unit));
    }

    #[test]
    fn cap_transfer_requires_capability_and_payload() {
        let caps = CapSet::empty();
        let result = dispatch(Syscall::CapTransfer, caps, None);
        assert_eq!(result, Err(Errno::InvalidArg));

        let result = dispatch(Syscall::CapTransfer, caps, Some(Capability::ConsoleWrite));
        assert_eq!(result, Err(Errno::NoPerm));

        let mut caps = CapSet::empty();
        caps.insert(Capability::ConsoleWrite);
        let result = dispatch(Syscall::CapTransfer, caps, Some(Capability::ConsoleWrite));
        assert_eq!(result, Ok(SyscallResult::Unit));
    }

    #[test]
    fn dispatch_time_returns_value() {
        let mut caps = CapSet::empty();
        caps.insert(Capability::Timer);
        let result = dispatch(Syscall::TimeNowNs, caps, None);
        assert_eq!(result, Ok(SyscallResult::Value(0)));
    }

    #[test]
    fn dispatch_sleep_allows_with_timer_capability() {
        let mut caps = CapSet::empty();
        caps.insert(Capability::Timer);
        let result = dispatch(Syscall::Sleep, caps, None);
        assert_eq!(result, Ok(SyscallResult::Unit));
    }

    #[test]
    fn dispatch_debug_log_requires_console_capability() {
        let caps = CapSet::empty();
        let result = dispatch(Syscall::DebugLog, caps, None);
        assert_eq!(result, Err(Errno::NoPerm));

        let mut caps = CapSet::empty();
        caps.insert(Capability::ConsoleWrite);
        let result = dispatch(Syscall::DebugLog, caps, None);
        assert_eq!(result, Ok(SyscallResult::Unit));
    }

    #[test]
    fn dispatch_requires_endpoint_and_shm_caps() {
        let caps = CapSet::empty();
        let result = dispatch(Syscall::EndpointCreate, caps, None);
        assert_eq!(result, Err(Errno::NoPerm));

        let mut caps = CapSet::empty();
        caps.insert(Capability::EndpointCreate);
        let result = dispatch(Syscall::EndpointCreate, caps, None);
        assert_eq!(result, Ok(SyscallResult::Unit));

        let caps = CapSet::empty();
        let result = dispatch(Syscall::ShmCreate, caps, None);
        assert_eq!(result, Err(Errno::NoPerm));
    }

    #[test]
    fn dispatch_allows_non_privileged_syscall() {
        let caps = CapSet::empty();
        let result = dispatch(Syscall::Yield, caps, None);
        assert_eq!(result, Ok(SyscallResult::Unit));
    }
}
