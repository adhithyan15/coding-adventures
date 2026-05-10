#![no_std]

use board_vm_usb_cdc::{
    BlockingUsbCdc, UsbCdcControlLineState, UsbCdcLineCoding, USB_CDC_FULL_SPEED_MAX_PACKET_BYTES,
};
#[cfg(target_arch = "arm")]
use board_vm_usb_cdc::{UsbCdcParity, UsbCdcStopBits};

pub const UNO_R4_WIFI_USB_VID: u16 = 0x2341;
pub const UNO_R4_WIFI_USB_PID: u16 = 0x006D;
pub const UNO_R4_WIFI_BOOTLOADER_USB_PID: u16 = 0x1002;
pub const UNO_R4_WIFI_USB_PRODUCT: &str = "UNO R4 WiFi";
pub const UNO_R4_WIFI_SERIAL_USB_NAME: &str = "SerialUSB";
pub const UNO_R4_WIFI_USB_CDC_INTERFACE: u8 = 0;
pub const UNO_R4_WIFI_USB_MAX_PACKET_BYTES: usize = USB_CDC_FULL_SPEED_MAX_PACKET_BYTES;
pub const UNO_R4_WIFI_CORTEX_VECTOR_TABLE_ENTRIES: usize = 16;
pub const UNO_R4_WIFI_ICU_VECTOR_TABLE_ENTRIES: usize = 32;
pub const UNO_R4_WIFI_MUTABLE_VECTOR_TABLE_ENTRIES: usize =
    UNO_R4_WIFI_CORTEX_VECTOR_TABLE_ENTRIES + UNO_R4_WIFI_ICU_VECTOR_TABLE_ENTRIES;
pub const UNO_R4_WIFI_MUTABLE_VECTOR_TABLE_ALIGNMENT_BYTES: usize = 256;
pub const ARDUINO_RENESAS_USB_START_SYMBOL: &str = "_Z10__USBStartv";
pub const ARDUINO_RENESAS_USB_INSTALL_SERIAL_SYMBOL: &str = "_Z18__USBInstallSerialv";
pub const UNO_R4_WIFI_CONFIGURE_USB_MUX_SYMBOL: &str = "_Z17configure_usb_muxv";
pub const UNO_R4_WIFI_USB_POST_INITIALIZATION_SYMBOL: &str = "_Z23usb_post_initializationv";
pub const UNO_R4_WIFI_USB_DESCRIPTOR_HEAP_BYTES: usize = 512;
pub const UNO_R4_WIFI_BOOTLOADER_TOUCH_BAUD: u32 = 1_200;
pub const UNO_R4_WIFI_BOOTLOADER_DOUBLE_TAP_MAGIC: u32 = 0x0773_8135;
pub const TINYUSB_CDC_LINE_STATE_CALLBACK_SYMBOL: &str = "tud_cdc_line_state_cb";
pub const TINYUSB_CDC_LINE_CODING_CALLBACK_SYMBOL: &str = "tud_cdc_line_coding_cb";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnoR4UsbCdcError {
    NotStarted,
    NotConnected,
    PacketTooLarge,
    WriteStalled,
}

pub trait TinyUsbCdcApi {
    fn start(&mut self) {}
    fn poll(&mut self);
    fn connected(&self) -> bool;
    fn available(&self) -> usize;
    fn read(&mut self, bytes: &mut [u8]) -> usize;
    fn write_available(&self) -> usize;
    fn write(&mut self, bytes: &[u8]) -> usize;
    fn flush(&mut self);

    fn line_coding(&self) -> UsbCdcLineCoding {
        UsbCdcLineCoding::default()
    }

    fn control_line_state(&self) -> UsbCdcControlLineState {
        UsbCdcControlLineState::new(self.connected(), false)
    }
}

