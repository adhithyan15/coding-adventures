// intel_8008_packager.hpp — Intel HEX ROM image encoder/decoder, header-only in
// pure ISO C++17 (namespace ca::intel_8008_packager). A faithful port of the
// Rust `intel-8008-packager` crate.
// ===========================================================================
//
// Converts raw binary machine code into the Intel HEX format used by EPROM
// programmers, and parses Intel HEX back to binary for round-trip verification.
//
// Record: `:LLAAAATTDD...CC` — start code, byte count, 16-bit big-endian load
// address, record type (00 data / 01 EOF), data bytes, and a checksum (two's
// complement of the field byte-sum, so every record byte sums to 0 mod 256).
//
// DIVERGENCE FROM RUST. `encode_hex` / `decode_hex` throw `PackagerError`
// (a std::runtime_error) where Rust returns `Result::Err`; `encode_hex` returns
// std::string and `decode_hex` returns `DecodedHex { origin, binary }`.
//
// Pure ISO C++17: compiles under GCC, Clang and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors; no <cmath>, no compiler extensions.
#ifndef CA_INTEL_8008_PACKAGER_HPP
#define CA_INTEL_8008_PACKAGER_HPP

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <map>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca {
namespace intel_8008_packager {

// Maximum decoded image span: the Intel 8008's 14-bit address space (16 KB).
inline constexpr std::size_t MAX_IMAGE_SIZE = 0x4000;

// Thrown by encode_hex / decode_hex on any error (mirrors Rust PackagerError).
class PackagerError : public std::runtime_error {
public:
    explicit PackagerError(const std::string& msg) : std::runtime_error(msg) {}
};

// Result of decoding: lowest load address and the assembled payload.
struct DecodedHex {
    std::size_t origin;
    std::vector<std::uint8_t> binary;
};

namespace detail {

inline constexpr std::size_t BYTES_PER_RECORD = 16;
inline constexpr std::uint8_t RECORD_TYPE_DATA = 0x00;
inline constexpr std::uint8_t RECORD_TYPE_EOF = 0x01;
inline constexpr std::size_t MAX_HEX_LINE_LEN = 1024;

// Intel HEX checksum: two's complement of the byte-sum, mod 256.
inline std::uint8_t checksum(const std::uint8_t* fields, std::size_t n) {
    std::uint32_t total = 0;
    for (std::size_t i = 0; i < n; i++) total += fields[i];
    return static_cast<std::uint8_t>((0x100u - (total % 0x100u)) % 0x100u);
}

inline void append_hex(std::string& out, std::uint8_t byte) {
    static const char DIG[] = "0123456789ABCDEF";
    out.push_back(DIG[(byte >> 4) & 0xF]);
    out.push_back(DIG[byte & 0xF]);
}

inline void append_data_record(std::string& out, std::size_t address,
                               const std::uint8_t* chunk, std::size_t len) {
    std::uint8_t n = static_cast<std::uint8_t>(len);
    std::uint8_t addr_hi = static_cast<std::uint8_t>((address >> 8) & 0xFF);
    std::uint8_t addr_lo = static_cast<std::uint8_t>(address & 0xFF);
    std::uint8_t fields[4 + BYTES_PER_RECORD];
    fields[0] = n;
    fields[1] = addr_hi;
    fields[2] = addr_lo;
    fields[3] = RECORD_TYPE_DATA;
    for (std::size_t i = 0; i < len; i++) fields[4 + i] = chunk[i];
    std::uint8_t cs = checksum(fields, 4 + len);

    out.push_back(':');
    append_hex(out, n);
    append_hex(out, addr_hi);
    append_hex(out, addr_lo);
    append_hex(out, RECORD_TYPE_DATA);
    for (std::size_t i = 0; i < len; i++) append_hex(out, chunk[i]);
    append_hex(out, cs);
    out.push_back('\n');
}

inline int hex_val(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

// Parse a hex string; returns false on odd length / non-hex.
inline bool parse_hex_bytes(const char* s, std::size_t slen,
                            std::vector<std::uint8_t>& out) {
    if (slen % 2 != 0) return false;
    out.clear();
    out.reserve(slen / 2);
    for (std::size_t i = 0; i < slen; i += 2) {
        int hi = hex_val(s[i]);
        int lo = hex_val(s[i + 1]);
        if (hi < 0 || lo < 0) return false;
        out.push_back(static_cast<std::uint8_t>(hi * 16 + lo));
    }
    return true;
}

}  // namespace detail

// Encode `binary` (non-empty) to an Intel HEX string loaded at `origin`.
inline std::string encode_hex(const std::vector<std::uint8_t>& binary,
                              std::size_t origin) {
    if (binary.empty()) throw PackagerError("binary must be non-empty");
    if (origin > 0xFFFF) throw PackagerError("origin must be 0-65535");
    // origin <= 0xFFFF so 0x10000-origin >= 1 (no underflow).
    if (binary.size() > 0x10000 - origin)
        throw PackagerError("image overflows 16-bit address space");

    std::string out;
    std::size_t offset = 0;
    while (offset < binary.size()) {
        std::size_t end = std::min(offset + detail::BYTES_PER_RECORD, binary.size());
        detail::append_data_record(out, origin + offset, binary.data() + offset,
                                   end - offset);
        offset = end;
    }
    out += ":00000001FF\n";
    return out;
}

// Decode an Intel HEX string to origin + binary; throws PackagerError on any
// malformed, mis-checksummed, overlapping, over-long, or unterminated input.
inline DecodedHex decode_hex(const std::string& text) {
    std::map<std::size_t, std::vector<std::uint8_t>> segments;  // sorted by addr
    bool found_eof = false;

    std::size_t pos = 0;
    while (pos < text.size()) {
        std::size_t nl = text.find('\n', pos);
        std::size_t end = (nl == std::string::npos) ? text.size() : nl;
        std::size_t line_beg = pos;
        std::size_t line_end = end;
        pos = (nl == std::string::npos) ? text.size() : nl + 1;

        // Trim ASCII whitespace.
        while (line_beg < line_end &&
               static_cast<unsigned char>(text[line_beg]) <= ' ')
            line_beg++;
        while (line_end > line_beg &&
               static_cast<unsigned char>(text[line_end - 1]) <= ' ')
            line_end--;
        std::size_t line_len = line_end - line_beg;
        if (line_len == 0) continue;

        if (line_len > detail::MAX_HEX_LINE_LEN)
            throw PackagerError("line too long");
        if (text[line_beg] != ':')
            throw PackagerError("expected ':' at start of record");

        std::vector<std::uint8_t> rec;
        if (!detail::parse_hex_bytes(text.data() + line_beg + 1, line_len - 1, rec))
            throw PackagerError("invalid hex data");

        if (rec.size() < 5) throw PackagerError("record too short");
        std::size_t byte_count = rec[0];
        std::size_t address = (static_cast<std::size_t>(rec[1]) << 8) | rec[2];
        std::uint8_t rec_type = rec[3];

        if (rec.size() < 4 + byte_count + 1)
            throw PackagerError("record too short");

        std::uint8_t stored_cs = rec[4 + byte_count];
        std::uint8_t computed_cs = detail::checksum(rec.data(), 4 + byte_count);
        if (computed_cs != stored_cs) throw PackagerError("checksum mismatch");

        if (rec_type == detail::RECORD_TYPE_EOF) {
            found_eof = true;
            break;
        }
        if (rec_type != detail::RECORD_TYPE_DATA)
            throw PackagerError("unsupported record type");

        // Reject overlap with the immediate lower / upper neighbour (segments
        // stay non-overlapping, so neighbour checks catch every overlap).
        auto it = segments.upper_bound(address);
        if (it != segments.begin()) {
            auto prev = std::prev(it);
            if (prev->first + prev->second.size() > address)
                throw PackagerError("record overlaps another record");
        }
        if (it != segments.end()) {
            if (address + byte_count > it->first)
                throw PackagerError("record overlaps another record");
        }

        segments.emplace(address, std::vector<std::uint8_t>(rec.begin() + 4,
                                                            rec.begin() + 4 + byte_count));
    }

    if (!found_eof)
        throw PackagerError("missing EOF record (file may be truncated)");

    if (segments.empty()) return DecodedHex{0, {}};

    std::size_t origin = segments.begin()->first;
    std::size_t image_end = origin;
    for (const auto& kv : segments)
        image_end = std::max(image_end, kv.first + kv.second.size());
    std::size_t span = image_end - origin;
    if (span > MAX_IMAGE_SIZE) throw PackagerError("decoded image too large");

    std::vector<std::uint8_t> buffer(span, 0);
    for (const auto& kv : segments) {
        std::size_t start = kv.first - origin;
        std::copy(kv.second.begin(), kv.second.end(), buffer.begin() + static_cast<std::ptrdiff_t>(start));
    }
    return DecodedHex{origin, std::move(buffer)};
}

}  // namespace intel_8008_packager
}  // namespace ca

#endif  // CA_INTEL_8008_PACKAGER_HPP
