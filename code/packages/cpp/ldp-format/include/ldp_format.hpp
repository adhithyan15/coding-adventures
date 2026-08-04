// ldp_format.hpp — a versioned binary codec for `.ldp` artefacts, C++17.
// ======================================================================
//
// A faithful port of the Rust `ldp-format` crate, in namespace `ca::ldp_format`.
// It reads and writes the LANG22 "profile artefact" binary format — a compact,
// deterministic on-disk representation of a JIT/AOT profiler's observations.
//
// The format (version 1, little-endian throughout):
//
//   Header (32 bytes): magic "LDP\0", version (u16 major, u16 minor),
//     language (16-byte NUL-padded ASCII), flags u32, record_count u32,
//     reserved u32.
//   String table: str_count u32, then per string: length u16, bytes, NUL.
//     All record strings reference this table by u32 index — so a name used
//     many times is stored once.
//   Module records → function records → instruction records, each a fixed
//     layout of the profiler's counters.
//
// DETERMINISM. `write` produces byte-identical output for equal input: the
// string table is built in first-occurrence order during a pre-walk, so the
// same file always serialises to the same bytes.
//
// SAFETY. `read` treats its input as untrusted — every field read is bounds
// checked (throwing `Error` on a truncated buffer or an out-of-range string
// index) and arrays grow incrementally as elements are read, so a corrupt
// record/string count can never drive a huge speculative allocation.
//
// Where the Rust crate returns `Result`, this port throws `ca::ldp_format::Error`
// carrying an `ErrorKind`. Pure ISO C++17.

#ifndef LDP_FORMAT_HPP
#define LDP_FORMAT_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <vector>

namespace ca {
namespace ldp_format {

// ── Enumerations (one byte each on the wire) ─────────────────────────────────
enum class TypeStatus { FullyTyped = 0, PartiallyTyped = 1, Untyped = 2 };
enum class PromotionState { Interp = 0, JITted = 1, Deopted = 2 };
enum class ObservedKind { Uninit = 0, Mono = 1, Poly = 2, Mega = 3 };

// ── Errors ───────────────────────────────────────────────────────────────────
enum class ErrorKind {
    BadMagic,
    UnsupportedMajorVersion,
    UnexpectedEof,
    BadStringIndex,
    BadObservedKind,
    BadTypeStatus,
    BadPromotionState,
    LanguageTooLong,
    LanguageNotAscii,
    StringTableOverflow,
    StringTooLong
};

class Error : public std::runtime_error {
  public:
    Error(ErrorKind kind, const std::string& what)
        : std::runtime_error(what), kind_(kind) {}
    ErrorKind kind() const noexcept { return kind_; }