pub const fn bootloader_touch_requests_reset(
    line_coding: UsbCdcLineCoding,
    control_line_state: UsbCdcControlLineState,
) -> bool {
    line_coding.baud_rate == UNO_R4_WIFI_BOOTLOADER_TOUCH_BAUD && !control_line_state.dtr
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnoR4WifiSerialUsbCdc<A> {
    api: A,
    started: bool,
}

impl<A> UnoR4WifiSerialUsbCdc<A> {
    pub const fn new(api: A) -> Self {
        Self {
            api,
            started: false,
        }
    }

    pub const fn started(api: A) -> Self {
        Self { api, started: true }
    }

    pub fn begin(&mut self)
    where
        A: TinyUsbCdcApi,
    {
        self.api.start();
        self.started = true;
    }

    pub fn end(&mut self) {
        self.started = false;
    }

    pub const fn is_started(&self) -> bool {
        self.started
    }

    pub fn api(&self) -> &A {
        &self.api
    }

    pub fn api_mut(&mut self) -> &mut A {
        &mut self.api
    }

    pub fn into_inner(self) -> A {
        self.api
    }
}

impl<A> BlockingUsbCdc for UnoR4WifiSerialUsbCdc<A>
where
    A: TinyUsbCdcApi,
{
    type Error = UnoR4UsbCdcError;

    fn read_byte(&mut self) -> Result<u8, Self::Error> {
        self.ensure_started()?;
        let mut byte = [0];
        loop {
            self.api.poll();
            if !self.api.connected() {
                return Err(UnoR4UsbCdcError::NotConnected);
            }
            if self.api.available() == 0 {
                core::hint::spin_loop();
                continue;
            }
            if self.api.read(&mut byte) == 1 {
                return Ok(byte[0]);
            }
            core::hint::spin_loop();
        }
    }

    fn try_read_byte(&mut self) -> Result<Option<u8>, Self::Error> {
        self.ensure_started()?;
        let mut byte = [0];
        self.api.poll();
        if !self.api.connected() {
            return Err(UnoR4UsbCdcError::NotConnected);
        }
        if self.api.available() == 0 {
            return Ok(None);
        }
        Ok((self.api.read(&mut byte) == 1).then_some(byte[0]))
    }

    fn write_packet(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.ensure_started()?;
        if bytes.len() > UNO_R4_WIFI_USB_MAX_PACKET_BYTES {
            return Err(UnoR4UsbCdcError::PacketTooLarge);
        }

        let mut offset = 0;
        while offset < bytes.len() {
            self.api.poll();
            if !self.api.connected() {
                return Err(UnoR4UsbCdcError::NotConnected);
            }

            let write_len = core::cmp::min(self.api.write_available(), bytes.len() - offset);
            if write_len == 0 {
                self.api.flush();
                core::hint::spin_loop();
                continue;
            }

            let written = self.api.write(&bytes[offset..offset + write_len]);
            if written == 0 || written > write_len {
                return Err(UnoR4UsbCdcError::WriteStalled);
            }
            offset += written;
            self.api.flush();
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.ensure_started()?;
        self.api.flush();
        Ok(())
    }

    fn line_coding(&self) -> UsbCdcLineCoding {
        self.api.line_coding()
    }

    fn control_line_state(&self) -> UsbCdcControlLineState {
        self.api.control_line_state()
    }
}

impl<A> UnoR4WifiSerialUsbCdc<A> {
    fn ensure_started(&self) -> Result<(), UnoR4UsbCdcError> {
        if self.started {
            Ok(())
        } else {
            Err(UnoR4UsbCdcError::NotStarted)
        }
    }
}

#[cfg(target_arch = "arm")]
pub type UnoR4WifiSerialUsb = UnoR4WifiSerialUsbCdc<TinyUsbCdc0>;

#[cfg(target_arch = "arm")]
impl UnoR4WifiSerialUsb {
    pub const fn serial_usb() -> Self {
        Self::new(TinyUsbCdc0::new())
    }
}

#[cfg(target_arch = "arm")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TinyUsbCdc0 {
    interface: u8,
}

#[cfg(target_arch = "arm")]
impl TinyUsbCdc0 {
    pub const fn new() -> Self {
        Self {
            interface: UNO_R4_WIFI_USB_CDC_INTERFACE,
        }
    }
}

#[cfg(target_arch = "arm")]
impl Default for TinyUsbCdc0 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "arm")]
impl TinyUsbCdcApi for TinyUsbCdc0 {
    fn start(&mut self) {
        unsafe {
            arduino_irq_vectors::install_mutable_table();
            ffi::arduino_renesas_usb_start();
        }
    }

    fn poll(&mut self) {
        unsafe {
            ffi::tud_task_ext(0, false);
        }
    }

    fn connected(&self) -> bool {
        unsafe { ffi::tud_cdc_n_connected(self.interface) }
    }

    fn available(&self) -> usize {
        unsafe { ffi::tud_cdc_n_available(self.interface) as usize }
    }

