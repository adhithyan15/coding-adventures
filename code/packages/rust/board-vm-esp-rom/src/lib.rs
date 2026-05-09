use std::fmt;
use std::io::{Read, Write};
use std::time::Duration;

use coding_adventures_md5::sum_md5;

pub const DEFAULT_BAUD_RATE: u32 = 115_200;
pub const DEFAULT_TIMEOUT_MS: u64 = 1_000;
pub const ESP_CHECKSUM_SEED: u8 = 0xEF;
pub const CHIP_DETECT_MAGIC_REG_ADDR: u32 = 0x4000_1000;
pub const DEFAULT_FLASH_BLOCK_SIZE: u32 = 0x400;
pub const DEFAULT_SPI_FLASH_BLOCK_SIZE: u32 = 0x1_0000;
pub const DEFAULT_SPI_FLASH_SECTOR_SIZE: u32 = 0x1000;
pub const DEFAULT_SPI_FLASH_PAGE_SIZE: u32 = 0x100;
pub const DEFAULT_SPI_FLASH_STATUS_MASK: u32 = 0xFFFF;
pub const ESP_IMAGE_MAGIC: u8 = 0xE9;
pub const ESP_IMAGE_HEADER_LEN: usize = 24;
pub const ESP_IMAGE_SEGMENT_HEADER_LEN: usize = 8;
pub const ESP_IMAGE_CHECKSUM_ALIGN: usize = 16;
pub const ESP_IMAGE_DEFAULT_WP_PIN: u8 = 0xEE;

pub const SLIP_END: u8 = 0xC0;
pub const SLIP_ESC: u8 = 0xDB;
pub const SLIP_ESC_END: u8 = 0xDC;
pub const SLIP_ESC_ESC: u8 = 0xDD;

const REQUEST_DIRECTION: u8 = 0x00;
const RESPONSE_DIRECTION: u8 = 0x01;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EspRomError {
    PayloadTooLarge(usize),
    OutputTooSmall {
        needed: usize,
        available: usize,
    },
    TruncatedPacket,
    InvalidDirection(u8),
    InvalidSlipEscape(u8),
    MissingSlipEnd,
    UnexpectedCommand {
        expected: u8,
        actual: u8,
    },
    RomStatus {
        status: u8,
        error: u8,
    },
    UnsupportedChipId(u32),
    UnsupportedMagicValue(u32),
    InvalidFlashBlockSize(u32),
    InvalidMd5Digest(usize),
    Md5Mismatch {
        expected: [u8; 16],
        actual: [u8; 16],
    },
    TooManyImageSegments(usize),
    Io(String),
    Serial(String),
}

impl fmt::Display for EspRomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PayloadTooLarge(len) => write!(f, "payload too large for ESP ROM packet: {len}"),
            Self::OutputTooSmall { needed, available } => {
                write!(
                    f,
                    "output buffer too small: needed {needed}, available {available}"
                )
            }
            Self::TruncatedPacket => write!(f, "truncated ESP ROM packet"),
            Self::InvalidDirection(direction) => {
                write!(f, "invalid ESP ROM packet direction: 0x{direction:02X}")
            }
            Self::InvalidSlipEscape(byte) => write!(f, "invalid SLIP escape byte: 0x{byte:02X}"),
            Self::MissingSlipEnd => write!(f, "missing SLIP frame terminator"),
            Self::UnexpectedCommand { expected, actual } => write!(
                f,
                "unexpected ESP ROM response command: expected 0x{expected:02X}, got 0x{actual:02X}"
            ),
            Self::RomStatus { status, error } => {
                write!(
                    f,
                    "ESP ROM command failed: status={status} error=0x{error:02X}"
                )
            }
            Self::UnsupportedChipId(chip_id) => {
                write!(f, "unsupported ESP chip ID: {chip_id}")
            }
            Self::UnsupportedMagicValue(value) => {
                write!(f, "unsupported ESP magic value: 0x{value:08X}")
            }
            Self::InvalidFlashBlockSize(size) => {
                write!(f, "invalid ESP flash block size: {size}")
            }
            Self::InvalidMd5Digest(len) => {
                write!(f, "invalid ESP flash MD5 digest payload length: {len}")
            }
            Self::Md5Mismatch { .. } => write!(f, "ESP flash MD5 verification failed"),
            Self::TooManyImageSegments(count) => {
                write!(f, "ESP image has too many segments: {count}")
            }
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Serial(error) => write!(f, "serial error: {error}"),
        }
    }
}

impl std::error::Error for EspRomError {}

