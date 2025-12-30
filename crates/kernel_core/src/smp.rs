extern crate alloc;

use alloc::vec::Vec;

/// CPU lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuState {
    Offline,
    Online,
    Halted,
}

/// CPU descriptor with simple load tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuCore {
    pub id: usize,
    pub state: CpuState,
    pub load: usize,
}

/// Errors from SMP topology operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmpError {
    InvalidId,
    Offline,
}

/// Simple SMP topology and load balancer.
#[derive(Debug, Default, Clone)]
pub struct CpuTopology {
    cores: Vec<CpuCore>,
}

impl CpuTopology {
    /// Builds a topology with the given number of cores.
    pub fn new(count: usize) -> Self {
        let mut cores = Vec::new();
        for id in 0..count {
            let state = if id == 0 { CpuState::Online } else { CpuState::Offline };
            cores.push(CpuCore { id, state, load: 0 });
        }
        Self { cores }
    }

    /// Returns total cores.
    pub fn total(&self) -> usize {
        self.cores.len()
    }

    /// Returns the number of online cores.
    pub fn online(&self) -> usize {
        self.cores
            .iter()
            .filter(|core| core.state == CpuState::Online)
            .count()
    }

    /// Updates the state of a core.
    pub fn set_state(&mut self, id: usize, state: CpuState) -> Result<(), SmpError> {
        let core = self.cores.get_mut(id).ok_or(SmpError::InvalidId)?;
        core.state = state;
        Ok(())
    }

    /// Adds load units to a core.
    pub fn add_load(&mut self, id: usize, units: usize) -> Result<(), SmpError> {
        let core = self.cores.get_mut(id).ok_or(SmpError::InvalidId)?;
        if core.state != CpuState::Online {
            return Err(SmpError::Offline);
        }
        core.load = core.load.saturating_add(units);
        Ok(())
    }

    /// Finds the least loaded online core.
    pub fn least_loaded_online(&self) -> Option<usize> {
        self.cores
            .iter()
            .filter(|core| core.state == CpuState::Online)
            .min_by_key(|core| core.load)
            .map(|core| core.id)
    }

    /// Distributes tasks across online cores in round-robin order.
    pub fn distribute(&mut self, tasks: usize) -> Vec<usize> {
        let online: Vec<usize> = self
            .cores
            .iter()
            .filter(|core| core.state == CpuState::Online)
            .map(|core| core.id)
            .collect();
        if online.is_empty() || tasks == 0 {
            return Vec::new();
        }
        let mut assignments = Vec::with_capacity(tasks);
        for idx in 0..tasks {
            let target = online[idx % online.len()];
            let _ = self.add_load(target, 1);
            assignments.push(target);
        }
        assignments
    }

    /// Returns a core descriptor by id.
    pub fn core(&self, id: usize) -> Option<&CpuCore> {
        self.cores.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topology_defaults_first_core_online() {
        let topo = CpuTopology::new(2);
        assert_eq!(topo.total(), 2);
        assert_eq!(topo.online(), 1);
        assert_eq!(topo.core(0).unwrap().state, CpuState::Online);
        assert_eq!(topo.core(1).unwrap().state, CpuState::Offline);
    }

    #[test]
    fn set_state_updates_core() {
        let mut topo = CpuTopology::new(1);
        topo.set_state(0, CpuState::Halted).unwrap();
        assert_eq!(topo.core(0).unwrap().state, CpuState::Halted);
        assert_eq!(topo.set_state(2, CpuState::Online), Err(SmpError::InvalidId));
    }

    #[test]
    fn add_load_rejects_offline() {
        let mut topo = CpuTopology::new(2);
        assert_eq!(topo.add_load(1, 1), Err(SmpError::Offline));
    }

    #[test]
    fn add_load_rejects_invalid_id() {
        let mut topo = CpuTopology::new(1);
        assert_eq!(topo.add_load(2, 1), Err(SmpError::InvalidId));
    }

    #[test]
    fn least_loaded_picks_smallest() {
        let mut topo = CpuTopology::new(2);
        topo.set_state(1, CpuState::Online).unwrap();
        topo.add_load(0, 3).unwrap();
        topo.add_load(1, 1).unwrap();
        assert_eq!(topo.least_loaded_online(), Some(1));
    }

    #[test]
    fn least_loaded_returns_none_when_offline() {
        let mut topo = CpuTopology::new(1);
        topo.set_state(0, CpuState::Offline).unwrap();
        assert_eq!(topo.least_loaded_online(), None);
    }

    #[test]
    fn distribute_round_robin() {
        let mut topo = CpuTopology::new(2);
        topo.set_state(1, CpuState::Online).unwrap();
        let assignments = topo.distribute(3);
        assert_eq!(assignments, vec![0, 1, 0]);
        assert_eq!(topo.core(0).unwrap().load, 2);
        assert_eq!(topo.core(1).unwrap().load, 1);
    }

    #[test]
    fn distribute_handles_empty() {
        let mut topo = CpuTopology::new(0);
        assert!(topo.distribute(2).is_empty());
        let mut topo = CpuTopology::new(1);
        topo.set_state(0, CpuState::Offline).unwrap();
        assert!(topo.distribute(2).is_empty());
    }

    #[test]
    fn distribute_handles_zero_tasks() {
        let mut topo = CpuTopology::new(2);
        topo.set_state(1, CpuState::Online).unwrap();
        assert!(topo.distribute(0).is_empty());
        assert_eq!(topo.core(0).unwrap().load, 0);
        assert_eq!(topo.core(1).unwrap().load, 0);
    }

    #[test]
    fn core_returns_none_for_invalid_id() {
        let topo = CpuTopology::new(1);
        assert!(topo.core(1).is_none());
    }
}