  private:
    ErrorKind kind_;
};

// ── Data model ───────────────────────────────────────────────────────────────
struct TypeSeen {
    std::string type_name;
    std::uint32_t count = 0;
    bool operator==(const TypeSeen& o) const {
        return type_name == o.type_name && count == o.count;
    }
};

struct InstructionRecord {
    std::uint32_t instr_index = 0;
    std::string opcode;
    std::uint32_t observation_count = 0;
    ObservedKind observed_kind = ObservedKind::Uninit;
    std::uint32_t observation_count_at_promotion = 0;
    std::uint64_t time_to_first_observation_ns = 0;
    std::uint64_t time_to_promotion_ns = 0;
    std::vector<TypeSeen> types_seen;
    bool operator==(const InstructionRecord& o) const {
        return instr_index == o.instr_index && opcode == o.opcode &&
               observation_count == o.observation_count &&
               observed_kind == o.observed_kind &&
               observation_count_at_promotion ==
                   o.observation_count_at_promotion &&
               time_to_first_observation_ns == o.time_to_first_observation_ns &&
               time_to_promotion_ns == o.time_to_promotion_ns &&
               types_seen == o.types_seen;
    }
};

struct FunctionRecord {
    std::string name;
    std::vector<std::string> params;
    std::uint64_t call_count = 0;
    std::uint64_t total_self_time_ns = 0;
    TypeStatus type_status = TypeStatus::Untyped;
    PromotionState promotion_state = PromotionState::Interp;
    std::vector<InstructionRecord> instructions;
    bool operator==(const FunctionRecord& o) const {
        return name == o.name && params == o.params &&
               call_count == o.call_count &&
               total_self_time_ns == o.total_self_time_ns &&
               type_status == o.type_status &&
               promotion_state == o.promotion_state &&
               instructions == o.instructions;
    }
};

struct ModuleRecord {
    std::string name;
    std::vector<FunctionRecord> functions;
    bool operator==(const ModuleRecord& o) const {
        return name == o.name && functions == o.functions;
    }
};

struct Header {
    std::uint16_t version_major = 1;
    std::uint16_t version_minor = 0;
    std::string language;
    std::uint32_t flags = 0;
    bool operator==(const Header& o) const {
        return version_major == o.version_major &&
               version_minor == o.version_minor && language == o.language &&
               flags == o.flags;
    }
};

struct LdpFile {
    Header header;
    std::vector<ModuleRecord> modules;
    bool operator==(const LdpFile& o) const {
        return header == o.header && modules == o.modules;
    }
};

// ── Internals ────────────────────────────────────────────────────────────────
namespace detail {
constexpr std::uint16_t kVersionMajor = 1;
constexpr std::uint16_t kVersionMinor = 0;
constexpr std::size_t kLanguageFieldLen = 16;

inline void put_u16(std::vector<std::uint8_t>& b, std::uint16_t v) {
    b.push_back(static_cast<std::uint8_t>(v & 0xFF));
    b.push_back(static_cast<std::uint8_t>((v >> 8) & 0xFF));
}
inline void put_u32(std::vector<std::uint8_t>& b, std::uint32_t v) {
    for (int i = 0; i < 4; ++i) {
        b.push_back(static_cast<std::uint8_t>((v >> (8 * i)) & 0xFF));
    }
}
inline void put_u64(std::vector<std::uint8_t>& b, std::uint64_t v) {
    for (int i = 0; i < 8; ++i) {
        b.push_back(static_cast<std::uint8_t>((v >> (8 * i)) & 0xFF));
    }
}

// A string-interning table: first-occurrence order, u32 indices.
class StringTable {
  public:
    std::uint32_t intern(const std::string& s) {
        auto it = index_.find(s);
        if (it != index_.end()) {
            return it->second;
        }
        if (strings_.size() >= 0xFFFFFFFFu) {
            throw Error(ErrorKind::StringTableOverflow,
                        "string table exceeds u32::MAX entries");
        }
        if (s.size() > 0xFFFF) {
            throw Error(ErrorKind::StringTooLong,
                        "individual string is " + std::to_string(s.size()) +
                            " bytes; max u16::MAX");
        }
        auto i = static_cast<std::uint32_t>(strings_.size());
        strings_.push_back(s);
        index_.emplace(s, i);
        return i;
    }
    const std::vector<std::string>& strings() const { return strings_; }

  private:
    std::vector<std::string> strings_;
    std::unordered_map<std::string, std::uint32_t> index_;
};

inline std::array<std::uint8_t, kLanguageFieldLen> encode_language(
    const std::string& s) {
    if (s.size() > kLanguageFieldLen) {
        throw Error(ErrorKind::LanguageTooLong,
                    "language tag is " + std::to_string(s.size()) +
                        " bytes; max 16");
    }
    for (unsigned char c : s) {
        if (c > 0x7F) {
            throw Error(ErrorKind::LanguageNotAscii,
                        "language tag contains non-ASCII bytes");
        }
    }
    std::array<std::uint8_t, kLanguageFieldLen> out{};
    for (std::size_t i = 0; i < s.size(); ++i) {
        out[i] = static_cast<std::uint8_t>(s[i]);
    }
    return out;
}

// Bounds-checked little-endian cursor over an untrusted buffer.
class ByteReader {
  public:
    ByteReader(const std::uint8_t* data, std::size_t len)
        : data_(data), len_(len), pos_(0) {}

