// zwave_core.hpp — Z-Wave identifier, region, and Serial API frame primitives.
// ===========================================================================
//
// A faithful, header-only port of the Rust `zwave-core` crate, in namespace
// `ca::zwave_core`. Not a controller: it provides a tested byte boundary for
// controller serial frames, node identity, command-class ids, and regional
// profile metadata.
//
// The two codecs parse UNTRUSTED bytes: SerialFrame::parse (SOF, length, type,
// function id, payload, XOR checksum) and CommandClassFrame::parse. Both
// bounds-check every field and throw `ca::zwave_core::Error` on bad input.
//
// Pure ISO C++17 — no <cmath>, no compiler extensions.
#ifndef ZWAVE_CORE_HPP
#define ZWAVE_CORE_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace ca {
namespace zwave_core {

// Serial control bytes.
inline constexpr std::uint8_t kSof = 0x01;
inline constexpr std::uint8_t kAck = 0x06;
inline constexpr std::uint8_t kNak = 0x15;
inline constexpr std::uint8_t kCan = 0x18;

// ── Errors ───────────────────────────────────────────────────────────────────

enum class ErrorKind {
    InvalidClassicNodeId,     // a = value
    InvalidLongRangeNodeId,   // a = value
    MissingStartOfFrame,      // a = byte
    InvalidLength,            // a = length
    InvalidFrameType,         // a = byte
    Truncated,                // a = needed, b = remaining
    PayloadTooLong,           // a = len
    CommandPayloadTooLong,    // a = len
    ChecksumMismatch          // a = expected, b = actual
};

class Error : public std::runtime_error {
   public:
    Error(ErrorKind kind, const std::string &msg, std::size_t a = 0,
          std::size_t b = 0)
        : std::runtime_error(msg), kind_(kind), a_(a), b_(b) {}
    ErrorKind kind() const noexcept { return kind_; }
    std::size_t a() const noexcept { return a_; }
    std::size_t b() const noexcept { return b_; }

   private:
    ErrorKind kind_;
    std::size_t a_;
    std::size_t b_;
};

// ── HomeId ───────────────────────────────────────────────────────────────────

inline std::array<std::uint8_t, 4> home_id_to_be_bytes(std::uint32_t home_id) {
    return {static_cast<std::uint8_t>((home_id >> 24) & 0xff),
            static_cast<std::uint8_t>((home_id >> 16) & 0xff),
            static_cast<std::uint8_t>((home_id >> 8) & 0xff),
            static_cast<std::uint8_t>(home_id & 0xff)};
}

// ── NodeId ───────────────────────────────────────────────────────────────────

struct NodeId {
    enum Kind { Classic, LongRange } kind = Classic;
    std::uint16_t value = 0;

    bool is_classic() const { return kind == Classic; }
    bool is_long_range() const { return kind == LongRange; }
    bool operator==(const NodeId &o) const {
        return kind == o.kind && value == o.value;
    }
    bool operator!=(const NodeId &o) const { return !(*this == o); }

    static NodeId classic(std::uint8_t value) {
        if (value == 0 || value > 232)
            throw Error(ErrorKind::InvalidClassicNodeId,
                        "invalid classic Z-Wave node id", value);
        return NodeId{Classic, value};
    }
    static NodeId long_range(std::uint16_t value) {
        if (value < 1 || value > 4000)
            throw Error(ErrorKind::InvalidLongRangeNodeId,
                        "invalid Z-Wave Long Range node id", value);
        return NodeId{LongRange, value};
    }
};

// ── RegionProfile ─────────────────────────────────────────────────────────────

enum class RegionProfile {
    Europe,
    UnitedStates,
    AustraliaNewZealand,
    HongKong,
    India,
    Israel,
    Russia,
    China,
    Japan,
    Korea,
    UnitedStatesLongRange,
    EuropeLongRange
};

inline const char *band_description(RegionProfile region) {
    switch (region) {
        case RegionProfile::Europe: return "EU sub-GHz";
        case RegionProfile::UnitedStates: return "US sub-GHz";
        case RegionProfile::AustraliaNewZealand: return "ANZ sub-GHz";
        case RegionProfile::HongKong: return "Hong Kong sub-GHz";
        case RegionProfile::India: return "India sub-GHz";
        case RegionProfile::Israel: return "Israel sub-GHz";
        case RegionProfile::Russia: return "Russia sub-GHz";
        case RegionProfile::China: return "China sub-GHz";
        case RegionProfile::Japan: return "Japan sub-GHz";
        case RegionProfile::Korea: return "Korea sub-GHz";
        case RegionProfile::UnitedStatesLongRange: return "US Z-Wave Long Range";
        case RegionProfile::EuropeLongRange: return "EU Z-Wave Long Range";
    }
    return "unknown region";
}
inline bool supports_long_range(RegionProfile region) {
    return region == RegionProfile::UnitedStatesLongRange ||
           region == RegionProfile::EuropeLongRange;
}

// ── CommandClassId ────────────────────────────────────────────────────────────

struct CommandClassId {
    std::uint16_t value = 0;
    constexpr CommandClassId() = default;
    constexpr explicit CommandClassId(std::uint16_t v) : value(v) {}

