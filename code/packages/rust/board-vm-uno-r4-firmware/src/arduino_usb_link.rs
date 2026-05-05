// Data-only link manifest for the Uno R4 WiFi Arduino/TinyUSB USB device stack.
//
// The Rust firmware owns the VM, protocol, device state, and CDC byte stream.
// Arduino's Renesas core still owns the RA4M1 TinyUSB descriptors, IRQ
// plumbing, and FSP USB driver objects needed by `__USBStart`.

pub const ARDUINO_RENESAS_UNO_CORE_VERSION: &str = "1.5.3";
pub const UNO_R4_WIFI_FQBN: &str = "arduino:renesas_uno:unor4wifi";
pub const UNO_R4_WIFI_VARIANT: &str = "UNOWIFIR4";
pub const UNO_R4_WIFI_RUNTIME_USB_VID: u16 = 0x2341;
pub const UNO_R4_WIFI_RUNTIME_USB_PID: u16 = 0x006D;
pub const UNO_R4_WIFI_BOOTLOADER_USB_PID: u16 = 0x1002;
pub const UNO_R4_WIFI_USB_RHPORT: u8 = 0;
pub const UNO_R4_WIFI_CORTEX_VECTOR_TABLE_ENTRIES: usize = 16;
pub const UNO_R4_WIFI_ICU_VECTOR_TABLE_ENTRIES: usize = 32;
pub const UNO_R4_WIFI_MUTABLE_VECTOR_TABLE_ENTRIES: usize =
    UNO_R4_WIFI_CORTEX_VECTOR_TABLE_ENTRIES + UNO_R4_WIFI_ICU_VECTOR_TABLE_ENTRIES;
pub const UNO_R4_WIFI_MUTABLE_VECTOR_TABLE_ALIGNMENT_BYTES: usize = 256;

pub const ARDUINO_CORE_ENV_VAR: &str = "BOARD_VM_UNO_R4_ARDUINO_CORE";
pub const ARDUINO_USB_LINK_ENV_VAR: &str = "BOARD_VM_UNO_R4_LINK_ARDUINO_USB";
pub const ARDUINO_ARM_GCC_ENV_VAR: &str = "BOARD_VM_UNO_R4_ARM_GCC";
pub const ARDUINO_ARM_GXX_ENV_VAR: &str = "BOARD_VM_UNO_R4_ARM_GXX";
pub const ARDUINO_ARM_AR_ENV_VAR: &str = "BOARD_VM_UNO_R4_ARM_AR";
pub const ARDUINO_ARM_COMPAT_ROOT_ENV_VAR: &str = "BOARD_VM_UNO_R4_ARM_COMPAT_ROOT";

pub const ARDUINO_USB_START_SYMBOL: &str = "_Z10__USBStartv";
pub const RUST_USB_INSTALL_SERIAL_SYMBOL: &str = "_Z18__USBInstallSerialv";
pub const RUST_USB_CONFIGURE_MUX_SYMBOL: &str = "_Z17configure_usb_muxv";
pub const RUST_USB_POST_INITIALIZATION_SYMBOL: &str = "_Z23usb_post_initializationv";

