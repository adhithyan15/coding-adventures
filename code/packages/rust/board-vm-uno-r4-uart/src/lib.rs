#![no_std]

use board_vm_uart::{BlockingUart, DataBits, Parity, StopBits, UartConfig};

pub const UNO_R4_WIFI_SERIAL_BAUD_RATE: u32 = 115_200;
pub const UNO_R4_WIFI_SERIAL_TX_PIN: u8 = 22;
pub const UNO_R4_WIFI_SERIAL_RX_PIN: u8 = 23;
pub const UNO_R4_WIFI_SCI_CHANNEL: u8 = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnoR4UartError {
    UnsupportedConfig,
    UnsupportedTarget,
    Overrun,
    Framing,
    Parity,
}

pub struct UnoR4WifiSerialUart {
    _private: (),
}

impl UnoR4WifiSerialUart {
    pub fn new() -> Result<Self, UnoR4UartError> {
        Self::with_config(UartConfig::new(UNO_R4_WIFI_SERIAL_BAUD_RATE))
    }

    pub fn with_config(config: UartConfig) -> Result<Self, UnoR4UartError> {
        if !is_supported_config(config) {
            return Err(UnoR4UartError::UnsupportedConfig);
        }
        #[cfg(target_arch = "arm")]
        unsafe {
            registers::configure_sci9_115200_8n1();
            Ok(Self { _private: () })
        }

        #[cfg(not(target_arch = "arm"))]
        {
            Err(UnoR4UartError::UnsupportedTarget)
        }
    }
}

#[cfg(target_arch = "arm")]
impl Default for UnoR4WifiSerialUart {
    fn default() -> Self {
        Self::new().expect("default Uno R4 WiFi UART config is supported")
    }
}

impl BlockingUart for UnoR4WifiSerialUart {
    type Error = UnoR4UartError;

    fn read_byte(&mut self) -> Result<u8, Self::Error> {
        #[cfg(target_arch = "arm")]
        unsafe {
            return registers::read_byte();
        }

        #[cfg(not(target_arch = "arm"))]
        {
            Err(UnoR4UartError::UnsupportedTarget)
        }
    }

    fn write_byte(&mut self, byte: u8) -> Result<(), Self::Error> {
        #[cfg(target_arch = "arm")]
        {
            unsafe {
                registers::write_byte(byte);
            }
            Ok(())
        }

        #[cfg(not(target_arch = "arm"))]
        {
            let _ = byte;
            Err(UnoR4UartError::UnsupportedTarget)
        }
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        #[cfg(target_arch = "arm")]
        {
            unsafe {
                registers::flush();
            }
            Ok(())
        }

        #[cfg(not(target_arch = "arm"))]
        {
            Err(UnoR4UartError::UnsupportedTarget)
        }
    }
}

pub const fn is_supported_config(config: UartConfig) -> bool {
    config.baud_rate == UNO_R4_WIFI_SERIAL_BAUD_RATE
        && matches!(config.data_bits, DataBits::Eight)
        && matches!(config.parity, Parity::None)
        && matches!(config.stop_bits, StopBits::One)
}

#[cfg(target_arch = "arm")]
mod registers {
    use core::ptr::{read_volatile, write_volatile};

    use super::UnoR4UartError;

    const MSTPCRB: *mut u32 = 0x4004_7000 as *mut u32;
    const SCI9_BASE: usize = 0x4007_0120;
    const SCI9_SMR: *mut u8 = SCI9_BASE as *mut u8;
    const SCI9_BRR: *mut u8 = (SCI9_BASE + 0x01) as *mut u8;
    const SCI9_SCR: *mut u8 = (SCI9_BASE + 0x02) as *mut u8;
    const SCI9_TDR: *mut u8 = (SCI9_BASE + 0x03) as *mut u8;
    const SCI9_SSR: *mut u8 = (SCI9_BASE + 0x04) as *mut u8;
    const SCI9_RDR: *const u8 = (SCI9_BASE + 0x05) as *const u8;
    const SCI9_SCMR: *mut u8 = (SCI9_BASE + 0x06) as *mut u8;
    const SCI9_SEMR: *mut u8 = (SCI9_BASE + 0x07) as *mut u8;
    const SCI9_MDDR: *mut u8 = (SCI9_BASE + 0x12) as *mut u8;

