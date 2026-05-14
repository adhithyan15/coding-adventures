use std::ffi::{c_char, c_int, c_long, c_void};
use std::ptr;
use std::slice;

use board_vm_host::{
    AdcReadProgram, BlinkProgram, DacWriteU12Program, GpioHandleCloseProgram,
    GpioHandleReadProgram, GpioHandleWriteProgram, GpioOpenProgram, GpioReadProgram,
    GpioWriteProgram, I2cOpenProgram, I2cReadU8Program, I2cWriteU8Program,
    LedMatrixFrameProgram, PwmWriteProgram, TimeNowProgram, TimeSleepMsProgram,
    ADC_READ_MODULE_LEN, BLINK_MODULE_LEN,
    DAC_WRITE_U12_MODULE_LEN, GPIO_HANDLE_CLOSE_MODULE_LEN, GPIO_HANDLE_READ_MODULE_LEN,
    GPIO_HANDLE_WRITE_MODULE_LEN, GPIO_OPEN_MODULE_LEN, GPIO_READ_MODULE_LEN, GPIO_WRITE_MODULE_LEN,
    I2C_OPEN_MODULE_LEN, I2C_READ_U8_MODULE_LEN, I2C_WRITE_U8_MODULE_LEN,
    LED_MATRIX_FRAME_MODULE_LEN, PWM_WRITE_MODULE_LEN, TIME_NOW_MODULE_LEN,
    TIME_SLEEP_MS_MODULE_LEN,
};
use board_vm_language_core::{
    bluetooth_backend_open_plan as core_bluetooth_backend_open_plan,
    bluetooth_endpoint_candidates_from_devices, bluetooth_transact_wire_frame, board_family_name,
    build_blink_module, build_caps_query_wire_frame, build_gpio_handle_close_module,
    build_gpio_handle_read_module, build_gpio_handle_write_module, build_gpio_open_module,
    build_dac_write_u12_module, build_gpio_read_module, build_gpio_write_module,
    build_hello_wire_frame, build_i2c_open_module, build_i2c_read_u8_module,
    build_i2c_write_u8_module,
    build_led_matrix_frame_module, build_program_begin_wire_frame, build_program_chunk_wire_frame,
    build_program_end_wire_frame, build_pwm_write_module, build_adc_read_module, build_raw_module,
    build_run_background_wire_frame, build_run_wire_frame, build_stop_wire_frame,
    build_store_program_wire_frame, build_time_now_module, build_time_sleep_ms_module,
    capability_board_metadata, capability_bytecode_callable, capability_flag_names,
    capability_protocol_feature, connection_transport_name, decode_wire_response,
    detect_target as core_detect_target,
    discover_bluetooth_devices as core_discover_bluetooth_devices,
    discover_devices as core_discover_devices, discover_devices_from_paths,
    discover_pico_bootsel_mounts as core_discover_pico_bootsel_mounts,
    discover_pico_bootsel_mounts_in_roots, esp_upload_options_for_target,
    host_endpoint_transport_name, known_targets, onboard_led_kind,
    parse_bluetooth_endpoint as core_parse_bluetooth_endpoint, pico_uf2_upload_options_for_target,
    program_format_name, raw_module_len, run_status_name, wireless_transport_name,
    BoardVmLanguageSession, DecodedLanguageResponse, DecodedLanguageResponseBody,
    LanguageBluetoothBackendOpenPlan, LanguageBluetoothDiscoveredDevice, LanguageBluetoothEndpoint,
    LanguageBluetoothEndpointCandidate, LanguageConnectionOption, LanguageCoreError,
    LanguageDigitalPin, LanguageEspUploadOptions, LanguageHostDevice, LanguageI2cBus, LanguageOnboardLed,
    LanguagePicoUf2UploadOptions, LanguageTargetInfo, LanguageValue, LanguageWirelessInterface,
};
use ruby_bridge::VALUE;

struct RubyBoardVmSession {
    inner: BoardVmLanguageSession,
}

unsafe extern "C" fn session_alloc(klass: VALUE) -> VALUE {
    ruby_bridge::wrap_data(
        klass,
        RubyBoardVmSession {
            inner: BoardVmLanguageSession::new(),
        },
    )
}

extern "C" fn session_initialize(self_val: VALUE) -> VALUE {
    self_val
}

extern "C" fn session_next_request_id(self_val: VALUE) -> VALUE {
    let session = unsafe { ruby_bridge::unwrap_data::<RubyBoardVmSession>(self_val) };
    ruby_bridge::usize_to_rb(session.inner.next_request_id() as usize)
}

extern "C" fn session_hello_wire(
    self_val: VALUE,
    host_name_val: VALUE,
    host_nonce_val: VALUE,
) -> VALUE {
    let host_name = ruby_bridge::str_from_rb(host_name_val)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("host_name must be a Ruby String"));
    let host_nonce = rb_u32(host_nonce_val, "host_nonce");

    with_session_mut(self_val, |session| {
        let mut wire = vec![0; host_name.len().saturating_add(64).max(128)];
        let written =
            build_hello_wire_frame(&mut session.inner, &host_name, host_nonce, &mut wire)?;
        Ok(bytes_result(&wire, written.len))
    })
}

extern "C" fn session_caps_query_wire(self_val: VALUE) -> VALUE {
    with_session_mut(self_val, |session| {
        let mut wire = [0u8; 64];
        let written = build_caps_query_wire_frame(&mut session.inner, &mut wire)?;
        Ok(bytes_result(&wire, written.len))
    })
}

