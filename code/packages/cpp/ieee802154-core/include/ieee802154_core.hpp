// ieee802154_core.hpp — IEEE 802.15.4 MAC frame primitives, header-only C++17.
// ============================================================================
//
// A faithful port of the Rust `ieee802154-core` crate, in namespace
// `ca::ieee802154_core`: a small, dependency-free parser/encoder for IEEE
// 802.15.4 MAC frames — the byte-level foundation Zigbee and Thread build on.
// Covers the frame-control field, addressing, the auxiliary security header,
// beacon payloads (superframe spec, GTS, pending addresses), and PAN
// descriptors / scan summaries.
//
// Every read is bounds-checked. Where the Rust returns `Result`, this port
// throws the corresponding error enum (`MacError` / `BeaconError`). Pure ISO
// C++17.

#ifndef IEEE802154_CORE_HPP
#define IEEE802154_CORE_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <vector>

namespace ca {
namespace ieee802154_core {

// ── Bit-field enums (value == the wire field) ────────────────────────────────
enum class FrameType : std::uint8_t {
    Beacon = 0,
    Data = 1,
    Acknowledgment = 2,
    MacCommand = 3,
    Reserved = 4,
    Multipurpose = 5,
    Fragment = 6,
    Extended = 7,
};

enum class AddressMode : std::uint8_t {
    None = 0,
    Reserved = 1,
    Short = 2,
    Extended = 3,
};

inline std::size_t encoded_len(AddressMode m) {
    switch (m) {
    case AddressMode::Short:
        return 2;
    case AddressMode::Extended:
        return 8;
    case AddressMode::None:
    case AddressMode::Reserved:
        return 0;
    }
    return 0;
}

enum class FrameVersion : std::uint8_t {
    Ieee2003 = 0,
    Ieee2006 = 1,
    Ieee2015 = 2,
    Reserved = 3,
};

struct Address {
    AddressMode mode = AddressMode::Short;  // Short or Extended
    std::uint16_t short_addr = 0;
    std::uint64_t extended_addr = 0;

    static Address make_short(std::uint16_t v) {
        return {AddressMode::Short, v, 0};
    }
    static Address make_extended(std::uint64_t v) {
        return {AddressMode::Extended, 0, v};
    }
    bool operator==(const Address& o) const {
        return mode == o.mode && short_addr == o.short_addr &&
               extended_addr == o.extended_addr;
    }
};

struct FrameControl {
    FrameType frame_type = FrameType::Beacon;
    bool security_enabled = false;
    bool frame_pending = false;
    bool ack_request = false;
    bool pan_id_compression = false;
    bool sequence_number_suppression = false;
    bool information_elements_present = false;
    AddressMode destination_address_mode = AddressMode::None;
    FrameVersion frame_version = FrameVersion::Ieee2003;
    AddressMode source_address_mode = AddressMode::None;