    std::size_t encoded_len() const { return value <= 0xff ? 1u : 2u; }
    void encode(std::vector<std::uint8_t> &out) const {
        if (value <= 0xff) {
            out.push_back(static_cast<std::uint8_t>(value));
        } else {
            out.push_back(static_cast<std::uint8_t>((value >> 8) & 0xff));
            out.push_back(static_cast<std::uint8_t>(value & 0xff));
        }
    }
    bool is_actuator() const {
        return value == 0x20 || value == 0x25 || value == 0x26 || value == 0x62;
    }
    bool is_sensor() const { return value == 0x30 || value == 0x31; }
    bool is_security() const { return value == 0x9f; }
    bool operator==(CommandClassId o) const { return value == o.value; }
    bool operator!=(CommandClassId o) const { return value != o.value; }
};

inline constexpr CommandClassId kBasic{0x20};
inline constexpr CommandClassId kSwitchBinary{0x25};
inline constexpr CommandClassId kSwitchMultilevel{0x26};
inline constexpr CommandClassId kSensorBinary{0x30};
inline constexpr CommandClassId kSensorMultilevel{0x31};
inline constexpr CommandClassId kDoorLock{0x62};
inline constexpr CommandClassId kBattery{0x80};
inline constexpr CommandClassId kSecurity2{0x9f};

// XOR-fold checksum used by the serial framing (seed 0xff).
inline std::uint8_t serial_checksum(const std::uint8_t *bytes,
                                    std::size_t len) {
    std::uint8_t acc = 0xff;
    for (std::size_t i = 0; i < len; ++i) acc ^= bytes[i];
    return acc;
}

// ── ZWaveNetworkSummary ───────────────────────────────────────────────────────

struct NetworkSummary {
    RegionProfile region = RegionProfile::Europe;
    bool supports_long_range = false;
    std::size_t classic_nodes = 0;
    std::size_t long_range_nodes = 0;
    std::size_t command_class_entries = 0;
    std::size_t actuator_command_classes = 0;
    std::size_t sensor_command_classes = 0;
    std::size_t security_command_classes = 0;

    bool has_nodes() const { return classic_nodes + long_range_nodes > 0; }
    bool has_long_range_nodes() const { return long_range_nodes > 0; }
    bool has_security() const { return security_command_classes > 0; }
    bool operator==(const NetworkSummary &o) const {
        return region == o.region &&
               supports_long_range == o.supports_long_range &&
               classic_nodes == o.classic_nodes &&
               long_range_nodes == o.long_range_nodes &&
               command_class_entries == o.command_class_entries &&
               actuator_command_classes == o.actuator_command_classes &&
               sensor_command_classes == o.sensor_command_classes &&
               security_command_classes == o.security_command_classes;
    }

    static NetworkSummary from_parts(
        RegionProfile region, const std::vector<NodeId> &nodes,
        const std::vector<CommandClassId> &command_classes) {
        NetworkSummary s;
        s.region = region;
        s.supports_long_range = zwave_core::supports_long_range(region);
        for (const auto &node : nodes) {
            if (node.is_long_range())
                ++s.long_range_nodes;
            else
                ++s.classic_nodes;
        }
        for (const auto &cc : command_classes) {
            ++s.command_class_entries;
            if (cc.is_actuator()) ++s.actuator_command_classes;
            if (cc.is_sensor()) ++s.sensor_command_classes;
            if (cc.is_security()) ++s.security_command_classes;
        }
        return s;
    }
};

// ── CommandClassFrame ─────────────────────────────────────────────────────────

namespace detail {
// Parse a command-class id: first byte >= 0xf0 takes a second byte.
inline std::pair<CommandClassId, std::size_t> parse_command_class_id(
    const std::uint8_t *bytes, std::size_t len) {
    if (len < 1) throw Error(ErrorKind::Truncated, "truncated", 1, 0);
    std::uint8_t first = bytes[0];
    if (first >= 0xf0) {
        if (len < 2) throw Error(ErrorKind::Truncated, "truncated", 2, len);
        return {CommandClassId(static_cast<std::uint16_t>(
                    (static_cast<std::uint16_t>(first) << 8) | bytes[1])),
                2};
    }
    return {CommandClassId(first), 1};
}
}  // namespace detail

struct CommandClassFrame {
    CommandClassId command_class_id;
    std::uint8_t command_id = 0;
    std::vector<std::uint8_t> payload;