extern "C" fn session_blink_module(
    _self_val: VALUE,
    pin_val: VALUE,
    high_ms_val: VALUE,
    low_ms_val: VALUE,
    max_stack_val: VALUE,
) -> VALUE {
    let pin = rb_u8(pin_val, "pin");
    let high_ms = rb_u16(high_ms_val, "high_ms");
    let low_ms = rb_u16(low_ms_val, "low_ms");
    let max_stack = rb_u8(max_stack_val, "max_stack");

    let module = build_blink_module_value(pin, high_ms, low_ms, max_stack)
        .unwrap_or_else(|error| raise_core_error("blink_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_gpio_read_module(
    _self_val: VALUE,
    pin_val: VALUE,
    mode_val: VALUE,
    max_stack_val: VALUE,
) -> VALUE {
    let pin = rb_u8(pin_val, "pin");
    let mode = rb_u8(mode_val, "mode");
    let max_stack = rb_u8(max_stack_val, "max_stack");

    let module = build_gpio_read_module_value(pin, mode, max_stack)
        .unwrap_or_else(|error| raise_core_error("gpio_read_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_gpio_write_module(
    _self_val: VALUE,
    pin_val: VALUE,
    value_val: VALUE,
    max_stack_val: VALUE,
) -> VALUE {
    let pin = rb_u8(pin_val, "pin");
    let value = rb_u8(value_val, "value") != 0;
    let max_stack = rb_u8(max_stack_val, "max_stack");

    let module = build_gpio_write_module_value(pin, value, max_stack)
        .unwrap_or_else(|error| raise_core_error("gpio_write_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_gpio_open_module(
    _self_val: VALUE,
    pin_val: VALUE,
    mode_val: VALUE,
    max_stack_val: VALUE,
) -> VALUE {
    let pin = rb_u8(pin_val, "pin");
    let mode = rb_u8(mode_val, "mode");
    let max_stack = rb_u8(max_stack_val, "max_stack");

    let module = build_gpio_open_module_value(pin, mode, max_stack)
        .unwrap_or_else(|error| raise_core_error("gpio_open_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_gpio_handle_read_module(_self_val: VALUE, max_stack_val: VALUE) -> VALUE {
    let max_stack = rb_u8(max_stack_val, "max_stack");

    let module = build_gpio_handle_read_module_value(max_stack)
        .unwrap_or_else(|error| raise_core_error("gpio_handle_read_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_gpio_handle_write_module(
    _self_val: VALUE,
    value_val: VALUE,
    max_stack_val: VALUE,
) -> VALUE {
    let value = rb_u8(value_val, "value") != 0;
    let max_stack = rb_u8(max_stack_val, "max_stack");

    let module = build_gpio_handle_write_module_value(value, max_stack)
        .unwrap_or_else(|error| raise_core_error("gpio_handle_write_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_gpio_handle_close_module(_self_val: VALUE, max_stack_val: VALUE) -> VALUE {
    let max_stack = rb_u8(max_stack_val, "max_stack");

    let module = build_gpio_handle_close_module_value(max_stack)
        .unwrap_or_else(|error| raise_core_error("gpio_handle_close_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_time_now_module(_self_val: VALUE, max_stack_val: VALUE) -> VALUE {
    let max_stack = rb_u8(max_stack_val, "max_stack");

    let module = build_time_now_module_value(max_stack)
        .unwrap_or_else(|error| raise_core_error("time_now_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_time_sleep_ms_module(
    _self_val: VALUE,
    duration_ms_val: VALUE,
    max_stack_val: VALUE,
) -> VALUE {
    let duration_ms = rb_u16(duration_ms_val, "duration_ms");
    let max_stack = rb_u8(max_stack_val, "max_stack");

    let module = build_time_sleep_ms_module_value(duration_ms, max_stack)
        .unwrap_or_else(|error| raise_core_error("time_sleep_ms_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_led_matrix_frame_module(
    _self_val: VALUE,
    word0_val: VALUE,
    word1_val: VALUE,
    word2_val: VALUE,
    max_stack_val: VALUE,
) -> VALUE {
    let word0 = rb_u32(word0_val, "word0");
    let word1 = rb_u32(word1_val, "word1");
    let word2 = rb_u32(word2_val, "word2");
    let max_stack = rb_u8(max_stack_val, "max_stack");

    let module = build_led_matrix_frame_module_value([word0, word1, word2], max_stack)
        .unwrap_or_else(|error| raise_core_error("led_matrix_frame_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_pwm_write_module(
    _self_val: VALUE,
    pin_val: VALUE,
    duty_val: VALUE,
    max_stack_val: VALUE,
) -> VALUE {
    let pin = rb_u8(pin_val, "pin");
    let duty = rb_u16(duty_val, "duty");
    let max_stack = rb_u8(max_stack_val, "max_stack");

    let module = build_pwm_write_module_value(pin, duty, max_stack)
        .unwrap_or_else(|error| raise_core_error("pwm_write_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_adc_read_module(
    _self_val: VALUE,
    pin_val: VALUE,
    max_stack_val: VALUE,
) -> VALUE {
    let pin = rb_u8(pin_val, "pin");
    let max_stack = rb_u8(max_stack_val, "max_stack");

    let module = build_adc_read_module_value(pin, max_stack)
        .unwrap_or_else(|error| raise_core_error("adc_read_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_dac_write_u12_module(
    _self_val: VALUE,
    pin_val: VALUE,
    sample_val: VALUE,
    max_stack_val: VALUE,
) -> VALUE {
    let pin = rb_u8(pin_val, "pin");
    let sample = rb_u16(sample_val, "sample");
    let max_stack = rb_u8(max_stack_val, "max_stack");

    let module = build_dac_write_u12_module_value(pin, sample, max_stack)
        .unwrap_or_else(|error| raise_core_error("dac_write_u12_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_i2c_open_module(
    _self_val: VALUE,
    bus_val: VALUE,
    max_stack_val: VALUE,
) -> VALUE {
    let bus = rb_u8(bus_val, "bus");
    let max_stack = rb_u8(max_stack_val, "max_stack");

    let module = build_i2c_open_module_value(bus, max_stack)
        .unwrap_or_else(|error| raise_core_error("i2c_open_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_i2c_write_u8_module(
    _self_val: VALUE,
    address_val: VALUE,
    byte_val: VALUE,
    max_stack_val: VALUE,
) -> VALUE {
    let address = rb_u16(address_val, "address");
    let byte = rb_u8(byte_val, "byte");
    let max_stack = rb_u8(max_stack_val, "max_stack");

    let module = build_i2c_write_u8_module_value(address, byte, max_stack)
        .unwrap_or_else(|error| raise_core_error("i2c_write_u8_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_i2c_read_u8_module(
    _self_val: VALUE,
    address_val: VALUE,
    max_stack_val: VALUE,
) -> VALUE {
    let address = rb_u16(address_val, "address");
    let max_stack = rb_u8(max_stack_val, "max_stack");

    let module = build_i2c_read_u8_module_value(address, max_stack)
        .unwrap_or_else(|error| raise_core_error("i2c_read_u8_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_raw_module(
    _self_val: VALUE,
    flags_val: VALUE,
    max_stack_val: VALUE,
    code_val: VALUE,
    const_pool_val: VALUE,
) -> VALUE {
    let flags = rb_u8(flags_val, "flags");
    let max_stack = rb_u8(max_stack_val, "max_stack");
    let code = ruby_bridge::bytes_from_rb(code_val)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("code must be a Ruby binary String"));
    let const_pool = ruby_bridge::bytes_from_rb(const_pool_val)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("const_pool must be a Ruby binary String"));

    let module = build_raw_module_value(flags, max_stack, &code, &const_pool)
        .unwrap_or_else(|error| raise_core_error("raw_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_program_begin_wire(
    self_val: VALUE,
    program_id_val: VALUE,
    module_val: VALUE,
) -> VALUE {
    let program_id = rb_u16(program_id_val, "program_id");
    let module = ruby_bridge::bytes_from_rb(module_val)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("module must be a Ruby binary String"));

    with_session_mut(self_val, |session| {
        let mut wire = [0u8; 96];
        let written =
            build_program_begin_wire_frame(&mut session.inner, program_id, &module, &mut wire)?;
        Ok(bytes_result(&wire, written.len))
    })
}

extern "C" fn session_program_chunk_wire(
    self_val: VALUE,
    program_id_val: VALUE,
    offset_val: VALUE,
    chunk_val: VALUE,
) -> VALUE {
    let program_id = rb_u16(program_id_val, "program_id");
    let offset = rb_u32(offset_val, "offset");
    let chunk = ruby_bridge::bytes_from_rb(chunk_val)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("chunk must be a Ruby binary String"));

    with_session_mut(self_val, |session| {
        let mut wire = vec![0; chunk.len().saturating_add(64).max(128)];
        let written = build_program_chunk_wire_frame(
            &mut session.inner,
            program_id,
            offset,
            &chunk,
            &mut wire,
        )?;
        Ok(bytes_result(&wire, written.len))
    })
}

extern "C" fn session_program_end_wire(self_val: VALUE, program_id_val: VALUE) -> VALUE {
    let program_id = rb_u16(program_id_val, "program_id");

    with_session_mut(self_val, |session| {
        let mut wire = [0u8; 64];
        let written = build_program_end_wire_frame(&mut session.inner, program_id, &mut wire)?;
        Ok(bytes_result(&wire, written.len))
    })
}

extern "C" fn session_store_program_wire(
    self_val: VALUE,
    program_id_val: VALUE,
    slot_val: VALUE,
    boot_policy_val: VALUE,
) -> VALUE {
    let program_id = rb_u16(program_id_val, "program_id");
    let slot = rb_u8(slot_val, "slot");
    let boot_policy = rb_u8(boot_policy_val, "boot_policy");

    with_session_mut(self_val, |session| {
        let mut wire = [0u8; 64];
        let written = build_store_program_wire_frame(
            &mut session.inner,
            program_id,
            slot,
            boot_policy,
            &mut wire,
        )?;
        Ok(bytes_result(&wire, written.len))
    })
}

extern "C" fn session_run_background_wire(
    self_val: VALUE,
    program_id_val: VALUE,
    instruction_budget_val: VALUE,
) -> VALUE {
    let program_id = rb_u16(program_id_val, "program_id");
    let instruction_budget = rb_u32(instruction_budget_val, "instruction_budget");

    with_session_mut(self_val, |session| {
        let mut wire = [0u8; 96];
        let written = build_run_background_wire_frame(
            &mut session.inner,
            program_id,
            instruction_budget,
            &mut wire,
        )?;
        Ok(bytes_result(&wire, written.len))
    })
}

extern "C" fn session_run_wire(
    self_val: VALUE,
    program_id_val: VALUE,
    flags_val: VALUE,
    instruction_budget_val: VALUE,
    time_budget_ms_val: VALUE,
) -> VALUE {
    let program_id = rb_u16(program_id_val, "program_id");
    let flags = rb_u8(flags_val, "flags");
    let instruction_budget = rb_u32(instruction_budget_val, "instruction_budget");
    let time_budget_ms = rb_u32(time_budget_ms_val, "time_budget_ms");

    with_session_mut(self_val, |session| {
        let mut wire = [0u8; 96];
        let written = build_run_wire_frame(
            &mut session.inner,
            program_id,
            flags,
            instruction_budget,
            time_budget_ms,
            &mut wire,
        )?;
        Ok(bytes_result(&wire, written.len))
    })
}

extern "C" fn session_stop_wire(self_val: VALUE) -> VALUE {
    with_session_mut(self_val, |session| {
        let mut wire = [0u8; 64];
        let written = build_stop_wire_frame(&mut session.inner, &mut wire)?;
        Ok(bytes_result(&wire, written.len))
    })
}

extern "C" fn session_blink_upload_run_frames(
    argc: c_int,
    argv: *const VALUE,
    self_val: VALUE,
) -> VALUE {
    if argc != 6 {
        ruby_bridge::raise_arg_error(
            "blink_upload_run_frames expects program_id, instruction_budget, pin, high_ms, low_ms, max_stack",
        );
    }
    let args = unsafe { slice::from_raw_parts(argv, argc as usize) };
    let program_id = rb_u16(args[0], "program_id");
    let instruction_budget = rb_u32(args[1], "instruction_budget");
    let pin = rb_u8(args[2], "pin");
    let high_ms = rb_u16(args[3], "high_ms");
    let low_ms = rb_u16(args[4], "low_ms");
    let max_stack = rb_u8(args[5], "max_stack");

    let module = build_blink_module_value(pin, high_ms, low_ms, max_stack)
        .unwrap_or_else(|error| raise_core_error("blink_upload_run_frames", error));

    with_session_mut(self_val, |session| {
        let frames = ruby_bridge::array_new();

        let mut begin_wire = [0u8; 96];
        let begin = build_program_begin_wire_frame(
            &mut session.inner,
            program_id,
            &module,
            &mut begin_wire,
        )?;
        ruby_bridge::array_push(frames, bytes_result(&begin_wire, begin.len));

        let mut chunk_wire = vec![0; module.len().saturating_add(64).max(128)];
        let chunk = build_program_chunk_wire_frame(
            &mut session.inner,
            program_id,
            0,
            &module,
            &mut chunk_wire,
        )?;
        ruby_bridge::array_push(frames, bytes_result(&chunk_wire, chunk.len));

        let mut end_wire = [0u8; 64];
        let end = build_program_end_wire_frame(&mut session.inner, program_id, &mut end_wire)?;
        ruby_bridge::array_push(frames, bytes_result(&end_wire, end.len));

        let mut run_wire = [0u8; 96];
        let run = build_run_background_wire_frame(
            &mut session.inner,
            program_id,
            instruction_budget,
            &mut run_wire,
        )?;
        ruby_bridge::array_push(frames, bytes_result(&run_wire, run.len));

        Ok(frames)
    })
}

extern "C" fn session_decode_response(_self_val: VALUE, wire_val: VALUE) -> VALUE {
    let wire = ruby_bridge::bytes_from_rb(wire_val)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("wire response must be a binary String"));
    let mut raw = vec![0; wire.len().max(64)];
    let decoded = decode_wire_response(&wire, &mut raw)
        .unwrap_or_else(|error| raise_core_error("decode_response", error));
    decoded_response_to_rb(&decoded)
}

extern "C" fn native_known_targets(_self_val: VALUE) -> VALUE {
    language_targets_to_rb(&known_targets())
}

extern "C" fn native_detect_target(_self_val: VALUE, selector_val: VALUE) -> VALUE {
    let selector = ruby_bridge::str_from_rb(selector_val)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("selector must be a Ruby String"));
    match core_detect_target(&selector) {
        Some(target) => language_target_to_rb(&target),
        None => ruby_bridge::nil_value(),
    }
}

extern "C" fn native_bluetooth_endpoint(_self_val: VALUE, endpoint_val: VALUE) -> VALUE {
    let endpoint = ruby_bridge::str_from_rb(endpoint_val)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("endpoint must be a Ruby String"));
    match core_parse_bluetooth_endpoint(&endpoint) {
        Some(endpoint) => bluetooth_endpoint_to_rb(&endpoint),
        None => ruby_bridge::nil_value(),
    }
}

extern "C" fn native_bluetooth_backend(_self_val: VALUE, endpoint_val: VALUE) -> VALUE {
    let endpoint = ruby_bridge::str_from_rb(endpoint_val)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("endpoint must be a Ruby String"));
    match core_bluetooth_backend_open_plan(&endpoint) {
        Some(plan) => bluetooth_backend_open_plan_to_rb(&plan),
        None => ruby_bridge::nil_value(),
    }
}

extern "C" fn native_bluetooth_transact(
    _self_val: VALUE,
    endpoint_val: VALUE,
    wire_val: VALUE,
) -> VALUE {
    let endpoint = ruby_bridge::str_from_rb(endpoint_val)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("endpoint must be a Ruby String"));
    let wire = ruby_bridge::bytes_from_rb(wire_val)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("wire frame must be a binary String"));
    let mut response = vec![0u8; wire.len().saturating_add(4096).max(4096)];
    let len = bluetooth_transact_wire_frame(&endpoint, &wire, &mut response)
        .unwrap_or_else(|error| raise_core_error("bluetooth_transact", error));
    ruby_bridge::bytes_to_rb(&response[..len])
}

extern "C" fn native_bluetooth_endpoint_candidates(_self_val: VALUE, devices_val: VALUE) -> VALUE {
    let devices = bluetooth_discovered_devices_from_rb(devices_val);
    bluetooth_endpoint_candidates_to_rb(&bluetooth_endpoint_candidates_from_devices(&devices))
}

extern "C" fn native_bluetooth_devices(_self_val: VALUE) -> VALUE {
    bluetooth_discovered_devices_to_rb(&core_discover_bluetooth_devices())
}

extern "C" fn native_esp_upload_options(_self_val: VALUE, selector_val: VALUE) -> VALUE {
    let selector = ruby_bridge::str_from_rb(selector_val)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("selector must be a Ruby String"));
    match esp_upload_options_for_target(&selector) {
        Some(options) => esp_upload_options_to_rb(&options),
        None => ruby_bridge::nil_value(),
    }
}

extern "C" fn native_pico_uf2_upload_options(_self_val: VALUE, selector_val: VALUE) -> VALUE {
    let selector = ruby_bridge::str_from_rb(selector_val)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("selector must be a Ruby String"));
    match pico_uf2_upload_options_for_target(&selector) {
        Some(options) => pico_uf2_upload_options_to_rb(&options),
        None => ruby_bridge::nil_value(),
    }
}

extern "C" fn native_discover_devices(_self_val: VALUE) -> VALUE {
    host_devices_to_rb(&core_discover_devices())
}

extern "C" fn native_classify_devices(_self_val: VALUE, paths_val: VALUE) -> VALUE {
    let paths = ruby_bridge::vec_str_from_rb(paths_val);
    host_devices_to_rb(&discover_devices_from_paths(paths))
}

extern "C" fn native_pico_uf2_mounts(_self_val: VALUE, roots_val: VALUE) -> VALUE {
    if roots_val == ruby_bridge::nil_value() {
        strings_to_rb(&core_discover_pico_bootsel_mounts())
    } else {
        let roots = ruby_bridge::vec_str_from_rb(roots_val);
        strings_to_rb(&discover_pico_bootsel_mounts_in_roots(roots))
    }
}

fn with_session_mut(
    self_val: VALUE,
    operation: impl FnOnce(&mut RubyBoardVmSession) -> Result<VALUE, LanguageCoreError>,
) -> VALUE {
    let session = unsafe { ruby_bridge::unwrap_data_mut::<RubyBoardVmSession>(self_val) };
    match operation(session) {
        Ok(value) => value,
        Err(error) => raise_core_error("BoardVM::Native::Session", error),
    }
}

fn bytes_result(buffer: &[u8], len: usize) -> VALUE {
    ruby_bridge::bytes_to_rb(&buffer[..len])
}

fn decoded_response_to_rb(decoded: &DecodedLanguageResponse) -> VALUE {
    let hash = ruby_bridge::hash_new();
    hash_set(hash, "request_id", rb_usize(decoded.request_id));
    hash_set(
        hash,
        "message_type",
        ruby_bridge::str_to_rb(message_type_name(decoded.message_type.0)),
    );
    hash_set(hash, "message_type_code", rb_usize(decoded.message_type.0));
    hash_set(hash, "flags", rb_usize(decoded.flags));
    hash_set(
        hash,
        "response",
        ruby_bridge::bool_to_rb(decoded.is_response()),
    );
    hash_set(
        hash,
        "error",
        ruby_bridge::bool_to_rb(decoded.is_error_response()),
    );
    hash_set(hash, "kind", ruby_bridge::str_to_rb(decoded.body.kind()));
    hash_set(hash, "payload_len", rb_usize(decoded.payload_len));
    hash_set(
        hash,
        "payload",
        response_body_to_rb(&decoded.body, decoded.payload_len),
    );
    hash
}

fn response_body_to_rb(body: &DecodedLanguageResponseBody, payload_len: usize) -> VALUE {
    let hash = ruby_bridge::hash_new();
    match body {
        DecodedLanguageResponseBody::HelloAck(ack) => {
            hash_set(hash, "selected_version", rb_usize(ack.selected_version));
            hash_set(hash, "board_name", ruby_bridge::str_to_rb(&ack.board_name));
            hash_set(
                hash,
                "runtime_name",
                ruby_bridge::str_to_rb(&ack.runtime_name),
            );
            hash_set(hash, "host_nonce", rb_usize(ack.host_nonce));
            hash_set(hash, "board_nonce", rb_usize(ack.board_nonce));
            hash_set(hash, "max_frame_payload", rb_usize(ack.max_frame_payload));
        }
        DecodedLanguageResponseBody::CapsReport(report) => {
            hash_set(hash, "board_id", ruby_bridge::str_to_rb(&report.board_id));
            hash_set(
                hash,
                "runtime_id",
                ruby_bridge::str_to_rb(&report.runtime_id),
            );
            hash_set(
                hash,
                "max_program_bytes",
                rb_usize(report.max_program_bytes),
            );
            hash_set(hash, "max_stack_values", rb_usize(report.max_stack_values));
            hash_set(hash, "max_handles", rb_usize(report.max_handles));
            hash_set(
                hash,
                "supports_store_program",
                ruby_bridge::bool_to_rb(report.supports_store_program),
            );
            let capabilities = ruby_bridge::array_new();
            for capability in &report.capabilities {
                let item = ruby_bridge::hash_new();
                hash_set(item, "id", rb_usize(capability.id));
                hash_set(item, "version", rb_usize(capability.version));
                hash_set(item, "flags", rb_usize(capability.flags));
                hash_set(item, "name", ruby_bridge::str_to_rb(&capability.name));
                hash_set(
                    item,
                    "bytecode_callable",
                    ruby_bridge::bool_to_rb(capability_bytecode_callable(capability.flags)),
                );
                hash_set(
                    item,
                    "protocol_feature",
                    ruby_bridge::bool_to_rb(capability_protocol_feature(capability.flags)),
                );
                hash_set(
                    item,
                    "board_metadata",
                    ruby_bridge::bool_to_rb(capability_board_metadata(capability.flags)),
                );
                hash_set(
                    item,
                    "flag_names",
                    capability_flag_names_to_rb(capability.flags),
                );
                ruby_bridge::array_push(capabilities, item);
            }
            hash_set(hash, "capabilities", capabilities);
        }
        DecodedLanguageResponseBody::ProgramBegin(begin) => {
            hash_set(hash, "program_id", rb_usize(begin.program_id));
            hash_set(
                hash,
                "format",
                ruby_bridge::str_to_rb(program_format_name(begin.format)),
            );
            hash_set(hash, "total_len", rb_usize(begin.total_len));
            hash_set(hash, "program_crc32", rb_usize(begin.program_crc32));
        }
        DecodedLanguageResponseBody::ProgramChunk(chunk) => {
            hash_set(hash, "program_id", rb_usize(chunk.program_id));
            hash_set(hash, "offset", rb_usize(chunk.offset));
            hash_set(hash, "len", rb_usize(chunk.len));
        }
        DecodedLanguageResponseBody::ProgramEnd(end) => {
            hash_set(hash, "program_id", rb_usize(end.program_id));
        }
        DecodedLanguageResponseBody::RunReport(report) => {
            hash_set(hash, "program_id", rb_usize(report.program_id));
            hash_set(
                hash,
                "status",
                ruby_bridge::str_to_rb(run_status_name(report.status)),
            );
            hash_set(hash, "status_code", rb_usize(report.status.as_u8()));
            hash_set(
                hash,
                "instructions_executed",
                rb_usize(report.instructions_executed),
            );
            hash_set(hash, "elapsed_ms", rb_usize(report.elapsed_ms));
            hash_set(hash, "stack_depth", rb_usize(report.stack_depth));
            hash_set(hash, "open_handles", rb_usize(report.open_handles));
            hash_set(hash, "return_count", rb_usize(report.return_count));
            hash_set(hash, "returns", language_values_to_rb(&report.returns));
        }
        DecodedLanguageResponseBody::Error(error) => {
            hash_set(hash, "code", rb_usize(error.code));
            hash_set(hash, "request_id", rb_usize(error.request_id));
            hash_set(hash, "program_id", rb_usize(error.program_id));
            hash_set(hash, "bytecode_offset", rb_usize(error.bytecode_offset));
            hash_set(hash, "message", ruby_bridge::str_to_rb(&error.message));
        }
        DecodedLanguageResponseBody::Raw => {
            hash_set(hash, "payload_len", rb_usize(payload_len));
        }
    }
    hash
}

fn capability_flag_names_to_rb(flags: u16) -> VALUE {
    let mut names = [""; 3];
    let count = capability_flag_names(flags, &mut names);
    let array = ruby_bridge::array_new();
    for name in &names[..count] {
        ruby_bridge::array_push(array, ruby_bridge::str_to_rb(name));
    }
    array
}

fn language_targets_to_rb(targets: &[LanguageTargetInfo]) -> VALUE {
    let array = ruby_bridge::array_new();
    for target in targets {
        ruby_bridge::array_push(array, language_target_to_rb(target));
    }
    array
}

fn language_target_to_rb(target: &LanguageTargetInfo) -> VALUE {
    let hash = ruby_bridge::hash_new();
    hash_set(hash, "board_id", ruby_bridge::str_to_rb(&target.board_id));
    hash_set(
        hash,
        "display_name",
        ruby_bridge::str_to_rb(&target.display_name),
    );
    hash_set(
        hash,
        "family",
        ruby_bridge::str_to_rb(board_family_name(target.family)),
    );
    hash_set(
        hash,
        "runtime_id",
        ruby_bridge::str_to_rb(&target.runtime_id),
    );
    hash_set(hash, "mcu", ruby_bridge::str_to_rb(&target.mcu));
    hash_set(hash, "core", ruby_bridge::str_to_rb(&target.core));
    hash_set(
        hash,
        "rust_target",
        ruby_bridge::str_to_rb(&target.rust_target),
    );
    hash_set(hash, "clock_hz", rb_usize(target.clock_hz));
    hash_set(
        hash,
        "operating_voltage_mv",
        rb_usize(target.operating_voltage_mv),
    );
    hash_set(
        hash,
        "onboard_led",
        language_onboard_led_to_rb(target.onboard_led),
    );
    hash_set(
        hash,
        "led_matrix",
        language_led_matrix_to_rb(target.led_matrix),
    );
    hash_set(
        hash,
        "digital_pin_count",
        rb_usize(target.digital_pin_count),
    );
    hash_set(
        hash,
        "digital_pins",
        language_digital_pins_to_rb(&target.digital_pins),
    );
    hash_set(hash, "i2c_buses", language_i2c_buses_to_rb(&target.i2c_buses));
    hash_set(hash, "wireless", language_wireless_to_rb(&target.wireless));
    hash_set(
        hash,
        "connection_options",
        language_connection_options_to_rb(&target.connection_options),
    );
    let capabilities = ruby_bridge::array_new();
    for capability in &target.capabilities {
        ruby_bridge::array_push(capabilities, ruby_bridge::str_to_rb(capability));
    }
    hash_set(hash, "capabilities", capabilities);
    hash
}

fn language_digital_pins_to_rb(pins: &[LanguageDigitalPin]) -> VALUE {
    let array = ruby_bridge::array_new();
    for pin in pins {
        let hash = ruby_bridge::hash_new();
        hash_set(hash, "pin", rb_usize(pin.pin));
        hash_set(hash, "label", ruby_bridge::str_to_rb(&pin.label));
        hash_set(
            hash,
            "supports_input",
            ruby_bridge::bool_to_rb(pin.supports_input),
        );
        hash_set(
            hash,
            "supports_output",
            ruby_bridge::bool_to_rb(pin.supports_output),
        );
        hash_set(
            hash,
            "supports_pullup",
            ruby_bridge::bool_to_rb(pin.supports_pullup),
        );
        hash_set(
            hash,
            "supports_pulldown",
            ruby_bridge::bool_to_rb(pin.supports_pulldown),
        );
        hash_set(
            hash,
            "supports_adc",
            ruby_bridge::bool_to_rb(pin.supports_adc),
        );
        hash_set(
            hash,
            "supports_pwm",
            ruby_bridge::bool_to_rb(pin.supports_pwm),
        );
        hash_set(
            hash,
            "supports_dac",
            ruby_bridge::bool_to_rb(pin.supports_dac),
        );
        hash_set(
            hash,
            "supports_touch",
            ruby_bridge::bool_to_rb(pin.supports_touch),
        );
        hash_set(
            hash,
            "supports_interrupt",
            ruby_bridge::bool_to_rb(pin.supports_interrupt),
        );
        hash_set(hash, "boot_strap", ruby_bridge::bool_to_rb(pin.boot_strap));
        hash_set(hash, "notes", ruby_bridge::str_to_rb(&pin.notes));
        ruby_bridge::array_push(array, hash);
    }
    array
}

fn language_led_matrix_to_rb(matrix: Option<board_vm_language_core::LanguageLedMatrix>) -> VALUE {
    let Some(matrix) = matrix else {
        return ruby_bridge::nil_value();
    };

    let hash = ruby_bridge::hash_new();
    hash_set(hash, "rows", rb_usize(matrix.rows as usize));
    hash_set(hash, "columns", rb_usize(matrix.columns as usize));
    hash
}

fn language_i2c_buses_to_rb(buses: &[LanguageI2cBus]) -> VALUE {
    let array = ruby_bridge::array_new();
    for bus in buses {
        let hash = ruby_bridge::hash_new();
        hash_set(hash, "bus", rb_usize(bus.bus));
        hash_set(hash, "name", ruby_bridge::str_to_rb(&bus.name));
        hash_set(hash, "sda_pin", rb_usize(bus.sda_pin));
        hash_set(hash, "scl_pin", rb_usize(bus.scl_pin));
        hash_set(hash, "qwiic", ruby_bridge::bool_to_rb(bus.qwiic));
        hash_set(hash, "notes", ruby_bridge::str_to_rb(&bus.notes));
        ruby_bridge::array_push(array, hash);
    }
    array
}

fn language_wireless_to_rb(interfaces: &[LanguageWirelessInterface]) -> VALUE {
    let array = ruby_bridge::array_new();
    for interface in interfaces {
        let hash = ruby_bridge::hash_new();
        hash_set(
            hash,
            "transport",
            ruby_bridge::str_to_rb(wireless_transport_name(interface.transport)),
        );
        hash_set(hash, "chip", ruby_bridge::str_to_rb(&interface.chip));
        hash_set(
            hash,
            "command_transport",
            ruby_bridge::bool_to_rb(interface.command_transport),
        );
        hash_set(
            hash,
            "ota_update",
            ruby_bridge::bool_to_rb(interface.ota_update),
        );
        ruby_bridge::array_push(array, hash);
    }
    array
}

fn language_connection_options_to_rb(options: &[LanguageConnectionOption]) -> VALUE {
    let array = ruby_bridge::array_new();
    for option in options {
        let hash = ruby_bridge::hash_new();
        hash_set(
            hash,
            "transport",
            ruby_bridge::str_to_rb(connection_transport_name(option.transport)),
        );
        hash_set(
            hash,
            "display_name",
            ruby_bridge::str_to_rb(&option.display_name),
        );
        hash_set(
            hash,
            "command_transport",
            ruby_bridge::bool_to_rb(option.command_transport),
        );
        hash_set(
            hash,
            "ota_update",
            ruby_bridge::bool_to_rb(option.ota_update),
        );
        hash_set(hash, "requires", ruby_bridge::str_to_rb(&option.requires));
        hash_set(
            hash,
            "endpoint_transport",
            ruby_bridge::str_to_rb(host_endpoint_transport_name(option.endpoint_transport)),
        );
        hash_set(
            hash,
            "endpoint_scheme",
            ruby_bridge::str_to_rb(&option.endpoint_scheme),
        );
        hash_set(
            hash,
            "wire_protocol",
            ruby_bridge::str_to_rb(&option.wire_protocol),
        );
        ruby_bridge::array_push(array, hash);
    }
    array
}

fn bluetooth_endpoint_to_rb(endpoint: &LanguageBluetoothEndpoint) -> VALUE {
    let hash = ruby_bridge::hash_new();
    hash_set(hash, "endpoint", ruby_bridge::str_to_rb(&endpoint.endpoint));
    hash_set(
        hash,
        "transport",
        ruby_bridge::str_to_rb(connection_transport_name(endpoint.transport)),
    );
    hash_set(
        hash,
        "endpoint_transport",
        ruby_bridge::str_to_rb(host_endpoint_transport_name(endpoint.endpoint_transport)),
    );
    hash_set(
        hash,
        "endpoint_scheme",
        ruby_bridge::str_to_rb(&endpoint.endpoint_scheme),
    );
    hash_set(hash, "device", ruby_bridge::str_to_rb(&endpoint.device));
    hash_set(
        hash,
        "service_uuid",
        endpoint
            .service_uuid
            .as_ref()
            .map(|value| ruby_bridge::str_to_rb(value))
            .unwrap_or_else(ruby_bridge::nil_value),
    );
    hash_set(
        hash,
        "write_characteristic_uuid",
        endpoint
            .write_characteristic_uuid
            .as_ref()
            .map(|value| ruby_bridge::str_to_rb(value))
            .unwrap_or_else(ruby_bridge::nil_value),
    );
    hash_set(
        hash,
        "notify_characteristic_uuid",
        endpoint
            .notify_characteristic_uuid
            .as_ref()
            .map(|value| ruby_bridge::str_to_rb(value))
            .unwrap_or_else(ruby_bridge::nil_value),
    );
    hash_set(
        hash,
        "channel",
        endpoint
            .channel
            .map(rb_usize)
            .unwrap_or_else(ruby_bridge::nil_value),
    );
    hash
}

fn bluetooth_backend_open_plan_to_rb(plan: &LanguageBluetoothBackendOpenPlan) -> VALUE {
    let hash = ruby_bridge::hash_new();
    hash_set(hash, "endpoint", bluetooth_endpoint_to_rb(&plan.endpoint));
    hash_set(hash, "backend", ruby_bridge::str_to_rb(&plan.backend));
    hash_set(hash, "status", ruby_bridge::str_to_rb(&plan.status));
    hash_set(
        hash,
        "stream_path",
        plan.stream_path
            .as_ref()
            .map(|value| ruby_bridge::str_to_rb(value))
            .unwrap_or_else(ruby_bridge::nil_value),
    );
    hash_set(
        hash,
        "native_transport",
        ruby_bridge::bool_to_rb(plan.native_transport),
    );
    hash_set(
        hash,
        "message",
        plan.message
            .as_ref()
            .map(|value| ruby_bridge::str_to_rb(value))
            .unwrap_or_else(ruby_bridge::nil_value),
    );
    hash
}

fn bluetooth_endpoint_candidates_to_rb(candidates: &[LanguageBluetoothEndpointCandidate]) -> VALUE {
    let array = ruby_bridge::array_new();
    for candidate in candidates {
        let hash = ruby_bridge::hash_new();
        hash_set(
            hash,
            "endpoint",
            bluetooth_endpoint_to_rb(&candidate.endpoint),
        );
        hash_set(hash, "device", ruby_bridge::str_to_rb(&candidate.device));
        hash_set(
            hash,
            "display_name",
            ruby_bridge::str_to_rb(&candidate.display_name),
        );
        hash_set(hash, "paired", ruby_bridge::bool_to_rb(candidate.paired));
        hash_set(
            hash,
            "requires_pairing",
            ruby_bridge::bool_to_rb(candidate.requires_pairing),
        );
        ruby_bridge::array_push(array, hash);
    }
    array
}

fn bluetooth_discovered_devices_to_rb(devices: &[LanguageBluetoothDiscoveredDevice]) -> VALUE {
    let array = ruby_bridge::array_new();
    for device in devices {
        let hash = ruby_bridge::hash_new();
        hash_set(hash, "id", ruby_bridge::str_to_rb(&device.id));
        hash_set(
            hash,
            "name",
            device
                .name
                .as_ref()
                .map(|value| ruby_bridge::str_to_rb(value))
                .unwrap_or_else(ruby_bridge::nil_value),
        );
        hash_set(
            hash,
            "address",
            device
                .address
                .as_ref()
                .map(|value| ruby_bridge::str_to_rb(value))
                .unwrap_or_else(ruby_bridge::nil_value),
        );
        hash_set(hash, "paired", ruby_bridge::bool_to_rb(device.paired));
        hash_set(hash, "service_uuids", strings_to_rb(&device.service_uuids));
        hash_set(
            hash,
            "characteristic_uuids",
            strings_to_rb(&device.characteristic_uuids),
        );
        let channels = ruby_bridge::array_new();
        for channel in &device.board_vm_rfcomm_channels {
            ruby_bridge::array_push(channels, rb_usize(*channel));
        }
        hash_set(hash, "board_vm_rfcomm_channels", channels);
        ruby_bridge::array_push(array, hash);
    }
    array
}

fn bluetooth_discovered_devices_from_rb(
    devices_val: VALUE,
) -> Vec<LanguageBluetoothDiscoveredDevice> {
    let len = ruby_bridge::array_len(devices_val);
    let mut devices = Vec::with_capacity(len);
    for index in 0..len {
        let device = ruby_bridge::array_entry(devices_val, index);
        let id = rb_hash_optional_str(device, "id").unwrap_or_else(|| {
            ruby_bridge::raise_arg_error("Bluetooth discovered device id must be a String")
        });
        devices.push(LanguageBluetoothDiscoveredDevice {
            id,
            name: rb_hash_optional_str(device, "name"),
            address: rb_hash_optional_str(device, "address"),
            paired: rb_hash_bool(device, "paired"),
            service_uuids: rb_hash_vec_str(device, "service_uuids"),
            characteristic_uuids: rb_hash_vec_str(device, "characteristic_uuids"),
            board_vm_rfcomm_channels: rb_hash_vec_u8(device, "board_vm_rfcomm_channels"),
        });
    }
    devices
}

fn rb_hash_value(hash: VALUE, key: &str) -> VALUE {
    let key = ruby_bridge::str_to_rb(key);
    unsafe {
        let mid = ruby_bridge::rb_intern(b"[]\0".as_ptr() as *const c_char);
        ruby_bridge::rb_funcallv(hash, mid, 1, &key)
    }
}

fn rb_hash_optional_str(hash: VALUE, key: &str) -> Option<String> {
    let value = rb_hash_value(hash, key);
    (value != ruby_bridge::nil_value())
        .then(|| ruby_bridge::str_from_rb(value))
        .flatten()
}

fn rb_hash_bool(hash: VALUE, key: &str) -> bool {
    let value = rb_hash_value(hash, key);
    value != ruby_bridge::nil_value() && value != ruby_bridge::QFALSE
}

fn rb_hash_vec_str(hash: VALUE, key: &str) -> Vec<String> {
    let value = rb_hash_value(hash, key);
    if value == ruby_bridge::nil_value() {
        Vec::new()
    } else {
        ruby_bridge::vec_str_from_rb(value)
    }
}

fn rb_hash_vec_u8(hash: VALUE, key: &str) -> Vec<u8> {
    let value = rb_hash_value(hash, key);
    if value == ruby_bridge::nil_value() {
        return Vec::new();
    }
    let len = ruby_bridge::array_len(value);
    let mut values = Vec::with_capacity(len);
    for index in 0..len {
        values.push(rb_u8(ruby_bridge::array_entry(value, index), key));
    }
    values
}

fn esp_upload_options_to_rb(options: &LanguageEspUploadOptions) -> VALUE {
    let hash = ruby_bridge::hash_new();
    hash_set(hash, "board_id", ruby_bridge::str_to_rb(&options.board_id));
    hash_set(hash, "baud_rate", rb_usize(options.baud_rate));
    hash_set(hash, "timeout_ms", rb_usize(options.timeout_ms));
    hash_set(
        hash,
        "reset_into_bootloader",
        ruby_bridge::bool_to_rb(options.reset_into_bootloader),
    );
    hash_set(hash, "offset", rb_usize(options.offset));
    hash_set(hash, "block_size", rb_usize(options.block_size));
    hash_set(
        hash,
        "flash_size",
        options
            .flash_size
            .map(rb_usize)
            .unwrap_or_else(ruby_bridge::nil_value),
    );
    hash_set(
        hash,
        "verify_md5",
        ruby_bridge::bool_to_rb(options.verify_md5),
    );
    hash_set(
        hash,
        "stay_in_bootloader",
        ruby_bridge::bool_to_rb(options.stay_in_bootloader),
    );
    hash
}

fn pico_uf2_upload_options_to_rb(options: &LanguagePicoUf2UploadOptions) -> VALUE {
    let hash = ruby_bridge::hash_new();
    hash_set(hash, "board_id", ruby_bridge::str_to_rb(&options.board_id));
    hash_set(hash, "command", ruby_bridge::str_to_rb(&options.command));
    hash_set(
        hash,
        "volume_label",
        ruby_bridge::str_to_rb(&options.volume_label),
    );
    hash_set(
        hash,
        "image_extension",
        ruby_bridge::str_to_rb(&options.image_extension),
    );
    hash_set(
        hash,
        "auto_detect_mount",
        ruby_bridge::bool_to_rb(options.auto_detect_mount),
    );
    hash
}

fn host_devices_to_rb(devices: &[LanguageHostDevice]) -> VALUE {
    let array = ruby_bridge::array_new();
    for device in devices {
        ruby_bridge::array_push(array, host_device_to_rb(device));
    }
    array
}

fn strings_to_rb(strings: &[String]) -> VALUE {
    let array = ruby_bridge::array_new();
    for string in strings {
        ruby_bridge::array_push(array, ruby_bridge::str_to_rb(string));
    }
    array
}

fn host_device_to_rb(device: &LanguageHostDevice) -> VALUE {
    let hash = ruby_bridge::hash_new();
    hash_set(hash, "id", ruby_bridge::str_to_rb(&device.id));
    hash_set(hash, "port", ruby_bridge::str_to_rb(&device.port));
    hash_set(hash, "transport", ruby_bridge::str_to_rb(&device.transport));
    hash_set(
        hash,
        "display_name",
        ruby_bridge::str_to_rb(&device.display_name),
    );
    hash_set(
        hash,
        "target",
        device
            .target
            .as_ref()
            .map(language_target_to_rb)
            .unwrap_or_else(ruby_bridge::nil_value),
    );
    hash_set(
        hash,
        "target_confidence",
        rb_usize(device.target_confidence),
    );
    hash_set(
        hash,
        "bootloader",
        ruby_bridge::bool_to_rb(device.bootloader),
    );
    let tags = ruby_bridge::array_new();
    for tag in &device.tags {
        ruby_bridge::array_push(tags, ruby_bridge::str_to_rb(tag));
    }
    hash_set(hash, "tags", tags);
    hash
}

fn language_onboard_led_to_rb(led: Option<LanguageOnboardLed>) -> VALUE {
    let Some(led) = led else {
        return ruby_bridge::nil_value();
    };
    let hash = ruby_bridge::hash_new();
    hash_set(hash, "kind", ruby_bridge::str_to_rb(onboard_led_kind(led)));
    let pin = match led {
        LanguageOnboardLed::Gpio(pin) | LanguageOnboardLed::WirelessChipGpio(pin) => pin,
    };
    hash_set(hash, "pin", rb_usize(pin));
    hash
}

fn language_values_to_rb(values: &[LanguageValue]) -> VALUE {
    let array = ruby_bridge::array_new();
    for value in values {
        ruby_bridge::array_push(array, language_value_to_rb(value));
    }
    array
}

fn language_value_to_rb(value: &LanguageValue) -> VALUE {
    let hash = ruby_bridge::hash_new();
    hash_set(hash, "kind", ruby_bridge::str_to_rb(value.kind()));
    let value_rb = match value {
        LanguageValue::Unit => ruby_bridge::nil_value(),
        LanguageValue::Bool(value) => ruby_bridge::bool_to_rb(*value),
        LanguageValue::U8(value) => rb_usize(*value),
        LanguageValue::U16(value) => rb_usize(*value),
        LanguageValue::U32(value) => rb_usize(*value),
        LanguageValue::I16(value) => rb_i64(*value as i64),
        LanguageValue::Handle(value) => rb_usize(*value),
        LanguageValue::Bytes(value) => ruby_bridge::bytes_to_rb(value),
        LanguageValue::String(value) => ruby_bridge::str_to_rb(value),
    };
    hash_set(hash, "value", value_rb);
    hash
}

fn message_type_name(code: u8) -> &'static str {
    match code {
        0x01 => "hello",
        0x02 => "hello_ack",
        0x03 => "caps_query",
        0x04 => "caps_report",
        0x05 => "program_begin",
        0x06 => "program_chunk",
        0x07 => "program_end",
        0x08 => "run",
        0x09 => "run_report",
        0x0A => "stop",
        0x0B => "reset_vm",
        0x0C => "store_program",
        0x0D => "run_stored",
        0x0E => "read_state",
        0x0F => "state_report",
        0x10 => "subscribe",
        0x11 => "event",
        0x12 => "log",
        0x13 => "error",
        0x14 => "ping",
        0x15 => "pong",
        _ => "unknown",
    }
}

fn hash_set(hash: VALUE, key: &str, value: VALUE) {
    ruby_bridge::hash_aset(hash, ruby_bridge::str_to_rb(key), value);
}

fn rb_usize(value: impl TryInto<usize>) -> VALUE {
    ruby_bridge::usize_to_rb(value.try_into().unwrap_or(usize::MAX))
}

fn rb_i64(value: i64) -> VALUE {
    unsafe { ruby_bridge::rb_int2inum(value as c_long) }
}

fn build_blink_module_value(
    pin: u8,
    high_ms: u16,
    low_ms: u16,
    max_stack: u8,
) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; BLINK_MODULE_LEN];
    let len = build_blink_module(
        BlinkProgram {
            pin,
            high_ms,
            low_ms,
            max_stack,
        },
        &mut module,
    )?;
    module.truncate(len);
    Ok(module)
}

fn build_time_now_module_value(max_stack: u8) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; TIME_NOW_MODULE_LEN];
    let len = build_time_now_module(TimeNowProgram { max_stack }, &mut module)?;
    module.truncate(len);
    Ok(module)
}

fn build_time_sleep_ms_module_value(
    duration_ms: u16,
    max_stack: u8,
) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; TIME_SLEEP_MS_MODULE_LEN];
    let len = build_time_sleep_ms_module(
        TimeSleepMsProgram {
            duration_ms,
            max_stack,
        },
        &mut module,
    )?;
    module.truncate(len);
    Ok(module)
}

fn build_led_matrix_frame_module_value(
    words: [u32; 3],
    max_stack: u8,
) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; LED_MATRIX_FRAME_MODULE_LEN];
    let len =
        build_led_matrix_frame_module(LedMatrixFrameProgram { words, max_stack }, &mut module)?;
    module.truncate(len);
    Ok(module)
}

fn build_pwm_write_module_value(
    pin: u8,
    duty: u16,
    max_stack: u8,
) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; PWM_WRITE_MODULE_LEN];
    let len = build_pwm_write_module(
        PwmWriteProgram {
            pin,
            duty,
            max_stack,
        },
        &mut module,
    )?;
    module.truncate(len);
    Ok(module)
}

fn build_adc_read_module_value(pin: u8, max_stack: u8) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; ADC_READ_MODULE_LEN];
    let len = build_adc_read_module(AdcReadProgram { pin, max_stack }, &mut module)?;
    module.truncate(len);
    Ok(module)
}

fn build_dac_write_u12_module_value(
    pin: u8,
    sample: u16,
    max_stack: u8,
) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; DAC_WRITE_U12_MODULE_LEN];
    let len = build_dac_write_u12_module(
        DacWriteU12Program {
            pin,
            sample,
            max_stack,
        },
        &mut module,
    )?;
    module.truncate(len);
    Ok(module)
}

fn build_i2c_open_module_value(bus: u8, max_stack: u8) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; I2C_OPEN_MODULE_LEN];
    let len = build_i2c_open_module(I2cOpenProgram { bus, max_stack }, &mut module)?;
    module.truncate(len);
    Ok(module)
}

fn build_i2c_write_u8_module_value(
    address: u16,
    byte: u8,
    max_stack: u8,
) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; I2C_WRITE_U8_MODULE_LEN];
    let len = build_i2c_write_u8_module(
        I2cWriteU8Program {
            address,
            byte,
            max_stack,
        },
        &mut module,
    )?;
    module.truncate(len);
    Ok(module)
}

fn build_i2c_read_u8_module_value(
    address: u16,
    max_stack: u8,
) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; I2C_READ_U8_MODULE_LEN];
    let len = build_i2c_read_u8_module(I2cReadU8Program { address, max_stack }, &mut module)?;
    module.truncate(len);
    Ok(module)
}

fn build_raw_module_value(
    flags: u8,
    max_stack: u8,
    code: &[u8],
    const_pool: &[u8],
) -> Result<Vec<u8>, LanguageCoreError> {
    let module_len = raw_module_len(code.len() as u64, const_pool.len() as u64)?;
    let mut module = vec![0; module_len];
    let len = build_raw_module(flags, max_stack, code, const_pool, &mut module)?;
    module.truncate(len);
    Ok(module)
}

fn build_gpio_read_module_value(
    pin: u8,
    mode: u8,
    max_stack: u8,
) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; GPIO_READ_MODULE_LEN];
    let len = build_gpio_read_module(
        GpioReadProgram {
            pin,
            mode,
            max_stack,
        },
        &mut module,
    )?;
    module.truncate(len);
    Ok(module)
}

