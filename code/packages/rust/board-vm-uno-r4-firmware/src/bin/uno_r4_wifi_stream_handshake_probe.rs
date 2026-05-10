#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]

#[cfg(target_arch = "arm")]
mod firmware {
    use board_vm_device::DeviceStreamEndpoint;
    use board_vm_protocol::{
        decode_frame, decode_hello_ack, decode_wire_frame, encode_frame, encode_hello,
        encode_wire_frame, Frame, Hello, MessageType, FLAG_IS_RESPONSE, FLAG_RESPONSE_REQUIRED,
    };
    use board_vm_runtime::Level;
    use board_vm_uno_r4::{UnoR4Board, UnoR4Device};
    use board_vm_uno_r4_firmware::{
        scripted_probe_stream::ProbeStream, uno_r4_wifi_backend::UnoR4WifiLedBackend,
    };
    use panic_halt as _;

    const HOST_NONCE: u32 = 0xB0A2_D002;
    const BOARD_NONCE: u32 = 0xB04D_1002;

    fn build_hello_wire(out: &mut [u8]) -> Result<usize, ()> {
        let mut payload = [0u8; 64];
        let mut raw = [0u8; 128];
        let payload_len = encode_hello(
            &Hello {
                min_version: 1,
                max_version: 1,
                host_name: "uno-r4-probe",
                host_nonce: HOST_NONCE,
            },
            &mut payload,
        )
        .map_err(|_| ())?;
        let raw_len = encode_frame(
            &Frame {
                flags: FLAG_RESPONSE_REQUIRED,
                message_type: MessageType::HELLO,
                request_id: 1,
                payload: &payload[..payload_len],
            },
            &mut raw,
        )
        .map_err(|_| ())?;
        encode_wire_frame(&raw[..raw_len], out).map_err(|_| ())
    }

    fn run_probe(device: &mut UnoR4Device<UnoR4WifiLedBackend>) -> bool {
        let mut hello_wire = [0u8; 192];
        let Ok(hello_wire_len) = build_hello_wire(&mut hello_wire) else {
            return false;
        };
        let Ok(stream) = ProbeStream::<192, 192>::with_read(&hello_wire[..hello_wire_len]) else {
            return false;
        };
        let mut endpoint = DeviceStreamEndpoint::<_, 192, 128, 128>::new(stream);
        if endpoint.serve_one(device).is_err() {
            return false;
        }

        let stream = endpoint.into_inner();
        let mut raw_response = [0u8; 128];
        let Ok(raw_len) = decode_wire_frame(stream.written(), &mut raw_response) else {
            return false;
        };
        let Ok(frame) = decode_frame(&raw_response[..raw_len]) else {
            return false;
        };
        if frame.flags != FLAG_IS_RESPONSE
            || frame.message_type != MessageType::HELLO_ACK
            || frame.request_id != 1
        {
            return false;
        }
        let Ok(ack) = decode_hello_ack(frame.payload) else {
            return false;
        };
        ack.host_nonce == HOST_NONCE
            && ack.board_nonce == BOARD_NONCE
            && ack.board_name == "arduino-uno-r4-wifi"
    }

    #[cortex_m_rt::entry]
    fn main() -> ! {
        let backend = UnoR4WifiLedBackend::new();
        let board = UnoR4Board::wifi(backend);
        let mut device = board.into_device(BOARD_NONCE);
        let ok = run_probe(&mut device);
        let backend = device.hal_mut().backend_mut();

        loop {
            if ok {
                backend.blink_pattern(3, 70, 90);
                backend.pause_ms(700);
            } else {
                backend.set_led(Level::High);
                backend.pause_ms(700);
                backend.set_led(Level::Low);
                backend.pause_ms(700);
            }
        }
    }
}

#[cfg(not(target_arch = "arm"))]
fn main() {}
