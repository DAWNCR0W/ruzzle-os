use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::caps::Capability;
use hal::Errno;

/// Maximum IPC message payload size in bytes.
pub const IPC_MAX_MESSAGE_SIZE: usize = 4096;

/// Maximum number of queued messages per endpoint.
pub const IPC_QUEUE_LEN: usize = 64;

/// IPC message stored in the endpoint queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    data: Vec<u8>,
    cap: Option<Capability>,
}

impl Message {
    fn new(data: Vec<u8>, cap: Option<Capability>) -> Self {
        Self { data, cap }
    }
}

/// Result of a receive operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecvResult {
    pub len: usize,
    pub cap: Option<Capability>,
}

/// IPC endpoint with a bounded message queue.
#[derive(Debug, Default)]
pub struct Endpoint {
    queue: VecDeque<Message>,
}

impl Endpoint {
    /// Creates a new endpoint with an empty queue.
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// Enqueues a message payload and optional capability transfer.
    pub fn send(&mut self, payload: &[u8], cap: Option<Capability>) -> Result<(), Errno> {
        if payload.len() > IPC_MAX_MESSAGE_SIZE {
            return Err(Errno::InvalidArg);
        }
        if self.queue.len() >= IPC_QUEUE_LEN {
            return Err(Errno::QueueFull);
        }
        self.queue
            .push_back(Message::new(payload.to_vec(), cap));
        Ok(())
    }

    /// Dequeues a message into the provided buffer.
    pub fn recv(&mut self, out: &mut [u8]) -> Result<RecvResult, Errno> {
        let message = self.queue.pop_front().ok_or(Errno::QueueEmpty)?;
        if out.len() < message.data.len() {
            self.queue.push_front(message);
            return Err(Errno::InvalidArg);
        }
        let len = message.data.len();
        out[..len].copy_from_slice(&message.data);
        Ok(RecvResult {
            len,
            cap: message.cap,
        })
    }

    /// Returns the number of queued messages.
    pub fn len(&self) -> usize {
        self.queue.len()
    }
}

/// Handle identifier for endpoints.
pub type EndpointHandle = u32;

/// Table mapping handles to endpoint objects.
#[derive(Debug, Default)]
pub struct EndpointTable {
    entries: Vec<Option<Endpoint>>,
}

impl EndpointTable {
    /// Creates a new endpoint table.
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Creates a new endpoint and returns its handle.
    pub fn create(&mut self) -> Result<EndpointHandle, Errno> {
        if let Some((index, slot)) = self
            .entries
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| slot.is_none())
        {
            *slot = Some(Endpoint::new());
            return Ok(index as EndpointHandle);
        }
        self.entries.push(Some(Endpoint::new()));
        Ok((self.entries.len() - 1) as EndpointHandle)
    }

    /// Returns a mutable reference to an endpoint by handle.
    pub fn get_mut(&mut self, handle: EndpointHandle) -> Result<&mut Endpoint, Errno> {
        self.entries
            .get_mut(handle as usize)
            .and_then(|slot| slot.as_mut())
            .ok_or(Errno::NotFound)
    }

    /// Removes an endpoint by handle.
    pub fn remove(&mut self, handle: EndpointHandle) -> Result<(), Errno> {
        let slot = self
            .entries
            .get_mut(handle as usize)
            .ok_or(Errno::NotFound)?;
        if slot.is_none() {
            return Err(Errno::NotFound);
        }
        *slot = None;
        Ok(())
    }

    /// Returns the number of active endpoints.
    pub fn count(&self) -> usize {
        self.entries.iter().filter(|slot| slot.is_some()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_send_and_recv_roundtrip() {
        let mut endpoint = Endpoint::new();
        let payload = b"hello";
        endpoint
            .send(payload, Some(Capability::ConsoleWrite))
            .expect("send should succeed");

        let mut buffer = [0u8; 16];
        let result = endpoint.recv(&mut buffer).expect("recv should succeed");
        assert_eq!(result.len, payload.len());
        assert_eq!(&buffer[..result.len], payload);
        assert_eq!(result.cap, Some(Capability::ConsoleWrite));
        assert_eq!(endpoint.len(), 0);
    }

    #[test]
    fn endpoint_rejects_oversized_payloads() {
        let mut endpoint = Endpoint::new();
        let payload = vec![0u8; IPC_MAX_MESSAGE_SIZE + 1];
        let result = endpoint.send(&payload, None);
        assert_eq!(result, Err(Errno::InvalidArg));
    }

    #[test]
    fn endpoint_rejects_receive_when_queue_empty() {
        let mut endpoint = Endpoint::new();
        let mut buffer = [0u8; 8];
        let result = endpoint.recv(&mut buffer);
        assert_eq!(result, Err(Errno::QueueEmpty));
    }

    #[test]
    fn endpoint_rejects_receive_when_buffer_too_small() {
        let mut endpoint = Endpoint::new();
        endpoint.send(&[1, 2, 3], None).expect("send should succeed");
        let mut buffer = [0u8; 2];
        let result = endpoint.recv(&mut buffer);
        assert_eq!(result, Err(Errno::InvalidArg));
        assert_eq!(endpoint.len(), 1);
    }

    #[test]
    fn endpoint_queue_full_returns_error() {
        let mut endpoint = Endpoint::new();
        for _ in 0..IPC_QUEUE_LEN {
            endpoint.send(&[1], None).expect("send should succeed");
        }
        let result = endpoint.send(&[2], None);
        assert_eq!(result, Err(Errno::QueueFull));
    }

    #[test]
    fn endpoint_table_create_and_remove() {
        let mut table = EndpointTable::new();
        let first = table.create().expect("create should succeed");
        let second = table.create().expect("create should succeed");
        assert_eq!(table.count(), 2);

        table.remove(first).expect("remove should succeed");
        assert_eq!(table.count(), 1);

        let reused = table.create().expect("create should succeed");
        assert_eq!(reused, first);
        assert_eq!(table.count(), 2);

        let endpoint = table.get_mut(second).expect("get should succeed");
        endpoint.send(&[9], None).expect("send should succeed");
    }

    #[test]
    fn endpoint_table_invalid_handle_errors() {
        let mut table = EndpointTable::new();
        assert_eq!(table.get_mut(0).unwrap_err(), Errno::NotFound);
        assert_eq!(table.remove(0).unwrap_err(), Errno::NotFound);
    }

    #[test]
    fn endpoint_table_remove_twice_errors() {
        let mut table = EndpointTable::new();
        let handle = table.create().expect("create should succeed");
        table.remove(handle).expect("remove should succeed");
        assert_eq!(table.remove(handle), Err(Errno::NotFound));
    }
}