fn build_gpio_write_module_value(
    pin: u8,
    value: bool,
    max_stack: u8,
) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; GPIO_WRITE_MODULE_LEN];
    let len = build_gpio_write_module(
        GpioWriteProgram {
            pin,
            value,
            max_stack,
        },
        &mut module,
    )?;
    module.truncate(len);
    Ok(module)
}

fn build_gpio_open_module_value(
    pin: u8,
    mode: u8,
    max_stack: u8,
) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; GPIO_OPEN_MODULE_LEN];
    let len = build_gpio_open_module(
        GpioOpenProgram {
            pin,
            mode,
            max_stack,
        },
        &mut module,
    )?;
    module.truncate(len);
    Ok(module)
}

fn build_gpio_handle_read_module_value(max_stack: u8) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; GPIO_HANDLE_READ_MODULE_LEN];
    let len = build_gpio_handle_read_module(GpioHandleReadProgram { max_stack }, &mut module)?;
    module.truncate(len);
    Ok(module)
}

fn build_gpio_handle_write_module_value(
    value: bool,
    max_stack: u8,
) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; GPIO_HANDLE_WRITE_MODULE_LEN];
    let len =
        build_gpio_handle_write_module(GpioHandleWriteProgram { value, max_stack }, &mut module)?;
    module.truncate(len);
    Ok(module)
}

