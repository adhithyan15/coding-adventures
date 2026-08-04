// board_vm_protocol.hpp — a host<->board VM wire protocol codec (header-only, ISO C++17).
// ---------------------------------------------------------------------------
//
// A faithful C++ port of the Rust `board-vm-protocol` crate, in namespace
// `ca::board_vm_protocol`.  It defines the framing and message payloads a host
// uses to talk to a tiny "board VM" (a microcontroller running a bytecode
// interpreter) over a byte stream such as a serial line.
//
// Three layers stack up:
//   1. Message payloads  — encode_hello / decode_hello, etc.
//   2. Frames            — encode_frame wraps a payload with a version byte,
//      flags, a message-type tag, a request id, a ULEB128 length, and a
//      trailing CRC-16/CCITT-FALSE.
//   3. Wire frames       — encode_wire_frame COBS-encodes a raw frame and
//      appends a 0x00 terminator so frames can be delimited on a raw stream.
//
// Errors.  Where the Rust crate returns `Result<_, ProtocolError>`, this port
// throws a `ProtocolError` exception carrying an `Error` code.
//
// Buffers.  Encoders return `std::vector<std::uint8_t>` (they grow as needed,
// so the encode path cannot report OutputTooSmall).  Decoders take a byte view
// and return borrowed views (`std::string_view` for strings, `ByteView` for
// raw bytes) that point INTO the caller's buffer — valid only while it lives.
#ifndef CA_BOARD_VM_PROTOCOL_HPP
#define CA_BOARD_VM_PROTOCOL_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <exception>
#include <string_view>
#include <utility>
#include <vector>

namespace ca {
namespace board_vm_protocol {

// A non-owning view over raw bytes (there is no std::span in C++17, and
// std::basic_string_view<uint8_t> relies on the non-standard, deprecated
// char_traits<unsigned char>, so we roll a tiny purpose-built view instead).
class ByteView {
public:
    constexpr ByteView() noexcept : data_(nullptr), size_(0) {}
    constexpr ByteView(const std::uint8_t* data, std::size_t size) noexcept
        : data_(data), size_(size) {}

    constexpr const std::uint8_t* data() const noexcept { return data_; }
    constexpr std::size_t size() const noexcept { return size_; }
    constexpr bool empty() const noexcept { return size_ == 0; }
    constexpr std::uint8_t operator[](std::size_t i) const noexcept { return data_[i]; }
    std::uint8_t back() const noexcept { return data_[size_ - 1]; }

