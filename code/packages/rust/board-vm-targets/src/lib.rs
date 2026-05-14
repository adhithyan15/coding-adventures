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
pub struct LedMatrixInfo {
    pub rows: u8,
    pub columns: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigitalPinInfo {
    pub pin: u8,
    pub label: &'static str,
    pub supports_input: bool,
    pub supports_output: bool,
    pub supports_pullup: bool,
    pub supports_pulldown: bool,
    pub supports_adc: bool,
    pub supports_pwm: bool,
    pub supports_dac: bool,
    pub supports_touch: bool,
    pub supports_interrupt: bool,
    pub boot_strap: bool,
    pub notes: &'static str,
}

impl DigitalPinInfo {
    const fn placeholder() -> Self {
        Self {
            pin: 0,
            label: "",
            supports_input: false,
            supports_output: false,
            supports_pullup: false,
            supports_pulldown: false,
            supports_adc: false,
            supports_pwm: false,
            supports_dac: false,
            supports_touch: false,
            supports_interrupt: false,
            boot_strap: false,
            notes: "",
        }
    }
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
    pub led_matrix: Option<LedMatrixInfo>,
    pub digital_pin_count: usize,
    pub digital_pins: &'static [DigitalPinInfo],
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

pub const UNO_R4_MINIMA_CAPABILITIES: [&str; 11] = [
    "transport.serial",
    "gpio.open",
    "gpio.write",
    "gpio.read",
    "gpio.close",
    "time.sleep_ms",
    "time.now_ms",
    "pwm.write",
    "adc.read",
    "dac.write_u12",
    "program.ram_exec",
];

pub const UNO_R4_WIFI_CAPABILITIES: [&str; 15] = [
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
    "pwm.write",
    "adc.read",
    "dac.write_u12",
    "led_matrix.frame",
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

pub const UNO_R4_WIFI_LED_MATRIX: LedMatrixInfo = LedMatrixInfo {
    rows: 8,
    columns: 12,
};

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

pub const UNO_R4_DIGITAL_PINS: [DigitalPinInfo; 20] =
    map_uno_r4_digital_pins(board_vm_uno_r4::UNO_R4_DIGITAL_PINS);

pub const ESP32_DIGITAL_PINS: [DigitalPinInfo; 30] =
    map_esp32_digital_pins(board_vm_esp32::ESP32_DEVKIT_V1_DIGITAL_PINS);

pub const PICO_DIGITAL_PINS: [DigitalPinInfo; 29] =
    map_pico_digital_pins(board_vm_pico::PICO_DIGITAL_PINS);

const fn map_uno_r4_digital_pins<const N: usize>(
    source: [board_vm_uno_r4::DigitalPinDescriptor; N],
) -> [DigitalPinInfo; N] {
    let mut pins = [DigitalPinInfo::placeholder(); N];
    let mut index = 0;
    while index < N {
        pins[index] = uno_r4_digital_pin(source[index]);
        index += 1;
    }
    pins
}

const fn uno_r4_digital_pin(pin: board_vm_uno_r4::DigitalPinDescriptor) -> DigitalPinInfo {
    DigitalPinInfo {
        pin: pin.arduino_pin,
        label: pin.label,
        supports_input: true,
        supports_output: true,
        supports_pullup: true,
        supports_pulldown: false,
        supports_adc: pin.supports_adc,
        supports_pwm: pin.supports_pwm,
        supports_dac: pin.supports_dac,
        supports_touch: false,
        supports_interrupt: pin.supports_interrupt,
        boot_strap: false,
        notes: pin.notes,
    }
}

const fn map_esp32_digital_pins<const N: usize>(
    source: [board_vm_esp32::DigitalPinDescriptor; N],
) -> [DigitalPinInfo; N] {
    let mut pins = [DigitalPinInfo::placeholder(); N];
    let mut index = 0;
    while index < N {
        pins[index] = esp32_digital_pin(source[index]);
        index += 1;
    }
    pins
}

const fn esp32_digital_pin(pin: board_vm_esp32::DigitalPinDescriptor) -> DigitalPinInfo {
    DigitalPinInfo {
        pin: pin.gpio,
        label: pin.label,
        supports_input: pin.supports_input,
        supports_output: pin.supports_output,
        supports_pullup: pin.supports_pullup,
        supports_pulldown: pin.supports_pulldown,
        supports_adc: pin.supports_adc,
        supports_pwm: pin.supports_output,
        supports_dac: false,
        supports_touch: pin.supports_touch,
        supports_interrupt: pin.supports_input,
        boot_strap: pin.boot_strap,
        notes: pin.notes,
    }
}

const fn map_pico_digital_pins<const N: usize>(
    source: [board_vm_pico::DigitalPinDescriptor; N],
) -> [DigitalPinInfo; N] {
    let mut pins = [DigitalPinInfo::placeholder(); N];
    let mut index = 0;
    while index < N {
        pins[index] = pico_digital_pin(source[index]);
        index += 1;
    }
    pins
}

const fn pico_digital_pin(pin: board_vm_pico::DigitalPinDescriptor) -> DigitalPinInfo {
    DigitalPinInfo {
        pin: pin.gpio,
        label: pin.label,
        supports_input: pin.supports_input,
        supports_output: pin.supports_output,
        supports_pullup: pin.supports_pullup,
        supports_pulldown: pin.supports_pulldown,
        supports_adc: pin.supports_adc,
        supports_pwm: pin.supports_pwm,
        supports_dac: false,
        supports_touch: false,
        supports_interrupt: pin.supports_input,
        boot_strap: false,
        notes: pin.notes,
    }
}

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
        led_matrix: None,
        digital_pin_count: UNO_R4_DIGITAL_PINS.len(),
        digital_pins: &UNO_R4_DIGITAL_PINS,
        wireless: &[],
        capabilities: &UNO_R4_MINIMA_CAPABILITIES,
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
        led_matrix: if board_vm_uno_r4::UNO_R4_WIFI.supports_led_matrix {
            Some(UNO_R4_WIFI_LED_MATRIX)
        } else {
            None
        },
        digital_pin_count: UNO_R4_DIGITAL_PINS.len(),
        digital_pins: &UNO_R4_DIGITAL_PINS,
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
        led_matrix: None,
        digital_pin_count: ESP32_DIGITAL_PINS.len(),
        digital_pins: &ESP32_DIGITAL_PINS,
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
        led_matrix: None,
        digital_pin_count: PICO_DIGITAL_PINS.len(),
        digital_pins: &PICO_DIGITAL_PINS,
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
        led_matrix: None,
        digital_pin_count: PICO_DIGITAL_PINS.len(),
        digital_pins: &PICO_DIGITAL_PINS,
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
    fn registry_exposes_board_local_led_matrix_metadata() {
        assert_eq!(
            find_target("arduino-uno-r4-wifi").unwrap().led_matrix,
            Some(LedMatrixInfo {
                rows: 8,
                columns: 12
            })
        );
        assert_eq!(
            find_target("arduino-uno-r4-minima").unwrap().led_matrix,
            None
        );
        assert_eq!(find_target("raspberry-pi-pico-w").unwrap().led_matrix, None);
    }

    #[test]
    fn registry_exposes_digital_pin_capability_metadata() {
        let uno = find_target("arduino-uno-r4-wifi").unwrap();
        assert_eq!(uno.digital_pin_count, uno.digital_pins.len());

        let d3 = uno.digital_pins.iter().find(|pin| pin.pin == 3).unwrap();
        assert_eq!(d3.label, "D3");
        assert!(d3.supports_input);
        assert!(d3.supports_output);
        assert!(d3.supports_pullup);
        assert!(!d3.supports_pulldown);
        assert!(d3.supports_pwm);
        assert!(d3.supports_interrupt);
        assert!(!d3.supports_adc);

        let d13 = uno.digital_pins.iter().find(|pin| pin.pin == 13).unwrap();
        assert!(d13.notes.contains("onboard LED"));
        assert!(!d13.supports_pwm);

        let a0 = uno.digital_pins.iter().find(|pin| pin.pin == 14).unwrap();
        assert_eq!(a0.label, "A0/D14");
        assert!(a0.supports_adc);
        assert!(a0.supports_dac);
        assert!(!a0.supports_pwm);

        let pico = find_target("raspberry-pi-pico").unwrap();
        let adc0 = pico.digital_pins.iter().find(|pin| pin.pin == 26).unwrap();
        assert!(adc0.supports_adc);
        assert!(adc0.supports_pwm);

        let esp32 = find_target("esp32-devkit-v1").unwrap();
        let boot = esp32.digital_pins.iter().find(|pin| pin.pin == 0).unwrap();
        assert!(boot.boot_strap);
        assert!(boot.supports_touch);
    }

    #[test]
    fn registry_exposes_common_runtime_capabilities() {
        let pico = find_target("raspberry-pi-pico").unwrap();
        assert_eq!(pico.runtime_id, "board-vm-pico");
        assert!(pico.capabilities.contains(&"transport.serial"));
        assert!(pico.capabilities.contains(&"gpio.open"));
        assert!(pico.capabilities.contains(&"program.ram_exec"));
        assert!(!pico.capabilities.contains(&"pwm.write"));

        let uno = find_target("arduino-uno-r4-wifi").unwrap();
        assert!(uno.capabilities.contains(&"pwm.write"));
        assert!(uno.capabilities.contains(&"adc.read"));
        assert!(uno.capabilities.contains(&"dac.write_u12"));
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
        assert!(uno_r4_wifi.capabilities.contains(&"led_matrix.frame"));
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