pub const UNO_R4_WIFI_FSP_ARCHIVE: &str = "variants/UNOWIFIR4/libs/libfsp.a";
pub const UNO_R4_WIFI_FSP_LINKER_SCRIPT: &str = "variants/UNOWIFIR4/fsp.ld";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArduinoSourceLanguage {
    C,
    Cxx,
    StaticArchive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArduinoLinkSource {
    pub language: ArduinoSourceLanguage,
    pub path: &'static str,
}

impl ArduinoLinkSource {
    pub const fn c(path: &'static str) -> Self {
        Self {
            language: ArduinoSourceLanguage::C,
            path,
        }
    }

    pub const fn cxx(path: &'static str) -> Self {
        Self {
            language: ArduinoSourceLanguage::Cxx,
            path,
        }
    }

    pub const fn static_archive(path: &'static str) -> Self {
        Self {
            language: ArduinoSourceLanguage::StaticArchive,
            path,
        }
    }
}

pub const ARDUINO_USB_LINK_SOURCES: &[ArduinoLinkSource] = &[
    ArduinoLinkSource::c("cores/arduino/tinyusb/tusb.c"),
    ArduinoLinkSource::c("cores/arduino/tinyusb/common/tusb_fifo.c"),
    ArduinoLinkSource::c("cores/arduino/tinyusb/device/usbd.c"),
    ArduinoLinkSource::c("cores/arduino/tinyusb/device/usbd_control.c"),
    ArduinoLinkSource::c("cores/arduino/tinyusb/class/cdc/cdc_device.c"),
    ArduinoLinkSource::c("cores/arduino/tinyusb/class/hid/hid_device.c"),
    ArduinoLinkSource::c("cores/arduino/tinyusb/rusb2/dcd_rusb2.c"),
    ArduinoLinkSource::c("cores/arduino/tinyusb/rusb2/rusb2_common.c"),
    ArduinoLinkSource::cxx("cores/arduino/IRQManager.cpp"),
    ArduinoLinkSource::cxx("cores/arduino/usb/USB.cpp"),
    ArduinoLinkSource::static_archive(UNO_R4_WIFI_FSP_ARCHIVE),
];

pub const ARDUINO_USB_LINK_INCLUDE_DIRS: &[&str] = &[
    "cores/arduino",
    "cores/arduino/api",
    "cores/arduino/tinyusb",
    "cores/arduino/tinyusb/common",
    "cores/arduino/tinyusb/device",
    "cores/arduino/usb",
    "cores/arduino/api/deprecated",
    "cores/arduino/api/deprecated-avr-comp",
    "variants/UNOWIFIR4",
    "variants/UNOWIFIR4/includes/ra/fsp/inc",
    "variants/UNOWIFIR4/includes/ra/fsp/inc/api",
    "variants/UNOWIFIR4/includes/ra/fsp/inc/instances",
    "variants/UNOWIFIR4/includes/ra/arm/CMSIS_5/CMSIS/Core/Include",
    "variants/UNOWIFIR4/includes/ra_gen",
    "variants/UNOWIFIR4/includes/ra_cfg/fsp_cfg/bsp",
    "variants/UNOWIFIR4/includes/ra_cfg/fsp_cfg",
    "variants/UNOWIFIR4/includes/ra/fsp/src/r_usb_basic/src/driver/inc",
];

pub const ARDUINO_USB_LINK_DEFINES: &[&str] = &[
    "ARDUINO=10607",
    "ARDUINO_UNOWIFIR4",
    "ARDUINO_ARCH_RENESAS_UNO",
    "ARDUINO_ARCH_RENESAS",
    "F_CPU=48000000",
    "NO_USB",
    "BACKTRACE_SUPPORT",
    "ARDUINO_UNOR4_WIFI",
    "ARDUINO_FSP",
    "_XOPEN_SOURCE=700",
    "_RA_CORE=CM4",
    "_RENESAS_RA_",
    "CFG_TUSB_MCU=OPT_MCU_RAXXX",
];

pub const ARDUINO_USB_LINK_CFLAGS: &[&str] = &[
    "-c",
    "-w",
    "-Os",
    "-g3",
    "-nostdlib",
    "-MMD",
    "-std=gnu11",
    "-mcpu=cortex-m4",
    "-mthumb",
    "-mfloat-abi=hard",
    "-mfpu=fpv4-sp-d16",
    "-ffunction-sections",
    "-fdata-sections",
    "-fsigned-char",
    "-fno-builtin",
];

pub const ARDUINO_USB_LINK_CXXFLAGS: &[&str] = &[
    "-c",
    "-w",
    "-Os",
    "-g3",
    "-fno-use-cxa-atexit",
    "-fno-threadsafe-statics",
    "-fno-rtti",
    "-fno-exceptions",
    "-MMD",
    "-nostdlib",
    "-std=gnu++17",
    "-mcpu=cortex-m4",
    "-mthumb",
    "-mfloat-abi=hard",
    "-mfpu=fpv4-sp-d16",
    "-ffunction-sections",
    "-fdata-sections",
    "-fsigned-char",
    "-fno-builtin",
];

pub const RUST_PROVIDED_USB_SYMBOLS: &[&str] = &[
    RUST_USB_INSTALL_SERIAL_SYMBOL,
    RUST_USB_CONFIGURE_MUX_SYMBOL,
    RUST_USB_POST_INITIALIZATION_SYMBOL,
];

pub const ARDUINO_PROVIDED_USB_SYMBOLS: &[&str] = &[
    ARDUINO_USB_START_SYMBOL,
    "R_IOPORT_PinCfg",
    "R_IOPORT_PinWrite",
    "tud_task_ext",
    "tud_cdc_n_connected",
    "tud_cdc_n_get_line_state",
    "tud_cdc_n_get_line_coding",
    "tud_cdc_n_available",
    "tud_cdc_n_read",
    "tud_cdc_n_write",
    "tud_cdc_n_write_flush",
    "tud_cdc_n_write_available",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn contains_source(path: &str) -> bool {
        ARDUINO_USB_LINK_SOURCES
            .iter()
            .any(|source| source.path == path)
    }

    #[test]
    fn records_uno_r4_wifi_arduino_usb_identity() {
        assert_eq!(ARDUINO_RENESAS_UNO_CORE_VERSION, "1.5.3");
        assert_eq!(UNO_R4_WIFI_FQBN, "arduino:renesas_uno:unor4wifi");
        assert_eq!(UNO_R4_WIFI_VARIANT, "UNOWIFIR4");
        assert_eq!(UNO_R4_WIFI_RUNTIME_USB_VID, 0x2341);
        assert_eq!(UNO_R4_WIFI_RUNTIME_USB_PID, 0x006D);
        assert_eq!(UNO_R4_WIFI_BOOTLOADER_USB_PID, 0x1002);
        assert_eq!(UNO_R4_WIFI_USB_RHPORT, 0);
        assert_eq!(UNO_R4_WIFI_CORTEX_VECTOR_TABLE_ENTRIES, 16);
        assert_eq!(UNO_R4_WIFI_ICU_VECTOR_TABLE_ENTRIES, 32);
        assert_eq!(UNO_R4_WIFI_MUTABLE_VECTOR_TABLE_ENTRIES, 48);
        assert_eq!(UNO_R4_WIFI_MUTABLE_VECTOR_TABLE_ALIGNMENT_BYTES, 256);
        assert_eq!(ARDUINO_USB_LINK_ENV_VAR, "BOARD_VM_UNO_R4_LINK_ARDUINO_USB");
        assert_eq!(
            ARDUINO_ARM_COMPAT_ROOT_ENV_VAR,
            "BOARD_VM_UNO_R4_ARM_COMPAT_ROOT"
        );
    }

    #[test]
    fn link_manifest_includes_usb_start_tinyusb_and_fsp_objects() {
        assert!(contains_source("cores/arduino/usb/USB.cpp"));
        assert!(contains_source("cores/arduino/IRQManager.cpp"));
        assert!(contains_source("cores/arduino/tinyusb/tusb.c"));
        assert!(contains_source(
            "cores/arduino/tinyusb/class/cdc/cdc_device.c"
        ));
        assert!(contains_source("cores/arduino/tinyusb/rusb2/dcd_rusb2.c"));
        assert!(contains_source(UNO_R4_WIFI_FSP_ARCHIVE));
        assert!(!contains_source("cores/arduino/usb/SerialUSB.cpp"));
    }

    #[test]
    fn link_manifest_carries_arduino_flags_needed_by_tinyusb() {
        assert!(ARDUINO_USB_LINK_INCLUDE_DIRS.contains(&"cores/arduino/tinyusb"));
        assert!(ARDUINO_USB_LINK_INCLUDE_DIRS.contains(&"cores/arduino/api/deprecated"));
        assert!(ARDUINO_USB_LINK_INCLUDE_DIRS.contains(&"variants/UNOWIFIR4"));
        assert!(ARDUINO_USB_LINK_DEFINES.contains(&"ARDUINO_UNOWIFIR4"));
        assert!(ARDUINO_USB_LINK_DEFINES.contains(&"ARDUINO_ARCH_RENESAS_UNO"));
        assert!(ARDUINO_USB_LINK_DEFINES.contains(&"CFG_TUSB_MCU=OPT_MCU_RAXXX"));
        assert!(ARDUINO_USB_LINK_DEFINES.contains(&"ARDUINO_UNOR4_WIFI"));
        assert!(ARDUINO_USB_LINK_CFLAGS.contains(&"-std=gnu11"));
        assert!(ARDUINO_USB_LINK_CFLAGS.contains(&"-mcpu=cortex-m4"));
        assert!(ARDUINO_USB_LINK_CXXFLAGS.contains(&"-fno-exceptions"));
        assert!(ARDUINO_USB_LINK_CXXFLAGS.contains(&"-fno-threadsafe-statics"));
    }

    #[test]
    fn rust_backend_supplies_the_serial_descriptor_and_mux_hooks() {
        assert_eq!(ARDUINO_USB_START_SYMBOL, "_Z10__USBStartv");
        assert!(RUST_PROVIDED_USB_SYMBOLS.contains(&"_Z18__USBInstallSerialv"));
        assert!(RUST_PROVIDED_USB_SYMBOLS.contains(&"_Z17configure_usb_muxv"));
        assert!(RUST_PROVIDED_USB_SYMBOLS.contains(&"_Z23usb_post_initializationv"));
        assert!(ARDUINO_PROVIDED_USB_SYMBOLS.contains(&"R_IOPORT_PinCfg"));
        assert!(ARDUINO_PROVIDED_USB_SYMBOLS.contains(&"R_IOPORT_PinWrite"));
        assert!(ARDUINO_PROVIDED_USB_SYMBOLS.contains(&"tud_cdc_n_read"));
        assert!(ARDUINO_PROVIDED_USB_SYMBOLS.contains(&"tud_cdc_n_write_flush"));
    }
}