    bool operator==(const ByteView& o) const {
        if (size_ != o.size_) {
            return false;
        }
        for (std::size_t i = 0; i < size_; ++i) {
            if (data_[i] != o.data_[i]) {
                return false;
            }
        }
        return true;
    }
    bool operator!=(const ByteView& o) const { return !(*this == o); }

private:
    const std::uint8_t* data_;
    std::size_t size_;
};

// ------------------------------------------------------------------ //
// Protocol constants                                                 //
// ------------------------------------------------------------------ //

inline constexpr std::uint8_t PROTOCOL_VERSION = 1;
inline constexpr std::size_t FRAME_CRC_BYTES = 2;

inline constexpr std::uint8_t FLAG_RESPONSE_REQUIRED = 0x01;
inline constexpr std::uint8_t FLAG_IS_RESPONSE = 0x02;
inline constexpr std::uint8_t FLAG_IS_ERROR_RESPONSE = 0x04;
inline constexpr std::uint8_t FLAG_COMPRESSED_PAYLOAD = 0x08;
inline constexpr std::uint8_t ALLOWED_V1_FLAGS =
    FLAG_RESPONSE_REQUIRED | FLAG_IS_RESPONSE | FLAG_IS_ERROR_RESPONSE;

inline constexpr std::uint16_t CAP_PROGRAM_RAM_EXEC = 0x7001;
inline constexpr std::uint16_t CAP_PROGRAM_STORE = 0x7002;
inline constexpr std::uint16_t CAP_TRANSPORT_PIPELINING = 0x7003;

inline constexpr std::uint16_t CAP_FLAG_BYTECODE_CALLABLE = 0x01;
inline constexpr std::uint16_t CAP_FLAG_PROTOCOL_FEATURE = 0x02;
inline constexpr std::uint16_t CAP_FLAG_BOARD_METADATA = 0x04;

inline constexpr std::uint8_t RUN_FLAG_RESET_VM_BEFORE_RUN = 0x01;
inline constexpr std::uint8_t RUN_FLAG_KEEP_HANDLES_AFTER_RUN = 0x02;
inline constexpr std::uint8_t RUN_FLAG_BACKGROUND_RUN = 0x04;
inline constexpr std::uint8_t ALLOWED_RUN_FLAGS =
    RUN_FLAG_RESET_VM_BEFORE_RUN | RUN_FLAG_KEEP_HANDLES_AFTER_RUN | RUN_FLAG_BACKGROUND_RUN;

inline constexpr std::uint8_t BOOT_STORE_ONLY = 0x00;
inline constexpr std::uint8_t BOOT_RUN_AT_BOOT = 0x01;
inline constexpr std::uint8_t BOOT_RUN_IF_NO_HOST = 0x02;

inline constexpr std::uint16_t NO_PROGRAM_ID = 0xFFFF;
inline constexpr std::uint32_t NO_BYTECODE_OFFSET = 0xFFFFFFFF;

// Message type tags.
enum class MessageType : std::uint8_t {
    Hello = 0x01,
    HelloAck = 0x02,
    CapsQuery = 0x03,
    CapsReport = 0x04,
    ProgramBegin = 0x05,
    ProgramChunk = 0x06,
    ProgramEnd = 0x07,
    Run = 0x08,
    RunReport = 0x09,
    Stop = 0x0A,
    ResetVm = 0x0B,
    StoreProgram = 0x0C,
    RunStored = 0x0D,
    ReadState = 0x0E,
    StateReport = 0x0F,
    Subscribe = 0x10,
    Event = 0x11,
    Log = 0x12,
    Error = 0x13,
    Ping = 0x14,
    Pong = 0x15,
    BootloaderReboot = 0x16
};

// Message type tags >= 0x80 are reserved for vendor extensions.
inline bool is_vendor_extension(MessageType t) {
    return static_cast<std::uint8_t>(t) >= 0x80;
}

// Program format (wire byte).
enum class ProgramFormat : std::uint8_t { BvmModule = 0x01 };

// Run status (wire byte).
enum class RunStatus : std::uint8_t {
    Halted = 0x00,
    Running = 0x01,
    Stopped = 0x02,
    BudgetExceeded = 0x03,
    Faulted = 0x04
};

// ------------------------------------------------------------------ //
// Errors                                                             //
// ------------------------------------------------------------------ //

enum class Error {
    OutputTooSmall,
    InputTooShort,
    MissingTerminator,
    InvalidCobs,
    TruncatedUleb,
    UlebOverflow,
    PayloadTooLarge,
    PayloadLengthMismatch,
    BadCrc,
    UnsupportedVersion,
    ReservedFlags,
    UnsupportedValue,
    InvalidBool,
    InvalidUtf8,
    TrailingBytes
};

// Thrown by every fallible routine; mirrors the Rust crate's ProtocolError.
class ProtocolError : public std::exception {
public:
    explicit ProtocolError(Error code) noexcept : code_(code) {}
    Error code() const noexcept { return code_; }
    const char* what() const noexcept override { return "board_vm_protocol error"; }

private:
    Error code_;
};

// ------------------------------------------------------------------ //
// Golden test vectors                                                //
// ------------------------------------------------------------------ //

inline constexpr std::array<std::uint8_t, 10> GOLDEN_HELLO_PAYLOAD_BVM_V1 = {
    0x01, 0x01, 0x03, 'b', 'v', 'm', 0xCD, 0xAB, 0x34, 0x12};
inline constexpr std::array<std::uint8_t, 18> GOLDEN_HELLO_RAW_FRAME_BVM_V1 = {
    0x01, 0x01, 0x01, 0x34, 0x12, 0x0A, 0x01, 0x01, 0x03, 'b', 'v', 'm',
    0xCD, 0xAB, 0x34, 0x12, 0x19, 0x49};
inline constexpr std::array<std::uint8_t, 20> GOLDEN_HELLO_WIRE_FRAME_BVM_V1 = {
    0x13, 0x01, 0x01, 0x01, 0x34, 0x12, 0x0A, 0x01, 0x01, 0x03, 'b', 'v', 'm',
    0xCD, 0xAB, 0x34, 0x12, 0x19, 0x49, 0x00};
inline constexpr std::array<std::uint8_t, 11> GOLDEN_PROGRAM_BEGIN_PAYLOAD_BVM_V1 = {
    0x01, 0x00, 0x01, 0x24, 0x00, 0x00, 0x00, 0xBE, 0xBA, 0xFE, 0xCA};
inline constexpr std::array<std::uint8_t, 11> GOLDEN_RUN_BACKGROUND_PAYLOAD_BVM_V1 = {
    0x01, 0x00, 0x05, 0xE8, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00};

// ------------------------------------------------------------------ //
// Message structs                                                    //
// ------------------------------------------------------------------ //

struct Frame {
    std::uint8_t flags = 0;
    MessageType message_type = MessageType::Hello;
    std::uint16_t request_id = 0;
    ByteView payload{};

    bool operator==(const Frame& o) const {
        return flags == o.flags && message_type == o.message_type &&
               request_id == o.request_id && payload == o.payload;
    }
    bool operator!=(const Frame& o) const { return !(*this == o); }
};

struct Hello {
    std::uint8_t min_version = 0;
    std::uint8_t max_version = 0;
    std::string_view host_name{};
    std::uint32_t host_nonce = 0;

    bool operator==(const Hello& o) const {
        return min_version == o.min_version && max_version == o.max_version &&
               host_name == o.host_name && host_nonce == o.host_nonce;
    }
};

struct HelloAck {
    std::uint8_t selected_version = 0;
    std::string_view board_name{};
    std::string_view runtime_name{};
    std::uint32_t host_nonce = 0;
    std::uint32_t board_nonce = 0;
    std::uint16_t max_frame_payload = 0;

    bool operator==(const HelloAck& o) const {
        return selected_version == o.selected_version && board_name == o.board_name &&
               runtime_name == o.runtime_name && host_nonce == o.host_nonce &&
               board_nonce == o.board_nonce && max_frame_payload == o.max_frame_payload;
    }
};

struct CapabilityDescriptor {
    std::uint16_t id = 0;
    std::uint8_t version = 0;
    std::uint16_t flags = 0;
    std::string_view name{};

    bool operator==(const CapabilityDescriptor& o) const {
        return id == o.id && version == o.version && flags == o.flags && name == o.name;
    }
};

struct CapsReportHeader {
    std::string_view board_id{};
    std::string_view runtime_id{};
    std::uint32_t max_program_bytes = 0;
    std::uint8_t max_stack_values = 0;
    std::uint8_t max_handles = 0;
    bool supports_store_program = false;
    std::uint32_t capability_count = 0;