fn build_gpio_handle_close_module_value(max_stack: u8) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; GPIO_HANDLE_CLOSE_MODULE_LEN];
    let len = build_gpio_handle_close_module(GpioHandleCloseProgram { max_stack }, &mut module)?;
    module.truncate(len);
    Ok(module)
}

fn rb_u8(value: VALUE, name: &str) -> u8 {
    let value = rb_nonnegative_integer(value, name);
    if value > u8::MAX as u64 {
        ruby_bridge::raise_arg_error(&format!("{name} must fit in u8"));
    }
    value as u8
}

fn rb_u16(value: VALUE, name: &str) -> u16 {
    let value = rb_nonnegative_integer(value, name);
    if value > u16::MAX as u64 {
        ruby_bridge::raise_arg_error(&format!("{name} must fit in u16"));
    }
    value as u16
}

fn rb_u32(value: VALUE, name: &str) -> u32 {
    let value = rb_nonnegative_integer(value, name);
    if value > u32::MAX as u64 {
        ruby_bridge::raise_arg_error(&format!("{name} must fit in u32"));
    }
    value as u32
}

fn rb_nonnegative_integer(value: VALUE, name: &str) -> u64 {
    let to_s = unsafe { ruby_bridge::rb_intern(b"to_s\0".as_ptr() as *const c_char) };
    let string_value = unsafe { ruby_bridge::rb_funcallv(value, to_s, 0, ptr::null()) };
    let text = ruby_bridge::str_from_rb(string_value)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error(&format!("{name} must be an integer")));
    text.parse::<u64>()
        .unwrap_or_else(|_| ruby_bridge::raise_arg_error(&format!("{name} must be non-negative")))
}

