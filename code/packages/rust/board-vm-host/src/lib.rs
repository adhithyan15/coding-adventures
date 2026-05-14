use board_vm_ir::{
    parse_module, validate, CapabilitySet, ModuleError, ValidateError, CAP_ADC_READ,
    CAP_DAC_WRITE_U12, CAP_GPIO_CLOSE, CAP_GPIO_OPEN, CAP_GPIO_READ, CAP_GPIO_WRITE,
    CAP_LED_MATRIX_FRAME, CAP_PWM_WRITE, CAP_TIME_NOW_MS, CAP_TIME_SLEEP_MS,
    FLAG_PROGRAM_MAY_RUN_FOREVER, FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES, MODULE_MAGIC,
    MODULE_VERSION,
};
use board_vm_protocol::{
    encode_frame, encode_hello, encode_program_begin, encode_program_chunk, encode_program_end,
    encode_run_request, encode_store_program, encode_stream_frame, encode_wire_frame, Frame, Hello,
    MessageType, ProgramBegin, ProgramChunk, ProgramEnd, ProgramFormat, ProtocolError, RunRequest,
    StoreProgram, BOOT_RUN_IF_NO_HOST, FLAG_RESPONSE_REQUIRED, RUN_FLAG_BACKGROUND_RUN,
    RUN_FLAG_RESET_VM_BEFORE_RUN,
};

pub const DEFAULT_HOST_NAME: &str = "board-vm-host";
pub const DEFAULT_PROGRAM_ID: u16 = 1;
pub const DEFAULT_INSTRUCTION_BUDGET: u32 = 1000;
pub const DEFAULT_RUN_FLAGS: u8 = RUN_FLAG_RESET_VM_BEFORE_RUN | RUN_FLAG_BACKGROUND_RUN;
pub const BLINK_CODE_LEN: usize = 26;
pub const BLINK_MODULE_LEN: usize = 36;
pub const GPIO_WRITE_CODE_LEN: usize = 12;
pub const GPIO_WRITE_MODULE_LEN: usize = 22;
pub const GPIO_READ_CODE_LEN: usize = 13;
pub const GPIO_READ_MODULE_LEN: usize = 23;
pub const GPIO_OPEN_CODE_LEN: usize = 8;
pub const GPIO_OPEN_MODULE_LEN: usize = 18;
pub const GPIO_HANDLE_READ_CODE_LEN: usize = 4;
pub const GPIO_HANDLE_READ_MODULE_LEN: usize = 14;
pub const GPIO_HANDLE_WRITE_CODE_LEN: usize = 4;
pub const GPIO_HANDLE_WRITE_MODULE_LEN: usize = 14;
pub const GPIO_HANDLE_CLOSE_CODE_LEN: usize = 2;
pub const GPIO_HANDLE_CLOSE_MODULE_LEN: usize = 12;
pub const TIME_NOW_CODE_LEN: usize = 3;
pub const TIME_NOW_MODULE_LEN: usize = 13;
pub const TIME_SLEEP_MS_CODE_LEN: usize = 5;
pub const TIME_SLEEP_MS_MODULE_LEN: usize = 15;
pub const PWM_WRITE_CODE_LEN: usize = 8;
pub const PWM_WRITE_MODULE_LEN: usize = 18;
pub const ADC_READ_CODE_LEN: usize = 5;
pub const ADC_READ_MODULE_LEN: usize = 15;
pub const DAC_WRITE_U12_CODE_LEN: usize = 8;
pub const DAC_WRITE_U12_MODULE_LEN: usize = 18;
pub const LED_MATRIX_FRAME_CODE_LEN: usize = 18;
pub const LED_MATRIX_FRAME_MODULE_LEN: usize = 28;

pub const GPIO_MODE_INPUT: u8 = 0;
pub const GPIO_MODE_OUTPUT: u8 = 1;
pub const GPIO_MODE_INPUT_PULLUP: u8 = 2;
pub const GPIO_MODE_INPUT_PULLDOWN: u8 = 3;

const OP_HALT: u8 = 0x00;
const OP_PUSH_FALSE: u8 = 0x10;
const OP_PUSH_TRUE: u8 = 0x11;
const OP_PUSH_U8: u8 = 0x12;
const OP_PUSH_U16: u8 = 0x13;
const OP_PUSH_U32: u8 = 0x14;
const OP_DUP: u8 = 0x20;
const OP_SWAP: u8 = 0x22;
const OP_JUMP_S8: u8 = 0x30;
const OP_CALL_U8: u8 = 0x40;
const OP_RETURN_TOP: u8 = 0x50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostError {
    OutputTooSmall,
    Protocol(ProtocolError),
    Module(ModuleError),
    Validate(ValidateError),
    ProgramTooLarge,
    JumpOutOfRange,
    InvalidGpioReadMode(u8),
}

impl From<ProtocolError> for HostError {
    fn from(value: ProtocolError) -> Self {
        Self::Protocol(value)
    }
}

impl From<ModuleError> for HostError {
    fn from(value: ModuleError) -> Self {
        Self::Module(value)
    }
}