    bool operator==(const CapsReportHeader& o) const {
        return board_id == o.board_id && runtime_id == o.runtime_id &&
               max_program_bytes == o.max_program_bytes &&
               max_stack_values == o.max_stack_values && max_handles == o.max_handles &&
               supports_store_program == o.supports_store_program &&
               capability_count == o.capability_count;
    }
};

struct ProgramBegin {
    std::uint16_t program_id = 0;
    ProgramFormat format = ProgramFormat::BvmModule;
    std::uint32_t total_len = 0;
    std::uint32_t program_crc32 = 0;

    bool operator==(const ProgramBegin& o) const {
        return program_id == o.program_id && format == o.format &&
               total_len == o.total_len && program_crc32 == o.program_crc32;
    }
};

struct ProgramChunk {
    std::uint16_t program_id = 0;
    std::uint32_t offset = 0;
    ByteView bytes{};

    bool operator==(const ProgramChunk& o) const {
        return program_id == o.program_id && offset == o.offset && bytes == o.bytes;
    }
};

struct ProgramEnd {
    std::uint16_t program_id = 0;
    bool operator==(const ProgramEnd& o) const { return program_id == o.program_id; }
};

struct RunRequest {
    std::uint16_t program_id = 0;
    std::uint8_t flags = 0;
    std::uint32_t instruction_budget = 0;
    std::uint32_t time_budget_ms = 0;

    bool operator==(const RunRequest& o) const {
        return program_id == o.program_id && flags == o.flags &&
               instruction_budget == o.instruction_budget &&
               time_budget_ms == o.time_budget_ms;
    }
};

struct RunReportHeader {
    std::uint16_t program_id = 0;
    RunStatus status = RunStatus::Halted;
    std::uint32_t instructions_executed = 0;
    std::uint32_t elapsed_ms = 0;
    std::uint8_t stack_depth = 0;
    std::uint8_t open_handles = 0;
    std::uint32_t return_count = 0;

    bool operator==(const RunReportHeader& o) const {
        return program_id == o.program_id && status == o.status &&
               instructions_executed == o.instructions_executed &&
               elapsed_ms == o.elapsed_ms && stack_depth == o.stack_depth &&
               open_handles == o.open_handles && return_count == o.return_count;
    }
};

struct StoreProgram {
    std::uint16_t program_id = 0;
    std::uint8_t slot = 0;
    std::uint8_t boot_policy = 0;

    bool operator==(const StoreProgram& o) const {
        return program_id == o.program_id && slot == o.slot && boot_policy == o.boot_policy;
    }
};

struct ErrorPayload {
    std::uint16_t code = 0;
    std::uint16_t request_id = 0;
    std::uint16_t program_id = 0;
    std::uint32_t bytecode_offset = 0;
    std::string_view message{};

    bool operator==(const ErrorPayload& o) const {
        return code == o.code && request_id == o.request_id &&
               program_id == o.program_id && bytecode_offset == o.bytecode_offset &&
               message == o.message;
    }
};

struct Ping {
    std::uint32_t nonce = 0;
    bool operator==(const Ping& o) const { return nonce == o.nonce; }
};

struct Pong {
    std::uint32_t nonce = 0;
    bool operator==(const Pong& o) const { return nonce == o.nonce; }
};

// Tagged VM value.  Handle and U16 share the same wire scalar but are distinct
// variants, so a plain std::variant<uint16_t, ...> would be ambiguous; a tagged
// struct keeps them apart while remaining trivially comparable.
struct Value {
    enum class Tag { Unit, Bool, U8, U16, U32, I16, Handle, Bytes, String };

    Tag tag = Tag::Unit;
    bool boolean = false;
    std::uint8_t u8 = 0;
    std::uint16_t u16 = 0;
    std::uint32_t u32 = 0;
    std::int16_t i16 = 0;
    std::uint16_t handle = 0;
    ByteView bytes{};
    std::string_view str{};

    static Value unit() { return Value{}; }
    static Value make_bool(bool b) { Value v; v.tag = Tag::Bool; v.boolean = b; return v; }
    static Value make_u8(std::uint8_t x) { Value v; v.tag = Tag::U8; v.u8 = x; return v; }
    static Value make_u16(std::uint16_t x) { Value v; v.tag = Tag::U16; v.u16 = x; return v; }
    static Value make_u32(std::uint32_t x) { Value v; v.tag = Tag::U32; v.u32 = x; return v; }
    static Value make_i16(std::int16_t x) { Value v; v.tag = Tag::I16; v.i16 = x; return v; }
    static Value make_handle(std::uint16_t x) { Value v; v.tag = Tag::Handle; v.handle = x; return v; }
    static Value make_bytes(ByteView b) { Value v; v.tag = Tag::Bytes; v.bytes = b; return v; }
    static Value make_string(std::string_view s) { Value v; v.tag = Tag::String; v.str = s; return v; }