    static FrameControl parse(std::uint16_t raw) {
        FrameControl fc;
        fc.frame_type = static_cast<FrameType>(raw & 0x7);
        fc.security_enabled = (raw & (1u << 3)) != 0;
        fc.frame_pending = (raw & (1u << 4)) != 0;
        fc.ack_request = (raw & (1u << 5)) != 0;
        fc.pan_id_compression = (raw & (1u << 6)) != 0;
        fc.sequence_number_suppression = (raw & (1u << 8)) != 0;
        fc.information_elements_present = (raw & (1u << 9)) != 0;
        fc.destination_address_mode =
            static_cast<AddressMode>((raw >> 10) & 0x3);
        fc.frame_version = static_cast<FrameVersion>((raw >> 12) & 0x3);
        fc.source_address_mode = static_cast<AddressMode>((raw >> 14) & 0x3);
        return fc;
    }
    std::uint16_t encode() const {
        std::uint16_t raw = static_cast<std::uint16_t>(frame_type);
        raw |= static_cast<std::uint16_t>((security_enabled ? 1u : 0u) << 3);
        raw |= static_cast<std::uint16_t>((frame_pending ? 1u : 0u) << 4);
        raw |= static_cast<std::uint16_t>((ack_request ? 1u : 0u) << 5);
        raw |= static_cast<std::uint16_t>((pan_id_compression ? 1u : 0u) << 6);
        raw |= static_cast<std::uint16_t>(
            (sequence_number_suppression ? 1u : 0u) << 8);
        raw |= static_cast<std::uint16_t>(
            (information_elements_present ? 1u : 0u) << 9);
        raw |= static_cast<std::uint16_t>(
            static_cast<unsigned>(destination_address_mode) << 10);
        raw |= static_cast<std::uint16_t>(
            static_cast<unsigned>(frame_version) << 12);
        raw |= static_cast<std::uint16_t>(
            static_cast<unsigned>(source_address_mode) << 14);
        return raw;
    }
    bool operator==(const FrameControl& o) const {
        return frame_type == o.frame_type &&
               security_enabled == o.security_enabled &&
               frame_pending == o.frame_pending &&
               ack_request == o.ack_request &&
               pan_id_compression == o.pan_id_compression &&
               sequence_number_suppression == o.sequence_number_suppression &&
               information_elements_present == o.information_elements_present &&
               destination_address_mode == o.destination_address_mode &&
               frame_version == o.frame_version &&
               source_address_mode == o.source_address_mode;
    }
};

enum class SecurityLevel : std::uint8_t {
    None = 0,
    Mic32 = 1,
    Mic64 = 2,
    Mic128 = 3,
    Enc = 4,
    EncMic32 = 5,
    EncMic64 = 6,
    EncMic128 = 7,
};

inline bool encrypts(SecurityLevel l) {
    return l == SecurityLevel::Enc || l == SecurityLevel::EncMic32 ||
           l == SecurityLevel::EncMic64 || l == SecurityLevel::EncMic128;
}
inline std::size_t mic_len(SecurityLevel l) {
    switch (l) {
    case SecurityLevel::None:
    case SecurityLevel::Enc:
        return 0;
    case SecurityLevel::Mic32:
    case SecurityLevel::EncMic32:
        return 4;
    case SecurityLevel::Mic64:
    case SecurityLevel::EncMic64:
        return 8;
    case SecurityLevel::Mic128:
    case SecurityLevel::EncMic128:
        return 16;
    }
    return 0;
}

enum class KeyIdMode : std::uint8_t {
    Implicit = 0,
    KeyIndex = 1,
    KeySource4 = 2,
    KeySource8 = 3,
};

struct SecurityControl {
    SecurityLevel security_level = SecurityLevel::None;
    KeyIdMode key_identifier_mode = KeyIdMode::Implicit;
    bool frame_counter_suppression = false;
    bool frame_counter_size_5 = false;