fn raise_core_error(context: &str, error: LanguageCoreError) -> ! {
    ruby_bridge::raise_runtime_error(&format!(
        "{context} failed in Rust language core: {error:?}"
    ))
}

#[no_mangle]
pub extern "C" fn Init_board_vm_native() {
    let coding_adventures = ruby_bridge::define_module("CodingAdventures");
    let board_vm = ruby_bridge::define_module_under(coding_adventures, "BoardVM");
    let native = ruby_bridge::define_module_under(board_vm, "Native");
    let session_class =
        ruby_bridge::define_class_under(native, "Session", ruby_bridge::object_class());

    ruby_bridge::define_module_function_raw(
        native,
        "known_targets",
        native_known_targets as *const c_void,
        0,
    );
    ruby_bridge::define_module_function_raw(
        native,
        "detect_target",
        native_detect_target as *const c_void,
        1,
    );
    ruby_bridge::define_module_function_raw(
        native,
        "bluetooth_endpoint",
        native_bluetooth_endpoint as *const c_void,
        1,
    );
    ruby_bridge::define_module_function_raw(
        native,
        "bluetooth_backend",
        native_bluetooth_backend as *const c_void,
        1,
    );
    ruby_bridge::define_module_function_raw(
        native,
        "bluetooth_transact",
        native_bluetooth_transact as *const c_void,
        2,
    );
    ruby_bridge::define_module_function_raw(
        native,
        "bluetooth_endpoint_candidates",
        native_bluetooth_endpoint_candidates as *const c_void,
        1,
    );
    ruby_bridge::define_module_function_raw(
        native,
        "bluetooth_devices",
        native_bluetooth_devices as *const c_void,
        0,
    );
    ruby_bridge::define_module_function_raw(
        native,
        "esp_upload_options",
        native_esp_upload_options as *const c_void,
        1,
    );
    ruby_bridge::define_module_function_raw(
        native,
        "pico_uf2_upload_options",
        native_pico_uf2_upload_options as *const c_void,
        1,
    );
    ruby_bridge::define_module_function_raw(
        native,
        "discover_devices",
        native_discover_devices as *const c_void,
        0,
    );
    ruby_bridge::define_module_function_raw(
        native,
        "classify_devices",
        native_classify_devices as *const c_void,
        1,
    );
    ruby_bridge::define_module_function_raw(
        native,
        "pico_uf2_mounts",
        native_pico_uf2_mounts as *const c_void,
        1,
    );

    ruby_bridge::define_alloc_func(session_class, session_alloc);
    ruby_bridge::define_method_raw(
        session_class,
        "initialize",
        session_initialize as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "next_request_id",
        session_next_request_id as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "hello_wire",
        session_hello_wire as *const c_void,
        2,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "caps_query_wire",
        session_caps_query_wire as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "blink_module",
        session_blink_module as *const c_void,
        4,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "gpio_read_module",
        session_gpio_read_module as *const c_void,
        3,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "gpio_write_module",
        session_gpio_write_module as *const c_void,
        3,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "gpio_open_module",
        session_gpio_open_module as *const c_void,
        3,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "gpio_handle_read_module",
        session_gpio_handle_read_module as *const c_void,
        1,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "gpio_handle_write_module",
        session_gpio_handle_write_module as *const c_void,
        2,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "gpio_handle_close_module",
        session_gpio_handle_close_module as *const c_void,
        1,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "time_now_module",
        session_time_now_module as *const c_void,
        1,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "time_sleep_ms_module",
        session_time_sleep_ms_module as *const c_void,
        2,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "led_matrix_frame_module",
        session_led_matrix_frame_module as *const c_void,
        4,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "pwm_write_module",
        session_pwm_write_module as *const c_void,
        3,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "adc_read_module",
        session_adc_read_module as *const c_void,
        2,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "dac_write_u12_module",
        session_dac_write_u12_module as *const c_void,
        3,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "i2c_open_module",
        session_i2c_open_module as *const c_void,
        2,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "i2c_write_u8_module",
        session_i2c_write_u8_module as *const c_void,
        3,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "i2c_read_u8_module",
        session_i2c_read_u8_module as *const c_void,
        2,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "raw_module",
        session_raw_module as *const c_void,
        4,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "program_begin_wire",
        session_program_begin_wire as *const c_void,
        2,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "program_chunk_wire",
        session_program_chunk_wire as *const c_void,
        3,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "program_end_wire",
        session_program_end_wire as *const c_void,
        1,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "store_program_wire",
        session_store_program_wire as *const c_void,
        3,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "run_background_wire",
        session_run_background_wire as *const c_void,
        2,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "run_wire",
        session_run_wire as *const c_void,
        4,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "stop_wire",
        session_stop_wire as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "blink_upload_run_frames",
        session_blink_upload_run_frames as *const c_void,
        -1,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "decode_response",
        session_decode_response as *const c_void,
        1,
    );
}