    void read_exact(std::uint8_t* out, std::size_t n, const char* context) {
        if (len_ - pos_ < n) {
            throw Error(ErrorKind::UnexpectedEof,
                        std::string("unexpected EOF in ") + context);
        }
        for (std::size_t i = 0; i < n; ++i) {
            out[i] = data_[pos_ + i];
        }
        pos_ += n;
    }
    std::uint8_t read_u8(const char* c) {
        std::uint8_t b;
        read_exact(&b, 1, c);
        return b;
    }
    std::uint16_t read_u16(const char* c) {
        std::uint8_t b[2];
        read_exact(b, 2, c);
        return static_cast<std::uint16_t>(b[0] | (b[1] << 8));
    }
    std::uint32_t read_u32(const char* c) {
        std::uint8_t b[4];
        read_exact(b, 4, c);
        return static_cast<std::uint32_t>(b[0]) |
               (static_cast<std::uint32_t>(b[1]) << 8) |
               (static_cast<std::uint32_t>(b[2]) << 16) |
               (static_cast<std::uint32_t>(b[3]) << 24);
    }
    std::uint64_t read_u64(const char* c) {
        std::uint8_t b[8];
        read_exact(b, 8, c);
        std::uint64_t v = 0;
        for (int i = 0; i < 8; ++i) {
            v |= static_cast<std::uint64_t>(b[i]) << (8 * i);
        }
        return v;
    }