    static SecurityControl parse(std::uint8_t raw) {
        SecurityControl sc;
        sc.security_level = static_cast<SecurityLevel>(raw & 0x7);
        sc.key_identifier_mode = static_cast<KeyIdMode>((raw >> 3) & 0x3);
        sc.frame_counter_suppression = (raw & (1u << 5)) != 0;
        sc.frame_counter_size_5 = (raw & (1u << 6)) != 0;
        return sc;
    }
    std::uint8_t encode() const {
        std::uint8_t raw = static_cast<std::uint8_t>(security_level);
        raw |= static_cast<std::uint8_t>(
            static_cast<unsigned>(key_identifier_mode) << 3);
        raw |= static_cast<std::uint8_t>((frame_counter_suppression ? 1u : 0u)
                                         << 5);
        raw |= static_cast<std::uint8_t>((frame_counter_size_5 ? 1u : 0u) << 6);
        return raw;
    }
};

struct FrameCounter {
    bool is_40bit = false;
    std::uint64_t value = 0;
};

struct KeyIdentifier {
    KeyIdMode mode = KeyIdMode::Implicit;
    std::uint8_t index = 0;
    std::array<std::uint8_t, 8> source{};
};

struct AuxSecurityHeader {
    SecurityControl security_control;
    std::optional<FrameCounter> frame_counter;
    KeyIdentifier key_identifier;
};

enum class MacError {
    Truncated,
    ReservedAddressMode,
    AddressModeMismatch,
    MissingSequenceNumber,
    MissingDestinationPanId,
    MissingSourcePanId,
    MissingAuxiliarySecurityHeader,
    UnexpectedAuxiliarySecurityHeader,
    MissingFrameCounter,
    UnexpectedFrameCounter,
    FrameCounterSizeMismatch,
    FrameCounterOutOfRange,
    KeyIdentifierModeMismatch,
};

enum class BeaconError {
    ExpectedBeaconFrame,
    MissingBeaconSourceAddress,
    MissingBeaconPanId,
    TruncatedField,
};

namespace detail {
class Cursor {
  public:
    Cursor(const std::uint8_t* b, std::size_t n) : bytes_(b), len_(n) {}
    std::size_t remaining() const { return len_ > off_ ? len_ - off_ : 0; }
    const std::uint8_t* read(std::size_t n) {
        if (remaining() < n) {
            throw MacError::Truncated;
        }
        const std::uint8_t* p = bytes_ + off_;
        off_ += n;
        return p;
    }
    std::uint8_t u8() { return read(1)[0]; }
    std::uint16_t u16() {
        const std::uint8_t* p = read(2);
        return static_cast<std::uint16_t>(p[0] | (std::uint16_t(p[1]) << 8));
    }
    std::uint32_t u32() {
        const std::uint8_t* p = read(4);
        return std::uint32_t(p[0]) | (std::uint32_t(p[1]) << 8) |
               (std::uint32_t(p[2]) << 16) | (std::uint32_t(p[3]) << 24);
    }
    std::uint64_t u40() {
        const std::uint8_t* p = read(5);
        return std::uint64_t(p[0]) | (std::uint64_t(p[1]) << 8) |
               (std::uint64_t(p[2]) << 16) | (std::uint64_t(p[3]) << 24) |
               (std::uint64_t(p[4]) << 32);
    }
    std::uint64_t u64() {
        const std::uint8_t* p = read(8);
        std::uint64_t v = 0;
        for (int i = 0; i < 8; ++i) {
            v |= std::uint64_t(p[i]) << (i * 8);
        }
        return v;
    }

  private:
    const std::uint8_t* bytes_;
    std::size_t len_;
    std::size_t off_ = 0;
};

inline void push_u16(std::vector<std::uint8_t>& o, std::uint16_t v) {
    o.push_back(static_cast<std::uint8_t>(v));
    o.push_back(static_cast<std::uint8_t>(v >> 8));
}

inline std::optional<Address> read_address(Cursor& c, AddressMode mode) {
    switch (mode) {
    case AddressMode::None:
        return std::nullopt;
    case AddressMode::Reserved:
        throw MacError::ReservedAddressMode;
    case AddressMode::Short:
        return Address::make_short(c.u16());
    case AddressMode::Extended:
        return Address::make_extended(c.u64());
    }
    return std::nullopt;
}
}  // namespace detail

struct MacFrameSummary {
    FrameType frame_type;
    FrameVersion frame_version;
    AddressMode destination_address_mode;
    AddressMode source_address_mode;
    bool security_enabled;
    bool has_auxiliary_security_header;
    bool ack_request;
    bool frame_pending;
    bool pan_id_compression;
    bool sequence_number_suppressed;
    bool information_elements_present;
    bool has_sequence_number;
    bool has_destination_pan_id;
    bool has_source_pan_id;
    bool has_destination;
    bool has_source;
    std::size_t payload_len;
    bool has_fcs;

    bool has_payload() const { return payload_len > 0; }
    bool has_addressing() const { return has_destination || has_source; }
};

struct MacFrame {
    FrameControl frame_control;
    std::optional<std::uint8_t> sequence_number;
    std::optional<std::uint16_t> destination_pan_id;
    std::optional<Address> destination;
    std::optional<std::uint16_t> source_pan_id;
    std::optional<Address> source;
    std::optional<AuxSecurityHeader> auxiliary_security_header;
    std::vector<std::uint8_t> payload;
    std::optional<std::uint16_t> fcs;

