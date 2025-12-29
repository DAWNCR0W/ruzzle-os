use hal::Errno;

/// Kernel virtual base for high-half mapping.
pub const KERNEL_VIRT_BASE: u64 = 0xFFFF_8000_0000_0000;

/// Returns true if the given virtual address is in user space.
pub fn is_user_address(addr: u64) -> bool {
    addr < KERNEL_VIRT_BASE
}

/// Validates that a user buffer does not cross into kernel space.
pub fn validate_user_buffer(addr: u64, len: u64) -> Result<(), Errno> {
    if len == 0 {
        return Err(Errno::InvalidArg);
    }
    let end = addr.checked_add(len - 1).ok_or(Errno::InvalidArg)?;
    if is_user_address(addr) && is_user_address(end) {
        Ok(())
    } else {
        Err(Errno::NoPerm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_address_check() {
        assert!(is_user_address(0x1000));
        assert!(!is_user_address(KERNEL_VIRT_BASE));
    }

    #[test]
    fn user_buffer_validation() {
        assert_eq!(validate_user_buffer(0x1000, 4), Ok(()));
        assert_eq!(validate_user_buffer(0x0, 0), Err(Errno::InvalidArg));
        assert_eq!(
            validate_user_buffer(KERNEL_VIRT_BASE, 4),
            Err(Errno::NoPerm)
        );
    }

    #[test]
    fn user_buffer_overflow_is_invalid() {
        let result = validate_user_buffer(u64::MAX - 1, 4);
        assert_eq!(result, Err(Errno::InvalidArg));
    }
}
