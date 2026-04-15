#![no_std]

use core::mem::size_of;

pub const BOOT_INFO_MAGIC: u32 = 0x424F_4F54;
pub const BOOT_INFO_VERSION: u16 = 1;
pub const DEFAULT_QEMU_VIRT_RAM_BASE: u64 = 0x8000_0000;
pub const DEFAULT_QEMU_VIRT_RAM_SIZE: u64 = 128 * 1024 * 1024;
pub const DEFAULT_QEMU_VIRT_UART_BASE: u64 = 0x1000_0000;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BootInfo {
    pub magic: u32,
    pub version: u16,
    pub size: u16,
    pub boot_hart_id: u64,
    pub phys_mem_base: u64,
    pub phys_mem_size: u64,
    pub uart_base: u64,
    pub kernel_start: u64,
    pub kernel_end: u64,
    pub dtb_ptr: u64,
    pub reserved: [u64; 4],
}

impl BootInfo {
    pub const fn new(
        boot_hart_id: u64,
        phys_mem_base: u64,
        phys_mem_size: u64,
        uart_base: u64,
        kernel_start: u64,
        kernel_end: u64,
        dtb_ptr: u64,
    ) -> Self {
        Self {
            magic: BOOT_INFO_MAGIC,
            version: BOOT_INFO_VERSION,
            size: size_of::<Self>() as u16,
            boot_hart_id,
            phys_mem_base,
            phys_mem_size,
            uart_base,
            kernel_start,
            kernel_end,
            dtb_ptr,
            reserved: [0; 4],
        }
    }

    pub const fn qemu_virt(boot_hart_id: u64, kernel_start: u64, kernel_end: u64, dtb_ptr: u64) -> Self {
        Self::new(
            boot_hart_id,
            DEFAULT_QEMU_VIRT_RAM_BASE,
            DEFAULT_QEMU_VIRT_RAM_SIZE,
            DEFAULT_QEMU_VIRT_UART_BASE,
            kernel_start,
            kernel_end,
            dtb_ptr,
        )
    }

    pub const fn is_valid(&self) -> bool {
        self.magic == BOOT_INFO_MAGIC
            && self.version == BOOT_INFO_VERSION
            && self.size as usize == size_of::<Self>()
            && self.kernel_end >= self.kernel_start
    }

    /// # Safety
    ///
    /// `ptr` must either be zero or point to a valid `BootInfo` in memory.
    pub unsafe fn from_ptr<'a>(ptr: usize) -> Option<&'a Self> {
        if ptr == 0 {
            return None;
        }

        let info = unsafe { &*(ptr as *const Self) };
        if info.is_valid() {
            Some(info)
        } else {
            None
        }
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_info_layout_is_word_aligned() {
        assert_eq!(size_of::<BootInfo>() % 8, 0);
    }

    #[test]
    fn test_boot_info_validity() {
        let info = BootInfo::qemu_virt(0, 0x8020_0000, 0x8020_1000, 0x8700_0000);
        assert!(info.is_valid());
    }

    #[test]
    fn test_boot_info_from_ptr() {
        let info = BootInfo::qemu_virt(1, 0x8020_0000, 0x8020_2000, 0);
        let ptr = &info as *const BootInfo as usize;
        let loaded = unsafe { BootInfo::from_ptr(ptr) }.unwrap();
        assert_eq!(loaded.boot_hart_id, 1);
        assert_eq!(loaded.kernel_end, 0x8020_2000);
    }
}