    fn read(&mut self, bytes: &mut [u8]) -> usize {
        if bytes.is_empty() {
            return 0;
        }
        unsafe {
            ffi::tud_cdc_n_read(
                self.interface,
                bytes.as_mut_ptr() as *mut core::ffi::c_void,
                bytes.len() as u32,
            ) as usize
        }
    }

    fn write_available(&self) -> usize {
        unsafe { ffi::tud_cdc_n_write_available(self.interface) as usize }
    }

    fn write(&mut self, bytes: &[u8]) -> usize {
        if bytes.is_empty() {
            return 0;
        }
        unsafe {
            ffi::tud_cdc_n_write(
                self.interface,
                bytes.as_ptr() as *const core::ffi::c_void,
                bytes.len() as u32,
            ) as usize
        }
    }

    fn flush(&mut self) {
        unsafe {
            ffi::tud_cdc_n_write_flush(self.interface);
        }
    }

    fn line_coding(&self) -> UsbCdcLineCoding {
        let mut coding = ffi::TinyUsbLineCoding::default();
        unsafe {
            ffi::tud_cdc_n_get_line_coding(self.interface, &mut coding);
        }
        line_coding_from_tinyusb(coding)
    }

    fn control_line_state(&self) -> UsbCdcControlLineState {
        control_line_state_from_tinyusb(unsafe { ffi::tud_cdc_n_get_line_state(self.interface) })
    }
}

#[cfg(target_arch = "arm")]
fn line_coding_from_tinyusb(coding: ffi::TinyUsbLineCoding) -> UsbCdcLineCoding {
    UsbCdcLineCoding::new(coding.bit_rate())
        .stop_bits(stop_bits_from_tinyusb(coding.stop_bits()))
        .parity(parity_from_tinyusb(coding.parity()))
        .data_bits(coding.data_bits())
}

#[cfg(target_arch = "arm")]
fn stop_bits_from_tinyusb(stop_bits: u8) -> UsbCdcStopBits {
    match stop_bits {
        1 => UsbCdcStopBits::OnePointFive,
        2 => UsbCdcStopBits::Two,
        _ => UsbCdcStopBits::One,
    }
}

#[cfg(target_arch = "arm")]
fn parity_from_tinyusb(parity: u8) -> UsbCdcParity {
    match parity {
        1 => UsbCdcParity::Odd,
        2 => UsbCdcParity::Even,
        3 => UsbCdcParity::Mark,
        4 => UsbCdcParity::Space,
        _ => UsbCdcParity::None,
    }
}

#[cfg(target_arch = "arm")]
fn control_line_state_from_tinyusb(line_state: u8) -> UsbCdcControlLineState {
    UsbCdcControlLineState::new(line_state & 0x01 != 0, line_state & 0x02 != 0)
}

#[cfg(target_arch = "arm")]
mod arduino_irq_vectors {
    use core::ptr::{addr_of_mut, read_volatile, write_volatile};
    use core::sync::atomic::{AtomicBool, Ordering};

    use super::{
        UNO_R4_WIFI_MUTABLE_VECTOR_TABLE_ALIGNMENT_BYTES, UNO_R4_WIFI_MUTABLE_VECTOR_TABLE_ENTRIES,
    };

    const SCB_VTOR: *mut u32 = 0xE000_ED08 as *mut u32;

    #[repr(C, align(256))]
    struct MutableVectorTable([u32; UNO_R4_WIFI_MUTABLE_VECTOR_TABLE_ENTRIES]);

    static INSTALLED: AtomicBool = AtomicBool::new(false);
    static mut VECTOR_TABLE: MutableVectorTable =
        MutableVectorTable([0; UNO_R4_WIFI_MUTABLE_VECTOR_TABLE_ENTRIES]);

    pub unsafe fn install_mutable_table() {
        if INSTALLED.swap(true, Ordering::AcqRel) {
            return;
        }

        let source = read_volatile(SCB_VTOR) as *const u32;
        let destination = addr_of_mut!(VECTOR_TABLE.0) as *mut u32;
        if source != destination {
            for index in 0..UNO_R4_WIFI_MUTABLE_VECTOR_TABLE_ENTRIES {
                write_volatile(destination.add(index), read_volatile(source.add(index)));
            }

            write_volatile(SCB_VTOR, destination as u32);
            synchronize_vector_table_update();
        }
    }

    unsafe fn synchronize_vector_table_update() {
        core::arch::asm!("dsb sy", "isb sy", options(nostack, preserves_flags));
    }