impl From<ValidateError> for HostError {
    fn from(value: ValidateError) -> Self {
        Self::Validate(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlinkProgram {
    pub pin: u8,
    pub high_ms: u16,
    pub low_ms: u16,
    pub max_stack: u8,
}

impl BlinkProgram {
    pub const fn onboard_led() -> Self {
        Self {
            pin: 13,
            high_ms: 250,
            low_ms: 250,
            max_stack: 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpioReadProgram {
    pub pin: u8,
    pub mode: u8,
    pub max_stack: u8,
}

impl GpioReadProgram {
    pub const fn input(pin: u8) -> Self {
        Self {
            pin,
            mode: GPIO_MODE_INPUT,
            max_stack: 2,
        }
    }

    pub const fn input_pullup(pin: u8) -> Self {
        Self {
            pin,
            mode: GPIO_MODE_INPUT_PULLUP,
            max_stack: 2,
        }
    }

    pub const fn input_pulldown(pin: u8) -> Self {
        Self {
            pin,
            mode: GPIO_MODE_INPUT_PULLDOWN,
            max_stack: 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpioWriteProgram {
    pub pin: u8,
    pub value: bool,
    pub max_stack: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpioOpenProgram {
    pub pin: u8,
    pub mode: u8,
    pub max_stack: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PwmWriteProgram {
    pub pin: u8,
    pub duty: u16,
    pub max_stack: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdcReadProgram {
    pub pin: u8,
    pub max_stack: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DacWriteU12Program {
    pub pin: u8,
    pub sample: u16,
    pub max_stack: u8,
}

impl GpioOpenProgram {
    pub const fn output(pin: u8) -> Self {
        Self {
            pin,
            mode: GPIO_MODE_OUTPUT,
            max_stack: 2,
        }
    }

    pub const fn input(pin: u8) -> Self {
        Self {
            pin,
            mode: GPIO_MODE_INPUT,
            max_stack: 2,
        }
    }

    pub const fn input_pullup(pin: u8) -> Self {
        Self {
            pin,
            mode: GPIO_MODE_INPUT_PULLUP,
            max_stack: 2,
        }
    }

    pub const fn input_pulldown(pin: u8) -> Self {
        Self {
            pin,
            mode: GPIO_MODE_INPUT_PULLDOWN,
            max_stack: 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpioHandleReadProgram {
    pub max_stack: u8,
}

impl GpioHandleReadProgram {
    pub const fn new() -> Self {
        Self { max_stack: 2 }
    }
}

impl Default for GpioHandleReadProgram {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpioHandleWriteProgram {
    pub value: bool,
    pub max_stack: u8,
}

impl GpioHandleWriteProgram {
    pub const fn high() -> Self {
        Self {
            value: true,
            max_stack: 3,
        }
    }

    pub const fn low() -> Self {
        Self {
            value: false,
            max_stack: 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpioHandleCloseProgram {
    pub max_stack: u8,
}

impl GpioHandleCloseProgram {
    pub const fn new() -> Self {
        Self { max_stack: 1 }
    }
}

impl Default for GpioHandleCloseProgram {
    fn default() -> Self {
        Self::new()
    }
}

impl PwmWriteProgram {
    pub const fn new(pin: u8, duty: u16) -> Self {
        Self {
            pin,
            duty,
            max_stack: 2,
        }
    }

    pub const fn off(pin: u8) -> Self {
        Self::new(pin, 0)
    }

    pub const fn full(pin: u8) -> Self {
        Self::new(pin, u16::MAX)
    }
}

impl AdcReadProgram {
    pub const fn new(pin: u8) -> Self {
        Self { pin, max_stack: 1 }
    }
}

impl DacWriteU12Program {
    pub const fn new(pin: u8, sample: u16) -> Self {
        Self {
            pin,
            sample,
            max_stack: 2,
        }
    }
}

impl GpioWriteProgram {
    pub const fn high(pin: u8) -> Self {
        Self {
            pin,
            value: true,
            max_stack: 3,
        }
    }

    pub const fn low(pin: u8) -> Self {
        Self {
            pin,
            value: false,
            max_stack: 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeNowProgram {
    pub max_stack: u8,
}

impl TimeNowProgram {
    pub const fn new() -> Self {
        Self { max_stack: 1 }
    }
}

impl Default for TimeNowProgram {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeSleepMsProgram {
    pub duration_ms: u16,
    pub max_stack: u8,
}

impl TimeSleepMsProgram {
    pub const fn new(duration_ms: u16) -> Self {
        Self {
            duration_ms,
            max_stack: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedMatrixFrameProgram {
    pub words: [u32; 3],
    pub max_stack: u8,
}

impl LedMatrixFrameProgram {
    pub const fn new(words: [u32; 3]) -> Self {
        Self {
            words,
            max_stack: 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleSpec<'a> {
    pub flags: u8,
    pub max_stack: u8,
    pub code: &'a [u8],
    pub const_pool: &'a [u8],
}

impl<'a> ModuleSpec<'a> {
    pub const fn new(flags: u8, max_stack: u8, code: &'a [u8]) -> Self {
        Self {
            flags,
            max_stack,
            code,
            const_pool: &[],
        }
    }

    pub const fn const_pool(mut self, const_pool: &'a [u8]) -> Self {
        self.const_pool = const_pool;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WrittenFrame {
    pub request_id: u16,
    pub len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostSession {
    next_request_id: u16,
}

impl HostSession {
    pub const fn new() -> Self {
        Self { next_request_id: 1 }
    }

    pub const fn with_next_request_id(next_request_id: u16) -> Self {
        Self {
            next_request_id: if next_request_id == 0 {
                1
            } else {
                next_request_id
            },
        }
    }

    pub const fn next_request_id(&self) -> u16 {
        self.next_request_id
    }

    pub fn hello_frame(
        &mut self,
        host_name: &str,
        host_nonce: u32,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<WrittenFrame, HostError> {
        let payload_len = encode_hello(
            &Hello {
                min_version: 1,
                max_version: 1,
                host_name,
                host_nonce,
            },
            payload_out,
        )?;
        self.request_frame(MessageType::HELLO, &payload_out[..payload_len], frame_out)
    }

    pub fn caps_query_frame(&mut self, frame_out: &mut [u8]) -> Result<WrittenFrame, HostError> {
        self.request_frame(MessageType::CAPS_QUERY, &[], frame_out)
    }

    pub fn program_begin_frame(
        &mut self,
        program_id: u16,
        module: &[u8],
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<WrittenFrame, HostError> {
        if module.len() > u32::MAX as usize {
            return Err(HostError::ProgramTooLarge);
        }
        let payload_len = encode_program_begin(
            &ProgramBegin {
                program_id,
                format: ProgramFormat::BvmModule,
                total_len: module.len() as u32,
                program_crc32: crc32_ieee(module),
            },
            payload_out,
        )?;
        self.request_frame(
            MessageType::PROGRAM_BEGIN,
            &payload_out[..payload_len],
            frame_out,
        )
    }

    pub fn program_chunk_frame(
        &mut self,
        program_id: u16,
        offset: u32,
        chunk: &[u8],
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<WrittenFrame, HostError> {
        let payload_len = encode_program_chunk(
            &ProgramChunk {
                program_id,
                offset,
                bytes: chunk,
            },
            payload_out,
        )?;
        self.request_frame(
            MessageType::PROGRAM_CHUNK,
            &payload_out[..payload_len],
            frame_out,
        )
    }

    pub fn program_end_frame(
        &mut self,
        program_id: u16,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<WrittenFrame, HostError> {
        let payload_len = encode_program_end(&ProgramEnd { program_id }, payload_out)?;
        self.request_frame(
            MessageType::PROGRAM_END,
            &payload_out[..payload_len],
            frame_out,
        )
    }

    pub fn run_background_frame(
        &mut self,
        program_id: u16,
        instruction_budget: u32,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<WrittenFrame, HostError> {
        self.run_frame(
            program_id,
            DEFAULT_RUN_FLAGS,
            instruction_budget,
            0,
            payload_out,
            frame_out,
        )
    }

    pub fn run_frame(
        &mut self,
        program_id: u16,
        flags: u8,
        instruction_budget: u32,
        time_budget_ms: u32,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<WrittenFrame, HostError> {
        let payload_len = encode_run_request(
            &RunRequest {
                program_id,
                flags,
                instruction_budget,
                time_budget_ms,
            },
            payload_out,
        )?;
        self.request_frame(MessageType::RUN, &payload_out[..payload_len], frame_out)
    }

    pub fn store_program_frame(
        &mut self,
        program_id: u16,
        slot: u8,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<WrittenFrame, HostError> {
        self.store_program_with_boot_policy_frame(
            program_id,
            slot,
            BOOT_RUN_IF_NO_HOST,
            payload_out,
            frame_out,
        )
    }

    pub fn store_program_with_boot_policy_frame(
        &mut self,
        program_id: u16,
        slot: u8,
        boot_policy: u8,
        payload_out: &mut [u8],
        frame_out: &mut [u8],
    ) -> Result<WrittenFrame, HostError> {
        let payload_len = encode_store_program(
            &StoreProgram {
                program_id,
                slot,
                boot_policy,
            },
            payload_out,
        )?;
        self.request_frame(
            MessageType::STORE_PROGRAM,
            &payload_out[..payload_len],
            frame_out,
        )
    }

    pub fn stop_frame(&mut self, frame_out: &mut [u8]) -> Result<WrittenFrame, HostError> {
        self.request_frame(MessageType::STOP, &[], frame_out)
    }

    pub fn bootloader_reboot_frame(
        &mut self,
        frame_out: &mut [u8],
    ) -> Result<WrittenFrame, HostError> {
        self.request_frame(MessageType::BOOTLOADER_REBOOT, &[], frame_out)
    }

    pub fn request_stream_frame(
        &mut self,
        message_type: MessageType,
        payload: &[u8],
        raw_out: &mut [u8],
        wire_out: &mut [u8],
    ) -> Result<WrittenFrame, HostError> {
        let request_id = self.take_request_id();
        let len = encode_stream_frame(
            &Frame {
                flags: FLAG_RESPONSE_REQUIRED,
                message_type,
                request_id,
                payload,
            },
            raw_out,
            wire_out,
        )?;
        Ok(WrittenFrame { request_id, len })
    }

    fn request_frame(
        &mut self,
        message_type: MessageType,
        payload: &[u8],
        frame_out: &mut [u8],
    ) -> Result<WrittenFrame, HostError> {
        let request_id = self.take_request_id();
        let len = encode_frame(
            &Frame {
                flags: FLAG_RESPONSE_REQUIRED,
                message_type,
                request_id,
                payload,
            },
            frame_out,
        )?;
        Ok(WrittenFrame { request_id, len })
    }

    fn take_request_id(&mut self) -> u16 {
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1);
        if self.next_request_id == 0 {
            self.next_request_id = 1;
        }
        request_id
    }
}

impl Default for HostSession {
    fn default() -> Self {
        Self::new()
    }
}

pub fn write_blink_code(program: BlinkProgram, out: &mut [u8]) -> Result<usize, HostError> {
    if out.len() < BLINK_CODE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut offset = 0;
    write_u8(out, &mut offset, OP_PUSH_U8)?;
    write_u8(out, &mut offset, program.pin)?;
    write_u8(out, &mut offset, OP_PUSH_U8)?;
    write_u8(out, &mut offset, 1)?;
    write_call_u8(out, &mut offset, CAP_GPIO_OPEN)?;

    let loop_start = offset;
    write_u8(out, &mut offset, OP_DUP)?;
    write_u8(out, &mut offset, OP_PUSH_TRUE)?;
    write_call_u8(out, &mut offset, CAP_GPIO_WRITE)?;
    write_push_u16(out, &mut offset, program.high_ms)?;
    write_call_u8(out, &mut offset, CAP_TIME_SLEEP_MS)?;
    write_u8(out, &mut offset, OP_DUP)?;
    write_u8(out, &mut offset, OP_PUSH_FALSE)?;
    write_call_u8(out, &mut offset, CAP_GPIO_WRITE)?;
    write_push_u16(out, &mut offset, program.low_ms)?;
    write_call_u8(out, &mut offset, CAP_TIME_SLEEP_MS)?;

    let jump_next = offset + 2;
    let jump_offset = loop_start as isize - jump_next as isize;
    if !(i8::MIN as isize..=i8::MAX as isize).contains(&jump_offset) {
        return Err(HostError::JumpOutOfRange);
    }
    write_u8(out, &mut offset, OP_JUMP_S8)?;
    write_u8(out, &mut offset, jump_offset as i8 as u8)?;
    Ok(offset)
}

pub fn write_blink_module(program: BlinkProgram, out: &mut [u8]) -> Result<usize, HostError> {
    if out.len() < BLINK_MODULE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut code = [0u8; BLINK_CODE_LEN];
    let code_len = write_blink_code(program, &mut code)?;
    let offset = write_module(
        ModuleSpec::new(
            FLAG_PROGRAM_MAY_RUN_FOREVER,
            program.max_stack,
            &code[..code_len],
        ),
        out,
    )?;
    let module = parse_module(&out[..offset])?;
    validate(&module, CapabilitySet::blink_mvp(), program.max_stack)?;
    Ok(offset)
}

pub fn write_gpio_read_code(program: GpioReadProgram, out: &mut [u8]) -> Result<usize, HostError> {
    if out.len() < GPIO_READ_CODE_LEN {
        return Err(HostError::OutputTooSmall);
    }
    validate_gpio_read_mode(program.mode)?;

    let mut offset = 0;
    write_u8(out, &mut offset, OP_PUSH_U8)?;
    write_u8(out, &mut offset, program.pin)?;
    write_u8(out, &mut offset, OP_PUSH_U8)?;
    write_u8(out, &mut offset, program.mode)?;
    write_call_u8(out, &mut offset, CAP_GPIO_OPEN)?;
    write_u8(out, &mut offset, OP_DUP)?;
    write_call_u8(out, &mut offset, CAP_GPIO_READ)?;
    write_u8(out, &mut offset, OP_SWAP)?;
    write_call_u8(out, &mut offset, CAP_GPIO_CLOSE)?;
    write_u8(out, &mut offset, OP_RETURN_TOP)?;
    Ok(offset)
}

pub fn write_gpio_write_code(
    program: GpioWriteProgram,
    out: &mut [u8],
) -> Result<usize, HostError> {
    if out.len() < GPIO_WRITE_CODE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut offset = 0;
    write_u8(out, &mut offset, OP_PUSH_U8)?;
    write_u8(out, &mut offset, program.pin)?;
    write_u8(out, &mut offset, OP_PUSH_U8)?;
    write_u8(out, &mut offset, GPIO_MODE_OUTPUT)?;
    write_call_u8(out, &mut offset, CAP_GPIO_OPEN)?;
    write_u8(out, &mut offset, OP_DUP)?;
    write_u8(
        out,
        &mut offset,
        if program.value {
            OP_PUSH_TRUE
        } else {
            OP_PUSH_FALSE
        },
    )?;
    write_call_u8(out, &mut offset, CAP_GPIO_WRITE)?;
    write_call_u8(out, &mut offset, CAP_GPIO_CLOSE)?;
    Ok(offset)
}

pub fn write_gpio_write_module(
    program: GpioWriteProgram,
    out: &mut [u8],
) -> Result<usize, HostError> {
    if out.len() < GPIO_WRITE_MODULE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut code = [0u8; GPIO_WRITE_CODE_LEN];
    let code_len = write_gpio_write_code(program, &mut code)?;
    let offset = write_module(
        ModuleSpec::new(0, program.max_stack, &code[..code_len]),
        out,
    )?;
    let module = parse_module(&out[..offset])?;
    validate(&module, CapabilitySet::blink_mvp(), program.max_stack)?;
    Ok(offset)
}

pub fn write_gpio_open_code(program: GpioOpenProgram, out: &mut [u8]) -> Result<usize, HostError> {
    if out.len() < GPIO_OPEN_CODE_LEN {
        return Err(HostError::OutputTooSmall);
    }
    validate_gpio_mode(program.mode)?;

    let mut offset = 0;
    write_u8(out, &mut offset, OP_PUSH_U8)?;
    write_u8(out, &mut offset, program.pin)?;
    write_u8(out, &mut offset, OP_PUSH_U8)?;
    write_u8(out, &mut offset, program.mode)?;
    write_call_u8(out, &mut offset, CAP_GPIO_OPEN)?;
    write_u8(out, &mut offset, OP_DUP)?;
    write_u8(out, &mut offset, OP_RETURN_TOP)?;
    Ok(offset)
}

pub fn write_gpio_open_module(
    program: GpioOpenProgram,
    out: &mut [u8],
) -> Result<usize, HostError> {
    if out.len() < GPIO_OPEN_MODULE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut code = [0u8; GPIO_OPEN_CODE_LEN];
    let code_len = write_gpio_open_code(program, &mut code)?;
    let offset = write_module(
        ModuleSpec::new(0, program.max_stack, &code[..code_len]),
        out,
    )?;
    let module = parse_module(&out[..offset])?;
    validate(&module, CapabilitySet::blink_mvp(), program.max_stack)?;
    Ok(offset)
}

pub fn write_gpio_handle_read_code(
    _program: GpioHandleReadProgram,
    out: &mut [u8],
) -> Result<usize, HostError> {
    if out.len() < GPIO_HANDLE_READ_CODE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut offset = 0;
    write_u8(out, &mut offset, OP_DUP)?;
    write_call_u8(out, &mut offset, CAP_GPIO_READ)?;
    write_u8(out, &mut offset, OP_RETURN_TOP)?;
    Ok(offset)
}

pub fn write_gpio_handle_read_module(
    program: GpioHandleReadProgram,
    out: &mut [u8],
) -> Result<usize, HostError> {
    if out.len() < GPIO_HANDLE_READ_MODULE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut code = [0u8; GPIO_HANDLE_READ_CODE_LEN];
    let code_len = write_gpio_handle_read_code(program, &mut code)?;
    let offset = write_module(
        ModuleSpec::new(
            FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES,
            program.max_stack,
            &code[..code_len],
        ),
        out,
    )?;
    let module = parse_module(&out[..offset])?;
    validate(&module, CapabilitySet::blink_mvp(), program.max_stack)?;
    Ok(offset)
}

pub fn write_gpio_handle_write_code(
    program: GpioHandleWriteProgram,
    out: &mut [u8],
) -> Result<usize, HostError> {
    if out.len() < GPIO_HANDLE_WRITE_CODE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut offset = 0;
    write_u8(out, &mut offset, OP_DUP)?;
    write_u8(
        out,
        &mut offset,
        if program.value {
            OP_PUSH_TRUE
        } else {
            OP_PUSH_FALSE
        },
    )?;
    write_call_u8(out, &mut offset, CAP_GPIO_WRITE)?;
    Ok(offset)
}

pub fn write_gpio_handle_write_module(
    program: GpioHandleWriteProgram,
    out: &mut [u8],
) -> Result<usize, HostError> {
    if out.len() < GPIO_HANDLE_WRITE_MODULE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut code = [0u8; GPIO_HANDLE_WRITE_CODE_LEN];
    let code_len = write_gpio_handle_write_code(program, &mut code)?;
    let offset = write_module(
        ModuleSpec::new(
            FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES,
            program.max_stack,
            &code[..code_len],
        ),
        out,
    )?;
    let module = parse_module(&out[..offset])?;
    validate(&module, CapabilitySet::blink_mvp(), program.max_stack)?;
    Ok(offset)
}

pub fn write_gpio_handle_close_code(
    _program: GpioHandleCloseProgram,
    out: &mut [u8],
) -> Result<usize, HostError> {
    if out.len() < GPIO_HANDLE_CLOSE_CODE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut offset = 0;
    write_call_u8(out, &mut offset, CAP_GPIO_CLOSE)?;
    Ok(offset)
}

pub fn write_gpio_handle_close_module(
    program: GpioHandleCloseProgram,
    out: &mut [u8],
) -> Result<usize, HostError> {
    if out.len() < GPIO_HANDLE_CLOSE_MODULE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut code = [0u8; GPIO_HANDLE_CLOSE_CODE_LEN];
    let code_len = write_gpio_handle_close_code(program, &mut code)?;
    let offset = write_module(
        ModuleSpec::new(
            FLAG_PROGRAM_REQUESTS_PERSISTENT_HANDLES,
            program.max_stack,
            &code[..code_len],
        ),
        out,
    )?;
    let module = parse_module(&out[..offset])?;
    validate(&module, CapabilitySet::blink_mvp(), program.max_stack)?;
    Ok(offset)
}

fn validate_gpio_read_mode(mode: u8) -> Result<(), HostError> {
    match mode {
        GPIO_MODE_INPUT | GPIO_MODE_INPUT_PULLUP | GPIO_MODE_INPUT_PULLDOWN => Ok(()),
        other => Err(HostError::InvalidGpioReadMode(other)),
    }
}

fn validate_gpio_mode(mode: u8) -> Result<(), HostError> {
    match mode {
        GPIO_MODE_INPUT | GPIO_MODE_OUTPUT | GPIO_MODE_INPUT_PULLUP | GPIO_MODE_INPUT_PULLDOWN => {
            Ok(())
        }
        other => Err(HostError::InvalidGpioReadMode(other)),
    }
}

pub fn write_gpio_read_module(
    program: GpioReadProgram,
    out: &mut [u8],
) -> Result<usize, HostError> {
    if out.len() < GPIO_READ_MODULE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut code = [0u8; GPIO_READ_CODE_LEN];
    let code_len = write_gpio_read_code(program, &mut code)?;
    let offset = write_module(
        ModuleSpec::new(0, program.max_stack, &code[..code_len]),
        out,
    )?;
    let module = parse_module(&out[..offset])?;
    validate(&module, CapabilitySet::blink_mvp(), program.max_stack)?;
    Ok(offset)
}

pub fn write_time_now_code(_program: TimeNowProgram, out: &mut [u8]) -> Result<usize, HostError> {
    if out.len() < TIME_NOW_CODE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut offset = 0;
    write_call_u8(out, &mut offset, CAP_TIME_NOW_MS)?;
    write_u8(out, &mut offset, OP_RETURN_TOP)?;
    Ok(offset)
}

pub fn write_time_now_module(program: TimeNowProgram, out: &mut [u8]) -> Result<usize, HostError> {
    if out.len() < TIME_NOW_MODULE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut code = [0u8; TIME_NOW_CODE_LEN];
    let code_len = write_time_now_code(program, &mut code)?;
    let offset = write_module(
        ModuleSpec::new(0, program.max_stack, &code[..code_len]),
        out,
    )?;
    let module = parse_module(&out[..offset])?;
    validate(&module, CapabilitySet::blink_mvp(), program.max_stack)?;
    Ok(offset)
}

pub fn write_time_sleep_ms_code(
    program: TimeSleepMsProgram,
    out: &mut [u8],
) -> Result<usize, HostError> {
    if out.len() < TIME_SLEEP_MS_CODE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut offset = 0;
    write_push_u16(out, &mut offset, program.duration_ms)?;
    write_call_u8(out, &mut offset, CAP_TIME_SLEEP_MS)?;
    Ok(offset)
}

pub fn write_time_sleep_ms_module(
    program: TimeSleepMsProgram,
    out: &mut [u8],
) -> Result<usize, HostError> {
    if out.len() < TIME_SLEEP_MS_MODULE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut code = [0u8; TIME_SLEEP_MS_CODE_LEN];
    let code_len = write_time_sleep_ms_code(program, &mut code)?;
    let offset = write_module(
        ModuleSpec::new(0, program.max_stack, &code[..code_len]),
        out,
    )?;
    let module = parse_module(&out[..offset])?;
    validate(&module, CapabilitySet::blink_mvp(), program.max_stack)?;
    Ok(offset)
}

pub fn write_pwm_write_code(program: PwmWriteProgram, out: &mut [u8]) -> Result<usize, HostError> {
    if out.len() < PWM_WRITE_CODE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut offset = 0;
    write_u8(out, &mut offset, OP_PUSH_U8)?;
    write_u8(out, &mut offset, program.pin)?;
    write_push_u16(out, &mut offset, program.duty)?;
    write_call_u8(out, &mut offset, CAP_PWM_WRITE)?;
    write_u8(out, &mut offset, OP_HALT)?;
    Ok(offset)
}

pub fn write_pwm_write_module(
    program: PwmWriteProgram,
    out: &mut [u8],
) -> Result<usize, HostError> {
    if out.len() < PWM_WRITE_MODULE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut code = [0u8; PWM_WRITE_CODE_LEN];
    let code_len = write_pwm_write_code(program, &mut code)?;
    let offset = write_module(
        ModuleSpec::new(0, program.max_stack, &code[..code_len]),
        out,
    )?;
    let module = parse_module(&out[..offset])?;
    validate(
        &module,
        CapabilitySet::blink_mvp().with_pwm(),
        program.max_stack,
    )?;
    Ok(offset)
}

pub fn write_adc_read_code(program: AdcReadProgram, out: &mut [u8]) -> Result<usize, HostError> {
    if out.len() < ADC_READ_CODE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut offset = 0;
    write_u8(out, &mut offset, OP_PUSH_U8)?;
    write_u8(out, &mut offset, program.pin)?;
    write_call_u8(out, &mut offset, CAP_ADC_READ)?;
    write_u8(out, &mut offset, OP_RETURN_TOP)?;
    Ok(offset)
}

pub fn write_adc_read_module(program: AdcReadProgram, out: &mut [u8]) -> Result<usize, HostError> {
    if out.len() < ADC_READ_MODULE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut code = [0u8; ADC_READ_CODE_LEN];
    let code_len = write_adc_read_code(program, &mut code)?;
    let offset = write_module(
        ModuleSpec::new(0, program.max_stack, &code[..code_len]),
        out,
    )?;
    let module = parse_module(&out[..offset])?;
    validate(
        &module,
        CapabilitySet::blink_mvp().with_adc(),
        program.max_stack,
    )?;
    Ok(offset)
}

pub fn write_dac_write_u12_code(
    program: DacWriteU12Program,
    out: &mut [u8],
) -> Result<usize, HostError> {
    if out.len() < DAC_WRITE_U12_CODE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut offset = 0;
    write_u8(out, &mut offset, OP_PUSH_U8)?;
    write_u8(out, &mut offset, program.pin)?;
    write_push_u16(out, &mut offset, program.sample)?;
    write_call_u8(out, &mut offset, CAP_DAC_WRITE_U12)?;
    write_u8(out, &mut offset, OP_HALT)?;
    Ok(offset)
}

pub fn write_dac_write_u12_module(
    program: DacWriteU12Program,
    out: &mut [u8],
) -> Result<usize, HostError> {
    if out.len() < DAC_WRITE_U12_MODULE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut code = [0u8; DAC_WRITE_U12_CODE_LEN];
    let code_len = write_dac_write_u12_code(program, &mut code)?;
    let offset = write_module(
        ModuleSpec::new(0, program.max_stack, &code[..code_len]),
        out,
    )?;
    let module = parse_module(&out[..offset])?;
    validate(
        &module,
        CapabilitySet::blink_mvp().with_dac(),
        program.max_stack,
    )?;
    Ok(offset)
}

pub fn write_led_matrix_frame_code(
    program: LedMatrixFrameProgram,
    out: &mut [u8],
) -> Result<usize, HostError> {
    if out.len() < LED_MATRIX_FRAME_CODE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut offset = 0;
    write_push_u32(out, &mut offset, program.words[0])?;
    write_push_u32(out, &mut offset, program.words[1])?;
    write_push_u32(out, &mut offset, program.words[2])?;
    write_call_u8(out, &mut offset, CAP_LED_MATRIX_FRAME)?;
    write_u8(out, &mut offset, OP_HALT)?;
    Ok(offset)
}

pub fn write_led_matrix_frame_module(
    program: LedMatrixFrameProgram,
    out: &mut [u8],
) -> Result<usize, HostError> {
    if out.len() < LED_MATRIX_FRAME_MODULE_LEN {
        return Err(HostError::OutputTooSmall);
    }

    let mut code = [0u8; LED_MATRIX_FRAME_CODE_LEN];
    let code_len = write_led_matrix_frame_code(program, &mut code)?;
    let offset = write_module(
        ModuleSpec::new(0, program.max_stack, &code[..code_len]),
        out,
    )?;
    let module = parse_module(&out[..offset])?;
    validate(
        &module,
        CapabilitySet::blink_mvp().with_led_matrix(),
        program.max_stack,
    )?;
    Ok(offset)
}

pub fn write_module(spec: ModuleSpec<'_>, out: &mut [u8]) -> Result<usize, HostError> {
    if spec.code.len() > u32::MAX as usize || spec.const_pool.len() > u32::MAX as usize {
        return Err(HostError::ProgramTooLarge);
    }

    let mut offset = 0;
    write_slice(out, &mut offset, &MODULE_MAGIC)?;
    write_u8(out, &mut offset, MODULE_VERSION)?;
    write_u8(out, &mut offset, spec.flags)?;
    write_u8(out, &mut offset, spec.max_stack)?;
    write_u8(out, &mut offset, 0)?;
    write_uleb128(out, &mut offset, spec.code.len() as u32)?;
    write_slice(out, &mut offset, spec.code)?;
    write_uleb128(out, &mut offset, spec.const_pool.len() as u32)?;
    write_slice(out, &mut offset, spec.const_pool)?;

    parse_module(&out[..offset])?;
    Ok(offset)
}

pub fn write_blink_upload_and_run_frames(
    session: &mut HostSession,
    program_id: u16,
    module: &[u8],
    payload_out: &mut [u8],
    frames_out: &mut [&mut [u8]; 4],
) -> Result<[WrittenFrame; 4], HostError> {
    Ok([
        session.program_begin_frame(program_id, module, payload_out, frames_out[0])?,
        session.program_chunk_frame(program_id, 0, module, payload_out, frames_out[1])?,
        session.program_end_frame(program_id, payload_out, frames_out[2])?,
        session.run_background_frame(
            program_id,
            DEFAULT_INSTRUCTION_BUDGET,
            payload_out,
            frames_out[3],
        )?,
    ])
}

pub fn write_wire_frame(raw_frame: &[u8], wire_out: &mut [u8]) -> Result<usize, HostError> {
    Ok(encode_wire_frame(raw_frame, wire_out)?)
}

pub fn crc32_ieee(bytes: &[u8]) -> u32 {
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

fn write_call_u8(out: &mut [u8], offset: &mut usize, capability: u16) -> Result<(), HostError> {
    write_u8(out, offset, OP_CALL_U8)?;
    write_u8(out, offset, capability as u8)
}

fn write_push_u16(out: &mut [u8], offset: &mut usize, value: u16) -> Result<(), HostError> {
    write_u8(out, offset, OP_PUSH_U16)?;
    write_slice(out, offset, &value.to_le_bytes())
}

fn write_push_u32(out: &mut [u8], offset: &mut usize, value: u32) -> Result<(), HostError> {
    write_u8(out, offset, OP_PUSH_U32)?;
    write_slice(out, offset, &value.to_le_bytes())
}

fn write_uleb128(out: &mut [u8], offset: &mut usize, mut value: u32) -> Result<(), HostError> {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        write_u8(out, offset, byte)?;
        if value == 0 {
            return Ok(());
        }
    }
}

fn write_u8(out: &mut [u8], offset: &mut usize, value: u8) -> Result<(), HostError> {
    write_slice(out, offset, &[value])
}

fn write_slice(out: &mut [u8], offset: &mut usize, value: &[u8]) -> Result<(), HostError> {
    let end = offset
        .checked_add(value.len())
        .ok_or(HostError::OutputTooSmall)?;
    if end > out.len() {
        return Err(HostError::OutputTooSmall);
    }
    out[*offset..end].copy_from_slice(value);
    *offset = end;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use board_vm_ir::{
        collect_required_capabilities, parse_module, validate, CapabilitySet, ModuleError,
        CAP_ADC_READ, CAP_DAC_WRITE_U12, CAP_LED_MATRIX_FRAME, CAP_PWM_WRITE,
    };
    use board_vm_protocol::{
        decode_frame, decode_program_begin, decode_program_chunk, decode_program_end,
        decode_run_request, MessageType, RUN_FLAG_BACKGROUND_RUN, RUN_FLAG_KEEP_HANDLES_AFTER_RUN,
        RUN_FLAG_RESET_VM_BEFORE_RUN,
    };

    const BLINK_MODULE_HEX: [u8; BLINK_MODULE_LEN] = [
        0x42, 0x56, 0x4D, 0x31, 0x01, 0x01, 0x04, 0x00, 0x1A, 0x12, 0x0D, 0x12, 0x01, 0x40, 0x01,
        0x20, 0x11, 0x40, 0x02, 0x13, 0xFA, 0x00, 0x40, 0x10, 0x20, 0x10, 0x40, 0x02, 0x13, 0xFA,
        0x00, 0x40, 0x10, 0x30, 0xEC, 0x00,
    ];
    const GPIO_READ_PULLUP_MODULE_HEX: [u8; GPIO_READ_MODULE_LEN] = [
        0x42, 0x56, 0x4D, 0x31, 0x01, 0x00, 0x02, 0x00, 0x0D, 0x12, 0x0D, 0x12, 0x02, 0x40, 0x01,
        0x20, 0x40, 0x03, 0x22, 0x40, 0x04, 0x50, 0x00,
    ];
    const GPIO_WRITE_HIGH_MODULE_HEX: [u8; GPIO_WRITE_MODULE_LEN] = [
        0x42, 0x56, 0x4D, 0x31, 0x01, 0x00, 0x03, 0x00, 0x0C, 0x12, 0x0D, 0x12, 0x01, 0x40, 0x01,
        0x20, 0x11, 0x40, 0x02, 0x40, 0x04, 0x00,
    ];
    const GPIO_OPEN_OUTPUT_MODULE_HEX: [u8; GPIO_OPEN_MODULE_LEN] = [
        0x42, 0x56, 0x4D, 0x31, 0x01, 0x00, 0x02, 0x00, 0x08, 0x12, 0x0D, 0x12, 0x01, 0x40, 0x01,
        0x20, 0x50, 0x00,
    ];
    const GPIO_HANDLE_READ_MODULE_HEX: [u8; GPIO_HANDLE_READ_MODULE_LEN] = [
        0x42, 0x56, 0x4D, 0x31, 0x01, 0x04, 0x02, 0x00, 0x04, 0x20, 0x40, 0x03, 0x50, 0x00,
    ];
    const GPIO_HANDLE_WRITE_HIGH_MODULE_HEX: [u8; GPIO_HANDLE_WRITE_MODULE_LEN] = [
        0x42, 0x56, 0x4D, 0x31, 0x01, 0x04, 0x03, 0x00, 0x04, 0x20, 0x11, 0x40, 0x02, 0x00,
    ];
    const GPIO_HANDLE_CLOSE_MODULE_HEX: [u8; GPIO_HANDLE_CLOSE_MODULE_LEN] = [
        0x42, 0x56, 0x4D, 0x31, 0x01, 0x04, 0x01, 0x00, 0x02, 0x40, 0x04, 0x00,
    ];
    const TIME_NOW_MODULE_HEX: [u8; TIME_NOW_MODULE_LEN] = [
        0x42, 0x56, 0x4D, 0x31, 0x01, 0x00, 0x01, 0x00, 0x03, 0x40, 0x11, 0x50, 0x00,
    ];
    const TIME_SLEEP_MS_MODULE_HEX: [u8; TIME_SLEEP_MS_MODULE_LEN] = [
        0x42, 0x56, 0x4D, 0x31, 0x01, 0x00, 0x01, 0x00, 0x05, 0x13, 0xFA, 0x00, 0x40, 0x10, 0x00,
    ];
    const PWM_WRITE_HALF_MODULE_HEX: [u8; PWM_WRITE_MODULE_LEN] = [
        0x42, 0x56, 0x4D, 0x31, 0x01, 0x00, 0x02, 0x00, 0x08, 0x12, 0x03, 0x13, 0x00, 0x80, 0x40,
        0x20, 0x00, 0x00,
    ];
    const ADC_READ_A0_MODULE_HEX: [u8; ADC_READ_MODULE_LEN] = [
        0x42, 0x56, 0x4D, 0x31, 0x01, 0x00, 0x01, 0x00, 0x05, 0x12, 0x0E, 0x40, 0x21, 0x50, 0x00,
    ];
    const DAC_WRITE_A0_MID_MODULE_HEX: [u8; DAC_WRITE_U12_MODULE_LEN] = [
        0x42, 0x56, 0x4D, 0x31, 0x01, 0x00, 0x02, 0x00, 0x08, 0x12, 0x0E, 0x13, 0x00, 0x08, 0x40,
        0x22, 0x00, 0x00,
    ];
    const LED_MATRIX_HEART_MODULE_HEX: [u8; LED_MATRIX_FRAME_MODULE_LEN] = [
        0x42, 0x56, 0x4D, 0x31, 0x01, 0x00, 0x03, 0x00, 0x12, 0x14, 0x44, 0xA4, 0x84, 0x31, 0x14,
        0x81, 0x20, 0x04, 0x44, 0x14, 0x40, 0x00, 0x0A, 0x10, 0x40, 0x30, 0x00, 0x00,
    ];

    #[test]
    fn builds_blink_module_from_bvm05_fixture() {
        let mut module = [0u8; BLINK_MODULE_LEN];
        let len = write_blink_module(BlinkProgram::onboard_led(), &mut module).unwrap();
        assert_eq!(len, BLINK_MODULE_LEN);
        assert_eq!(module, BLINK_MODULE_HEX);

        let parsed = parse_module(&module).unwrap();
        validate(&parsed, CapabilitySet::blink_mvp(), 4).unwrap();
    }

    #[test]
    fn builds_gpio_read_module_with_close_and_return() {
        let mut module = [0u8; GPIO_READ_MODULE_LEN];
        let len = write_gpio_read_module(GpioReadProgram::input_pullup(13), &mut module).unwrap();
        assert_eq!(len, GPIO_READ_MODULE_LEN);
        assert_eq!(module, GPIO_READ_PULLUP_MODULE_HEX);

        let parsed = parse_module(&module).unwrap();
        validate(&parsed, CapabilitySet::blink_mvp(), 2).unwrap();
        let mut capabilities = [0u16; 4];
        let count = collect_required_capabilities(&parsed, &mut capabilities).unwrap();
        assert_eq!(
            &capabilities[..count],
            &[CAP_GPIO_OPEN, CAP_GPIO_READ, CAP_GPIO_CLOSE]
        );
    }

    #[test]
    fn rejects_output_mode_for_gpio_read_module() {
        let mut module = [0u8; GPIO_READ_MODULE_LEN];
        let program = GpioReadProgram {
            pin: 13,
            mode: GPIO_MODE_OUTPUT,
            max_stack: 2,
        };

        assert_eq!(
            write_gpio_read_module(program, &mut module),
            Err(HostError::InvalidGpioReadMode(GPIO_MODE_OUTPUT))
        );
    }

    #[test]
    fn builds_gpio_write_module_with_close() {
        let mut module = [0u8; GPIO_WRITE_MODULE_LEN];
        let len = write_gpio_write_module(GpioWriteProgram::high(13), &mut module).unwrap();
        assert_eq!(len, GPIO_WRITE_MODULE_LEN);
        assert_eq!(module, GPIO_WRITE_HIGH_MODULE_HEX);

        let parsed = parse_module(&module).unwrap();
        validate(&parsed, CapabilitySet::blink_mvp(), 3).unwrap();
        let mut capabilities = [0u16; 4];
        let count = collect_required_capabilities(&parsed, &mut capabilities).unwrap();
        assert_eq!(
            &capabilities[..count],
            &[CAP_GPIO_OPEN, CAP_GPIO_WRITE, CAP_GPIO_CLOSE]
        );
    }

    #[test]
    fn builds_gpio_handle_modules_for_repl_sessions() {
        let mut open = [0u8; GPIO_OPEN_MODULE_LEN];
        let open_len = write_gpio_open_module(GpioOpenProgram::output(13), &mut open).unwrap();
        assert_eq!(open_len, GPIO_OPEN_MODULE_LEN);
        assert_eq!(open, GPIO_OPEN_OUTPUT_MODULE_HEX);
        let parsed = parse_module(&open).unwrap();
        assert_eq!(parsed.flags, 0);
        validate(&parsed, CapabilitySet::blink_mvp(), 2).unwrap();

        let mut read = [0u8; GPIO_HANDLE_READ_MODULE_LEN];
        let read_len =
            write_gpio_handle_read_module(GpioHandleReadProgram::new(), &mut read).unwrap();
        assert_eq!(read_len, GPIO_HANDLE_READ_MODULE_LEN);
        assert_eq!(read, GPIO_HANDLE_READ_MODULE_HEX);
        let parsed = parse_module(&read).unwrap();
        validate(&parsed, CapabilitySet::blink_mvp(), 2).unwrap();

        let mut write = [0u8; GPIO_HANDLE_WRITE_MODULE_LEN];
        let write_len =
            write_gpio_handle_write_module(GpioHandleWriteProgram::high(), &mut write).unwrap();
        assert_eq!(write_len, GPIO_HANDLE_WRITE_MODULE_LEN);
        assert_eq!(write, GPIO_HANDLE_WRITE_HIGH_MODULE_HEX);
        let parsed = parse_module(&write).unwrap();
        validate(&parsed, CapabilitySet::blink_mvp(), 3).unwrap();

        let mut close = [0u8; GPIO_HANDLE_CLOSE_MODULE_LEN];
        let close_len =
            write_gpio_handle_close_module(GpioHandleCloseProgram::new(), &mut close).unwrap();
        assert_eq!(close_len, GPIO_HANDLE_CLOSE_MODULE_LEN);
        assert_eq!(close, GPIO_HANDLE_CLOSE_MODULE_HEX);
        let parsed = parse_module(&close).unwrap();
        validate(&parsed, CapabilitySet::blink_mvp(), 1).unwrap();
    }

    #[test]
    fn builds_time_now_module_with_return() {
        let mut module = [0u8; TIME_NOW_MODULE_LEN];
        let len = write_time_now_module(TimeNowProgram::new(), &mut module).unwrap();
        assert_eq!(len, TIME_NOW_MODULE_LEN);
        assert_eq!(module, TIME_NOW_MODULE_HEX);

        let parsed = parse_module(&module).unwrap();
        validate(&parsed, CapabilitySet::blink_mvp(), 1).unwrap();
        let mut capabilities = [0u16; 1];
        let count = collect_required_capabilities(&parsed, &mut capabilities).unwrap();
        assert_eq!(&capabilities[..count], &[CAP_TIME_NOW_MS]);
    }

    #[test]
    fn builds_time_sleep_ms_module_without_return() {
        let mut module = [0u8; TIME_SLEEP_MS_MODULE_LEN];
        let len = write_time_sleep_ms_module(TimeSleepMsProgram::new(250), &mut module).unwrap();
        assert_eq!(len, TIME_SLEEP_MS_MODULE_LEN);
        assert_eq!(module, TIME_SLEEP_MS_MODULE_HEX);

        let parsed = parse_module(&module).unwrap();
        validate(&parsed, CapabilitySet::blink_mvp(), 1).unwrap();
        let mut capabilities = [0u16; 1];
        let count = collect_required_capabilities(&parsed, &mut capabilities).unwrap();
        assert_eq!(&capabilities[..count], &[CAP_TIME_SLEEP_MS]);
    }

    #[test]
    fn builds_led_matrix_frame_module() {
        let mut module = [0u8; LED_MATRIX_FRAME_MODULE_LEN];
        let len = write_led_matrix_frame_module(
            LedMatrixFrameProgram::new([0x3184_A444, 0x4404_2081, 0x100A_0040]),
            &mut module,
        )
        .unwrap();
        assert_eq!(len, LED_MATRIX_FRAME_MODULE_LEN);
        assert_eq!(module, LED_MATRIX_HEART_MODULE_HEX);

        let parsed = parse_module(&module).unwrap();
        validate(&parsed, CapabilitySet::blink_mvp().with_led_matrix(), 3).unwrap();
        let mut capabilities = [0u16; 1];
        let count = collect_required_capabilities(&parsed, &mut capabilities).unwrap();
        assert_eq!(&capabilities[..count], &[CAP_LED_MATRIX_FRAME]);
    }

    #[test]
    fn builds_pwm_write_module() {
        let mut module = [0u8; PWM_WRITE_MODULE_LEN];
        let len = write_pwm_write_module(PwmWriteProgram::new(3, 0x8000), &mut module).unwrap();
        assert_eq!(len, PWM_WRITE_MODULE_LEN);
        assert_eq!(module, PWM_WRITE_HALF_MODULE_HEX);

        let parsed = parse_module(&module).unwrap();
        validate(&parsed, CapabilitySet::blink_mvp().with_pwm(), 2).unwrap();
        let mut capabilities = [0u16; 1];
        let count = collect_required_capabilities(&parsed, &mut capabilities).unwrap();
        assert_eq!(&capabilities[..count], &[CAP_PWM_WRITE]);
    }

    #[test]
    fn builds_adc_read_module() {
        let mut module = [0u8; ADC_READ_MODULE_LEN];
        let len = write_adc_read_module(AdcReadProgram::new(14), &mut module).unwrap();
        assert_eq!(len, ADC_READ_MODULE_LEN);
        assert_eq!(module, ADC_READ_A0_MODULE_HEX);

        let parsed = parse_module(&module).unwrap();
        validate(&parsed, CapabilitySet::blink_mvp().with_adc(), 1).unwrap();
        let mut capabilities = [0u16; 1];
        let count = collect_required_capabilities(&parsed, &mut capabilities).unwrap();
        assert_eq!(&capabilities[..count], &[CAP_ADC_READ]);
    }

    #[test]
    fn builds_dac_write_u12_module() {
        let mut module = [0u8; DAC_WRITE_U12_MODULE_LEN];
        let len =
            write_dac_write_u12_module(DacWriteU12Program::new(14, 0x0800), &mut module).unwrap();
        assert_eq!(len, DAC_WRITE_U12_MODULE_LEN);
        assert_eq!(module, DAC_WRITE_A0_MID_MODULE_HEX);

        let parsed = parse_module(&module).unwrap();
        validate(&parsed, CapabilitySet::blink_mvp().with_dac(), 2).unwrap();
        let mut capabilities = [0u16; 1];
        let count = collect_required_capabilities(&parsed, &mut capabilities).unwrap();
        assert_eq!(&capabilities[..count], &[CAP_DAC_WRITE_U12]);
    }

    #[test]
    fn writes_generic_module_from_code_and_const_pool() {
        let code = [0x00];
        let const_pool = [0xAA, 0x55];
        let mut module = [0u8; 32];

        let len = write_module(
            ModuleSpec::new(0, 1, &code).const_pool(&const_pool),
            &mut module,
        )
        .unwrap();

        let parsed = parse_module(&module[..len]).unwrap();
        assert_eq!(parsed.flags, 0);
        assert_eq!(parsed.max_stack, 1);
        assert_eq!(parsed.code, &code);
        assert_eq!(parsed.const_pool, &const_pool);
    }

    #[test]
    fn rejects_invalid_generic_module_flags() {
        let code = [0x00];
        let mut module = [0u8; 16];

        assert_eq!(
            write_module(ModuleSpec::new(0x80, 1, &code), &mut module),
            Err(HostError::Module(ModuleError::ReservedFlags(0x80)))
        );
    }

    #[test]
    fn crc32_matches_standard_check_vector() {
        assert_eq!(crc32_ieee(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32_ieee(&BLINK_MODULE_HEX), 0xBAD6_949E);
    }

    #[test]
    fn writes_handshake_and_caps_frames() {
        let mut session = HostSession::new();
        let mut payload = [0u8; 64];
        let mut frame = [0u8; 96];

        let hello = session
            .hello_frame(DEFAULT_HOST_NAME, 0xAABB_CCDD, &mut payload, &mut frame)
            .unwrap();
        assert_eq!(hello.request_id, 1);
        let decoded = decode_frame(&frame[..hello.len]).unwrap();
        assert_eq!(decoded.message_type, MessageType::HELLO);
        assert_eq!(decoded.request_id, 1);
        assert_eq!(decoded.flags, FLAG_RESPONSE_REQUIRED);

        let caps = session.caps_query_frame(&mut frame).unwrap();
        assert_eq!(caps.request_id, 2);
        let decoded = decode_frame(&frame[..caps.len]).unwrap();
        assert_eq!(decoded.message_type, MessageType::CAPS_QUERY);
        assert_eq!(decoded.payload, &[]);
    }

    #[test]
    fn writes_upload_and_run_sequence() {
        let mut session = HostSession::with_next_request_id(10);
        let mut module = [0u8; BLINK_MODULE_LEN];
        let module_len = write_blink_module(BlinkProgram::onboard_led(), &mut module).unwrap();
        let module = &module[..module_len];
        let mut payload = [0u8; 96];
        let mut begin_frame = [0u8; 128];
        let mut chunk_frame = [0u8; 160];
        let mut end_frame = [0u8; 64];
        let mut run_frame = [0u8; 96];

        let begin = session
            .program_begin_frame(DEFAULT_PROGRAM_ID, module, &mut payload, &mut begin_frame)
            .unwrap();
        let decoded = decode_frame(&begin_frame[..begin.len]).unwrap();
        assert_eq!(begin.request_id, 10);
        assert_eq!(decoded.message_type, MessageType::PROGRAM_BEGIN);
        let begin_payload = decode_program_begin(decoded.payload).unwrap();
        assert_eq!(begin_payload.program_id, DEFAULT_PROGRAM_ID);
        assert_eq!(begin_payload.total_len, BLINK_MODULE_LEN as u32);
        assert_eq!(begin_payload.program_crc32, crc32_ieee(module));

        let chunk = session
            .program_chunk_frame(
                DEFAULT_PROGRAM_ID,
                0,
                module,
                &mut payload,
                &mut chunk_frame,
            )
            .unwrap();
        let decoded = decode_frame(&chunk_frame[..chunk.len]).unwrap();
        assert_eq!(chunk.request_id, 11);
        assert_eq!(decoded.message_type, MessageType::PROGRAM_CHUNK);
        let chunk_payload = decode_program_chunk(decoded.payload).unwrap();
        assert_eq!(chunk_payload.offset, 0);
        assert_eq!(chunk_payload.bytes, module);

        let end = session
            .program_end_frame(DEFAULT_PROGRAM_ID, &mut payload, &mut end_frame)
            .unwrap();
        let decoded = decode_frame(&end_frame[..end.len]).unwrap();
        assert_eq!(end.request_id, 12);
        assert_eq!(decoded.message_type, MessageType::PROGRAM_END);
        assert_eq!(
            decode_program_end(decoded.payload).unwrap().program_id,
            DEFAULT_PROGRAM_ID
        );

        let run = session
            .run_background_frame(
                DEFAULT_PROGRAM_ID,
                DEFAULT_INSTRUCTION_BUDGET,
                &mut payload,
                &mut run_frame,
            )
            .unwrap();
        let decoded = decode_frame(&run_frame[..run.len]).unwrap();
        assert_eq!(run.request_id, 13);
        assert_eq!(decoded.message_type, MessageType::RUN);
        let run_payload = decode_run_request(decoded.payload).unwrap();
        assert_eq!(run_payload.program_id, DEFAULT_PROGRAM_ID);
        assert_eq!(
            run_payload.flags,
            RUN_FLAG_RESET_VM_BEFORE_RUN | RUN_FLAG_BACKGROUND_RUN
        );
        assert_eq!(run_payload.instruction_budget, DEFAULT_INSTRUCTION_BUDGET);
    }

    #[test]
    fn writes_configurable_run_frame() {
        let mut session = HostSession::new();
        let mut payload = [0u8; 16];
        let mut frame = [0u8; 48];

        let written = session
            .run_frame(
                42,
                RUN_FLAG_KEEP_HANDLES_AFTER_RUN,
                777,
                250,
                &mut payload,
                &mut frame,
            )
            .unwrap();
        let decoded = decode_frame(&frame[..written.len]).unwrap();
        assert_eq!(decoded.message_type, MessageType::RUN);
        let run_payload = decode_run_request(decoded.payload).unwrap();
        assert_eq!(run_payload.program_id, 42);
        assert_eq!(run_payload.flags, RUN_FLAG_KEEP_HANDLES_AFTER_RUN);
        assert_eq!(run_payload.instruction_budget, 777);
        assert_eq!(run_payload.time_budget_ms, 250);
    }

    #[test]
    fn writes_bootloader_reboot_frame() {
        let mut session = HostSession::with_next_request_id(42);
        let mut frame = [0u8; 32];

        let written = session.bootloader_reboot_frame(&mut frame).unwrap();
        let decoded = decode_frame(&frame[..written.len]).unwrap();

        assert_eq!(written.request_id, 42);
        assert_eq!(decoded.flags, FLAG_RESPONSE_REQUIRED);
        assert_eq!(decoded.message_type, MessageType::BOOTLOADER_REBOOT);
        assert!(decoded.payload.is_empty());
    }

    #[test]
    fn request_ids_wrap_without_using_zero() {
        let mut session = HostSession::with_next_request_id(u16::MAX);
        let mut frame = [0u8; 32];

        let first = session.stop_frame(&mut frame).unwrap();
        let second = session.stop_frame(&mut frame).unwrap();

        assert_eq!(first.request_id, u16::MAX);
        assert_eq!(second.request_id, 1);
        assert_eq!(session.next_request_id(), 2);
    }
}