    bool operator==(const Value& o) const {
        if (tag != o.tag) {
            return false;
        }
        switch (tag) {
        case Tag::Unit: return true;
        case Tag::Bool: return boolean == o.boolean;
        case Tag::U8: return u8 == o.u8;
        case Tag::U16: return u16 == o.u16;
        case Tag::U32: return u32 == o.u32;
        case Tag::I16: return i16 == o.i16;
        case Tag::Handle: return handle == o.handle;
        case Tag::Bytes: return bytes == o.bytes;
        case Tag::String: return str == o.str;
        }
        return false;
    }
};

// ------------------------------------------------------------------ //
// UTF-8 validation (mirrors Rust's str::from_utf8 acceptance)        //
// ------------------------------------------------------------------ //

namespace detail {

inline bool is_utf8(const std::uint8_t* s, std::size_t len) {
    std::size_t i = 0;
    while (i < len) {
        std::uint8_t b0 = s[i];
        if (b0 < 0x80) {
            i += 1;
        } else if ((b0 & 0xE0) == 0xC0) {
            if (b0 < 0xC2) {
                return false;
            }
            if (i + 1 >= len || (s[i + 1] & 0xC0) != 0x80) {
                return false;
            }
            i += 2;
        } else if ((b0 & 0xF0) == 0xE0) {
            if (i + 2 >= len) {
                return false;
            }
            std::uint8_t b1 = s[i + 1];
            if ((b1 & 0xC0) != 0x80 || (s[i + 2] & 0xC0) != 0x80) {
                return false;
            }
            if (b0 == 0xE0 && b1 < 0xA0) {
                return false; // overlong
            }
            if (b0 == 0xED && b1 >= 0xA0) {
                return false; // surrogate
            }
            i += 3;
        } else if ((b0 & 0xF8) == 0xF0) {
            if (b0 > 0xF4) {
                return false;
            }
            if (i + 3 >= len) {
                return false;
            }
            std::uint8_t b1 = s[i + 1];
            if ((b1 & 0xC0) != 0x80 || (s[i + 2] & 0xC0) != 0x80 ||
                (s[i + 3] & 0xC0) != 0x80) {
                return false;
            }
            if (b0 == 0xF0 && b1 < 0x90) {
                return false; // overlong
            }
            if (b0 == 0xF4 && b1 >= 0x90) {
                return false; // > U+10FFFF
            }
            i += 4;
        } else {
            return false;
        }
    }
    return true;
}

[[noreturn]] inline void fail(Error e) { throw ProtocolError(e); }

} // namespace detail

// ------------------------------------------------------------------ //
// Encoder                                                            //
// ------------------------------------------------------------------ //

// Appends encoded bytes to a caller-owned vector; grows as needed.
class Encoder {
public:
    explicit Encoder(std::vector<std::uint8_t>& out) : out_(out) {}

    std::size_t len() const { return out_.size(); }
    bool is_empty() const { return out_.empty(); }

    void write_slice(const std::uint8_t* value, std::size_t len) {
        out_.insert(out_.end(), value, value + len);
    }
    void write_u8(std::uint8_t value) { out_.push_back(value); }
    void write_bool(bool value) { out_.push_back(value ? 1 : 0); }
    void write_u16(std::uint16_t value) {
        out_.push_back(static_cast<std::uint8_t>(value & 0xFF));
        out_.push_back(static_cast<std::uint8_t>((value >> 8) & 0xFF));
    }
    void write_u32(std::uint32_t value) {
        out_.push_back(static_cast<std::uint8_t>(value & 0xFF));
        out_.push_back(static_cast<std::uint8_t>((value >> 8) & 0xFF));
        out_.push_back(static_cast<std::uint8_t>((value >> 16) & 0xFF));
        out_.push_back(static_cast<std::uint8_t>((value >> 24) & 0xFF));
    }
    void write_i16(std::int16_t value) { write_u16(static_cast<std::uint16_t>(value)); }

    void write_uleb128(std::uint32_t value) {
        for (;;) {
            std::uint8_t byte = static_cast<std::uint8_t>(value & 0x7F);
            value >>= 7;
            if (value != 0) {
                byte |= 0x80;
            }
            out_.push_back(byte);
            if (value == 0) {
                return;
            }
        }
    }

    void write_bytes(const std::uint8_t* value, std::size_t len) {
        if (len > static_cast<std::size_t>(UINT32_MAX)) {
            detail::fail(Error::PayloadTooLarge);
        }
        write_uleb128(static_cast<std::uint32_t>(len));
        write_slice(value, len);
    }
    void write_bytes(ByteView v) { write_bytes(v.data(), v.size()); }
    void write_string(std::string_view v) {
        write_bytes(reinterpret_cast<const std::uint8_t*>(v.data()), v.size());
    }

    void write_capability_descriptor(const CapabilityDescriptor& v) {
        write_u16(v.id);
        write_u8(v.version);
        write_u16(v.flags);
        write_string(v.name);
    }

    void write_value(const Value& v) {
        switch (v.tag) {
        case Value::Tag::Unit: write_u8(0x00); break;
        case Value::Tag::Bool: write_u8(0x01); write_bool(v.boolean); break;
        case Value::Tag::U8: write_u8(0x02); write_u8(v.u8); break;
        case Value::Tag::U16: write_u8(0x03); write_u16(v.u16); break;
        case Value::Tag::U32: write_u8(0x04); write_u32(v.u32); break;
        case Value::Tag::I16: write_u8(0x05); write_i16(v.i16); break;
        case Value::Tag::Handle: write_u8(0x06); write_u16(v.handle); break;
        case Value::Tag::Bytes: write_u8(0x07); write_bytes(v.bytes); break;
        case Value::Tag::String: write_u8(0x08); write_string(v.str); break;
        }
    }

private:
    std::vector<std::uint8_t>& out_;
};

// ------------------------------------------------------------------ //
// Decoder                                                            //
// ------------------------------------------------------------------ //

class Decoder {
public:
    Decoder() : input_(nullptr), input_len_(0), offset_(0) {}
    Decoder(const std::uint8_t* input, std::size_t len)
        : input_(input), input_len_(len), offset_(0) {}
    explicit Decoder(ByteView input) : input_(input.data()), input_len_(input.size()), offset_(0) {}

    std::size_t offset() const { return offset_; }
    std::size_t remaining_len() const { return input_len_ - offset_; }