    static MacFrame parse_without_fcs(const std::vector<std::uint8_t>& b) {
        return parse(b.data(), b.size(), false);
    }
    static MacFrame parse_with_fcs(const std::vector<std::uint8_t>& b) {
        return parse(b.data(), b.size(), true);
    }
    static MacFrame parse(const std::uint8_t* bytes, std::size_t len,
                          bool has_fcs) {
        using namespace detail;
        Cursor c(bytes, len);
        MacFrame f;
        f.frame_control = FrameControl::parse(c.u16());
        if (f.frame_control.destination_address_mode == AddressMode::Reserved ||
            f.frame_control.source_address_mode == AddressMode::Reserved) {
            throw MacError::ReservedAddressMode;
        }
        if (!f.frame_control.sequence_number_suppression) {
            f.sequence_number = c.u8();
        }
        if (f.frame_control.destination_address_mode != AddressMode::None) {
            f.destination_pan_id = c.u16();
            f.destination =
                read_address(c, f.frame_control.destination_address_mode);
        }
        if (f.frame_control.source_address_mode != AddressMode::None) {
            if (f.frame_control.pan_id_compression && f.destination_pan_id) {
                f.source_pan_id = f.destination_pan_id;
            } else {
                f.source_pan_id = c.u16();
            }
        }
        f.source = read_address(c, f.frame_control.source_address_mode);
        if (f.frame_control.security_enabled) {
            f.auxiliary_security_header = parse_aux(c);
        }
        std::size_t remaining = c.remaining();
        std::size_t payload_len;
        if (has_fcs) {
            if (remaining < 2) {
                throw MacError::Truncated;
            }
            payload_len = remaining - 2;
        } else {
            payload_len = remaining;
        }
        const std::uint8_t* pl = c.read(payload_len);
        f.payload.assign(pl, pl + payload_len);
        if (has_fcs) {
            f.fcs = c.u16();
        }
        return f;
    }

    std::vector<std::uint8_t> encode() const {
        using namespace detail;
        validate_modes();
        std::vector<std::uint8_t> out;
        push_u16(out, frame_control.encode());
        if (!frame_control.sequence_number_suppression) {
            if (!sequence_number) {
                throw MacError::MissingSequenceNumber;
            }
            out.push_back(*sequence_number);
        }
        if (destination) {
            if (!destination_pan_id) {
                throw MacError::MissingDestinationPanId;
            }
            push_u16(out, *destination_pan_id);
            encode_address(*destination, out);
        }
        if (source) {
            if (!frame_control.pan_id_compression || !destination_pan_id) {
                if (!source_pan_id) {
                    throw MacError::MissingSourcePanId;
                }
                push_u16(out, *source_pan_id);
            }
            encode_address(*source, out);
        }
        if (frame_control.security_enabled) {
            encode_aux(*auxiliary_security_header, out);
        }
        out.insert(out.end(), payload.begin(), payload.end());
        if (fcs) {
            push_u16(out, *fcs);
        }
        return out;
    }

    MacFrameSummary summary() const {
        MacFrameSummary s;
        s.frame_type = frame_control.frame_type;
        s.frame_version = frame_control.frame_version;
        s.destination_address_mode = frame_control.destination_address_mode;
        s.source_address_mode = frame_control.source_address_mode;
        s.security_enabled = frame_control.security_enabled;
        s.has_auxiliary_security_header =
            auxiliary_security_header.has_value();
        s.ack_request = frame_control.ack_request;
        s.frame_pending = frame_control.frame_pending;
        s.pan_id_compression = frame_control.pan_id_compression;
        s.sequence_number_suppressed =
            frame_control.sequence_number_suppression;
        s.information_elements_present =
            frame_control.information_elements_present;
        s.has_sequence_number = sequence_number.has_value();
        s.has_destination_pan_id = destination_pan_id.has_value();
        s.has_source_pan_id = source_pan_id.has_value();
        s.has_destination = destination.has_value();
        s.has_source = source.has_value();
        s.payload_len = payload.size();
        s.has_fcs = fcs.has_value();
        return s;
    }

