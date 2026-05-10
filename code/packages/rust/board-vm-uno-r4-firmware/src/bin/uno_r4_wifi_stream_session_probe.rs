#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]

#[cfg(target_arch = "arm")]
mod firmware {
    use board_vm_device::DeviceStreamEndpoint;
    use board_vm_protocol::{
        decode_caps_report_header, decode_frame, decode_hello_ack, decode_program_begin,
        decode_program_chunk, decode_program_end, decode_run_report_header, decode_wire_frame,
        encode_frame, encode_hello, encode_program_begin, encode_program_chunk, encode_program_end,
        encode_run_request, encode_wire_frame, Frame, Hello, MessageType, ProgramBegin,
        ProgramChunk, ProgramEnd, ProgramFormat, RunRequest, RunStatus, FLAG_IS_RESPONSE,
        FLAG_RESPONSE_REQUIRED, RUN_FLAG_BACKGROUND_RUN, RUN_FLAG_RESET_VM_BEFORE_RUN,
    };
    use board_vm_runtime::Level;
    use board_vm_uno_r4::{UnoR4Board, UnoR4Device, UNO_R4_VM_RUNTIME_ID};
    use board_vm_uno_r4_firmware::{
        scripted_probe_stream::ProbeStream, uno_r4_wifi_backend::UnoR4WifiLedBackend,
        EMBEDDED_BLINK_MODULE, SMOKE_INSTRUCTION_BUDGET,
    };
    use panic_halt as _;

    const HOST_NONCE: u32 = 0xB0A2_D003;
    const BOARD_NONCE: u32 = 0xB04D_1003;
    const PROGRAM_ID: u16 = 1;

    fn encode_request(
        message_type: MessageType,
        request_id: u16,
        payload: &[u8],
        out: &mut [u8],
    ) -> Result<usize, ()> {
        let mut raw = [0u8; 192];
        let raw_len = encode_frame(
            &Frame {
                flags: FLAG_RESPONSE_REQUIRED,
                message_type,
                request_id,
                payload,
            },
            &mut raw,
        )
        .map_err(|_| ())?;
        encode_wire_frame(&raw[..raw_len], out).map_err(|_| ())
    }

    fn serve_request(
        device: &mut UnoR4Device<UnoR4WifiLedBackend>,
        request_wire: &[u8],
        raw_response: &mut [u8],
    ) -> Result<usize, ()> {
        let stream = ProbeStream::<256, 256>::with_read(request_wire).map_err(|_| ())?;
        let mut endpoint = DeviceStreamEndpoint::<_, 256, 192, 256>::new(stream);
        endpoint.serve_one(device).map_err(|_| ())?;
        let stream = endpoint.into_inner();
        decode_wire_frame(stream.written(), raw_response).map_err(|_| ())
    }