    const _: () = assert!(UNO_R4_WIFI_MUTABLE_VECTOR_TABLE_ALIGNMENT_BYTES == 256);
}

#[cfg(target_arch = "arm")]
#[unsafe(export_name = "_Z18__USBInstallSerialv")]
pub extern "C" fn arduino_renesas_usb_install_serial() {}

#[cfg(target_arch = "arm")]
#[unsafe(export_name = "_Z17configure_usb_muxv")]
pub extern "C" fn arduino_uno_r4_wifi_configure_usb_mux() {
    unsafe {
        uno_r4_wifi_usb_mux::route_usb_c_to_ra4m1();
    }
}

#[cfg(target_arch = "arm")]
#[unsafe(export_name = "_Z23usb_post_initializationv")]
pub extern "C" fn arduino_uno_r4_wifi_usb_post_initialization() {
    unsafe {
        uno_r4_wifi_usb_mux::enable_ra4m1_usb_regulator();
    }
}

#[cfg(target_arch = "arm")]
#[unsafe(no_mangle)]
pub extern "C" fn tud_cdc_line_state_cb(interface: u8, dtr: bool, rts: bool) {
    if interface == UNO_R4_WIFI_USB_CDC_INTERFACE {
        arduino_bootloader_touch::update_line_state(dtr, rts);
    }
}

#[cfg(target_arch = "arm")]
#[unsafe(no_mangle)]
pub extern "C" fn tud_cdc_line_coding_cb(
    interface: u8,
    line_coding: *const ffi::TinyUsbLineCoding,
) {
    if interface != UNO_R4_WIFI_USB_CDC_INTERFACE || line_coding.is_null() {
        return;
    }

    let line_coding = unsafe { *line_coding };
    arduino_bootloader_touch::update_line_coding(line_coding_from_tinyusb(line_coding));
}

#[cfg(target_arch = "arm")]
mod arduino_bootloader_touch {
    use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    use board_vm_usb_cdc::{UsbCdcControlLineState, UsbCdcLineCoding};

    use super::{arduino_bootloader_reset, bootloader_touch_requests_reset};

    static CDC_BAUD_RATE: AtomicU32 = AtomicU32::new(115_200);
    static CDC_DTR: AtomicBool = AtomicBool::new(false);
    static CDC_RTS: AtomicBool = AtomicBool::new(false);

    pub fn update_line_state(dtr: bool, rts: bool) {
        CDC_DTR.store(dtr, Ordering::Release);
        CDC_RTS.store(rts, Ordering::Release);
        maybe_reset_to_bootloader();
    }

    pub fn update_line_coding(line_coding: UsbCdcLineCoding) {
        CDC_BAUD_RATE.store(line_coding.baud_rate, Ordering::Release);
        maybe_reset_to_bootloader();
    }

    fn maybe_reset_to_bootloader() {
        let line_coding = UsbCdcLineCoding::new(CDC_BAUD_RATE.load(Ordering::Acquire));
        let line_state = UsbCdcControlLineState::new(
            CDC_DTR.load(Ordering::Acquire),
            CDC_RTS.load(Ordering::Acquire),
        );

        if bootloader_touch_requests_reset(line_coding, line_state) {
            unsafe {
                arduino_bootloader_reset::go_bootloader();
            }
        }
    }
}

#[cfg(target_arch = "arm")]
mod arduino_bootloader_reset {
    use core::ptr::{read_volatile, write_volatile};

    use super::UNO_R4_WIFI_BOOTLOADER_DOUBLE_TAP_MAGIC;

    const SYSTEM_PRCR: *mut u16 = 0x4001_E3FE as *mut u16;
    const SYSTEM_VBTBKR0: *mut u32 = 0x4001_E500 as *mut u32;
    const USBFS_SYSCFG: *mut u16 = 0x4009_0000 as *mut u16;
    const SCB_AIRCR: *mut u32 = 0xE000_ED0C as *mut u32;

    const PRCR_KEY: u16 = 0xA500;
    const PRCR_PRC1_UNLOCK: u16 = PRCR_KEY | 0x0002;
    const PRCR_LOCK: u16 = PRCR_KEY;
    const USB_SYSCFG_DPRPU: u16 = 1 << 4;
    const AIRCR_VECTKEY: u32 = 0x05FA << 16;
    const AIRCR_PRIGROUP_MASK: u32 = 0x0000_0700;
    const AIRCR_SYSRESETREQ: u32 = 1 << 2;