  private:
    static AuxSecurityHeader parse_aux(detail::Cursor& c) {
        AuxSecurityHeader h;
        h.security_control = SecurityControl::parse(c.u8());
        if (!h.security_control.frame_counter_suppression) {
            if (h.security_control.frame_counter_size_5) {
                h.frame_counter = FrameCounter{true, c.u40()};
            } else {
                h.frame_counter = FrameCounter{false, c.u32()};
            }
        }
        h.key_identifier.mode = h.security_control.key_identifier_mode;
        switch (h.security_control.key_identifier_mode) {
        case KeyIdMode::Implicit:
            break;
        case KeyIdMode::KeyIndex:
            h.key_identifier.index = c.u8();
            break;
        case KeyIdMode::KeySource4: {
            const std::uint8_t* p = c.read(4);
            for (int i = 0; i < 4; ++i) {
                h.key_identifier.source[static_cast<std::size_t>(i)] = p[i];
            }
            h.key_identifier.index = c.u8();
            break;
        }
        case KeyIdMode::KeySource8: {
            const std::uint8_t* p = c.read(8);
            for (int i = 0; i < 8; ++i) {
                h.key_identifier.source[static_cast<std::size_t>(i)] = p[i];
            }
            h.key_identifier.index = c.u8();
            break;
        }
        }
        return h;
    }

    static void encode_address(const Address& a,
                               std::vector<std::uint8_t>& out) {
        if (a.mode == AddressMode::Short) {
            detail::push_u16(out, a.short_addr);
        } else {
            for (int i = 0; i < 8; ++i) {
                out.push_back(
                    static_cast<std::uint8_t>(a.extended_addr >> (i * 8)));
            }
        }
    }
    static void encode_aux(const AuxSecurityHeader& h,
                           std::vector<std::uint8_t>& out) {
        out.push_back(h.security_control.encode());
        if (h.frame_counter) {
            int n = h.frame_counter->is_40bit ? 5 : 4;
            for (int i = 0; i < n; ++i) {
                out.push_back(
                    static_cast<std::uint8_t>(h.frame_counter->value >> (i * 8)));
            }
        }
        switch (h.key_identifier.mode) {
        case KeyIdMode::Implicit:
            break;
        case KeyIdMode::KeyIndex:
            out.push_back(h.key_identifier.index);
            break;
        case KeyIdMode::KeySource4:
            out.insert(out.end(), h.key_identifier.source.begin(),
                       h.key_identifier.source.begin() + 4);
            out.push_back(h.key_identifier.index);
            break;
        case KeyIdMode::KeySource8:
            out.insert(out.end(), h.key_identifier.source.begin(),
                       h.key_identifier.source.end());
            out.push_back(h.key_identifier.index);
            break;
        }
    }