    bool operator==(const CommandClassFrame &o) const {
        return command_class_id == o.command_class_id &&
               command_id == o.command_id && payload == o.payload;
    }

    static CommandClassFrame parse(const std::uint8_t *bytes, std::size_t len) {
        auto [cc, cmd_off] = detail::parse_command_class_id(bytes, len);
        if (cmd_off >= len)
            throw Error(ErrorKind::Truncated, "truncated", cmd_off + 1, len);
        CommandClassFrame f;
        f.command_class_id = cc;
        f.command_id = bytes[cmd_off];
        f.payload.assign(bytes + cmd_off + 1, bytes + len);
        return f;
    }
    static CommandClassFrame parse(const std::vector<std::uint8_t> &bytes) {
        return parse(bytes.data(), bytes.size());
    }

    std::vector<std::uint8_t> encode() const {
        std::size_t total =
            command_class_id.encoded_len() + 1 + payload.size();
        if (total > 255 || payload.size() > 255)
            throw Error(ErrorKind::CommandPayloadTooLong,
                        "command-class payload too long", payload.size());
        std::vector<std::uint8_t> out;
        out.reserve(total);
        command_class_id.encode(out);
        out.push_back(command_id);
        out.insert(out.end(), payload.begin(), payload.end());
        return out;
    }
};

struct CommandClassFrameSummary {
    std::size_t frame_count = 0;
    std::size_t short_command_class_frames = 0;
    std::size_t extended_command_class_frames = 0;
    std::size_t security_2_frames = 0;
    std::size_t total_payload_bytes = 0;
    std::size_t max_payload_bytes = 0;

    bool has_extended_command_classes() const {
        return extended_command_class_frames > 0;
    }
    bool has_security_2_frames() const { return security_2_frames > 0; }
    bool is_empty() const { return frame_count == 0; }
    bool operator==(const CommandClassFrameSummary &o) const {
        return frame_count == o.frame_count &&
               short_command_class_frames == o.short_command_class_frames &&
               extended_command_class_frames ==
                   o.extended_command_class_frames &&
               security_2_frames == o.security_2_frames &&
               total_payload_bytes == o.total_payload_bytes &&
               max_payload_bytes == o.max_payload_bytes;
    }

    static CommandClassFrameSummary from_frames(
        const std::vector<CommandClassFrame> &frames) {
        CommandClassFrameSummary s;
        for (const auto &f : frames) {
            ++s.frame_count;
            if (f.command_class_id.encoded_len() == 1)
                ++s.short_command_class_frames;
            else
                ++s.extended_command_class_frames;
            if (f.command_class_id == kSecurity2) ++s.security_2_frames;
            s.total_payload_bytes += f.payload.size();
            if (f.payload.size() > s.max_payload_bytes)
                s.max_payload_bytes = f.payload.size();
        }
        return s;
    }
};

// ── SerialFrame ───────────────────────────────────────────────────────────────

enum class SerialFrameType { Request, Response };

struct SerialFrame {
    SerialFrameType frame_type = SerialFrameType::Request;
    std::uint8_t function_id = 0;
    std::vector<std::uint8_t> payload;

    bool operator==(const SerialFrame &o) const {
        return frame_type == o.frame_type && function_id == o.function_id &&
               payload == o.payload;
    }

    static SerialFrame parse(const std::uint8_t *bytes, std::size_t len) {
        if (len < 5) throw Error(ErrorKind::Truncated, "truncated", 5, len);
        if (bytes[0] != kSof)
            throw Error(ErrorKind::MissingStartOfFrame, "missing SOF", bytes[0]);
        std::size_t declared = bytes[1];
        if (declared < 3)
            throw Error(ErrorKind::InvalidLength, "invalid length", declared);
        std::size_t frame_len = declared + 2;
        if (len < frame_len)
            throw Error(ErrorKind::Truncated, "truncated", frame_len, len);
        std::uint8_t checksum = bytes[frame_len - 1];
        std::uint8_t expected = serial_checksum(bytes + 1, frame_len - 2);
        if (checksum != expected)
            throw Error(ErrorKind::ChecksumMismatch, "checksum mismatch",
                        expected, checksum);
        SerialFrame f;
        f.frame_type = type_from_byte(bytes[2]);
        f.function_id = bytes[3];
        f.payload.assign(bytes + 4, bytes + frame_len - 1);
        return f;
    }
    static SerialFrame parse(const std::vector<std::uint8_t> &bytes) {
        return parse(bytes.data(), bytes.size());
    }