    pub unsafe fn go_bootloader() -> ! {
        unlock_system_registers();
        write_volatile(SYSTEM_VBTBKR0, UNO_R4_WIFI_BOOTLOADER_DOUBLE_TAP_MAGIC);
        lock_system_registers();

        let syscfg = read_volatile(USBFS_SYSCFG);
        write_volatile(USBFS_SYSCFG, syscfg & !USB_SYSCFG_DPRPU);
        system_reset();
    }

    unsafe fn unlock_system_registers() {
        write_volatile(SYSTEM_PRCR, PRCR_PRC1_UNLOCK);
    }

    unsafe fn lock_system_registers() {
        write_volatile(SYSTEM_PRCR, PRCR_LOCK);
    }

    unsafe fn system_reset() -> ! {
        core::arch::asm!("dsb sy", options(nostack, preserves_flags));
        let aircr = read_volatile(SCB_AIRCR) & AIRCR_PRIGROUP_MASK;
        write_volatile(SCB_AIRCR, AIRCR_VECTKEY | aircr | AIRCR_SYSRESETREQ);
        core::arch::asm!("dsb sy", options(nostack, preserves_flags));

        loop {
            core::hint::spin_loop();
        }
    }
}

#[cfg(target_arch = "arm")]
mod uno_r4_wifi_usb_mux {
    use core::ptr::{null_mut, read_volatile, write_volatile};

    const SYSTEM_PRCR: *mut u16 = 0x4001_E3FE as *mut u16;
    const SYSTEM_VBTBKR1: *mut u8 = 0x4001_E501 as *mut u8;
    const USBFS_USBMC: *mut u16 = 0x4009_00CC as *mut u16;

    const PRCR_KEY: u16 = 0xA500;
    const PRCR_PRC1_UNLOCK: u16 = PRCR_KEY | 0x0002;
    const PRCR_LOCK: u16 = PRCR_KEY;
    const USB_MUX_BACKUP_MARKER: u8 = 40;
    const USB_SWITCH_PIN_P408: u16 = 0x0408;
    const IOPORT_CFG_PORT_DIRECTION_OUTPUT: u32 = 0x0000_0004;
    const BSP_IO_LEVEL_HIGH: u32 = 1;
    const USBMC_VDCEN: u16 = 1 << 7;

    pub unsafe fn route_usb_c_to_ra4m1() {
        unlock_system_registers();
        write_volatile(SYSTEM_VBTBKR1, USB_MUX_BACKUP_MARKER);
        write_volatile(SYSTEM_VBTBKR1.add(1), 0);
        write_volatile(SYSTEM_VBTBKR1.add(2), 0);
        write_volatile(SYSTEM_VBTBKR1.add(3), 0);
        lock_system_registers();

        unsafe {
            ffi::R_IOPORT_PinCfg(
                null_mut(),
                USB_SWITCH_PIN_P408,
                IOPORT_CFG_PORT_DIRECTION_OUTPUT,
            );
            ffi::R_IOPORT_PinWrite(null_mut(), USB_SWITCH_PIN_P408, BSP_IO_LEVEL_HIGH);
        }
    }

    pub unsafe fn enable_ra4m1_usb_regulator() {
        let usbmc = read_volatile(USBFS_USBMC);
        write_volatile(USBFS_USBMC, usbmc | USBMC_VDCEN);
    }

    unsafe fn unlock_system_registers() {
        write_volatile(SYSTEM_PRCR, PRCR_PRC1_UNLOCK);
    }

    unsafe fn lock_system_registers() {
        write_volatile(SYSTEM_PRCR, PRCR_LOCK);
    }

    mod ffi {
        unsafe extern "C" {
            pub fn R_IOPORT_PinCfg(p_ctrl: *mut core::ffi::c_void, pin: u16, cfg: u32) -> i32;
            pub fn R_IOPORT_PinWrite(p_ctrl: *mut core::ffi::c_void, pin: u16, level: u32) -> i32;
        }
    }
}

#[cfg(target_arch = "arm")]
mod arduino_usb_malloc {
    use core::ptr::null_mut;
    use core::sync::atomic::{AtomicUsize, Ordering};

    use super::UNO_R4_WIFI_USB_DESCRIPTOR_HEAP_BYTES;

    #[repr(align(8))]
    struct AlignedHeap([u8; UNO_R4_WIFI_USB_DESCRIPTOR_HEAP_BYTES]);