    void validate_modes() const {
        if (frame_control.destination_address_mode == AddressMode::Reserved ||
            frame_control.source_address_mode == AddressMode::Reserved) {
            throw MacError::ReservedAddressMode;
        }
        AddressMode dm =
            destination ? destination->mode : AddressMode::None;
        if (dm != frame_control.destination_address_mode) {
            throw MacError::AddressModeMismatch;
        }
        AddressMode sm = source ? source->mode : AddressMode::None;
        if (sm != frame_control.source_address_mode) {
            throw MacError::AddressModeMismatch;
        }
        if (frame_control.security_enabled) {
            if (!auxiliary_security_header) {
                throw MacError::MissingAuxiliarySecurityHeader;
            }
            validate_aux(*auxiliary_security_header);
        } else if (auxiliary_security_header) {
            throw MacError::UnexpectedAuxiliarySecurityHeader;
        }
    }
    static void validate_aux(const AuxSecurityHeader& h) {
        if (h.key_identifier.mode != h.security_control.key_identifier_mode) {
            throw MacError::KeyIdentifierModeMismatch;
        }
        if (h.security_control.frame_counter_suppression) {
            if (h.frame_counter) {
                throw MacError::UnexpectedFrameCounter;
            }
            return;
        }
        if (!h.frame_counter) {
            throw MacError::MissingFrameCounter;
        }
        if (!h.security_control.frame_counter_size_5) {
            if (h.frame_counter->is_40bit) {
                throw MacError::FrameCounterSizeMismatch;
            }
            return;
        }
        if (!h.frame_counter->is_40bit) {
            throw MacError::FrameCounterSizeMismatch;
        }
        if (h.frame_counter->value > 0xFFFFFFFFFFull) {
            throw MacError::FrameCounterOutOfRange;
        }
    }
};

struct SuperframeSpecification {
    std::uint16_t raw = 0;
    std::uint8_t beacon_order() const {
        return static_cast<std::uint8_t>(raw & 0xF);
    }
    std::uint8_t superframe_order() const {
        return static_cast<std::uint8_t>((raw >> 4) & 0xF);
    }
    std::uint8_t final_cap_slot() const {
        return static_cast<std::uint8_t>((raw >> 8) & 0xF);
    }
    bool battery_life_extension() const { return (raw & (1u << 12)) != 0; }
    bool pan_coordinator() const { return (raw & (1u << 14)) != 0; }
    bool association_permit() const { return (raw & (1u << 15)) != 0; }
};

struct GtsDescriptor {
    std::uint16_t short_address;
    std::uint8_t starting_slot;
    std::uint8_t length;
};

struct GtsFields {
    std::uint8_t descriptor_count = 0;
    bool permit = false;
    std::optional<std::uint8_t> directions;
    std::vector<GtsDescriptor> descriptors;
};

struct BeaconPayload {
    SuperframeSpecification superframe;
    GtsFields gts;
    std::vector<std::uint16_t> short_addresses;
    std::vector<std::uint64_t> extended_addresses;
    std::vector<std::uint8_t> payload;

    static BeaconPayload parse(const std::vector<std::uint8_t>& bytes) {
        return parse(bytes.data(), bytes.size());
    }
    static BeaconPayload parse(const std::uint8_t* bytes, std::size_t len) {
        BeaconPayload bp;
        std::size_t off = 0;
        bp.superframe.raw = read_u16(bytes, len, off);
        std::uint8_t gts_spec = read_u8(bytes, len, off);
        bp.gts.descriptor_count = gts_spec & 0x07;
        bp.gts.permit = (gts_spec & 0x80) != 0;
        if (bp.gts.descriptor_count != 0) {
            bp.gts.directions = read_u8(bytes, len, off);
        }
        for (std::uint8_t i = 0; i < bp.gts.descriptor_count; ++i) {
            std::uint16_t addr = read_u16(bytes, len, off);
            std::uint8_t sl = read_u8(bytes, len, off);
            bp.gts.descriptors.push_back(
                GtsDescriptor{addr, static_cast<std::uint8_t>(sl & 0x0F),
                              static_cast<std::uint8_t>((sl >> 4) & 0x0F)});
        }
        std::uint8_t pending = read_u8(bytes, len, off);
        std::uint8_t short_count = pending & 0x07;
        std::uint8_t ext_count = (pending >> 4) & 0x07;
        for (std::uint8_t i = 0; i < short_count; ++i) {
            bp.short_addresses.push_back(read_u16(bytes, len, off));
        }
        for (std::uint8_t i = 0; i < ext_count; ++i) {
            bp.extended_addresses.push_back(read_u64(bytes, len, off));
        }
        bp.payload.assign(bytes + off, bytes + len);
        return bp;
    }

