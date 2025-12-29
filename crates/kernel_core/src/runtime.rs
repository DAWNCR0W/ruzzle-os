use hal::Errno;

use crate::caps::Capability;
use crate::ipc::{EndpointHandle, RecvResult};
use crate::process::Process;

/// Creates a new endpoint for the process.
pub fn endpoint_create(process: &mut Process) -> Result<EndpointHandle, Errno> {
    if !process.caps.contains(Capability::EndpointCreate) {
        return Err(Errno::NoPerm);
    }
    process.endpoints.create()
}

/// Sends a message over the endpoint owned by the process.
pub fn send(process: &mut Process, handle: EndpointHandle, payload: &[u8]) -> Result<(), Errno> {
    let cap = process.pending_cap.take();
    process.endpoints.get_mut(handle)?.send(payload, cap)
}

/// Receives a message from the endpoint owned by the process.
pub fn recv(
    process: &mut Process,
    handle: EndpointHandle,
    buffer: &mut [u8],
) -> Result<RecvResult, Errno> {
    process.endpoints.get_mut(handle)?.recv(buffer)
}

/// Attaches a capability for transfer on the next send operation.
pub fn cap_transfer(process: &mut Process, cap: Capability) -> Result<(), Errno> {
    if !process.caps.contains(cap) {
        return Err(Errno::NoPerm);
    }
    process.pending_cap = Some(cap);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::caps::CapSet;

    #[test]
    fn endpoint_create_requires_capability() {
        let mut process = Process::new(1, 0);
        assert_eq!(endpoint_create(&mut process), Err(Errno::NoPerm));

        process.caps.insert(Capability::EndpointCreate);
        let handle = endpoint_create(&mut process).expect("create should succeed");
        assert_eq!(handle, 0);
    }

    #[test]
    fn cap_transfer_attaches_to_send() {
        let mut process = Process::new(1, 0);
        process.caps.insert(Capability::EndpointCreate);
        process.caps.insert(Capability::ConsoleWrite);
        let handle = endpoint_create(&mut process).expect("create should succeed");

        cap_transfer(&mut process, Capability::ConsoleWrite).expect("transfer should succeed");
        send(&mut process, handle, b"ping").expect("send should succeed");

        let mut buffer = [0u8; 8];
        let result = recv(&mut process, handle, &mut buffer).expect("recv should succeed");
        assert_eq!(result.cap, Some(Capability::ConsoleWrite));
        assert_eq!(result.len, 4);
        assert_eq!(&buffer[..4], b"ping");
    }

    #[test]
    fn cap_transfer_requires_possession() {
        let mut process = Process::new(1, 0);
        process.caps = CapSet::empty();
        let result = cap_transfer(&mut process, Capability::ConsoleWrite);
        assert_eq!(result, Err(Errno::NoPerm));
    }

    #[test]
    fn send_invalid_handle_clears_pending_cap() {
        let mut process = Process::new(1, 0);
        process.caps.insert(Capability::ConsoleWrite);
        process.pending_cap = Some(Capability::ConsoleWrite);

        let result = send(&mut process, 99, b"data");
        assert_eq!(result, Err(Errno::NotFound));
        assert!(process.pending_cap.is_none());
    }

    #[test]
    fn send_without_pending_cap_sends_none() {
        let mut process = Process::new(1, 0);
        process.caps.insert(Capability::EndpointCreate);
        let handle = endpoint_create(&mut process).expect("create should succeed");
        send(&mut process, handle, b"data").expect("send should succeed");
        let mut buffer = [0u8; 8];
        let result = recv(&mut process, handle, &mut buffer).expect("recv should succeed");
        assert_eq!(result.cap, None);
    }

    #[test]
    fn recv_invalid_handle_errors() {
        let mut process = Process::new(1, 0);
        let mut buffer = [0u8; 4];
        let result = recv(&mut process, 42, &mut buffer);
        assert_eq!(result, Err(Errno::NotFound));
    }
}