    void finish() const {
        if (offset_ != input_len_) {
            detail::fail(Error::TrailingBytes);
        }
    }

    std::uint8_t read_u8() {
        if (offset_ >= input_len_) {
            detail::fail(Error::InputTooShort);
        }
        return input_[offset_++];
    }

    bool read_bool() {
        std::uint8_t b = read_u8();
        if (b == 0) {
            return false;
        }
        if (b == 1) {
            return true;
        }
        detail::fail(Error::InvalidBool);
    }

    std::uint16_t read_u16() {
        if (offset_ > input_len_ || input_len_ - offset_ < 2) {
            detail::fail(Error::InputTooShort);
        }
        std::uint16_t v = static_cast<std::uint16_t>(
            static_cast<std::uint16_t>(input_[offset_]) |
            (static_cast<std::uint16_t>(input_[offset_ + 1]) << 8));
        offset_ += 2;
        return v;
    }

    std::uint32_t read_u32() {
        if (offset_ > input_len_ || input_len_ - offset_ < 4) {
            detail::fail(Error::InputTooShort);
        }
        std::uint32_t v = static_cast<std::uint32_t>(input_[offset_]) |
                          (static_cast<std::uint32_t>(input_[offset_ + 1]) << 8) |
                          (static_cast<std::uint32_t>(input_[offset_ + 2]) << 16) |
                          (static_cast<std::uint32_t>(input_[offset_ + 3]) << 24);
        offset_ += 4;
        return v;
    }

    std::int16_t read_i16() { return static_cast<std::int16_t>(read_u16()); }

    std::uint32_t read_uleb128() {
        std::uint32_t value = 0;
        unsigned shift = 0;
        for (;;) {
            if (shift >= 35) {
                detail::fail(Error::UlebOverflow);
            }
            std::uint8_t byte;
            if (offset_ >= input_len_) {
                detail::fail(Error::TruncatedUleb);
            }
            byte = input_[offset_++];
            std::uint32_t chunk = static_cast<std::uint32_t>(byte & 0x7F);
            if (shift == 28 && chunk > 0x0F) {
                detail::fail(Error::UlebOverflow);
            }
            value |= chunk << shift;
            if ((byte & 0x80) == 0) {
                return value;
            }
            shift += 7;
        }
    }

    ByteView read_slice(std::size_t len) {
        if (len > static_cast<std::size_t>(-1) - offset_) {
            detail::fail(Error::PayloadTooLarge);
        }
        std::size_t end = offset_ + len;
        if (end > input_len_) {
            detail::fail(Error::InputTooShort);
        }
        ByteView slice(input_ + offset_, len);
        offset_ = end;
        return slice;
    }

    ByteView read_bytes() {
        std::size_t len = static_cast<std::size_t>(read_uleb128());
        return read_slice(len);
    }

    std::string_view read_string() {
        ByteView bytes = read_bytes();
        if (!detail::is_utf8(bytes.data(), bytes.size())) {
            detail::fail(Error::InvalidUtf8);
        }
        return std::string_view(reinterpret_cast<const char*>(bytes.data()), bytes.size());
    }

    CapabilityDescriptor read_capability_descriptor() {
        CapabilityDescriptor d;
        d.id = read_u16();
        d.version = read_u8();
        d.flags = read_u16();
        d.name = read_string();
        return d;
    }

