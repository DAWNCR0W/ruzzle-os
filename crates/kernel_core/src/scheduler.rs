use alloc::collections::VecDeque;

/// Simple round-robin scheduler for single-core systems.
#[derive(Debug, Default)]
pub struct Scheduler {
    ready: VecDeque<u32>,
    current: Option<u32>,
}

impl Scheduler {
    /// Creates a new empty scheduler.
    pub fn new() -> Self {
        Self {
            ready: VecDeque::new(),
            current: None,
        }
    }

    /// Enqueues a process in the ready queue.
    pub fn push_ready(&mut self, pid: u32) {
        self.ready.push_back(pid);
    }

    /// Returns the currently running process ID.
    pub fn current(&self) -> Option<u32> {
        self.current
    }

    /// Performs a round-robin tick and returns the next process ID.
    pub fn schedule_next(&mut self) -> Option<u32> {
        if let Some(current) = self.current.take() {
            self.ready.push_back(current);
        }
        self.current = self.ready.pop_front();
        self.current
    }

    /// Removes the current process from the running slot.
    pub fn block_current(&mut self) -> Option<u32> {
        self.current.take()
    }

    /// Returns the number of ready processes.
    pub fn ready_count(&self) -> usize {
        self.ready.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_round_robin_cycles_processes() {
        let mut scheduler = Scheduler::new();
        scheduler.push_ready(1);
        scheduler.push_ready(2);

        assert_eq!(scheduler.schedule_next(), Some(1));
        assert_eq!(scheduler.current(), Some(1));
        assert_eq!(scheduler.ready_count(), 1);

        assert_eq!(scheduler.schedule_next(), Some(2));
        assert_eq!(scheduler.current(), Some(2));
        assert_eq!(scheduler.ready_count(), 1);

        assert_eq!(scheduler.schedule_next(), Some(1));
        assert_eq!(scheduler.current(), Some(1));
    }

    #[test]
    fn scheduler_block_current_removes_running_process() {
        let mut scheduler = Scheduler::new();
        scheduler.push_ready(7);
        scheduler.schedule_next();
        assert_eq!(scheduler.block_current(), Some(7));
        assert_eq!(scheduler.current(), None);
    }
}