impl From<std::io::Error> for EspRomError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl From<serialport::Error> for EspRomError {
    fn from(value: serialport::Error) -> Self {
        Self::Serial(value.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EspRomCommand {
    FlashBegin = 0x02,
    FlashData = 0x03,
    FlashEnd = 0x04,
    MemBegin = 0x05,
    MemEnd = 0x06,
    MemData = 0x07,
    Sync = 0x08,
    WriteReg = 0x09,
    ReadReg = 0x0A,
    SpiSetParams = 0x0B,
    SpiAttach = 0x0D,
    ChangeBaudrate = 0x0F,
    FlashDeflBegin = 0x10,
    FlashDeflData = 0x11,
    FlashDeflEnd = 0x12,
    SpiFlashMd5 = 0x13,
    GetSecurityInfo = 0x14,
}

impl EspRomCommand {
    pub const fn code(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionSet {
    Xtensa,
    RiscV,
}

impl InstructionSet {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Xtensa => "xtensa",
            Self::RiscV => "risc-v",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EspChip {
    Esp32,
    Esp32S2,
    Esp32S3,
    Esp32C2,
    Esp32C3,
    Esp32C5,
    Esp32C6,
    Esp32H2,
    Esp32P4,
}

impl EspChip {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Esp32 => "ESP32",
            Self::Esp32S2 => "ESP32-S2",
            Self::Esp32S3 => "ESP32-S3",
            Self::Esp32C2 => "ESP32-C2",
            Self::Esp32C3 => "ESP32-C3",
            Self::Esp32C5 => "ESP32-C5",
            Self::Esp32C6 => "ESP32-C6",
            Self::Esp32H2 => "ESP32-H2",
            Self::Esp32P4 => "ESP32-P4",
        }
    }

    pub const fn image_chip_id(self) -> u32 {
        match self {
            Self::Esp32 => 0,
            Self::Esp32S2 => 2,
            Self::Esp32C3 => 5,
            Self::Esp32S3 => 9,
            Self::Esp32C2 => 12,
            Self::Esp32C6 => 13,
            Self::Esp32H2 => 16,
            Self::Esp32P4 => 18,
            Self::Esp32C5 => 23,
        }
    }

    pub const fn magic_value(self) -> Option<u32> {
        match self {
            Self::Esp32 => Some(0x00F0_1D83),
            Self::Esp32S2 => Some(0x0000_07C6),
            _ => None,
        }
    }

    pub const fn instruction_set(self) -> InstructionSet {
        match self {
            Self::Esp32 | Self::Esp32S2 | Self::Esp32S3 => InstructionSet::Xtensa,
            Self::Esp32C2
            | Self::Esp32C3
            | Self::Esp32C5
            | Self::Esp32C6
            | Self::Esp32H2
            | Self::Esp32P4 => InstructionSet::RiscV,
        }
    }

    pub const fn rust_target_hint(self) -> &'static str {
        match self {
            Self::Esp32 => "xtensa-esp32-none-elf",
            Self::Esp32S2 => "xtensa-esp32s2-none-elf",
            Self::Esp32S3 => "xtensa-esp32s3-none-elf",
            Self::Esp32C2
            | Self::Esp32C3
            | Self::Esp32C5
            | Self::Esp32C6
            | Self::Esp32H2
            | Self::Esp32P4 => "riscv32imc-unknown-none-elf",
        }
    }

    pub const fn from_image_chip_id(chip_id: u32) -> Option<Self> {
        match chip_id {
            0 => Some(Self::Esp32),
            2 => Some(Self::Esp32S2),
            5 => Some(Self::Esp32C3),
            9 => Some(Self::Esp32S3),
            12 => Some(Self::Esp32C2),
            13 => Some(Self::Esp32C6),
            16 => Some(Self::Esp32H2),
            18 => Some(Self::Esp32P4),
            23 => Some(Self::Esp32C5),
            _ => None,
        }
    }

    pub const fn from_magic_value(value: u32) -> Option<Self> {
        match value {
            0x00F0_1D83 => Some(Self::Esp32),
            0x0000_07C6 => Some(Self::Esp32S2),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityInfo {
    pub flags: u32,
    pub flash_crypt_cnt: u8,
    pub key_purposes: [u8; 7],
    pub chip_id: Option<u32>,
    pub api_version: Option<u32>,
}

impl SecurityInfo {
    pub fn parse(payload: &[u8]) -> Result<Self, EspRomError> {
        if payload.len() != 12 && payload.len() != 20 {
            return Err(EspRomError::TruncatedPacket);
        }
        let flags = read_u32_le(payload, 0)?;
        let flash_crypt_cnt = payload[4];
        let mut key_purposes = [0u8; 7];
        key_purposes.copy_from_slice(&payload[5..12]);
        let (chip_id, api_version) = if payload.len() == 20 {
            (
                Some(read_u32_le(payload, 12)?),
                Some(read_u32_le(payload, 16)?),
            )
        } else {
            (None, None)
        };
        Ok(Self {
            flags,
            flash_crypt_cnt,
            key_purposes,
            chip_id,
            api_version,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EspDetection {
    pub chip: EspChip,
    pub instruction_set: InstructionSet,
    pub rust_target_hint: &'static str,
    pub chip_id: Option<u32>,
    pub magic_value: Option<u32>,
    pub api_version: Option<u32>,
}

impl EspDetection {
    pub fn from_chip_id(chip_id: u32, api_version: Option<u32>) -> Result<Self, EspRomError> {
        let chip =
            EspChip::from_image_chip_id(chip_id).ok_or(EspRomError::UnsupportedChipId(chip_id))?;
        Ok(Self::new(chip, Some(chip_id), None, api_version))
    }

    pub fn from_magic_value(magic_value: u32) -> Result<Self, EspRomError> {
        let chip = EspChip::from_magic_value(magic_value)
            .ok_or(EspRomError::UnsupportedMagicValue(magic_value))?;
        Ok(Self::new(chip, None, Some(magic_value), None))
    }

    fn new(
        chip: EspChip,
        chip_id: Option<u32>,
        magic_value: Option<u32>,
        api_version: Option<u32>,
    ) -> Self {
        Self {
            chip,
            instruction_set: chip.instruction_set(),
            rust_target_hint: chip.rust_target_hint(),
            chip_id,
            magic_value,
            api_version,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EspRomResponse {
    pub command: u8,
    pub value: u32,
    pub data: Vec<u8>,
}

impl EspRomResponse {
    pub fn parse(frame: &[u8]) -> Result<Self, EspRomError> {
        if frame.len() < 8 {
            return Err(EspRomError::TruncatedPacket);
        }
        if frame[0] != RESPONSE_DIRECTION {
            return Err(EspRomError::InvalidDirection(frame[0]));
        }
        let size = u16::from_le_bytes([frame[2], frame[3]]) as usize;
        let needed = 8 + size;
        if frame.len() < needed {
            return Err(EspRomError::TruncatedPacket);
        }
        Ok(Self {
            command: frame[1],
            value: u32::from_le_bytes([frame[4], frame[5], frame[6], frame[7]]),
            data: frame[8..needed].to_vec(),
        })
    }

    pub fn expect_command(self, command: EspRomCommand) -> Result<Self, EspRomError> {
        let expected = command.code();
        if self.command != expected {
            return Err(EspRomError::UnexpectedCommand {
                expected,
                actual: self.command,
            });
        }
        Ok(self)
    }

    pub fn check_status(&self) -> Result<(), EspRomError> {
        let Some((status, error)) = self.status_bytes() else {
            return Ok(());
        };
        if status == 0 {
            Ok(())
        } else {
            Err(EspRomError::RomStatus { status, error })
        }
    }

    pub fn payload_without_status(&self) -> &[u8] {
        match self.status_len() {
            Some(len) if self.data.len() >= len => &self.data[..self.data.len() - len],
            _ => &self.data,
        }
    }

    fn status_bytes(&self) -> Option<(u8, u8)> {
        let len = self.status_len()?;
        Some((
            self.data[self.data.len() - len],
            self.data[self.data.len() - len + 1],
        ))
    }

    fn status_len(&self) -> Option<usize> {
        match self.data.len() {
            0 | 1 => None,
            4 if self.data[2] == 0 && self.data[3] == 0 => Some(4),
            _ => Some(2),
        }
    }
}

pub fn checksum(data: &[u8]) -> u8 {
    data.iter()
        .fold(ESP_CHECKSUM_SEED, |state, byte| state ^ byte)
}

pub fn sync_payload() -> [u8; 36] {
    let mut payload = [0x55; 36];
    payload[0..4].copy_from_slice(&[0x07, 0x07, 0x12, 0x20]);
    payload
}

pub fn read_reg_payload(address: u32) -> [u8; 4] {
    address.to_le_bytes()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EspImageSpiMode {
    Qio = 0x00,
    Qout = 0x01,
    Dio = 0x02,
    Dout = 0x03,
}

impl EspImageSpiMode {
    pub const fn code(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EspImageFlashFrequency {
    Mhz40 = 0x00,
    Mhz26 = 0x01,
    Mhz20 = 0x02,
    Mhz80 = 0x0F,
}

impl EspImageFlashFrequency {
    pub const fn code(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EspImageFlashSize {
    Mb1 = 0x00,
    Mb2 = 0x01,
    Mb4 = 0x02,
    Mb8 = 0x03,
    Mb16 = 0x04,
    Mb32 = 0x05,
    Mb64 = 0x06,
    Mb128 = 0x07,
}

impl EspImageFlashSize {
    pub const fn code(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EspImageFlashConfig {
    pub mode: EspImageSpiMode,
    pub frequency: EspImageFlashFrequency,
    pub size: EspImageFlashSize,
}

impl EspImageFlashConfig {
    pub const fn dio_40mhz_4mb() -> Self {
        Self {
            mode: EspImageSpiMode::Dio,
            frequency: EspImageFlashFrequency::Mhz40,
            size: EspImageFlashSize::Mb4,
        }
    }

    pub const fn size_frequency(self) -> u8 {
        (self.size.code() << 4) | self.frequency.code()
    }
}

impl Default for EspImageFlashConfig {
    fn default() -> Self {
        Self::dio_40mhz_4mb()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EspImageBuildOptions {
    pub chip: EspChip,
    pub entry_addr: u32,
    pub flash_config: EspImageFlashConfig,
    pub wp_pin: u8,
    pub spi_pin_drives: [u8; 3],
    pub min_chip_revision: u16,
    pub max_chip_revision: u16,
    pub hash_appended: bool,
}

impl EspImageBuildOptions {
    pub const fn new(chip: EspChip, entry_addr: u32) -> Self {
        Self {
            chip,
            entry_addr,
            flash_config: EspImageFlashConfig::dio_40mhz_4mb(),
            wp_pin: ESP_IMAGE_DEFAULT_WP_PIN,
            spi_pin_drives: [0; 3],
            min_chip_revision: 0,
            max_chip_revision: u16::MAX,
            hash_appended: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EspImageSegment<'a> {
    pub load_addr: u32,
    pub data: &'a [u8],
}

impl<'a> EspImageSegment<'a> {
    pub const fn new(load_addr: u32, data: &'a [u8]) -> Self {
        Self { load_addr, data }
    }
}

pub fn esp_image_checksum(segments: &[EspImageSegment<'_>]) -> u8 {
    segments.iter().fold(ESP_CHECKSUM_SEED, |state, segment| {
        segment.data.iter().fold(state, |state, byte| state ^ byte)
    })
}

pub fn esp_image_len(segments: &[EspImageSegment<'_>]) -> Result<usize, EspRomError> {
    validate_image_segments(segments)?;
    let mut raw_len = ESP_IMAGE_HEADER_LEN;
    for segment in segments {
        raw_len = checked_len_add(raw_len, ESP_IMAGE_SEGMENT_HEADER_LEN)?;
        raw_len = checked_len_add(raw_len, segment.data.len())?;
    }
    Ok(raw_len + esp_image_checksum_padding(raw_len) + 1)
}

pub fn build_esp_image(
    options: EspImageBuildOptions,
    segments: &[EspImageSegment<'_>],
    out: &mut [u8],
) -> Result<usize, EspRomError> {
    let needed = esp_image_len(segments)?;
    if out.len() < needed {
        return Err(EspRomError::OutputTooSmall {
            needed,
            available: out.len(),
        });
    }

    out[..needed].fill(0);
    out[0] = ESP_IMAGE_MAGIC;
    out[1] = segments.len() as u8;
    out[2] = options.flash_config.mode.code();
    out[3] = options.flash_config.size_frequency();
    write_u32_le(out, 4, options.entry_addr);
    out[8] = options.wp_pin;
    out[9..12].copy_from_slice(&options.spi_pin_drives);
    out[12..14].copy_from_slice(&(options.chip.image_chip_id() as u16).to_le_bytes());
    out[14] = options.min_chip_revision.min(u8::MAX as u16) as u8;
    out[15..17].copy_from_slice(&options.min_chip_revision.to_le_bytes());
    out[17..19].copy_from_slice(&options.max_chip_revision.to_le_bytes());
    out[23] = u8::from(options.hash_appended);

    let mut index = ESP_IMAGE_HEADER_LEN;
    for segment in segments {
        write_u32_le(out, index, segment.load_addr);
        write_u32_le(out, index + 4, segment.data.len() as u32);
        index += ESP_IMAGE_SEGMENT_HEADER_LEN;
        out[index..index + segment.data.len()].copy_from_slice(segment.data);
        index += segment.data.len();
    }
    index += esp_image_checksum_padding(index);
    out[index] = esp_image_checksum(segments);
    Ok(needed)
}

pub fn flash_begin_params_for_image(
    offset: u32,
    image: &[u8],
    block_size: u32,
) -> Result<FlashBeginParams, EspRomError> {
    if image.len() > u32::MAX as usize {
        return Err(EspRomError::PayloadTooLarge(image.len()));
    }
    FlashBeginParams::for_region(offset, image.len() as u32, block_size)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EspFlashImageUploadOptions {
    pub offset: u32,
    pub block_size: u32,
    pub spi_connection: Option<u32>,
    pub flash_params: Option<SpiFlashParams>,
    pub stay_in_bootloader: bool,
    pub verify_md5: bool,
    pub pad_byte: u8,
}

impl EspFlashImageUploadOptions {
    pub const fn new(offset: u32) -> Self {
        Self {
            offset,
            block_size: DEFAULT_FLASH_BLOCK_SIZE,
            spi_connection: Some(0),
            flash_params: None,
            stay_in_bootloader: false,
            verify_md5: true,
            pad_byte: 0xFF,
        }
    }

    pub const fn block_size(mut self, block_size: u32) -> Self {
        self.block_size = block_size;
        self
    }

    pub const fn spi_connection(mut self, spi_connection: Option<u32>) -> Self {
        self.spi_connection = spi_connection;
        self
    }

    pub const fn flash_params(mut self, flash_params: Option<SpiFlashParams>) -> Self {
        self.flash_params = flash_params;
        self
    }

    pub const fn stay_in_bootloader(mut self, stay_in_bootloader: bool) -> Self {
        self.stay_in_bootloader = stay_in_bootloader;
        self
    }

    pub const fn verify_md5(mut self, verify_md5: bool) -> Self {
        self.verify_md5 = verify_md5;
        self
    }

    pub const fn pad_byte(mut self, pad_byte: u8) -> Self {
        self.pad_byte = pad_byte;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EspFlashImageUploadReport {
    pub offset: u32,
    pub image_size: u32,
    pub block_size: u32,
    pub block_count: u32,
    pub written_size: u64,
    pub md5_digest: Option<[u8; 16]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiFlashParams {
    pub flash_id: u32,
    pub total_size: u32,
    pub block_size: u32,
    pub sector_size: u32,
    pub page_size: u32,
    pub status_mask: u32,
}

impl SpiFlashParams {
    pub const fn default_for_size(total_size: u32) -> Self {
        Self {
            flash_id: 0,
            total_size,
            block_size: DEFAULT_SPI_FLASH_BLOCK_SIZE,
            sector_size: DEFAULT_SPI_FLASH_SECTOR_SIZE,
            page_size: DEFAULT_SPI_FLASH_PAGE_SIZE,
            status_mask: DEFAULT_SPI_FLASH_STATUS_MASK,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlashBeginParams {
    pub erase_size: u32,
    pub block_count: u32,
    pub block_size: u32,
    pub offset: u32,
}

impl FlashBeginParams {
    pub fn for_region(offset: u32, size: u32, block_size: u32) -> Result<Self, EspRomError> {
        if block_size == 0 {
            return Err(EspRomError::InvalidFlashBlockSize(block_size));
        }
        Ok(Self {
            erase_size: size,
            block_count: ceil_div_u32(size, block_size),
            block_size,
            offset,
        })
    }
}

pub fn spi_attach_payload(spi_connection: u32) -> [u8; 4] {
    spi_connection.to_le_bytes()
}

pub fn spi_set_params_payload(params: SpiFlashParams) -> [u8; 24] {
    let mut payload = [0u8; 24];
    write_u32_le(&mut payload, 0, params.flash_id);
    write_u32_le(&mut payload, 4, params.total_size);
    write_u32_le(&mut payload, 8, params.block_size);
    write_u32_le(&mut payload, 12, params.sector_size);
    write_u32_le(&mut payload, 16, params.page_size);
    write_u32_le(&mut payload, 20, params.status_mask);
    payload
}

pub fn flash_begin_payload(params: FlashBeginParams) -> [u8; 16] {
    let mut payload = [0u8; 16];
    write_u32_le(&mut payload, 0, params.erase_size);
    write_u32_le(&mut payload, 4, params.block_count);
    write_u32_le(&mut payload, 8, params.block_size);
    write_u32_le(&mut payload, 12, params.offset);
    payload
}

pub fn flash_data_payload(
    sequence: u32,
    data: &[u8],
    out: &mut [u8],
) -> Result<usize, EspRomError> {
    let needed = 16 + data.len();
    if data.len() > u32::MAX as usize {
        return Err(EspRomError::PayloadTooLarge(data.len()));
    }
    if out.len() < needed {
        return Err(EspRomError::OutputTooSmall {
            needed,
            available: out.len(),
        });
    }
    write_u32_le(out, 0, data.len() as u32);
    write_u32_le(out, 4, sequence);
    write_u32_le(out, 8, 0);
    write_u32_le(out, 12, 0);
    out[16..needed].copy_from_slice(data);
    Ok(needed)
}

pub fn flash_end_payload(stay_in_bootloader: bool) -> [u8; 4] {
    u32::from(stay_in_bootloader).to_le_bytes()
}

pub fn spi_flash_md5_payload(address: u32, size: u32) -> [u8; 16] {
    let mut payload = [0u8; 16];
    write_u32_le(&mut payload, 0, address);
    write_u32_le(&mut payload, 4, size);
    write_u32_le(&mut payload, 8, 0);
    write_u32_le(&mut payload, 12, 0);
    payload
}

pub fn parse_spi_flash_md5_payload(payload: &[u8]) -> Result<[u8; 16], EspRomError> {
    if payload.len() >= 32 && payload[..32].iter().all(|byte| byte.is_ascii_hexdigit()) {
        let mut digest = [0u8; 16];
        for (index, slot) in digest.iter_mut().enumerate() {
            let high = hex_nibble(payload[index * 2])
                .ok_or(EspRomError::InvalidMd5Digest(payload.len()))?;
            let low = hex_nibble(payload[index * 2 + 1])
                .ok_or(EspRomError::InvalidMd5Digest(payload.len()))?;
            *slot = (high << 4) | low;
        }
        return Ok(digest);
    }
    if payload.len() >= 16 {
        let mut digest = [0u8; 16];
        digest.copy_from_slice(&payload[..16]);
        return Ok(digest);
    }
    Err(EspRomError::InvalidMd5Digest(payload.len()))
}

pub fn command_packet(
    command: EspRomCommand,
    payload: &[u8],
    checksum: u8,
    out: &mut [u8],
) -> Result<usize, EspRomError> {
    if payload.len() > u16::MAX as usize {
        return Err(EspRomError::PayloadTooLarge(payload.len()));
    }
    let needed = 8 + payload.len();
    if out.len() < needed {
        return Err(EspRomError::OutputTooSmall {
            needed,
            available: out.len(),
        });
    }
    out[0] = REQUEST_DIRECTION;
    out[1] = command.code();
    out[2..4].copy_from_slice(&(payload.len() as u16).to_le_bytes());
    out[4..8].copy_from_slice(&(checksum as u32).to_le_bytes());
    out[8..needed].copy_from_slice(payload);
    Ok(needed)
}

pub fn slip_encode(payload: &[u8], out: &mut [u8]) -> Result<usize, EspRomError> {
    let mut needed = 2;
    for byte in payload {
        needed += match *byte {
            SLIP_END | SLIP_ESC => 2,
            _ => 1,
        };
    }
    if out.len() < needed {
        return Err(EspRomError::OutputTooSmall {
            needed,
            available: out.len(),
        });
    }
    let mut index = 0;
    out[index] = SLIP_END;
    index += 1;
    for byte in payload {
        match *byte {
            SLIP_END => {
                out[index] = SLIP_ESC;
                out[index + 1] = SLIP_ESC_END;
                index += 2;
            }
            SLIP_ESC => {
                out[index] = SLIP_ESC;
                out[index + 1] = SLIP_ESC_ESC;
                index += 2;
            }
            byte => {
                out[index] = byte;
                index += 1;
            }
        }
    }
    out[index] = SLIP_END;
    Ok(index + 1)
}

pub fn slip_decode(wire: &[u8], out: &mut [u8]) -> Result<usize, EspRomError> {
    let start = wire
        .iter()
        .position(|byte| *byte == SLIP_END)
        .ok_or(EspRomError::MissingSlipEnd)?;
    let mut index = 0;
    let mut escaped = false;
    for byte in &wire[start + 1..] {
        if escaped {
            let decoded = match *byte {
                SLIP_ESC_END => SLIP_END,
                SLIP_ESC_ESC => SLIP_ESC,
                other => return Err(EspRomError::InvalidSlipEscape(other)),
            };
            if index >= out.len() {
                return Err(EspRomError::OutputTooSmall {
                    needed: index + 1,
                    available: out.len(),
                });
            }
            out[index] = decoded;
            index += 1;
            escaped = false;
            continue;
        }
        match *byte {
            SLIP_END => return Ok(index),
            SLIP_ESC => escaped = true,
            byte => {
                if index >= out.len() {
                    return Err(EspRomError::OutputTooSmall {
                        needed: index + 1,
                        available: out.len(),
                    });
                }
                out[index] = byte;
                index += 1;
            }
        }
    }
    Err(EspRomError::MissingSlipEnd)
}

pub struct EspRomSession<S> {
    stream: S,
    frame: [u8; 8192],
    wire: [u8; 16384],
}

impl<S> EspRomSession<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            frame: [0; 8192],
            wire: [0; 16384],
        }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S> EspRomSession<S>
where
    S: Read + Write,
{
    pub fn sync(&mut self) -> Result<(), EspRomError> {
        let response = self.exchange(EspRomCommand::Sync, &sync_payload(), 0)?;
        response.check_status()
    }

    pub fn read_reg(&mut self, address: u32) -> Result<u32, EspRomError> {
        let response = self.exchange(EspRomCommand::ReadReg, &read_reg_payload(address), 0)?;
        response.check_status()?;
        Ok(response.value)
    }

    pub fn get_security_info(&mut self) -> Result<SecurityInfo, EspRomError> {
        let response = self.exchange(EspRomCommand::GetSecurityInfo, &[], 0)?;
        response.check_status()?;
        SecurityInfo::parse(response.payload_without_status()).or_else(|_| {
            if response.data.len() >= 20 {
                SecurityInfo::parse(&response.data[..20])
            } else if response.data.len() >= 12 {
                SecurityInfo::parse(&response.data[..12])
            } else {
                Err(EspRomError::TruncatedPacket)
            }
        })
    }

    pub fn detect_chip(&mut self) -> Result<EspDetection, EspRomError> {
        self.sync()?;
        if let Ok(info) = self.get_security_info() {
            if let Some(chip_id) = info.chip_id {
                return EspDetection::from_chip_id(chip_id, info.api_version);
            }
        }
        let magic_value = self.read_reg(CHIP_DETECT_MAGIC_REG_ADDR)?;
        EspDetection::from_magic_value(magic_value)
    }

    pub fn spi_attach(&mut self, spi_connection: u32) -> Result<(), EspRomError> {
        let response = self.exchange(
            EspRomCommand::SpiAttach,
            &spi_attach_payload(spi_connection),
            0,
        )?;
        response.check_status()
    }

    pub fn spi_set_params(&mut self, params: SpiFlashParams) -> Result<(), EspRomError> {
        let payload = spi_set_params_payload(params);
        let response = self.exchange(EspRomCommand::SpiSetParams, &payload, 0)?;
        response.check_status()
    }

    pub fn flash_begin(&mut self, params: FlashBeginParams) -> Result<(), EspRomError> {
        let payload = flash_begin_payload(params);
        let response = self.exchange(EspRomCommand::FlashBegin, &payload, 0)?;
        response.check_status()
    }

    pub fn flash_data(&mut self, sequence: u32, data: &[u8]) -> Result<(), EspRomError> {
        let payload_len = flash_data_payload(sequence, data, &mut self.frame)?;
        let payload = self.frame[..payload_len].to_vec();
        let response = self.exchange(EspRomCommand::FlashData, &payload, checksum(data))?;
        response.check_status()
    }

    pub fn flash_end(&mut self, stay_in_bootloader: bool) -> Result<(), EspRomError> {
        let payload = flash_end_payload(stay_in_bootloader);
        let response = self.exchange(EspRomCommand::FlashEnd, &payload, 0)?;
        response.check_status()
    }

    pub fn spi_flash_md5(&mut self, address: u32, size: u32) -> Result<[u8; 16], EspRomError> {
        let payload = spi_flash_md5_payload(address, size);
        let response = self.exchange(EspRomCommand::SpiFlashMd5, &payload, 0)?;
        response.check_status()?;
        parse_spi_flash_md5_payload(response.payload_without_status())
    }

    pub fn upload_flash_image(
        &mut self,
        image: &[u8],
        options: EspFlashImageUploadOptions,
    ) -> Result<EspFlashImageUploadReport, EspRomError> {
        if let Some(spi_connection) = options.spi_connection {
            self.spi_attach(spi_connection)?;
        }
        if let Some(flash_params) = options.flash_params {
            self.spi_set_params(flash_params)?;
        }

        let begin = flash_begin_params_for_image(options.offset, image, options.block_size)?;
        self.flash_begin(begin)?;

        let block_size = options.block_size as usize;
        let mut padded_block = Vec::new();
        for sequence in 0..begin.block_count {
            let start = sequence as usize * block_size;
            let end = image.len().min(start + block_size);
            let chunk = &image[start..end];
            if chunk.len() == block_size {
                self.flash_data(sequence, chunk)?;
            } else {
                padded_block.clear();
                padded_block.extend_from_slice(chunk);
                padded_block.resize(block_size, options.pad_byte);
                self.flash_data(sequence, &padded_block)?;
            }
        }

        let md5_digest = if options.verify_md5 {
            let actual = self.spi_flash_md5(options.offset, image.len() as u32)?;
            let expected = sum_md5(image);
            if actual != expected {
                return Err(EspRomError::Md5Mismatch { expected, actual });
            }
            Some(actual)
        } else {
            None
        };

        self.flash_end(options.stay_in_bootloader)?;
        Ok(EspFlashImageUploadReport {
            offset: options.offset,
            image_size: image.len() as u32,
            block_size: options.block_size,
            block_count: begin.block_count,
            written_size: begin.block_count as u64 * options.block_size as u64,
            md5_digest,
        })
    }

    fn exchange(
        &mut self,
        command: EspRomCommand,
        payload: &[u8],
        checksum: u8,
    ) -> Result<EspRomResponse, EspRomError> {
        let packet_len = command_packet(command, payload, checksum, &mut self.frame)?;
        let wire_len = slip_encode(&self.frame[..packet_len], &mut self.wire)?;
        self.stream.write_all(&self.wire[..wire_len])?;
        self.stream.flush()?;
        loop {
            let frame_len = read_slip_frame(&mut self.stream, &mut self.frame)?;
            let response = EspRomResponse::parse(&self.frame[..frame_len])?;
            if response.command == command.code() {
                return Ok(response);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EspRomSerialOptions {
    pub port: String,
    pub baud_rate: u32,
    pub timeout: Duration,
    pub reset_into_bootloader: bool,
}

impl EspRomSerialOptions {
    pub fn new(port: impl Into<String>) -> Self {
        Self {
            port: port.into(),
            baud_rate: DEFAULT_BAUD_RATE,
            timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
            reset_into_bootloader: true,
        }
    }

    pub fn baud_rate(mut self, baud_rate: u32) -> Self {
        self.baud_rate = baud_rate;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn reset_into_bootloader(mut self, reset_into_bootloader: bool) -> Self {
        self.reset_into_bootloader = reset_into_bootloader;
        self
    }
}

pub fn detect_esp_rom(options: &EspRomSerialOptions) -> Result<EspDetection, EspRomError> {
    let mut port = serialport::new(&options.port, options.baud_rate)
        .timeout(options.timeout)
        .open()?;
    if options.reset_into_bootloader {
        enter_uart_bootloader(&mut *port)?;
    }
    EspRomSession::new(port).detect_chip()
}

pub fn enter_uart_bootloader(port: &mut dyn serialport::SerialPort) -> Result<(), EspRomError> {
    port.write_data_terminal_ready(false)?;
    port.write_request_to_send(true)?;
    std::thread::sleep(Duration::from_millis(100));
    port.write_data_terminal_ready(true)?;
    port.write_request_to_send(false)?;
    std::thread::sleep(Duration::from_millis(50));
    port.write_data_terminal_ready(false)?;
    std::thread::sleep(Duration::from_millis(100));
    Ok(())
}

fn read_slip_frame<R: Read>(reader: &mut R, out: &mut [u8]) -> Result<usize, EspRomError> {
    let mut started = false;
    let mut escaped = false;
    let mut index = 0;
    let mut byte = [0u8; 1];
    loop {
        reader.read_exact(&mut byte)?;
        let byte = byte[0];
        if !started {
            if byte == SLIP_END {
                started = true;
            }
            continue;
        }
        if escaped {
            let decoded = match byte {
                SLIP_ESC_END => SLIP_END,
                SLIP_ESC_ESC => SLIP_ESC,
                other => return Err(EspRomError::InvalidSlipEscape(other)),
            };
            if index >= out.len() {
                return Err(EspRomError::OutputTooSmall {
                    needed: index + 1,
                    available: out.len(),
                });
            }
            out[index] = decoded;
            index += 1;
            escaped = false;
            continue;
        }
        match byte {
            SLIP_END if index == 0 => continue,
            SLIP_END => return Ok(index),
            SLIP_ESC => escaped = true,
            byte => {
                if index >= out.len() {
                    return Err(EspRomError::OutputTooSmall {
                        needed: index + 1,
                        available: out.len(),
                    });
                }
                out[index] = byte;
                index += 1;
            }
        }
    }
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, EspRomError> {
    let Some(bytes) = bytes.get(offset..offset + 4) else {
        return Err(EspRomError::TruncatedPacket);
    };
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn write_u32_le(out: &mut [u8], offset: usize, value: u32) {
    out[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn ceil_div_u32(value: u32, divisor: u32) -> u32 {
    value / divisor + u32::from(value % divisor != 0)
}

fn validate_image_segments(segments: &[EspImageSegment<'_>]) -> Result<(), EspRomError> {
    if segments.len() > u8::MAX as usize {
        return Err(EspRomError::TooManyImageSegments(segments.len()));
    }
    for segment in segments {
        if segment.data.len() > u32::MAX as usize {
            return Err(EspRomError::PayloadTooLarge(segment.data.len()));
        }
    }
    Ok(())
}

fn checked_len_add(lhs: usize, rhs: usize) -> Result<usize, EspRomError> {
    lhs.checked_add(rhs)
        .ok_or(EspRomError::PayloadTooLarge(lhs))
}

fn esp_image_checksum_padding(raw_len: usize) -> usize {
    (ESP_IMAGE_CHECKSUM_ALIGN - 1 - (raw_len % ESP_IMAGE_CHECKSUM_ALIGN)) % ESP_IMAGE_CHECKSUM_ALIGN
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::io;

    #[derive(Default)]
    struct ScriptedStream {
        read: VecDeque<u8>,
        written: Vec<u8>,
    }

    impl ScriptedStream {
        fn with_read(read: &[u8]) -> Self {
            Self {
                read: read.iter().copied().collect(),
                written: Vec::new(),
            }
        }
    }

    impl Read for ScriptedStream {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            if self.read.is_empty() {
                return Ok(0);
            }
            out[0] = self.read.pop_front().unwrap();
            Ok(1)
        }
    }

    impl Write for ScriptedStream {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.written.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn rom_response(command: EspRomCommand, value: u32, data: &[u8]) -> Vec<u8> {
        let mut frame = vec![0u8; 8 + data.len()];
        frame[0] = RESPONSE_DIRECTION;
        frame[1] = command.code();
        frame[2..4].copy_from_slice(&(data.len() as u16).to_le_bytes());
        frame[4..8].copy_from_slice(&value.to_le_bytes());
        frame[8..].copy_from_slice(data);
        let mut wire = vec![0u8; frame.len() * 2 + 2];
        let wire_len = slip_encode(&frame, &mut wire).unwrap();
        wire.truncate(wire_len);
        wire
    }

    fn decode_written_frames(wire: &[u8]) -> Vec<Vec<u8>> {
        let mut cursor = io::Cursor::new(wire);
        let mut frames = Vec::new();
        loop {
            let mut frame = [0u8; 2048];
            match read_slip_frame(&mut cursor, &mut frame) {
                Ok(frame_len) => frames.push(frame[..frame_len].to_vec()),
                Err(_) => return frames,
            }
        }
    }

    #[test]
    fn slip_round_trips_escaped_bytes() {
        let payload = [0x00, SLIP_END, SLIP_ESC, 0x55];
        let mut encoded = [0u8; 16];
        let encoded_len = slip_encode(&payload, &mut encoded).unwrap();
        assert_eq!(
            &encoded[..encoded_len],
            &[
                SLIP_END,
                0x00,
                SLIP_ESC,
                SLIP_ESC_END,
                SLIP_ESC,
                SLIP_ESC_ESC,
                0x55,
                SLIP_END
            ]
        );

        let mut decoded = [0u8; 8];
        let decoded_len = slip_decode(&encoded[..encoded_len], &mut decoded).unwrap();
        assert_eq!(&decoded[..decoded_len], payload);
    }

    #[test]
    fn sync_command_matches_rom_trace_shape() {
        let mut packet = [0u8; 64];
        let payload = sync_payload();
        let packet_len = command_packet(EspRomCommand::Sync, &payload, 0, &mut packet).unwrap();

        assert_eq!(&packet[..8], &[0x00, 0x08, 0x24, 0x00, 0, 0, 0, 0]);
        assert_eq!(&packet[8..12], &[0x07, 0x07, 0x12, 0x20]);
        assert!(packet[12..packet_len].iter().all(|byte| *byte == 0x55));
    }

    #[test]
    fn flash_payload_builders_match_rom_protocol() {
        assert_eq!(spi_attach_payload(0), [0, 0, 0, 0]);

        let params = SpiFlashParams::default_for_size(4 * 1024 * 1024);
        assert_eq!(
            spi_set_params_payload(params),
            [0, 0, 0, 0, 0, 0, 0x40, 0, 0, 0, 1, 0, 0, 0x10, 0, 0, 0, 1, 0, 0, 0xFF, 0xFF, 0, 0,]
        );

        let begin = FlashBeginParams::for_region(0x10000, 1500, DEFAULT_FLASH_BLOCK_SIZE).unwrap();
        assert_eq!(begin.block_count, 2);
        assert_eq!(
            flash_begin_payload(begin),
            [0xDC, 0x05, 0, 0, 2, 0, 0, 0, 0, 4, 0, 0, 0, 0, 1, 0,]
        );

        let data = [0xC0, 0xDB, 0x34];
        let mut payload = [0u8; 32];
        let len = flash_data_payload(7, &data, &mut payload).unwrap();
        assert_eq!(
            &payload[..16],
            &[3, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(&payload[16..len], &data);
        assert_eq!(checksum(&data), ESP_CHECKSUM_SEED ^ 0xC0 ^ 0xDB ^ 0x34);

        assert_eq!(flash_end_payload(false), [0, 0, 0, 0]);
        assert_eq!(flash_end_payload(true), [1, 0, 0, 0]);
        assert_eq!(
            spi_flash_md5_payload(0x10000, 0x2000),
            [0, 0, 1, 0, 0, 0x20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            parse_spi_flash_md5_payload(b"00112233445566778899aabbccddeeff").unwrap(),
            [
                0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
                0xEE, 0xFF,
            ]
        );
    }

    #[test]
    fn esp_image_builder_writes_header_segments_padding_and_checksum() {
        let code = [0xC0, 0xDB, 0x34];
        let rodata = [0xAA];
        let segments = [
            EspImageSegment::new(0x4038_0000, &code),
            EspImageSegment::new(0x3FC8_0000, &rodata),
        ];
        let options = EspImageBuildOptions::new(EspChip::Esp32C3, 0x4038_0000);
        let mut image = [0xFF; 64];

        let len = build_esp_image(options, &segments, &mut image).unwrap();

        assert_eq!(len, 48);
        assert_eq!(esp_image_len(&segments).unwrap(), len);
        assert_eq!(len % ESP_IMAGE_CHECKSUM_ALIGN, 0);
        assert_eq!(
            &image[..ESP_IMAGE_HEADER_LEN],
            &[
                ESP_IMAGE_MAGIC,
                2,
                EspImageSpiMode::Dio.code(),
                EspImageFlashConfig::dio_40mhz_4mb().size_frequency(),
                0x00,
                0x00,
                0x38,
                0x40,
                ESP_IMAGE_DEFAULT_WP_PIN,
                0,
                0,
                0,
                5,
                0,
                0,
                0,
                0,
                0xFF,
                0xFF,
                0,
                0,
                0,
                0,
                0,
            ]
        );
        assert_eq!(
            &image[ESP_IMAGE_HEADER_LEN..ESP_IMAGE_HEADER_LEN + ESP_IMAGE_SEGMENT_HEADER_LEN],
            &[0x00, 0x00, 0x38, 0x40, 3, 0, 0, 0]
        );
        assert_eq!(&image[32..35], &code);
        assert_eq!(&image[35..43], &[0x00, 0x00, 0xC8, 0x3F, 1, 0, 0, 0]);
        assert_eq!(&image[43..44], &rodata);
        assert_eq!(&image[44..47], &[0, 0, 0]);
        assert_eq!(image[47], ESP_CHECKSUM_SEED ^ 0xC0 ^ 0xDB ^ 0x34 ^ 0xAA);

        let flash =
            flash_begin_params_for_image(0x1000, &image[..len], DEFAULT_SPI_FLASH_BLOCK_SIZE)
                .unwrap();
        assert_eq!(flash.erase_size, len as u32);
        assert_eq!(flash.block_count, 1);
        assert_eq!(flash.offset, 0x1000);
    }

    #[test]
    fn esp_image_builder_reports_buffer_and_segment_limits() {
        let data = [1, 2, 3, 4];
        let segments = [EspImageSegment::new(0x4008_0000, &data)];
        let mut image = [0u8; 16];

        assert_eq!(
            build_esp_image(
                EspImageBuildOptions::new(EspChip::Esp32, 0x4008_0000),
                &segments,
                &mut image,
            ),
            Err(EspRomError::OutputTooSmall {
                needed: 48,
                available: 16,
            })
        );

        let many_data = [0u8; 256];
        let many: Vec<_> = many_data
            .iter()
            .map(|byte| EspImageSegment::new(0, std::slice::from_ref(byte)))
            .collect();
        assert_eq!(
            esp_image_len(&many),
            Err(EspRomError::TooManyImageSegments(256))
        );
    }

    #[test]
    fn parses_read_reg_response_value_and_rom_status() {
        let frame = [
            0x01, 0x0A, 0x04, 0x00, 0x83, 0x1D, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let response = EspRomResponse::parse(&frame)
            .unwrap()
            .expect_command(EspRomCommand::ReadReg)
            .unwrap();

        assert_eq!(response.value, 0x00F0_1D83);
        response.check_status().unwrap();
        assert_eq!(response.payload_without_status(), &[]);
    }

    #[test]
    fn security_info_parses_chip_id_and_api_version() {
        let payload = [
            0x04, 0x00, 0x00, 0x00, 0x03, 1, 2, 3, 4, 5, 6, 7, 0x05, 0x00, 0x00, 0x00, 0x02, 0x00,
            0x00, 0x00,
        ];
        let info = SecurityInfo::parse(&payload).unwrap();

        assert_eq!(info.flags, 4);
        assert_eq!(info.flash_crypt_cnt, 3);
        assert_eq!(info.key_purposes, [1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(info.chip_id, Some(5));
        assert_eq!(info.api_version, Some(2));
    }

    #[test]
    fn chip_mapping_selects_instruction_sets() {
        let c3 = EspDetection::from_chip_id(5, Some(2)).unwrap();
        assert_eq!(c3.chip, EspChip::Esp32C3);
        assert_eq!(c3.instruction_set, InstructionSet::RiscV);
        assert_eq!(c3.rust_target_hint, "riscv32imc-unknown-none-elf");

        let s3 = EspDetection::from_chip_id(9, Some(2)).unwrap();
        assert_eq!(s3.chip, EspChip::Esp32S3);
        assert_eq!(s3.instruction_set, InstructionSet::Xtensa);
        assert_eq!(s3.rust_target_hint, "xtensa-esp32s3-none-elf");

        let legacy = EspDetection::from_magic_value(0x00F0_1D83).unwrap();
        assert_eq!(legacy.chip, EspChip::Esp32);
        assert_eq!(legacy.instruction_set, InstructionSet::Xtensa);
    }

    #[test]
    fn session_detects_chip_from_security_info() {
        let mut sync_response = [0u8; 12];
        sync_response[0..8].copy_from_slice(&[0x01, 0x08, 0x04, 0x00, 0x07, 0x07, 0x12, 0x20]);
        let mut security_response = [0u8; 30];
        security_response[0..8].copy_from_slice(&[0x01, 0x14, 0x16, 0x00, 0, 0, 0, 0]);
        security_response[8..28]
            .copy_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 13, 0, 0, 0, 2, 0, 0, 0]);
        security_response[28..30].copy_from_slice(&[0, 0]);

        let mut wire = [0u8; 96];
        let sync_wire_len = slip_encode(&sync_response, &mut wire).unwrap();
        let security_start = sync_wire_len;
        let security_wire_len =
            slip_encode(&security_response, &mut wire[security_start..]).unwrap();
        let read = &wire[..security_start + security_wire_len];
        let stream = ScriptedStream::with_read(read);

        let mut session = EspRomSession::new(stream);
        let detection = session.detect_chip().unwrap();

        assert_eq!(detection.chip, EspChip::Esp32C6);
        assert_eq!(detection.instruction_set, InstructionSet::RiscV);
        let stream = session.into_inner();
        assert!(stream.written.starts_with(&[SLIP_END, 0x00, 0x08]));
        assert!(stream
            .written
            .windows(3)
            .any(|window| window == [SLIP_END, 0x00, 0x14]));
    }

    #[test]
    fn session_writes_flash_command_sequence() {
        let data = [1, 2, 3, 4];
        let md5 = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
            0xEE, 0xFF,
        ];
        let mut md5_response = md5.to_vec();
        md5_response.extend_from_slice(&[0, 0]);
        let mut read = Vec::new();
        for command in [
            EspRomCommand::SpiAttach,
            EspRomCommand::SpiSetParams,
            EspRomCommand::FlashBegin,
            EspRomCommand::FlashData,
            EspRomCommand::FlashEnd,
        ] {
            read.extend_from_slice(&rom_response(command, 0, &[0, 0]));
        }
        read.extend_from_slice(&rom_response(EspRomCommand::SpiFlashMd5, 0, &md5_response));
        let stream = ScriptedStream::with_read(&read);

        let mut session = EspRomSession::new(stream);
        session.spi_attach(0).unwrap();
        session
            .spi_set_params(SpiFlashParams::default_for_size(4 * 1024 * 1024))
            .unwrap();
        session
            .flash_begin(FlashBeginParams::for_region(0x10000, data.len() as u32, 1024).unwrap())
            .unwrap();
        session.flash_data(0, &data).unwrap();
        session.flash_end(false).unwrap();
        assert_eq!(
            session.spi_flash_md5(0x10000, data.len() as u32).unwrap(),
            md5
        );

        let frames = decode_written_frames(&session.into_inner().written);
        let commands: Vec<u8> = frames.iter().map(|frame| frame[1]).collect();
        assert_eq!(commands, vec![0x0D, 0x0B, 0x02, 0x03, 0x04, 0x13]);
        let flash_data = &frames[3];
        assert_eq!(&flash_data[0..4], &[0x00, 0x03, 0x14, 0x00]);
        assert_eq!(flash_data[4], checksum(&data));
        assert_eq!(
            &flash_data[8..24],
            &[4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(&flash_data[24..28], &data);
    }

    #[test]
    fn session_uploads_flash_image_with_padding_and_md5_verify() {
        let image: Vec<u8> = (0..1500).map(|index| (index % 251) as u8).collect();
        let md5 = sum_md5(&image);
        let mut md5_response = md5.to_vec();
        md5_response.extend_from_slice(&[0, 0]);
        let mut read = Vec::new();
        for command in [
            EspRomCommand::SpiAttach,
            EspRomCommand::SpiSetParams,
            EspRomCommand::FlashBegin,
            EspRomCommand::FlashData,
            EspRomCommand::FlashData,
        ] {
            read.extend_from_slice(&rom_response(command, 0, &[0, 0]));
        }
        read.extend_from_slice(&rom_response(EspRomCommand::SpiFlashMd5, 0, &md5_response));
        read.extend_from_slice(&rom_response(EspRomCommand::FlashEnd, 0, &[0, 0]));
        let stream = ScriptedStream::with_read(&read);

        let options = EspFlashImageUploadOptions::new(0x1000)
            .block_size(1024)
            .flash_params(Some(SpiFlashParams::default_for_size(4 * 1024 * 1024)));
        let mut session = EspRomSession::new(stream);
        let report = session.upload_flash_image(&image, options).unwrap();

        assert_eq!(
            report,
            EspFlashImageUploadReport {
                offset: 0x1000,
                image_size: 1500,
                block_size: 1024,
                block_count: 2,
                written_size: 2048,
                md5_digest: Some(md5),
            }
        );

        let frames = decode_written_frames(&session.into_inner().written);
        let commands: Vec<u8> = frames.iter().map(|frame| frame[1]).collect();
        assert_eq!(commands, vec![0x0D, 0x0B, 0x02, 0x03, 0x03, 0x13, 0x04]);

        let begin = &frames[2];
        assert_eq!(
            &begin[8..24],
            &[0xDC, 0x05, 0, 0, 2, 0, 0, 0, 0, 4, 0, 0, 0, 0x10, 0, 0]
        );

        let first_data = &frames[3];
        assert_eq!(&first_data[8..12], &[0, 4, 0, 0]);
        assert_eq!(&first_data[12..16], &[0, 0, 0, 0]);
        assert_eq!(&first_data[24..24 + 1024], &image[..1024]);

        let second_data = &frames[4];
        assert_eq!(&second_data[8..12], &[0, 4, 0, 0]);
        assert_eq!(&second_data[12..16], &[1, 0, 0, 0]);
        assert_eq!(&second_data[24..24 + 476], &image[1024..]);
        assert!(second_data[24 + 476..24 + 1024]
            .iter()
            .all(|byte| *byte == 0xFF));

        let md5_frame = &frames[5];
        assert_eq!(
            &md5_frame[8..24],
            &[0, 0x10, 0, 0, 0xDC, 0x05, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        let end = &frames[6];
        assert_eq!(&end[8..12], &[0, 0, 0, 0]);
    }

    #[test]
    fn session_upload_flash_image_rejects_md5_mismatch_before_boot() {
        let image = [1, 2, 3, 4];
        let mut read = Vec::new();
        for command in [
            EspRomCommand::SpiAttach,
            EspRomCommand::FlashBegin,
            EspRomCommand::FlashData,
        ] {
            read.extend_from_slice(&rom_response(command, 0, &[0, 0]));
        }
        let mut wrong_md5 = [0u8; 18];
        wrong_md5[16..18].copy_from_slice(&[0, 0]);
        read.extend_from_slice(&rom_response(EspRomCommand::SpiFlashMd5, 0, &wrong_md5));
        let stream = ScriptedStream::with_read(&read);

        let mut session = EspRomSession::new(stream);
        let error = session
            .upload_flash_image(
                &image,
                EspFlashImageUploadOptions::new(0x1000).block_size(1024),
            )
            .unwrap_err();

        assert_eq!(
            error,
            EspRomError::Md5Mismatch {
                expected: sum_md5(&image),
                actual: [0; 16],
            }
        );
        let frames = decode_written_frames(&session.into_inner().written);
        let commands: Vec<u8> = frames.iter().map(|frame| frame[1]).collect();
        assert_eq!(commands, vec![0x0D, 0x02, 0x03, 0x13]);
    }
}