    Value read_value() {
        std::uint8_t tag = read_u8();
        switch (tag) {
        case 0x00: return Value::unit();
        case 0x01: return Value::make_bool(read_bool());
        case 0x02: return Value::make_u8(read_u8());
        case 0x03: return Value::make_u16(read_u16());
        case 0x04: return Value::make_u32(read_u32());
        case 0x05: return Value::make_i16(read_i16());
        case 0x06: return Value::make_handle(read_u16());
        case 0x07: return Value::make_bytes(read_bytes());
        case 0x08: return Value::make_string(read_string());
        default: detail::fail(Error::UnsupportedValue);
        }
    }

private:
    const std::uint8_t* input_;
    std::size_t input_len_;
    std::size_t offset_;
};

// ------------------------------------------------------------------ //
// Flag / boot-policy / enum validation                               //
// ------------------------------------------------------------------ //

namespace detail {

inline void validate_flags(std::uint8_t flags) {
    if ((flags & static_cast<std::uint8_t>(~ALLOWED_V1_FLAGS)) != 0) {
        fail(Error::ReservedFlags);
    }
}

inline void validate_boot_policy(std::uint8_t value) {
    switch (value) {
    case BOOT_STORE_ONLY:
    case BOOT_RUN_AT_BOOT:
    case BOOT_RUN_IF_NO_HOST:
        return;
    default:
        fail(Error::UnsupportedValue);
    }
}

inline ProgramFormat program_format_from_u8(std::uint8_t value) {
    if (value == static_cast<std::uint8_t>(ProgramFormat::BvmModule)) {
        return ProgramFormat::BvmModule;
    }
    fail(Error::UnsupportedValue);
}

inline RunStatus run_status_from_u8(std::uint8_t value) {
    switch (value) {
    case 0x00: return RunStatus::Halted;
    case 0x01: return RunStatus::Running;
    case 0x02: return RunStatus::Stopped;
    case 0x03: return RunStatus::BudgetExceeded;
    case 0x04: return RunStatus::Faulted;
    default: fail(Error::UnsupportedValue);
    }
}

} // namespace detail

// ------------------------------------------------------------------ //
// CRC-16 / COBS                                                      //
// ------------------------------------------------------------------ //

inline std::uint16_t crc16_ccitt_false(const std::uint8_t* bytes, std::size_t len) {
    std::uint16_t crc = 0xFFFF;
    for (std::size_t i = 0; i < len; ++i) {
        crc ^= static_cast<std::uint16_t>(static_cast<std::uint16_t>(bytes[i]) << 8);
        for (int bit = 0; bit < 8; ++bit) {
            if ((crc & 0x8000) != 0) {
                crc = static_cast<std::uint16_t>((crc << 1) ^ 0x1021);
            } else {
                crc = static_cast<std::uint16_t>(crc << 1);
            }
        }
    }
    return crc;
}
inline std::uint16_t crc16_ccitt_false(ByteView bytes) {
    return crc16_ccitt_false(bytes.data(), bytes.size());
}

// COBS encode.  The output vector grows, so (unlike the buffer-based Rust
// crate) this cannot report OutputTooSmall; the algorithm is otherwise
// identical, including the code_index backpatching.
inline std::vector<std::uint8_t> cobs_encode(const std::uint8_t* input, std::size_t input_len) {
    std::vector<std::uint8_t> out;
    out.push_back(0); // placeholder for the first code byte
    std::size_t code_index = 0;
    std::uint8_t code = 1;

    for (std::size_t ri = 0; ri < input_len; ++ri) {
        if (input[ri] == 0) {
            out[code_index] = code;
            code_index = out.size();
            out.push_back(0);
            code = 1;
        } else {
            out.push_back(input[ri]);
            code += 1;
            if (code == 0xFF) {
                out[code_index] = code;
                if (ri + 1 == input_len) {
                    return out;
                }
                code_index = out.size();
                out.push_back(0);
                code = 1;
            }
        }
    }
    out[code_index] = code;
    return out;
}
inline std::vector<std::uint8_t> cobs_encode(ByteView input) {
    return cobs_encode(input.data(), input.size());
}

inline std::vector<std::uint8_t> cobs_decode(const std::uint8_t* input, std::size_t input_len) {
    std::vector<std::uint8_t> out;
    std::size_t read_index = 0;

    while (read_index < input_len) {
        std::uint8_t code = input[read_index];
        if (code == 0) {
            detail::fail(Error::InvalidCobs);
        }
        read_index += 1;

        std::size_t span = static_cast<std::size_t>(code - 1);
        if (span > static_cast<std::size_t>(-1) - read_index) {
            detail::fail(Error::InvalidCobs);
        }
        std::size_t end = read_index + span;
        if (end > input_len) {
            detail::fail(Error::InvalidCobs);
        }
        out.insert(out.end(), input + read_index, input + end);
        read_index = end;

        if (code != 0xFF && read_index < input_len) {
            out.push_back(0);
        }
    }
    return out;
}
inline std::vector<std::uint8_t> cobs_decode(ByteView input) {
    return cobs_decode(input.data(), input.size());
}

// ------------------------------------------------------------------ //
// Frame / wire-frame codec                                           //
// ------------------------------------------------------------------ //

inline std::vector<std::uint8_t> encode_frame(const Frame& frame) {
    detail::validate_flags(frame.flags);
    if (frame.payload.size() > static_cast<std::size_t>(UINT32_MAX)) {
        detail::fail(Error::PayloadTooLarge);
    }
    std::vector<std::uint8_t> out;
    Encoder enc(out);
    enc.write_u8(PROTOCOL_VERSION);
    enc.write_u8(frame.flags);
    enc.write_u8(static_cast<std::uint8_t>(frame.message_type));
    enc.write_u16(frame.request_id);
    enc.write_uleb128(static_cast<std::uint32_t>(frame.payload.size()));
    enc.write_slice(frame.payload.data(), frame.payload.size());
    std::uint16_t crc = crc16_ccitt_false(out.data(), out.size());
    enc.write_u16(crc);
    return out;
}

inline Frame decode_frame(ByteView bytes) {
    if (bytes.size() < 8) {
        detail::fail(Error::InputTooShort);
    }
    std::size_t crc_offset = bytes.size() - FRAME_CRC_BYTES;
    std::uint16_t expected_crc = static_cast<std::uint16_t>(
        static_cast<std::uint16_t>(bytes[crc_offset]) |
        (static_cast<std::uint16_t>(bytes[crc_offset + 1]) << 8));
    std::uint16_t actual_crc = crc16_ccitt_false(bytes.data(), crc_offset);
    if (expected_crc != actual_crc) {
        detail::fail(Error::BadCrc);
    }

    Decoder dec(bytes.data(), crc_offset);
    std::uint8_t version = dec.read_u8();
    if (version != PROTOCOL_VERSION) {
        detail::fail(Error::UnsupportedVersion);
    }
    std::uint8_t flags = dec.read_u8();
    detail::validate_flags(flags);
    MessageType message_type = static_cast<MessageType>(dec.read_u8());
    std::uint16_t request_id = dec.read_u16();
    std::size_t payload_len = static_cast<std::size_t>(dec.read_uleb128());
    if (dec.remaining_len() != payload_len) {
        detail::fail(Error::PayloadLengthMismatch);
    }
    ByteView payload = dec.read_slice(payload_len);
    Frame f;
    f.flags = flags;
    f.message_type = message_type;
    f.request_id = request_id;
    f.payload = payload;
    return f;
}

inline std::vector<std::uint8_t> encode_wire_frame(ByteView raw_with_crc) {
    std::vector<std::uint8_t> out = cobs_encode(raw_with_crc);
    out.push_back(0); // terminator
    return out;
}

inline std::vector<std::uint8_t> decode_wire_frame(ByteView wire) {
    if (wire.empty()) {
        detail::fail(Error::InputTooShort);
    }
    if (wire.back() != 0) {
        detail::fail(Error::MissingTerminator);
    }
    return cobs_decode(wire.data(), wire.size() - 1);
}

// Returns the COBS-framed wire bytes.  `raw_out` receives the intermediate raw
// frame (matching the Rust two-buffer signature) for callers that want it.
inline std::vector<std::uint8_t> encode_stream_frame(const Frame& frame,
                                                     std::vector<std::uint8_t>* raw_out = nullptr) {
    std::vector<std::uint8_t> raw = encode_frame(frame);
    std::vector<std::uint8_t> wire =
        encode_wire_frame(ByteView(raw.data(), raw.size()));
    if (raw_out != nullptr) {
        *raw_out = std::move(raw);
    }
    return wire;
}

// Decodes a wire frame back into a Frame.  `raw_out` holds the de-COBSed raw
// bytes the returned Frame borrows from, so it must outlive the Frame.
inline Frame decode_stream_frame(ByteView wire, std::vector<std::uint8_t>& raw_out) {
    raw_out = decode_wire_frame(wire);
    return decode_frame(ByteView(raw_out.data(), raw_out.size()));
}

// ------------------------------------------------------------------ //
// Per-message payload codecs                                         //
// ------------------------------------------------------------------ //

inline std::vector<std::uint8_t> encode_hello(const Hello& v) {
    std::vector<std::uint8_t> out;
    Encoder enc(out);
    enc.write_u8(v.min_version);
    enc.write_u8(v.max_version);
    enc.write_string(v.host_name);
    enc.write_u32(v.host_nonce);
    return out;
}

inline Hello decode_hello(ByteView bytes) {
    Decoder dec(bytes);
    Hello v;
    v.min_version = dec.read_u8();
    v.max_version = dec.read_u8();
    v.host_name = dec.read_string();
    v.host_nonce = dec.read_u32();
    dec.finish();
    return v;
}

inline std::vector<std::uint8_t> encode_hello_ack(const HelloAck& v) {
    std::vector<std::uint8_t> out;
    Encoder enc(out);
    enc.write_u8(v.selected_version);
    enc.write_string(v.board_name);
    enc.write_string(v.runtime_name);
    enc.write_u32(v.host_nonce);
    enc.write_u32(v.board_nonce);
    enc.write_u16(v.max_frame_payload);
    return out;
}

inline HelloAck decode_hello_ack(ByteView bytes) {
    Decoder dec(bytes);
    HelloAck v;
    v.selected_version = dec.read_u8();
    v.board_name = dec.read_string();
    v.runtime_name = dec.read_string();
    v.host_nonce = dec.read_u32();
    v.board_nonce = dec.read_u32();
    v.max_frame_payload = dec.read_u16();
    dec.finish();
    return v;
}

inline std::vector<std::uint8_t> encode_capability_descriptor(const CapabilityDescriptor& v) {
    std::vector<std::uint8_t> out;
    Encoder enc(out);
    enc.write_capability_descriptor(v);
    return out;
}

inline CapabilityDescriptor decode_capability_descriptor(ByteView bytes) {
    Decoder dec(bytes);
    CapabilityDescriptor v = dec.read_capability_descriptor();
    dec.finish();
    return v;
}

inline std::vector<std::uint8_t> encode_caps_report(const CapsReportHeader& header,
                                                    const std::vector<CapabilityDescriptor>& caps) {
    if (static_cast<std::size_t>(header.capability_count) != caps.size()) {
        detail::fail(Error::PayloadLengthMismatch);
    }
    std::vector<std::uint8_t> out;
    Encoder enc(out);
    enc.write_string(header.board_id);
    enc.write_string(header.runtime_id);
    enc.write_u32(header.max_program_bytes);
    enc.write_u8(header.max_stack_values);
    enc.write_u8(header.max_handles);
    enc.write_bool(header.supports_store_program);
    enc.write_uleb128(header.capability_count);
    for (const CapabilityDescriptor& c : caps) {
        enc.write_capability_descriptor(c);
    }
    return out;
}

// Returns the header and a decoder positioned at the first capability.
inline std::pair<CapsReportHeader, Decoder> decode_caps_report_header(ByteView bytes) {
    Decoder dec(bytes);
    CapsReportHeader h;
    h.board_id = dec.read_string();
    h.runtime_id = dec.read_string();
    h.max_program_bytes = dec.read_u32();
    h.max_stack_values = dec.read_u8();
    h.max_handles = dec.read_u8();
    h.supports_store_program = dec.read_bool();
    h.capability_count = dec.read_uleb128();
    return std::make_pair(h, dec);
}

inline std::vector<std::uint8_t> encode_program_begin(const ProgramBegin& v) {
    std::vector<std::uint8_t> out;
    Encoder enc(out);
    enc.write_u16(v.program_id);
    enc.write_u8(static_cast<std::uint8_t>(v.format));
    enc.write_u32(v.total_len);
    enc.write_u32(v.program_crc32);
    return out;
}

inline ProgramBegin decode_program_begin(ByteView bytes) {
    Decoder dec(bytes);
    ProgramBegin v;
    v.program_id = dec.read_u16();
    v.format = detail::program_format_from_u8(dec.read_u8());
    v.total_len = dec.read_u32();
    v.program_crc32 = dec.read_u32();
    dec.finish();
    return v;
}

inline std::vector<std::uint8_t> encode_program_chunk(const ProgramChunk& v) {
    std::vector<std::uint8_t> out;
    Encoder enc(out);
    enc.write_u16(v.program_id);
    enc.write_u32(v.offset);
    enc.write_bytes(v.bytes);
    return out;
}

inline ProgramChunk decode_program_chunk(ByteView bytes) {
    Decoder dec(bytes);
    ProgramChunk v;
    v.program_id = dec.read_u16();
    v.offset = dec.read_u32();
    v.bytes = dec.read_bytes();
    dec.finish();
    return v;
}

inline std::vector<std::uint8_t> encode_program_end(const ProgramEnd& v) {
    std::vector<std::uint8_t> out;
    Encoder enc(out);
    enc.write_u16(v.program_id);
    return out;
}

inline ProgramEnd decode_program_end(ByteView bytes) {
    Decoder dec(bytes);
    ProgramEnd v;
    v.program_id = dec.read_u16();
    dec.finish();
    return v;
}

inline std::vector<std::uint8_t> encode_run_request(const RunRequest& v) {
    if ((v.flags & static_cast<std::uint8_t>(~ALLOWED_RUN_FLAGS)) != 0) {
        detail::fail(Error::ReservedFlags);
    }
    std::vector<std::uint8_t> out;
    Encoder enc(out);
    enc.write_u16(v.program_id);
    enc.write_u8(v.flags);
    enc.write_u32(v.instruction_budget);
    enc.write_u32(v.time_budget_ms);
    return out;
}

inline RunRequest decode_run_request(ByteView bytes) {
    Decoder dec(bytes);
    std::uint16_t program_id = dec.read_u16();
    std::uint8_t flags = dec.read_u8();
    if ((flags & static_cast<std::uint8_t>(~ALLOWED_RUN_FLAGS)) != 0) {
        detail::fail(Error::ReservedFlags);
    }
    RunRequest v;
    v.program_id = program_id;
    v.flags = flags;
    v.instruction_budget = dec.read_u32();
    v.time_budget_ms = dec.read_u32();
    dec.finish();
    return v;
}

inline std::vector<std::uint8_t> encode_run_report_header(const RunReportHeader& v) {
    std::vector<std::uint8_t> out;
    Encoder enc(out);
    enc.write_u16(v.program_id);
    enc.write_u8(static_cast<std::uint8_t>(v.status));
    enc.write_u32(v.instructions_executed);
    enc.write_u32(v.elapsed_ms);
    enc.write_u8(v.stack_depth);
    enc.write_u8(v.open_handles);
    enc.write_uleb128(v.return_count);
    return out;
}

inline std::pair<RunReportHeader, Decoder> decode_run_report_header(ByteView bytes) {
    Decoder dec(bytes);
    RunReportHeader v;
    v.program_id = dec.read_u16();
    v.status = detail::run_status_from_u8(dec.read_u8());
    v.instructions_executed = dec.read_u32();
    v.elapsed_ms = dec.read_u32();
    v.stack_depth = dec.read_u8();
    v.open_handles = dec.read_u8();
    v.return_count = dec.read_uleb128();
    return std::make_pair(v, dec);
}

inline std::vector<std::uint8_t> encode_store_program(const StoreProgram& v) {
    detail::validate_boot_policy(v.boot_policy);
    std::vector<std::uint8_t> out;
    Encoder enc(out);
    enc.write_u16(v.program_id);
    enc.write_u8(v.slot);
    enc.write_u8(v.boot_policy);
    return out;
}

inline StoreProgram decode_store_program(ByteView bytes) {
    Decoder dec(bytes);
    StoreProgram v;
    v.program_id = dec.read_u16();
    v.slot = dec.read_u8();
    v.boot_policy = dec.read_u8();
    detail::validate_boot_policy(v.boot_policy);
    dec.finish();
    return v;
}

inline std::vector<std::uint8_t> encode_error_payload(const ErrorPayload& v) {
    std::vector<std::uint8_t> out;
    Encoder enc(out);
    enc.write_u16(v.code);
    enc.write_u16(v.request_id);
    enc.write_u16(v.program_id);
    enc.write_u32(v.bytecode_offset);
    enc.write_string(v.message);
    return out;
}

inline ErrorPayload decode_error_payload(ByteView bytes) {
    Decoder dec(bytes);
    ErrorPayload v;
    v.code = dec.read_u16();
    v.request_id = dec.read_u16();
    v.program_id = dec.read_u16();
    v.bytecode_offset = dec.read_u32();
    v.message = dec.read_string();
    dec.finish();
    return v;
}

inline std::vector<std::uint8_t> encode_ping(const Ping& v) {
    std::vector<std::uint8_t> out;
    Encoder enc(out);
    enc.write_u32(v.nonce);
    return out;
}

inline Ping decode_ping(ByteView bytes) {
    Decoder dec(bytes);
    Ping v;
    v.nonce = dec.read_u32();
    dec.finish();
    return v;
}

inline std::vector<std::uint8_t> encode_pong(const Pong& v) {
    std::vector<std::uint8_t> out;
    Encoder enc(out);
    enc.write_u32(v.nonce);
    return out;
}

inline Pong decode_pong(ByteView bytes) {
    Decoder dec(bytes);
    Pong v;
    v.nonce = dec.read_u32();
    dec.finish();
    return v;
}

inline std::vector<std::uint8_t> encode_value(const Value& v) {
    std::vector<std::uint8_t> out;
    Encoder enc(out);
    enc.write_value(v);
    return out;
}

inline Value decode_value(ByteView bytes) {
    Decoder dec(bytes);
    Value v = dec.read_value();
    dec.finish();
    return v;
}

} // namespace board_vm_protocol
} // namespace ca

#endif // CA_BOARD_VM_PROTOCOL_HPP