    const PMISC_PWPR: *mut u8 = 0x4004_0D03 as *mut u8;
    const PFS_P109: *mut u32 = 0x4004_0864 as *mut u32;
    const PFS_P110: *mut u32 = 0x4004_0868 as *mut u32;

    const MSTPB22_SCI9_STOP: u32 = 1 << 22;
    const PWPR_B0WI: u8 = 1 << 7;
    const PWPR_PFSWE: u8 = 1 << 6;
    const PFS_PERIPHERAL_PIN: u32 = 1 << 16;
    const PFS_SCI1_3_5_7_9: u32 = 0x05 << 24;
    const SCI9_PFS: u32 = PFS_PERIPHERAL_PIN | PFS_SCI1_3_5_7_9;

    const SCMR_RESET_ASYNC: u8 = 0xF2;
    const SCR_TE_RE: u8 = (1 << 5) | (1 << 4);
    const SSR_PER: u8 = 1 << 3;
    const SSR_FER: u8 = 1 << 4;
    const SSR_ORER: u8 = 1 << 5;
    const SSR_RDRF: u8 = 1 << 6;
    const SSR_TDRE: u8 = 1 << 7;
    const SSR_TEND: u8 = 1 << 2;

    pub unsafe fn configure_sci9_115200_8n1() {
        let mstpcrb = read_volatile(MSTPCRB);
        write_volatile(MSTPCRB, mstpcrb & !MSTPB22_SCI9_STOP);
        let _ = read_volatile(MSTPCRB);

        enable_pfs_writes();
        write_volatile(PFS_P109, SCI9_PFS);
        write_volatile(PFS_P110, SCI9_PFS);
        disable_pfs_writes();

        write_volatile(SCI9_SCR, 0);
        write_volatile(SCI9_SMR, 0);
        write_volatile(SCI9_SCMR, SCMR_RESET_ASYNC);
        write_volatile(SCI9_SEMR, 0);
        write_volatile(SCI9_MDDR, 0);
        write_volatile(SCI9_BRR, 12);
        write_volatile(
            SCI9_SSR,
            read_volatile(SCI9_SSR) & !(SSR_PER | SSR_FER | SSR_ORER),
        );
        write_volatile(SCI9_SCR, SCR_TE_RE);
    }

    pub unsafe fn read_byte() -> Result<u8, UnoR4UartError> {
        loop {
            let status = read_volatile(SCI9_SSR);
            if status & SSR_ORER != 0 {
                clear_errors(status);
                return Err(UnoR4UartError::Overrun);
            }
            if status & SSR_FER != 0 {
                clear_errors(status);
                return Err(UnoR4UartError::Framing);
            }
            if status & SSR_PER != 0 {
                clear_errors(status);
                return Err(UnoR4UartError::Parity);
            }
            if status & SSR_RDRF != 0 {
                return Ok(read_volatile(SCI9_RDR));
            }
            core::hint::spin_loop();
        }
    }

    pub unsafe fn write_byte(byte: u8) {
        while read_volatile(SCI9_SSR) & SSR_TDRE == 0 {
            core::hint::spin_loop();
        }
        write_volatile(SCI9_TDR, byte);
    }

    pub unsafe fn flush() {
        while read_volatile(SCI9_SSR) & SSR_TEND == 0 {
            core::hint::spin_loop();
        }
    }

    unsafe fn clear_errors(status: u8) {
        write_volatile(SCI9_SSR, status & !(SSR_PER | SSR_FER | SSR_ORER));
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_the_first_uno_r4_wifi_profile() {
        assert!(is_supported_config(UartConfig::new(115_200)));
        assert!(!is_supported_config(UartConfig::new(57_600)));
        assert!(!is_supported_config(
            UartConfig::new(115_200).data_bits(DataBits::Seven)
        ));
        assert!(!is_supported_config(
            UartConfig::new(115_200).parity(Parity::Even)
        ));
        assert!(!is_supported_config(
            UartConfig::new(115_200).stop_bits(StopBits::Two)
        ));
    }

    #[test]
    fn exposes_uno_r4_wifi_serial_route() {
        assert_eq!(UNO_R4_WIFI_SERIAL_TX_PIN, 22);
        assert_eq!(UNO_R4_WIFI_SERIAL_RX_PIN, 23);
        assert_eq!(UNO_R4_WIFI_SCI_CHANNEL, 9);
    }
}
