use core::sync::atomic::{AtomicUsize, Ordering};

static CPU_TOTAL: AtomicUsize = AtomicUsize::new(1);
static CPU_ONLINE: AtomicUsize = AtomicUsize::new(1);

/// Initializes the SMP topology from the bootloader-provided CPU count.
pub fn init(cpu_total: usize) {
    let total = cpu_total.max(1);
    CPU_TOTAL.store(total, Ordering::SeqCst);
    CPU_ONLINE.store(1, Ordering::SeqCst);
}

/// Marks the number of online CPUs (boot CPU included).
pub fn set_online(count: usize) {
    let total = CPU_TOTAL.load(Ordering::SeqCst);
    let clamped = count.clamp(1, total);
    CPU_ONLINE.store(clamped, Ordering::SeqCst);
}

/// Returns total detected CPUs.
pub fn cpu_total() -> usize {
    CPU_TOTAL.load(Ordering::SeqCst)
}

/// Returns online CPUs.
pub fn cpu_online() -> usize {
    CPU_ONLINE.load(Ordering::SeqCst)
}