    fn expect_response<'a>(
        raw_response: &'a [u8],
        expected_type: MessageType,
        request_id: u16,
    ) -> Result<Frame<'a>, ()> {
        let frame = decode_frame(raw_response).map_err(|_| ())?;
        if frame.flags != FLAG_IS_RESPONSE
            || frame.message_type != expected_type
            || frame.request_id != request_id
        {
            return Err(());
        }
        Ok(frame)
    }

    fn serve_and_expect<'a>(
        device: &mut UnoR4Device<UnoR4WifiLedBackend>,
        message_type: MessageType,
        expected_type: MessageType,
        request_id: u16,
        payload: &[u8],
        raw_response: &'a mut [u8],
    ) -> Result<Frame<'a>, ()> {
        let mut request_wire = [0u8; 256];
        let wire_len = encode_request(message_type, request_id, payload, &mut request_wire)?;
        let response_len =
            serve_request(device, &request_wire[..wire_len], raw_response).map_err(|_| ())?;
        expect_response(&raw_response[..response_len], expected_type, request_id)
    }

    fn crc32_ieee(bytes: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for byte in bytes {
            crc ^= *byte as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xEDB8_8320;
                } else {
                    crc >>= 1;
                }
            }
        }
        !crc
    }

    fn run_probe(device: &mut UnoR4Device<UnoR4WifiLedBackend>) -> bool {
        let mut payload = [0u8; 128];
        let mut raw_response = [0u8; 256];

        let Ok(payload_len) = encode_hello(
            &Hello {
                min_version: 1,
                max_version: 1,
                host_name: "uno-r4-session-probe",
                host_nonce: HOST_NONCE,
            },
            &mut payload,
        ) else {
            return false;
        };
        let Ok(frame) = serve_and_expect(
            device,
            MessageType::HELLO,
            MessageType::HELLO_ACK,
            1,
            &payload[..payload_len],
            &mut raw_response,
        ) else {
            return false;
        };
        let Ok(ack) = decode_hello_ack(frame.payload) else {
            return false;
        };
        if ack.host_nonce != HOST_NONCE
            || ack.board_nonce != BOARD_NONCE
            || ack.board_name != "arduino-uno-r4-wifi"
            || ack.runtime_name != UNO_R4_VM_RUNTIME_ID
        {
            return false;
        }

        let Ok(frame) = serve_and_expect(
            device,
            MessageType::CAPS_QUERY,
            MessageType::CAPS_REPORT,
            2,
            &[],
            &mut raw_response,
        ) else {
            return false;
        };
        let Ok((caps, mut caps_decoder)) = decode_caps_report_header(frame.payload) else {
            return false;
        };
        for _ in 0..caps.capability_count {
            if caps_decoder.read_capability_descriptor().is_err() {
                return false;
            }
        }
        if caps_decoder.finish().is_err()
            || caps.board_id != "arduino-uno-r4-wifi"
            || caps.runtime_id != UNO_R4_VM_RUNTIME_ID
        {
            return false;
        }

        let Ok(payload_len) = encode_program_begin(
            &ProgramBegin {
                program_id: PROGRAM_ID,
                format: ProgramFormat::BvmModule,
                total_len: EMBEDDED_BLINK_MODULE.len() as u32,
                program_crc32: crc32_ieee(&EMBEDDED_BLINK_MODULE),
            },
            &mut payload,
        ) else {
            return false;
        };
        let Ok(frame) = serve_and_expect(
            device,
            MessageType::PROGRAM_BEGIN,
            MessageType::PROGRAM_BEGIN,
            3,
            &payload[..payload_len],
            &mut raw_response,
        ) else {
            return false;
        };
        let Ok(begin) = decode_program_begin(frame.payload) else {
            return false;
        };
        if begin.program_id != PROGRAM_ID {
            return false;
        }

        let Ok(payload_len) = encode_program_chunk(
            &ProgramChunk {
                program_id: PROGRAM_ID,
                offset: 0,
                bytes: &EMBEDDED_BLINK_MODULE,
            },
            &mut payload,
        ) else {
            return false;
        };
        let Ok(frame) = serve_and_expect(
            device,
            MessageType::PROGRAM_CHUNK,
            MessageType::PROGRAM_CHUNK,
            4,
            &payload[..payload_len],
            &mut raw_response,
        ) else {
            return false;
        };
        let Ok(chunk) = decode_program_chunk(frame.payload) else {
            return false;
        };
        if chunk.program_id != PROGRAM_ID
            || chunk.offset != 0
            || chunk.bytes.len() != EMBEDDED_BLINK_MODULE.len()
        {
            return false;
        }

        let Ok(payload_len) = encode_program_end(
            &ProgramEnd {
                program_id: PROGRAM_ID,
            },
            &mut payload,
        ) else {
            return false;
        };
        let Ok(frame) = serve_and_expect(
            device,
            MessageType::PROGRAM_END,
            MessageType::PROGRAM_END,
            5,
            &payload[..payload_len],
            &mut raw_response,
        ) else {
            return false;
        };
        let Ok(end) = decode_program_end(frame.payload) else {
            return false;
        };
        if end.program_id != PROGRAM_ID {
            return false;
        }

        let Ok(payload_len) = encode_run_request(
            &RunRequest {
                program_id: PROGRAM_ID,
                flags: RUN_FLAG_RESET_VM_BEFORE_RUN | RUN_FLAG_BACKGROUND_RUN,
                instruction_budget: SMOKE_INSTRUCTION_BUDGET,
                time_budget_ms: 0,
            },
            &mut payload,
        ) else {
            return false;
        };
        let Ok(frame) = serve_and_expect(
            device,
            MessageType::RUN,
            MessageType::RUN_REPORT,
            6,
            &payload[..payload_len],
            &mut raw_response,
        ) else {
            return false;
        };
        let Ok((report, decoder)) = decode_run_report_header(frame.payload) else {
            return false;
        };
        decoder.finish().is_ok()
            && report.program_id == PROGRAM_ID
            && report.status == RunStatus::Running
            && report.instructions_executed > 0
            && report.open_handles == 1
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
                backend.blink_pattern(4, 55, 75);
                backend.pause_ms(800);
            } else {
                backend.set_led(Level::High);
                backend.pause_ms(900);
                backend.set_led(Level::Low);
                backend.pause_ms(900);
            }
        }
    }
}

#[cfg(not(target_arch = "arm"))]
fn main() {}