  private:
    const std::uint8_t* data_;
    std::size_t len_;
    std::size_t pos_;
};

inline ObservedKind decode_observed_kind(std::uint8_t b) {
    if (b <= 3) {
        return static_cast<ObservedKind>(b);
    }
    throw Error(ErrorKind::BadObservedKind,
                "bad observed_kind byte: " + std::to_string(b));
}
inline TypeStatus decode_type_status(std::uint8_t b) {
    if (b <= 2) {
        return static_cast<TypeStatus>(b);
    }
    throw Error(ErrorKind::BadTypeStatus,
                "bad type_status byte: " + std::to_string(b));
}
inline PromotionState decode_promotion_state(std::uint8_t b) {
    if (b <= 2) {
        return static_cast<PromotionState>(b);
    }
    throw Error(ErrorKind::BadPromotionState,
                "bad promotion_state byte: " + std::to_string(b));
}

inline const std::string& lookup_str(const std::vector<std::string>& strings,
                                     std::uint32_t idx) {
    if (idx >= strings.size()) {
        throw Error(ErrorKind::BadStringIndex,
                    "string index " + std::to_string(idx) +
                        " out of range (table_len=" +
                        std::to_string(strings.size()) + ")");
    }
    return strings[idx];
}

}  // namespace detail

// ── Write ────────────────────────────────────────────────────────────────────
inline std::vector<std::uint8_t> write(const LdpFile& file) {
    using namespace detail;
    StringTable table;

    // Pre-walk to populate the string table in first-occurrence order.
    for (const auto& m : file.modules) {
        table.intern(m.name);
        for (const auto& f : m.functions) {
            table.intern(f.name);
            for (const auto& p : f.params) {
                table.intern(p);
            }
            for (const auto& instr : f.instructions) {
                table.intern(instr.opcode);
                for (const auto& ts : instr.types_seen) {
                    table.intern(ts.type_name);
                }
            }
        }
    }

    std::vector<std::uint8_t> out;
    // Header.
    out.push_back('L');
    out.push_back('D');
    out.push_back('P');
    out.push_back(0);
    put_u16(out, kVersionMajor);
    put_u16(out, kVersionMinor);
    auto lang = encode_language(file.header.language);
    out.insert(out.end(), lang.begin(), lang.end());
    put_u32(out, file.header.flags);
    if (file.modules.size() > 0xFFFFFFFFu) {
        throw Error(ErrorKind::StringTableOverflow, "too many modules");
    }
    put_u32(out, static_cast<std::uint32_t>(file.modules.size()));
    put_u32(out, 0);  // reserved

    // String table.
    put_u32(out, static_cast<std::uint32_t>(table.strings().size()));
    for (const auto& s : table.strings()) {
        put_u16(out, static_cast<std::uint16_t>(s.size()));
        out.insert(out.end(), s.begin(), s.end());
        out.push_back(0);  // NUL terminator
    }

    // Module records.
    for (const auto& m : file.modules) {
        put_u32(out, table.intern(m.name));
        put_u32(out, static_cast<std::uint32_t>(m.functions.size()));
        for (const auto& f : m.functions) {
            put_u32(out, table.intern(f.name));
            std::uint8_t param_count =
                f.params.size() > 0xFF
                    ? 0xFF
                    : static_cast<std::uint8_t>(f.params.size());
            out.push_back(param_count);
            out.push_back(0);
            out.push_back(0);
            out.push_back(0);
            for (const auto& p : f.params) {
                put_u32(out, table.intern(p));
            }
            put_u64(out, f.call_count);
            put_u64(out, f.total_self_time_ns);
            out.push_back(static_cast<std::uint8_t>(f.type_status));
            out.push_back(static_cast<std::uint8_t>(f.promotion_state));
            out.push_back(0);
            out.push_back(0);
            put_u32(out, static_cast<std::uint32_t>(f.instructions.size()));
            for (const auto& instr : f.instructions) {
                put_u32(out, instr.instr_index);
                put_u32(out, table.intern(instr.opcode));
                put_u32(out, instr.observation_count);
                out.push_back(static_cast<std::uint8_t>(instr.observed_kind));
                out.push_back(0);
                out.push_back(0);
                out.push_back(0);
                put_u32(out, instr.observation_count_at_promotion);
                put_u64(out, instr.time_to_first_observation_ns);
                put_u64(out, instr.time_to_promotion_ns);
                put_u32(out,
                        static_cast<std::uint32_t>(instr.types_seen.size()));
                for (const auto& ts : instr.types_seen) {
                    put_u32(out, table.intern(ts.type_name));
                    put_u32(out, ts.count);
                }
                put_u32(out, 0);  // ic_entry_count, reserved in v1
            }
        }
    }
    return out;
}

// ── Read ─────────────────────────────────────────────────────────────────────
inline LdpFile read(const std::uint8_t* data, std::size_t len) {
    using namespace detail;
    ByteReader r(data, len);

    std::uint8_t magic[4];
    r.read_exact(magic, 4, "magic");
    if (magic[0] != 'L' || magic[1] != 'D' || magic[2] != 'P' ||
        magic[3] != 0) {
        throw Error(ErrorKind::BadMagic, "bad magic");
    }
    std::uint16_t version_major = r.read_u16("version_major");
    std::uint16_t version_minor = r.read_u16("version_minor");
    if (version_major != kVersionMajor) {
        throw Error(ErrorKind::UnsupportedMajorVersion,
                    "unsupported major version: got " +
                        std::to_string(version_major));
    }
    std::uint8_t lang_bytes[kLanguageFieldLen];
    r.read_exact(lang_bytes, kLanguageFieldLen, "language");
    std::size_t lang_end = 0;
    while (lang_end < kLanguageFieldLen && lang_bytes[lang_end] != 0) {
        ++lang_end;
    }
    std::string language(reinterpret_cast<const char*>(lang_bytes), lang_end);
    std::uint32_t flags = r.read_u32("flags");
    std::uint32_t record_count = r.read_u32("record_count");
    (void)r.read_u32("reserved");

    // String table (grow incrementally — never pre-allocate from the count).
    std::uint32_t str_count = r.read_u32("str_count");
    std::vector<std::string> strings;
    for (std::uint32_t i = 0; i < str_count; ++i) {
        std::uint16_t slen = r.read_u16("string length");
        std::string s(slen, '\0');
        if (slen > 0) {
            r.read_exact(reinterpret_cast<std::uint8_t*>(&s[0]), slen,
                         "string bytes");
        }
        (void)r.read_u8("string NUL terminator");
        strings.push_back(std::move(s));
    }

    LdpFile file;
    file.header.version_major = version_major;
    file.header.version_minor = version_minor;
    file.header.language = std::move(language);
    file.header.flags = flags;

    for (std::uint32_t mi = 0; mi < record_count; ++mi) {
        ModuleRecord module;
        std::uint32_t module_name_idx = r.read_u32("module_name_idx");
        module.name = lookup_str(strings, module_name_idx);
        std::uint32_t function_count = r.read_u32("function_count");
        for (std::uint32_t fi = 0; fi < function_count; ++fi) {
            FunctionRecord fn;
            std::uint32_t function_name_idx = r.read_u32("function_name_idx");
            std::uint8_t param_count = r.read_u8("param_count");
            std::uint8_t pad3[3];
            r.read_exact(pad3, 3, "function _pad");
            for (std::uint8_t p = 0; p < param_count; ++p) {
                std::uint32_t idx = r.read_u32("param type_idx");
                fn.params.push_back(lookup_str(strings, idx));
            }
            fn.call_count = r.read_u64("call_count");
            fn.total_self_time_ns = r.read_u64("total_self_time_ns");
            std::uint8_t ts_byte = r.read_u8("type_status");
            std::uint8_t ps_byte = r.read_u8("promotion_state");
            std::uint8_t pad2[2];
            r.read_exact(pad2, 2, "function _pad2");
            std::uint32_t instr_count = r.read_u32("instr_count");
            for (std::uint32_t ii = 0; ii < instr_count; ++ii) {
                InstructionRecord instr;
                instr.instr_index = r.read_u32("instr_index");
                std::uint32_t opcode_idx = r.read_u32("opcode_idx");
                instr.observation_count = r.read_u32("observation_count");
                std::uint8_t kind_byte = r.read_u8("observed_kind");
                std::uint8_t pad3b[3];
                r.read_exact(pad3b, 3, "instr _pad");
                instr.observation_count_at_promotion =
                    r.read_u32("observation_count_at_promotion");
                instr.time_to_first_observation_ns =
                    r.read_u64("time_to_first_observation_ns");
                instr.time_to_promotion_ns = r.read_u64("time_to_promotion_ns");
                std::uint32_t types_seen_count = r.read_u32("types_seen_count");
                for (std::uint32_t t = 0; t < types_seen_count; ++t) {
                    std::uint32_t type_idx = r.read_u32("type_idx");
                    std::uint32_t type_count = r.read_u32("type_count");
                    TypeSeen ts;
                    ts.type_name = lookup_str(strings, type_idx);
                    ts.count = type_count;
                    instr.types_seen.push_back(std::move(ts));
                }
                (void)r.read_u32("ic_entry_count");
                instr.opcode = lookup_str(strings, opcode_idx);
                instr.observed_kind = decode_observed_kind(kind_byte);
                fn.instructions.push_back(std::move(instr));
            }
            fn.name = lookup_str(strings, function_name_idx);
            fn.type_status = decode_type_status(ts_byte);
            fn.promotion_state = decode_promotion_state(ps_byte);
            module.functions.push_back(std::move(fn));
        }
        file.modules.push_back(std::move(module));
    }
    return file;
}

inline LdpFile read(const std::vector<std::uint8_t>& data) {
    return read(data.data(), data.size());
}

}  // namespace ldp_format
}  // namespace ca

#endif  // LDP_FORMAT_HPP