    std::vector<std::uint8_t> encode() const {
        std::size_t declared = payload.size() + 3;
        if (declared > 255 || payload.size() > 255)
            throw Error(ErrorKind::PayloadTooLong, "serial payload too long",
                        payload.size());
        std::vector<std::uint8_t> out;
        out.reserve(declared + 2);
        out.push_back(kSof);
        out.push_back(static_cast<std::uint8_t>(declared));
        out.push_back(type_as_byte(frame_type));
        out.push_back(function_id);
        out.insert(out.end(), payload.begin(), payload.end());
        out.push_back(serial_checksum(out.data() + 1, out.size() - 1));
        return out;
    }

   private:
    static SerialFrameType type_from_byte(std::uint8_t byte) {
        if (byte == 0x00) return SerialFrameType::Request;
        if (byte == 0x01) return SerialFrameType::Response;
        throw Error(ErrorKind::InvalidFrameType, "invalid frame type", byte);
    }
    static std::uint8_t type_as_byte(SerialFrameType t) {
        return t == SerialFrameType::Request ? 0x00 : 0x01;
    }
};

struct SerialFrameSummary {
    SerialFrameType frame_type = SerialFrameType::Request;
    std::uint8_t function_id = 0;
    std::size_t payload_len = 0;

    bool is_request() const { return frame_type == SerialFrameType::Request; }
    bool is_response() const { return frame_type == SerialFrameType::Response; }
    bool has_payload() const { return payload_len > 0; }
    bool is_function(std::uint8_t f) const { return function_id == f; }
    bool is_empty_payload() const { return payload_len == 0; }
    bool operator==(const SerialFrameSummary &o) const {
        return frame_type == o.frame_type && function_id == o.function_id &&
               payload_len == o.payload_len;
    }

    static SerialFrameSummary from_frame(const SerialFrame &frame) {
        return {frame.frame_type, frame.function_id, frame.payload.size()};
    }
};

struct SerialFrameBatchSummary {
    std::size_t frame_count = 0;
    std::size_t request_frames = 0;
    std::size_t response_frames = 0;
    std::size_t total_payload_bytes = 0;
    std::size_t max_payload_bytes = 0;

    bool has_requests() const { return request_frames > 0; }
    bool has_responses() const { return response_frames > 0; }
    bool is_empty() const { return frame_count == 0; }
    bool operator==(const SerialFrameBatchSummary &o) const {
        return frame_count == o.frame_count &&
               request_frames == o.request_frames &&
               response_frames == o.response_frames &&
               total_payload_bytes == o.total_payload_bytes &&
               max_payload_bytes == o.max_payload_bytes;
    }

    static SerialFrameBatchSummary from_frames(
        const std::vector<SerialFrame> &frames) {
        SerialFrameBatchSummary s;
        for (const auto &f : frames) {
            ++s.frame_count;
            if (f.frame_type == SerialFrameType::Request)
                ++s.request_frames;
            else
                ++s.response_frames;
            s.total_payload_bytes += f.payload.size();
            if (f.payload.size() > s.max_payload_bytes)
                s.max_payload_bytes = f.payload.size();
        }
        return s;
    }
};

// ── ControllerReadinessSummary ────────────────────────────────────────────────

struct ControllerReadinessSummary {
    NetworkSummary network;
    CommandClassFrameSummary command_frames;
    SerialFrameBatchSummary serial_frames;
    bool has_nodes = false;
    bool has_command_class_coverage = false;
    bool has_serial_requests = false;
    bool has_serial_responses = false;
    bool has_security_coverage = false;
    bool long_range_region_mismatch = false;

    bool is_ready() const {
        return has_nodes && has_command_class_coverage && has_serial_requests &&
               has_serial_responses && !long_range_region_mismatch;
    }
    bool needs_node_discovery() const { return !has_nodes; }
    bool needs_command_class_interview() const {
        return has_nodes && !has_command_class_coverage;
    }
    bool needs_serial_probe() const { return !has_serial_requests; }
    bool waiting_for_serial_response() const {
        return has_serial_requests && !has_serial_responses;
    }
    bool needs_region_review() const { return long_range_region_mismatch; }

    static ControllerReadinessSummary from_summaries(
        NetworkSummary network, CommandClassFrameSummary command_frames,
        SerialFrameBatchSummary serial_frames) {
        ControllerReadinessSummary s;
        s.network = network;
        s.command_frames = command_frames;
        s.serial_frames = serial_frames;
        s.has_nodes = network.has_nodes();
        s.has_command_class_coverage =
            network.command_class_entries > 0 || !command_frames.is_empty();
        s.has_serial_requests = serial_frames.has_requests();
        s.has_serial_responses = serial_frames.has_responses();
        s.has_security_coverage =
            network.has_security() || command_frames.has_security_2_frames();
        s.long_range_region_mismatch =
            network.has_long_range_nodes() && !network.supports_long_range;
        return s;
    }
};

}  // namespace zwave_core
}  // namespace ca

#endif  // ZWAVE_CORE_HPP
