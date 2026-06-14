use core::ptr::{read_volatile, write_volatile};

pub const UNO_R4_WIFI_LED_PIN: u8 = 13;

const LED_MASK: u16 = 1 << 2;

const PORT1_PCNTR1: *mut u32 = 0x4004_0020 as *mut u32;
const PORT1_PORR: *mut u16 = 0x4004_0028 as *mut u16;
const PORT1_POSR: *mut u16 = 0x4004_002A as *mut u16;
const PMISC_PWPR: *mut u8 = 0x4004_0D03 as *mut u8;
const PFS_P102: *mut u32 = 0x4004_0848 as *mut u32;

const PWPR_B0WI: u8 = 1 << 7;
const PWPR_PFSWE: u8 = 1 << 6;
const PFS_PDR_OUTPUT: u32 = 1 << 2;

pub struct UnoR4WifiLed;

impl UnoR4WifiLed {
    pub fn configure_output() -> Self {
        unsafe {
            enable_pfs_writes();

            // Uno R4 WiFi bootloader leaves D13/P102 in a PWM peripheral state.
            // Reset PFS to GPIO output with a low initial value.
            write_volatile(PFS_P102, PFS_PDR_OUTPUT);

            disable_pfs_writes();

            let mut pcntr1 = read_volatile(PORT1_PCNTR1);
            pcntr1 |= LED_MASK as u32;
            pcntr1 &= !((LED_MASK as u32) << 16);
            write_volatile(PORT1_PCNTR1, pcntr1);
        }
        Self
    }

    pub fn set_high(&mut self) {
        unsafe {
            write_volatile(PORT1_PORR, LED_MASK);
        }
    }

    pub fn set_low(&mut self) {
        unsafe {
            write_volatile(PORT1_POSR, LED_MASK);
        }
    }
}

unsafe fn enable_pfs_writes() {
    let mut pwpr = read_volatile(PMISC_PWPR);
    pwpr &= !PWPR_B0WI;
    write_volatile(PMISC_PWPR, pwpr);
    write_volatile(PMISC_PWPR, pwpr | PWPR_PFSWE);
}

unsafe fn disable_pfs_writes() {
    let mut pwpr = read_volatile(PMISC_PWPR);
    pwpr &= !PWPR_B0WI;
    write_volatile(PMISC_PWPR, pwpr);
    write_volatile(PMISC_PWPR, pwpr & !PWPR_PFSWE);
}
