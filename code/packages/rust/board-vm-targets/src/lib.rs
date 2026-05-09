#![no_std]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardFamily {
    ArduinoUnoR4,
    Esp32,
    RaspberryPiPico,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardLed {
    Gpio(u8),
    WirelessChipGpio(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WirelessTransport {
    Wifi,
    BluetoothLe,
    BluetoothClassic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WirelessInterfaceInfo {
    pub transport: WirelessTransport,
    pub chip: &'static str,
    pub command_transport: bool,
    pub ota_update: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoardTargetInfo {
    pub board_id: &'static str,
    pub display_name: &'static str,
    pub family: BoardFamily,
    pub runtime_id: &'static str,
    pub mcu: &'static str,
    pub core: &'static str,
    pub rust_target: &'static str,
    pub clock_hz: u32,
    pub operating_voltage_mv: u16,
    pub onboard_led: Option<OnboardLed>,
    pub digital_pin_count: usize,
    pub wireless: &'static [WirelessInterfaceInfo],
    pub capabilities: &'static [&'static str],
}

pub const BLINK_MVP_CAPABILITIES: [&str; 8] = [
    "transport.serial",
    "gpio.open",
    "gpio.write",
    "gpio.read",
    "gpio.close",
    "time.sleep_ms",
    "time.now_ms",
    "program.ram_exec",
];

pub const UNO_R4_WIFI_CAPABILITIES: [&str; 11] = [
    "transport.serial",
    "transport.wifi",
    "transport.bluetooth_le",
    "ota.wifi",
    "gpio.open",
    "gpio.write",
    "gpio.read",
    "gpio.close",
    "time.sleep_ms",
    "time.now_ms",
    "program.ram_exec",
];

pub const ESP32_CAPABILITIES: [&str; 12] = [
    "transport.serial",
    "transport.wifi",
    "transport.bluetooth_le",
    "transport.bluetooth_classic",
    "ota.wifi",
    "gpio.open",
    "gpio.write",
    "gpio.read",
    "gpio.close",
    "time.sleep_ms",
    "time.now_ms",
    "program.ram_exec",
];

pub const PICO_W_CAPABILITIES: [&str; 12] = [
    "transport.serial",
    "transport.wifi",
    "transport.bluetooth_le",
    "transport.bluetooth_classic",
    "ota.wifi",
    "gpio.open",
    "gpio.write",
    "gpio.read",
    "gpio.close",
    "time.sleep_ms",
    "time.now_ms",
    "program.ram_exec",
];

pub const UNO_R4_WIFI_WIRELESS: [WirelessInterfaceInfo; 2] = [
    WirelessInterfaceInfo {
        transport: WirelessTransport::Wifi,
        chip: "ESP32-S3 coprocessor",
        command_transport: true,
        ota_update: true,
    },
    WirelessInterfaceInfo {
        transport: WirelessTransport::BluetoothLe,
        chip: "ESP32-S3 coprocessor",
        command_transport: true,
        ota_update: false,
    },
];

pub const ESP32_WIRELESS: [WirelessInterfaceInfo; 3] = [
    WirelessInterfaceInfo {
        transport: WirelessTransport::Wifi,
        chip: "ESP32-WROOM-32",
        command_transport: true,
        ota_update: true,
    },
    WirelessInterfaceInfo {
        transport: WirelessTransport::BluetoothLe,
        chip: "ESP32-WROOM-32",
        command_transport: true,
        ota_update: false,
    },
    WirelessInterfaceInfo {
        transport: WirelessTransport::BluetoothClassic,
        chip: "ESP32-WROOM-32",
        command_transport: true,
        ota_update: false,
    },
];

pub const PICO_W_WIRELESS: [WirelessInterfaceInfo; 3] = [
    WirelessInterfaceInfo {
        transport: WirelessTransport::Wifi,
        chip: "Infineon CYW43439",
        command_transport: true,
        ota_update: true,
    },
    WirelessInterfaceInfo {
        transport: WirelessTransport::BluetoothLe,
        chip: "Infineon CYW43439",
        command_transport: true,
        ota_update: false,
    },
    WirelessInterfaceInfo {
        transport: WirelessTransport::BluetoothClassic,
        chip: "Infineon CYW43439",
        command_transport: true,
        ota_update: false,
    },
];

pub const BOARD_TARGETS: [BoardTargetInfo; 5] = [
    BoardTargetInfo {
        board_id: board_vm_uno_r4::UNO_R4_MINIMA.board_id,
        display_name: board_vm_uno_r4::UNO_R4_MINIMA.display_name,
        family: BoardFamily::ArduinoUnoR4,
        runtime_id: board_vm_uno_r4::UNO_R4_VM_RUNTIME_ID,
        mcu: board_vm_uno_r4::UNO_R4_MINIMA.mcu,
        core: board_vm_uno_r4::UNO_R4_MINIMA.core,
        rust_target: board_vm_uno_r4::UNO_R4_MINIMA.rust_target,
        clock_hz: board_vm_uno_r4::UNO_R4_MINIMA.clock_hz,
        operating_voltage_mv: board_vm_uno_r4::UNO_R4_MINIMA.operating_voltage_mv,
        onboard_led: Some(OnboardLed::Gpio(
            board_vm_uno_r4::UNO_R4_MINIMA.onboard_led_pin,
        )),
        digital_pin_count: board_vm_uno_r4::UNO_R4_MINIMA.digital_pins.len(),
        wireless: &[],
        capabilities: &BLINK_MVP_CAPABILITIES,
    },
    BoardTargetInfo {
        board_id: board_vm_uno_r4::UNO_R4_WIFI.board_id,
        display_name: board_vm_uno_r4::UNO_R4_WIFI.display_name,
        family: BoardFamily::ArduinoUnoR4,
        runtime_id: board_vm_uno_r4::UNO_R4_VM_RUNTIME_ID,
        mcu: board_vm_uno_r4::UNO_R4_WIFI.mcu,
        core: board_vm_uno_r4::UNO_R4_WIFI.core,
        rust_target: board_vm_uno_r4::UNO_R4_WIFI.rust_target,
        clock_hz: board_vm_uno_r4::UNO_R4_WIFI.clock_hz,
        operating_voltage_mv: board_vm_uno_r4::UNO_R4_WIFI.operating_voltage_mv,
        onboard_led: Some(OnboardLed::Gpio(
            board_vm_uno_r4::UNO_R4_WIFI.onboard_led_pin,
        )),
        digital_pin_count: board_vm_uno_r4::UNO_R4_WIFI.digital_pins.len(),
        wireless: &UNO_R4_WIFI_WIRELESS,
        capabilities: &UNO_R4_WIFI_CAPABILITIES,
    },
    BoardTargetInfo {
        board_id: board_vm_esp32::ESP32_DEVKIT_V1.board_id,
        display_name: board_vm_esp32::ESP32_DEVKIT_V1.display_name,
        family: BoardFamily::Esp32,
        runtime_id: board_vm_esp32::ESP32_VM_RUNTIME_ID,
        mcu: board_vm_esp32::ESP32_DEVKIT_V1.mcu,
        core: board_vm_esp32::ESP32_DEVKIT_V1.core,
        rust_target: board_vm_esp32::ESP32_DEVKIT_V1.rust_target,
        clock_hz: board_vm_esp32::ESP32_DEVKIT_V1.clock_hz,
        operating_voltage_mv: board_vm_esp32::ESP32_DEVKIT_V1.operating_voltage_mv,
        onboard_led: match board_vm_esp32::ESP32_DEVKIT_V1.onboard_led_pin {
            Some(pin) => Some(OnboardLed::Gpio(pin)),
            None => None,
        },
        digital_pin_count: board_vm_esp32::ESP32_DEVKIT_V1.digital_pins.len(),
        wireless: &ESP32_WIRELESS,
        capabilities: &ESP32_CAPABILITIES,
    },
    BoardTargetInfo {
        board_id: board_vm_pico::PICO.board_id,
        display_name: board_vm_pico::PICO.display_name,
        family: BoardFamily::RaspberryPiPico,
        runtime_id: board_vm_pico::PICO_VM_RUNTIME_ID,
        mcu: board_vm_pico::PICO.mcu,
        core: board_vm_pico::PICO.core,
        rust_target: board_vm_pico::PICO.rust_target,
        clock_hz: board_vm_pico::PICO.clock_hz,
        operating_voltage_mv: board_vm_pico::PICO.operating_voltage_mv,
        onboard_led: match board_vm_pico::PICO.onboard_led {
            Some(board_vm_pico::OnboardLed::Gpio(pin)) => Some(OnboardLed::Gpio(pin)),
            Some(board_vm_pico::OnboardLed::WirelessChipGpio(pin)) => {
                Some(OnboardLed::WirelessChipGpio(pin))
            }
            None => None,
        },
        digital_pin_count: board_vm_pico::PICO.digital_pins.len(),
        wireless: &[],
        capabilities: &BLINK_MVP_CAPABILITIES,
    },
    BoardTargetInfo {
        board_id: board_vm_pico::PICO_W.board_id,
        display_name: board_vm_pico::PICO_W.display_name,
        family: BoardFamily::RaspberryPiPico,
        runtime_id: board_vm_pico::PICO_VM_RUNTIME_ID,
        mcu: board_vm_pico::PICO_W.mcu,
        core: board_vm_pico::PICO_W.core,
        rust_target: board_vm_pico::PICO_W.rust_target,
        clock_hz: board_vm_pico::PICO_W.clock_hz,
        operating_voltage_mv: board_vm_pico::PICO_W.operating_voltage_mv,
        onboard_led: match board_vm_pico::PICO_W.onboard_led {
            Some(board_vm_pico::OnboardLed::Gpio(pin)) => Some(OnboardLed::Gpio(pin)),
            Some(board_vm_pico::OnboardLed::WirelessChipGpio(pin)) => {
                Some(OnboardLed::WirelessChipGpio(pin))
            }
            None => None,
        },
        digital_pin_count: board_vm_pico::PICO_W.digital_pins.len(),
        wireless: &PICO_W_WIRELESS,
        capabilities: &PICO_W_CAPABILITIES,
    },
];

pub fn all_targets() -> &'static [BoardTargetInfo] {
    &BOARD_TARGETS
}

pub fn find_target(board_id: &str) -> Option<&'static BoardTargetInfo> {
    BOARD_TARGETS
        .iter()
        .find(|target| target.board_id == board_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_lists_current_board_families() {
        assert!(find_target("arduino-uno-r4-wifi").is_some());
        assert!(find_target("esp32-devkit-v1").is_some());
        assert!(find_target("raspberry-pi-pico").is_some());
        assert!(find_target("raspberry-pi-pico-w").is_some());
    }

    #[test]
    fn registry_preserves_led_routes() {
        assert_eq!(
            find_target("raspberry-pi-pico").unwrap().onboard_led,
            Some(OnboardLed::Gpio(25))
        );
        assert_eq!(
            find_target("raspberry-pi-pico-w").unwrap().onboard_led,
            Some(OnboardLed::WirelessChipGpio(0))
        );
        assert_eq!(
            find_target("esp32-devkit-v1").unwrap().onboard_led,
            Some(OnboardLed::Gpio(2))
        );
    }

    #[test]
    fn registry_exposes_common_runtime_capabilities() {
        let pico = find_target("raspberry-pi-pico").unwrap();
        assert_eq!(pico.runtime_id, "board-vm-pico");
        assert!(pico.capabilities.contains(&"transport.serial"));
        assert!(pico.capabilities.contains(&"gpio.open"));
        assert!(pico.capabilities.contains(&"program.ram_exec"));
    }

    #[test]
    fn registry_exposes_wireless_only_for_wireless_boards() {
        let uno_r4_minima = find_target("arduino-uno-r4-minima").unwrap();
        let uno_r4_wifi = find_target("arduino-uno-r4-wifi").unwrap();
        let esp32 = find_target("esp32-devkit-v1").unwrap();
        let pico = find_target("raspberry-pi-pico").unwrap();
        let pico_w = find_target("raspberry-pi-pico-w").unwrap();

        assert!(uno_r4_minima.wireless.is_empty());
        assert!(pico.wireless.is_empty());
        assert!(uno_r4_wifi.capabilities.contains(&"transport.wifi"));
        assert!(uno_r4_wifi.capabilities.contains(&"transport.bluetooth_le"));
        assert!(!uno_r4_wifi
            .capabilities
            .contains(&"transport.bluetooth_classic"));
        assert!(esp32.capabilities.contains(&"transport.bluetooth_classic"));
        assert!(pico_w.capabilities.contains(&"transport.wifi"));
        assert!(pico_w.capabilities.contains(&"ota.wifi"));
        assert!(pico_w
            .wireless
            .iter()
            .any(|interface| interface.transport == WirelessTransport::Wifi
                && interface.command_transport
                && interface.ota_update));
    }
}