    static mut HEAP: AlignedHeap = AlignedHeap([0; UNO_R4_WIFI_USB_DESCRIPTOR_HEAP_BYTES]);
    static NEXT: AtomicUsize = AtomicUsize::new(0);

    #[unsafe(no_mangle)]
    pub extern "C" fn malloc(size: usize) -> *mut core::ffi::c_void {
        if size == 0 {
            return null_mut();
        }

        let mut current = NEXT.load(Ordering::Relaxed);
        loop {
            let aligned = align_up(current, 8);
            let Some(next) = aligned.checked_add(size) else {
                return null_mut();
            };
            if next > UNO_R4_WIFI_USB_DESCRIPTOR_HEAP_BYTES {
                return null_mut();
            }

            match NEXT.compare_exchange(current, next, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => unsafe {
                    let base = core::ptr::addr_of_mut!(HEAP.0) as *mut u8;
                    return base.add(aligned) as *mut core::ffi::c_void;
                },
                Err(observed) => current = observed,
            }
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn free(_ptr: *mut core::ffi::c_void) {}

    const fn align_up(value: usize, align: usize) -> usize {
        (value + align - 1) & !(align - 1)
    }
}

#[cfg(target_arch = "arm")]
mod ffi {
    #[repr(C, packed)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TinyUsbLineCoding {
        bit_rate: u32,
        stop_bits: u8,
        parity: u8,
        data_bits: u8,
    }

    impl TinyUsbLineCoding {
        pub const fn bit_rate(self) -> u32 {
            self.bit_rate
        }

        pub const fn stop_bits(self) -> u8 {
            self.stop_bits
        }

        pub const fn parity(self) -> u8 {
            self.parity
        }

        pub const fn data_bits(self) -> u8 {
            self.data_bits
        }
    }

    impl Default for TinyUsbLineCoding {
        fn default() -> Self {
            Self {
                bit_rate: 115_200,
                stop_bits: 0,
                parity: 0,
                data_bits: 8,
            }
        }
    }

    unsafe extern "C" {
        #[link_name = "_Z10__USBStartv"]
        pub fn arduino_renesas_usb_start();
        pub fn tud_task_ext(timeout_ms: u32, in_isr: bool);
        pub fn tud_cdc_n_connected(itf: u8) -> bool;
        pub fn tud_cdc_n_get_line_state(itf: u8) -> u8;
        pub fn tud_cdc_n_get_line_coding(itf: u8, coding: *mut TinyUsbLineCoding);
        pub fn tud_cdc_n_available(itf: u8) -> u32;
        pub fn tud_cdc_n_read(itf: u8, buffer: *mut core::ffi::c_void, bufsize: u32) -> u32;
        pub fn tud_cdc_n_write(itf: u8, buffer: *const core::ffi::c_void, bufsize: u32) -> u32;
        pub fn tud_cdc_n_write_flush(itf: u8) -> u32;
        pub fn tud_cdc_n_write_available(itf: u8) -> u32;
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use board_vm_device::DeviceByteStream;
    use board_vm_usb_cdc::UsbCdcByteStream;
    use board_vm_usb_cdc::{UsbCdcParity, UsbCdcStopBits};

    struct FakeTinyUsb {
        connected: bool,
        line_coding: UsbCdcLineCoding,
        line_state: UsbCdcControlLineState,
        read: [u8; 8],
        read_len: usize,
        read_offset: usize,
        written: [u8; 96],
        written_len: usize,
        write_window: usize,
        polls: usize,
        flushes: usize,
        writes: usize,
        starts: usize,
    }

    impl FakeTinyUsb {
        fn new(read: &[u8]) -> Self {
            let mut input = [0; 8];
            input[..read.len()].copy_from_slice(read);
            Self {
                connected: true,
                line_coding: UsbCdcLineCoding::new(115_200),
                line_state: UsbCdcControlLineState::new(true, false),
                read: input,
                read_len: read.len(),
                read_offset: 0,
                written: [0; 96],
                written_len: 0,
                write_window: 96,
                polls: 0,
                flushes: 0,
                writes: 0,
                starts: 0,
            }
        }

        fn disconnected() -> Self {
            let mut api = Self::new(&[]);
            api.connected = false;
            api.line_state = UsbCdcControlLineState::disconnected();
            api
        }
    }

    impl TinyUsbCdcApi for FakeTinyUsb {
        fn start(&mut self) {
            self.starts += 1;
        }

        fn poll(&mut self) {
            self.polls += 1;
        }

        fn connected(&self) -> bool {
            self.connected
        }

        fn available(&self) -> usize {
            self.read_len - self.read_offset
        }

        fn read(&mut self, bytes: &mut [u8]) -> usize {
            let len = core::cmp::min(bytes.len(), self.available());
            let end = self.read_offset + len;
            bytes[..len].copy_from_slice(&self.read[self.read_offset..end]);
            self.read_offset = end;
            len
        }

        fn write_available(&self) -> usize {
            core::cmp::min(self.write_window, self.written.len() - self.written_len)
        }

        fn write(&mut self, bytes: &[u8]) -> usize {
            self.writes += 1;
            let len = core::cmp::min(bytes.len(), self.write_available());
            let end = self.written_len + len;
            self.written[self.written_len..end].copy_from_slice(&bytes[..len]);
            self.written_len = end;
            len
        }

        fn flush(&mut self) {
            self.flushes += 1;
        }

        fn line_coding(&self) -> UsbCdcLineCoding {
            self.line_coding
        }

        fn control_line_state(&self) -> UsbCdcControlLineState {
            self.line_state
        }
    }

    #[test]
    fn records_uno_r4_wifi_serial_usb_identity() {
        assert_eq!(UNO_R4_WIFI_USB_VID, 0x2341);
        assert_eq!(UNO_R4_WIFI_USB_PID, 0x006D);
        assert_eq!(UNO_R4_WIFI_BOOTLOADER_USB_PID, 0x1002);
        assert_eq!(UNO_R4_WIFI_USB_PRODUCT, "UNO R4 WiFi");
        assert_eq!(UNO_R4_WIFI_SERIAL_USB_NAME, "SerialUSB");
        assert_eq!(UNO_R4_WIFI_USB_MAX_PACKET_BYTES, 64);
        assert_eq!(UNO_R4_WIFI_CORTEX_VECTOR_TABLE_ENTRIES, 16);
        assert_eq!(UNO_R4_WIFI_ICU_VECTOR_TABLE_ENTRIES, 32);
        assert_eq!(UNO_R4_WIFI_MUTABLE_VECTOR_TABLE_ENTRIES, 48);
        assert_eq!(UNO_R4_WIFI_MUTABLE_VECTOR_TABLE_ALIGNMENT_BYTES, 256);
        assert_eq!(ARDUINO_RENESAS_USB_START_SYMBOL, "_Z10__USBStartv");
        assert_eq!(
            ARDUINO_RENESAS_USB_INSTALL_SERIAL_SYMBOL,
            "_Z18__USBInstallSerialv"
        );
        assert_eq!(
            UNO_R4_WIFI_CONFIGURE_USB_MUX_SYMBOL,
            "_Z17configure_usb_muxv"
        );
        assert_eq!(
            UNO_R4_WIFI_USB_POST_INITIALIZATION_SYMBOL,
            "_Z23usb_post_initializationv"
        );
        assert_eq!(UNO_R4_WIFI_USB_DESCRIPTOR_HEAP_BYTES, 512);
        assert_eq!(UNO_R4_WIFI_BOOTLOADER_TOUCH_BAUD, 1_200);
        assert_eq!(UNO_R4_WIFI_BOOTLOADER_DOUBLE_TAP_MAGIC, 0x0773_8135);
        assert_eq!(
            TINYUSB_CDC_LINE_STATE_CALLBACK_SYMBOL,
            "tud_cdc_line_state_cb"
        );
        assert_eq!(
            TINYUSB_CDC_LINE_CODING_CALLBACK_SYMBOL,
            "tud_cdc_line_coding_cb"
        );
    }

    #[test]
    fn bootloader_touch_matches_arduino_serialusb_reset_rule() {
        assert!(bootloader_touch_requests_reset(
            UsbCdcLineCoding::new(1_200),
            UsbCdcControlLineState::disconnected(),
        ));
        assert!(bootloader_touch_requests_reset(
            UsbCdcLineCoding::new(1_200),
            UsbCdcControlLineState::new(false, true),
        ));
        assert!(!bootloader_touch_requests_reset(
            UsbCdcLineCoding::new(1_200),
            UsbCdcControlLineState::new(true, false),
        ));
        assert!(!bootloader_touch_requests_reset(
            UsbCdcLineCoding::new(115_200),
            UsbCdcControlLineState::disconnected(),
        ));
    }

    #[test]
    fn must_be_started_before_streaming() {
        let mut cdc = UnoR4WifiSerialUsbCdc::new(FakeTinyUsb::new(&[0x42]));
        assert_eq!(cdc.read_byte(), Err(UnoR4UsbCdcError::NotStarted));

        cdc.begin();
        assert_eq!(cdc.read_byte().unwrap(), 0x42);
        assert_eq!(cdc.api().polls, 1);
        assert_eq!(cdc.api().starts, 1);
    }

    #[test]
    fn adapts_tinyusb_api_to_generic_usb_cdc_byte_stream() {
        let cdc = UnoR4WifiSerialUsbCdc::started(FakeTinyUsb::new(&[0x11, 0x22]));
        let mut stream = UsbCdcByteStream::new(cdc);

        assert_eq!(stream.read_byte().unwrap(), 0x11);
        stream.write_all(&[0xA0, 0xB1]).unwrap();
        stream.flush().unwrap();

        let cdc = stream.into_inner();
        let api = cdc.into_inner();
        assert_eq!(&api.written[..api.written_len], &[0xA0, 0xB1]);
        assert_eq!(api.writes, 1);
        assert_eq!(api.flushes, 2);
        assert_eq!(api.line_coding, UsbCdcLineCoding::new(115_200));
        assert!(api.line_state.connected());
    }

    #[test]
    fn try_read_byte_polls_once_and_reports_idle() {
        let mut cdc = UnoR4WifiSerialUsbCdc::started(FakeTinyUsb::new(&[]));

        assert_eq!(cdc.try_read_byte().unwrap(), None);
        assert_eq!(cdc.api().polls, 1);
    }

    #[test]
    fn try_read_byte_returns_available_byte_without_spinning() {
        let mut cdc = UnoR4WifiSerialUsbCdc::started(FakeTinyUsb::new(&[0x33]));

        assert_eq!(cdc.try_read_byte().unwrap(), Some(0x33));
        assert_eq!(cdc.api().polls, 1);
        assert_eq!(cdc.api().read_offset, 1);
    }

    #[test]
    fn writes_packets_through_tinyusb_space_windows() {
        let mut cdc = UnoR4WifiSerialUsbCdc::started(FakeTinyUsb::new(&[]));
        cdc.api_mut().write_window = 2;

        cdc.write_packet(&[1, 2, 3, 4, 5]).unwrap();

        let api = cdc.into_inner();
        assert_eq!(&api.written[..api.written_len], &[1, 2, 3, 4, 5]);
        assert_eq!(api.writes, 3);
        assert_eq!(api.flushes, 3);
    }

    #[test]
    fn rejects_packets_larger_than_one_full_speed_cdc_frame() {
        let mut cdc = UnoR4WifiSerialUsbCdc::started(FakeTinyUsb::new(&[]));

        assert_eq!(
            cdc.write_packet(&[0x55; 65]),
            Err(UnoR4UsbCdcError::PacketTooLarge)
        );
    }

    #[test]
    fn reports_disconnected_ports() {
        let mut cdc = UnoR4WifiSerialUsbCdc::started(FakeTinyUsb::disconnected());

        assert_eq!(cdc.read_byte(), Err(UnoR4UsbCdcError::NotConnected));
        assert_eq!(
            cdc.write_packet(&[0x99]),
            Err(UnoR4UsbCdcError::NotConnected)
        );
        assert!(!cdc.control_line_state().connected());
    }

    #[test]
    fn exposes_host_supplied_line_settings() {
        let mut api = FakeTinyUsb::new(&[]);
        api.line_coding = UsbCdcLineCoding::new(57_600)
            .stop_bits(UsbCdcStopBits::Two)
            .parity(UsbCdcParity::Even)
            .data_bits(7);
        api.line_state = UsbCdcControlLineState::new(true, true);
        let cdc = UnoR4WifiSerialUsbCdc::started(api);

        assert_eq!(cdc.line_coding().baud_rate, 57_600);
        assert_eq!(cdc.line_coding().stop_bits, UsbCdcStopBits::Two);
        assert_eq!(cdc.line_coding().parity, UsbCdcParity::Even);
        assert_eq!(cdc.line_coding().data_bits, 7);
        assert_eq!(
            cdc.control_line_state(),
            UsbCdcControlLineState::new(true, true)
        );
    }
}