  private:
    static std::size_t need(const std::uint8_t* bytes, std::size_t len,
                            std::size_t off, std::size_t n) {
        std::size_t remaining = len > off ? len - off : 0;
        if (remaining < n) {
            throw BeaconError::TruncatedField;
        }
        (void)bytes;
        return off;
    }
    static std::uint8_t read_u8(const std::uint8_t* b, std::size_t len,
                                std::size_t& off) {
        need(b, len, off, 1);
        return b[off++];
    }
    static std::uint16_t read_u16(const std::uint8_t* b, std::size_t len,
                                  std::size_t& off) {
        need(b, len, off, 2);
        std::uint16_t v =
            static_cast<std::uint16_t>(b[off] | (std::uint16_t(b[off + 1]) << 8));
        off += 2;
        return v;
    }
    static std::uint64_t read_u64(const std::uint8_t* b, std::size_t len,
                                  std::size_t& off) {
        need(b, len, off, 8);
        std::uint64_t v = 0;
        for (int i = 0; i < 8; ++i) {
            v |= std::uint64_t(b[off + static_cast<std::size_t>(i)])
                 << (i * 8);
        }
        off += 8;
        return v;
    }
};

struct PanDescriptor {
    std::uint16_t coordinator_pan_id;
    Address coordinator_address;
    std::uint8_t channel;
    std::uint8_t channel_page;
    std::uint8_t link_quality;
    BeaconPayload beacon;

    static PanDescriptor from_beacon_frame(const MacFrame& frame,
                                           std::uint8_t channel,
                                           std::uint8_t channel_page,
                                           std::uint8_t link_quality) {
        if (frame.frame_control.frame_type != FrameType::Beacon) {
            throw BeaconError::ExpectedBeaconFrame;
        }
        if (!frame.source) {
            throw BeaconError::MissingBeaconSourceAddress;
        }
        std::optional<std::uint16_t> pan =
            frame.source_pan_id ? frame.source_pan_id : frame.destination_pan_id;
        if (!pan) {
            throw BeaconError::MissingBeaconPanId;
        }
        PanDescriptor pd;
        pd.coordinator_pan_id = *pan;
        pd.coordinator_address = *frame.source;
        pd.channel = channel;
        pd.channel_page = channel_page;
        pd.link_quality = link_quality;
        pd.beacon = BeaconPayload::parse(frame.payload);
        return pd;
    }
    bool association_permitted() const {
        return beacon.superframe.association_permit();
    }
};

struct PanScanSummary {
    std::uint64_t scanned_at_ms = 0;
    std::vector<PanDescriptor> descriptors;

    bool is_empty() const { return descriptors.empty(); }
    std::size_t len() const { return descriptors.size(); }

    std::vector<const PanDescriptor*> descriptors_for_channel(
        std::uint8_t channel) const {
        std::vector<const PanDescriptor*> out;
        for (const auto& d : descriptors) {
            if (d.channel == channel) {
                out.push_back(&d);
            }
        }
        return out;
    }
    std::vector<const PanDescriptor*> association_candidates() const {
        std::vector<const PanDescriptor*> out;
        for (const auto& d : descriptors) {
            if (d.association_permitted()) {
                out.push_back(&d);
            }
        }
        return out;
    }
    const PanDescriptor* best_association_candidate() const {
        const PanDescriptor* best = nullptr;
        for (const auto& d : descriptors) {
            if (!d.association_permitted()) {
                continue;
            }
            // max_by_key on (link_quality, pan_coordinator); on a tie the last
            // element wins, so replace on >=.
            if (best == nullptr || key_ge(d, *best)) {
                best = &d;
            }
        }
        return best;
    }

  private:
    static bool key_ge(const PanDescriptor& a, const PanDescriptor& b) {
        if (a.link_quality != b.link_quality) {
            return a.link_quality > b.link_quality;
        }
        return static_cast<int>(a.beacon.superframe.pan_coordinator()) >=
               static_cast<int>(b.beacon.superframe.pan_coordinator());
    }
};

}  // namespace ieee802154_core
}  // namespace ca

#endif  // IEEE802154_CORE_HPP
