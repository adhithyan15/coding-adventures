// dns_message.hpp — the DNS wire-format layer, header-only ISO C++17.
// ===================================================================
//
// A faithful port of the Rust `dns-message` crate, in namespace `ca::dns_message`.
// It turns structured DNS questions and answers into bytes and back; it does NOT
// open sockets, retry, cache, or choose a nameserver.
//
// Wire format (RFC 1035): a 12-byte header (id, packed flag word, four section
// counts) followed by the question / answer / authority / additional sections.
// Names are length-prefixed labels ending in a zero byte; a length byte whose
// top two bits are 11 is a compression pointer. The decoder follows pointers
// under a 128-hop cap (and a 255-byte encoded-name cap), so a malicious message
// can't loop it forever.
//
// Errors are C++ exceptions (`ca::dns_message::Error`). Owned data lives in
// std::vector / std::string, so there is nothing to free by hand.
//
// Pure ISO C++17 — no <cmath>, no compiler extensions.
#ifndef DNS_MESSAGE_HPP
#define DNS_MESSAGE_HPP

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca {
namespace dns_message {

namespace limits {
inline constexpr std::size_t kHeaderLen = 12;
inline constexpr std::size_t kMaxLabelLen = 63;
inline constexpr std::size_t kMaxEncodedNameLen = 255;
inline constexpr std::size_t kMinQuestionWireLen = 5;
inline constexpr std::size_t kMinRecordWireLen = 11;
inline constexpr std::size_t kMaxNamePointerHops = 128;
}  // namespace limits

// ── Errors ───────────────────────────────────────────────────────────────────

enum class ErrorKind {
    TruncatedHeader,
    UnexpectedEof,
    LabelTooLong,        // detail() = length
    NameTooLong,
    PointerOutOfBounds,  // detail() = offset
    PointerLoop,
    NonAsciiLabel,
    InvalidSectionCount,
    Unsupported          // what()/reason carries the specific message
};

class Error : public std::runtime_error {
   public:
    explicit Error(ErrorKind kind, const std::string &message,
                   std::size_t detail = 0)
        : std::runtime_error(message), kind_(kind), detail_(detail) {}
    ErrorKind kind() const noexcept { return kind_; }
    std::size_t detail() const noexcept { return detail_; }

   private:
    ErrorKind kind_;
    std::size_t detail_;
};

namespace detail {
[[noreturn]] inline void fail(ErrorKind k, const std::string &m,
                              std::size_t d = 0) {
    throw Error(k, m, d);
}
}  // namespace detail

// ── DnsName ──────────────────────────────────────────────────────────────────

// A domain name as human-readable ASCII labels (root = empty label list).
struct DnsName {
    std::vector<std::string> labels;

    DnsName() = default;
    explicit DnsName(std::vector<std::string> l) : labels(std::move(l)) {}

    bool is_root() const { return labels.empty(); }

    std::string to_string() const {
        if (labels.empty()) return ".";
        std::string out;
        for (std::size_t i = 0; i < labels.size(); ++i) {
            if (i > 0) out += '.';
            out += labels[i];
        }
        return out;
    }

    bool operator==(const DnsName &o) const { return labels == o.labels; }
    bool operator!=(const DnsName &o) const { return !(*this == o); }

    // Parse a dotted ASCII name ("." and a trailing "." both mean the root).
    static DnsName from_ascii(const std::string &input);
};

// ── Header flags / enums (each an "enum with an Unknown(value)") ─────────────

struct Opcode {
    enum Kind { Query, Unknown } kind = Query;
    std::uint8_t value = 0;  // for Unknown
    bool operator==(const Opcode &o) const {
        return kind == o.kind && (kind != Unknown || value == o.value);
    }
    static Opcode from_bits(std::uint8_t bits) {
        return bits == 0 ? Opcode{Query, 0} : Opcode{Unknown, bits};
    }
    std::uint8_t to_bits() const {
        return kind == Query ? 0 : static_cast<std::uint8_t>(value & 0x0f);
    }
};

struct ResponseCode {
    enum Kind {
        NoError,
        FormatError,
        ServerFailure,
        NameError,
        NotImplemented,
        Refused,
        Unknown
    } kind = NoError;
    std::uint8_t value = 0;  // for Unknown
    bool operator==(const ResponseCode &o) const {
        return kind == o.kind && (kind != Unknown || value == o.value);
    }
    static ResponseCode from_bits(std::uint8_t bits) {
        switch (bits) {
            case 0: return {NoError, 0};
            case 1: return {FormatError, 0};
            case 2: return {ServerFailure, 0};
            case 3: return {NameError, 0};
            case 4: return {NotImplemented, 0};
            case 5: return {Refused, 0};
            default: return {Unknown, bits};
        }
    }
    std::uint8_t to_bits() const {
        switch (kind) {
            case NoError: return 0;
            case FormatError: return 1;
            case ServerFailure: return 2;
            case NameError: return 3;
            case NotImplemented: return 4;
            case Refused: return 5;
            case Unknown: return static_cast<std::uint8_t>(value & 0x0f);
        }
        return 0;
    }
};

struct Flags {
    bool is_response = false;
    Opcode opcode{};
    bool authoritative_answer = false;
    bool truncated = false;
    bool recursion_desired = false;
    bool recursion_available = false;
    ResponseCode response_code{};

    bool operator==(const Flags &o) const {
        return is_response == o.is_response && opcode == o.opcode &&
               authoritative_answer == o.authoritative_answer &&
               truncated == o.truncated &&
               recursion_desired == o.recursion_desired &&
               recursion_available == o.recursion_available &&
               response_code == o.response_code;
    }

    static Flags query() {
        Flags f;
        f.recursion_desired = true;
        return f;
    }
    static Flags parse(std::uint16_t word) {
        Flags f;
        f.is_response = (word & 0x8000) != 0;
        f.opcode = Opcode::from_bits(static_cast<std::uint8_t>((word >> 11) & 0x0f));
        f.authoritative_answer = (word & 0x0400) != 0;
        f.truncated = (word & 0x0200) != 0;
        f.recursion_desired = (word & 0x0100) != 0;
        f.recursion_available = (word & 0x0080) != 0;
        f.response_code = ResponseCode::from_bits(static_cast<std::uint8_t>(word & 0x000f));
        return f;
    }
    std::uint16_t serialize() const {
        std::uint16_t word = 0;
        if (is_response) word |= 0x8000;
        word = static_cast<std::uint16_t>(word | (static_cast<std::uint16_t>(opcode.to_bits()) << 11));
        if (authoritative_answer) word |= 0x0400;
        if (truncated) word |= 0x0200;
        if (recursion_desired) word |= 0x0100;
        if (recursion_available) word |= 0x0080;
        word = static_cast<std::uint16_t>(word | response_code.to_bits());
        return word;
    }
};

struct Header {
    std::uint16_t id = 0;
    Flags flags{};
    std::uint16_t question_count = 0;
    std::uint16_t answer_count = 0;
    std::uint16_t authority_count = 0;
    std::uint16_t additional_count = 0;
};

// ── Record type / class ──────────────────────────────────────────────────────

struct RecordType {
    enum Kind { A, NS, CNAME, SOA, PTR, MX, TXT, AAAA, SRV, Unknown } kind = A;
    std::uint16_t value = 0;  // for Unknown

    bool operator==(const RecordType &o) const {
        return kind == o.kind && (kind != Unknown || value == o.value);
    }
    bool operator!=(const RecordType &o) const { return !(*this == o); }

    static RecordType from_u16(std::uint16_t v) {
        switch (v) {
            case 1: return {A, 0};
            case 2: return {NS, 0};
            case 5: return {CNAME, 0};
            case 6: return {SOA, 0};
            case 12: return {PTR, 0};
            case 15: return {MX, 0};
            case 16: return {TXT, 0};
            case 28: return {AAAA, 0};
            case 33: return {SRV, 0};
            default: return {Unknown, v};
        }
    }
    std::uint16_t to_u16() const {
        switch (kind) {
            case A: return 1;
            case NS: return 2;
            case CNAME: return 5;
            case SOA: return 6;
            case PTR: return 12;
            case MX: return 15;
            case TXT: return 16;
            case AAAA: return 28;
            case SRV: return 33;
            case Unknown: return value;
        }
        return 0;
    }
};

struct Class {
    enum Kind { IN, Unknown } kind = IN;
    std::uint16_t value = 0;  // for Unknown
    bool operator==(const Class &o) const {
        return kind == o.kind && (kind != Unknown || value == o.value);
    }
    static Class from_u16(std::uint16_t v) {
        return v == 1 ? Class{IN, 0} : Class{Unknown, v};
    }
    std::uint16_t to_u16() const { return kind == IN ? 1 : value; }
};

// ── Questions / records ──────────────────────────────────────────────────────

struct Question {
    DnsName name;
    RecordType qtype{};
    Class qclass{};
    bool operator==(const Question &o) const {
        return name == o.name && qtype == o.qtype && qclass == o.qclass;
    }
};

struct SrvRecord {
    std::uint16_t priority = 0;
    std::uint16_t weight = 0;
    std::uint16_t port = 0;
    DnsName target;
    bool operator==(const SrvRecord &o) const {
        return priority == o.priority && weight == o.weight && port == o.port &&
               target == o.target;
    }
};

// The interpreted payload of a resource record (a tagged struct; CNAME and PTR
// both carry a name, which rules out a plain std::variant).
struct RecordData {
    enum Kind { A, AAAA, CNAME, PTR, SRV, RAW } kind = RAW;
    std::array<std::uint8_t, 4> a{};
    std::array<std::uint8_t, 16> aaaa{};
    DnsName name;             // CNAME / PTR
    SrvRecord srv;            // SRV
    std::vector<std::uint8_t> raw;  // RAW

    bool operator==(const RecordData &o) const {
        if (kind != o.kind) return false;
        switch (kind) {
            case A: return a == o.a;
            case AAAA: return aaaa == o.aaaa;
            case CNAME:
            case PTR: return name == o.name;
            case SRV: return srv == o.srv;
            case RAW: return raw == o.raw;
        }
        return false;
    }
};

struct ResourceRecord {
    DnsName name;
    RecordType rrtype{};
    Class rclass{};
    std::uint32_t ttl = 0;
    RecordData data{};
    bool operator==(const ResourceRecord &o) const {
        return name == o.name && rrtype == o.rrtype && rclass == o.rclass &&
               ttl == o.ttl && data == o.data;
    }
};

struct Message {
    Header header{};
    std::vector<Question> questions;
    std::vector<ResourceRecord> answers;
    std::vector<ResourceRecord> authorities;
    std::vector<ResourceRecord> additionals;

    bool is_success() const {
        return header.flags.is_response &&
               header.flags.response_code.kind == ResponseCode::NoError;
    }
    const ResourceRecord *first_answer_of_type(RecordType qtype) const {
        for (const auto &r : answers)
            if (r.rrtype == qtype) return &r;
        return nullptr;
    }
    std::vector<std::array<std::uint8_t, 4>> ipv4_answers() const {
        std::vector<std::array<std::uint8_t, 4>> out;
        for (const auto &r : answers)
            if (r.data.kind == RecordData::A) out.push_back(r.data.a);
        return out;
    }
    std::vector<std::array<std::uint8_t, 16>> ipv6_answers() const {
        std::vector<std::array<std::uint8_t, 16>> out;
        for (const auto &r : answers)
            if (r.data.kind == RecordData::AAAA) out.push_back(r.data.aaaa);
        return out;
    }
};

// ── Validation helpers ───────────────────────────────────────────────────────

namespace detail {
inline void validate_label(const std::uint8_t *label, std::size_t len) {
    if (len > limits::kMaxLabelLen)
        fail(ErrorKind::LabelTooLong, "label too long", len);
    for (std::size_t i = 0; i < len; ++i)
        if (label[i] > 0x7f) fail(ErrorKind::NonAsciiLabel, "non-ASCII label");
}
inline void validate_encoded_name_len(const DnsName &name) {
    std::size_t total = 1;
    for (const auto &label : name.labels) {
        if (label.size() > limits::kMaxLabelLen)
            fail(ErrorKind::LabelTooLong, "label too long", label.size());
        total += 1 + label.size();
    }
    if (total > limits::kMaxEncodedNameLen)
        fail(ErrorKind::NameTooLong, "name too long");
}
}  // namespace detail

inline DnsName DnsName::from_ascii(const std::string &input) {
    std::size_t trimmed = input.size();
    while (trimmed > 0 && input[trimmed - 1] == '.') --trimmed;
    if (input == "." || trimmed == 0) return DnsName{};

    DnsName name;
    std::size_t start = 0;
    for (std::size_t i = 0; i <= trimmed; ++i) {
        if (i == trimmed || input[i] == '.') {
            std::size_t ll = i - start;
            if (ll == 0)
                detail::fail(ErrorKind::Unsupported, "empty DNS label");
            detail::validate_label(
                reinterpret_cast<const std::uint8_t *>(input.data() + start),
                ll);
            name.labels.emplace_back(input, start, ll);
            start = i + 1;
        }
    }
    detail::validate_encoded_name_len(name);
    return name;
}

// ── Reader ───────────────────────────────────────────────────────────────────

namespace detail {

class Reader {
   public:
    Reader(const std::uint8_t *data, std::size_t len) : data_(data), len_(len) {}

    std::size_t pos() const { return pos_; }
    void set_pos(std::size_t p) { pos_ = p; }
    bool at_end() const { return pos_ >= len_; }

    std::uint16_t u16() {
        if (len_ - pos_ < 2) fail(ErrorKind::UnexpectedEof, "unexpected eof");
        std::uint16_t v = static_cast<std::uint16_t>(
            (static_cast<std::uint16_t>(data_[pos_]) << 8) | data_[pos_ + 1]);
        pos_ += 2;
        return v;
    }
    std::uint32_t u32() {
        if (len_ - pos_ < 4) fail(ErrorKind::UnexpectedEof, "unexpected eof");
        std::uint32_t v = (static_cast<std::uint32_t>(data_[pos_]) << 24) |
                          (static_cast<std::uint32_t>(data_[pos_ + 1]) << 16) |
                          (static_cast<std::uint32_t>(data_[pos_ + 2]) << 8) |
                          static_cast<std::uint32_t>(data_[pos_ + 3]);
        pos_ += 4;
        return v;
    }

    // Read a (possibly compressed) name, advancing pos_ past it in the stream.
    //
    // Loop safety: each iteration advances toward one of two hard caps — a label
    // read grows `encoded_len` (<= kMaxEncodedNameLen) and a pointer increments
    // `pointer_hops` (<= kMaxNamePointerHops) — so any pointer chain terminates
    // in a bounded number of steps. (The Rust original also keeps a per-name
    // visited-set; we rely on the two caps instead, which avoids an
    // O(message_len) allocation per name.)
    DnsName name() {
        DnsName out;
        std::size_t offset = pos_;
        std::optional<std::size_t> consumed;
        std::size_t pointer_hops = 0;
        std::size_t encoded_len = 1;

        for (;;) {
            if (offset >= len_) fail(ErrorKind::UnexpectedEof, "unexpected eof");
            std::uint8_t l = data_[offset];
            unsigned top = l & 0xc0u;
            if (top == 0x00u) {
                offset += 1;
                if (l == 0) {
                    pos_ = consumed.value_or(offset);
                    validate_encoded_name_len(out);
                    return out;
                }
                std::size_t label_len = l;
                if (label_len > limits::kMaxLabelLen)
                    fail(ErrorKind::LabelTooLong, "label too long", label_len);
                if (len_ - offset < label_len)
                    fail(ErrorKind::UnexpectedEof, "unexpected eof");
                const std::uint8_t *lb = data_ + offset;
                validate_label(lb, label_len);
                encoded_len += 1 + label_len;
                if (encoded_len > limits::kMaxEncodedNameLen)
                    fail(ErrorKind::NameTooLong, "name too long");
                out.labels.emplace_back(reinterpret_cast<const char *>(lb),
                                        label_len);
                offset += label_len;
            } else if (top == 0xc0u) {
                if (len_ - offset < 2)
                    fail(ErrorKind::UnexpectedEof, "unexpected eof");
                if (!consumed.has_value()) consumed = offset + 2;
                if (++pointer_hops > limits::kMaxNamePointerHops)
                    fail(ErrorKind::PointerLoop, "pointer loop");
                std::size_t pointer =
                    ((static_cast<std::size_t>(l) & 0x3f) << 8) | data_[offset + 1];
                if (pointer >= len_)
                    fail(ErrorKind::PointerOutOfBounds, "pointer out of bounds",
                         pointer);
                offset = pointer;
            } else {
                fail(ErrorKind::Unsupported, "reserved DNS label prefix");
            }
        }
    }

   private:
    const std::uint8_t *data_;
    std::size_t len_;
    std::size_t pos_ = 0;
};

// Read a name that must consume exactly [start, end) of the rdata.
inline DnsName read_single_rdata_name(const std::uint8_t *data, std::size_t len,
                                      std::size_t start, std::size_t end,
                                      const char *trailing_error) {
    Reader r(data, len);
    r.set_pos(start);
    DnsName name = r.name();
    if (r.pos() > end) fail(ErrorKind::UnexpectedEof, "unexpected eof");
    if (r.pos() != end) fail(ErrorKind::Unsupported, trailing_error);
    return name;
}

inline std::size_t section_capacity(std::size_t len, std::size_t cursor,
                                    std::uint16_t count,
                                    std::size_t min_entry_len) {
    std::size_t possible = (len - cursor) / min_entry_len;
    std::size_t c = count;
    return c < possible ? c : possible;
}

inline ResourceRecord parse_one_record(const std::uint8_t *data,
                                       std::size_t len, Reader &r) {
    ResourceRecord out;
    out.name = r.name();
    std::uint16_t t = r.u16();
    std::uint16_t cl = r.u16();
    out.ttl = r.u32();
    std::size_t rdlength = r.u16();
    if (len - r.pos() < rdlength)
        fail(ErrorKind::UnexpectedEof, "unexpected eof");
    std::size_t rdata_start = r.pos();
    std::size_t rdata_end = rdata_start + rdlength;
    out.rrtype = RecordType::from_u16(t);
    out.rclass = Class::from_u16(cl);

    switch (out.rrtype.kind) {
        case RecordType::A:
            if (rdlength != 4)
                fail(ErrorKind::Unsupported, "A record data must be 4 bytes");
            out.data.kind = RecordData::A;
            std::copy(data + rdata_start, data + rdata_start + 4,
                      out.data.a.begin());
            break;
        case RecordType::AAAA:
            if (rdlength != 16)
                fail(ErrorKind::Unsupported, "AAAA record data must be 16 bytes");
            out.data.kind = RecordData::AAAA;
            std::copy(data + rdata_start, data + rdata_start + 16,
                      out.data.aaaa.begin());
            break;
        case RecordType::CNAME:
            out.data.kind = RecordData::CNAME;
            out.data.name = read_single_rdata_name(
                data, len, rdata_start, rdata_end,
                "CNAME record data must contain exactly one DNS name");
            break;
        case RecordType::PTR:
            out.data.kind = RecordData::PTR;
            out.data.name = read_single_rdata_name(
                data, len, rdata_start, rdata_end,
                "PTR record data must contain exactly one DNS name");
            break;
        case RecordType::SRV: {
            if (rdlength < 7)
                fail(ErrorKind::Unsupported,
                     "SRV record data must contain priority, weight, port, and "
                     "target");
            Reader dr(data, len);
            dr.set_pos(rdata_start);
            out.data.kind = RecordData::SRV;
            out.data.srv.priority = dr.u16();
            out.data.srv.weight = dr.u16();
            out.data.srv.port = dr.u16();
            out.data.srv.target = dr.name();
            if (dr.pos() > rdata_end)
                fail(ErrorKind::UnexpectedEof, "unexpected eof");
            if (dr.pos() != rdata_end)
                fail(ErrorKind::Unsupported,
                     "SRV record data must contain exactly priority, weight, "
                     "port, and target");
            break;
        }
        default:
            out.data.kind = RecordData::RAW;
            out.data.raw.assign(data + rdata_start, data + rdata_end);
            break;
    }

    r.set_pos(rdata_end);
    return out;
}

}  // namespace detail

// ── Top-level codec ──────────────────────────────────────────────────────────

// Build a standard recursive single-question query.
inline Message build_query(std::uint16_t id, DnsName name, RecordType qtype) {
    Message m;
    m.header.id = id;
    m.header.flags = Flags::query();
    m.header.question_count = 1;
    Question q;
    q.name = std::move(name);
    q.qtype = qtype;
    q.qclass = Class{Class::IN, 0};
    m.questions.push_back(std::move(q));
    return m;
}

// Parse raw wire bytes into a structured message (throws Error on malformed
// input).
inline Message parse_message(const std::uint8_t *input, std::size_t len) {
    if (len < limits::kHeaderLen)
        detail::fail(ErrorKind::TruncatedHeader, "truncated header");

    detail::Reader r(input, len);
    Message m;
    m.header.id = r.u16();
    m.header.flags = Flags::parse(r.u16());
    m.header.question_count = r.u16();
    m.header.answer_count = r.u16();
    m.header.authority_count = r.u16();
    m.header.additional_count = r.u16();

    m.questions.reserve(detail::section_capacity(
        len, r.pos(), m.header.question_count, limits::kMinQuestionWireLen));
    for (std::uint16_t i = 0; i < m.header.question_count; ++i) {
        Question q;
        q.name = r.name();
        q.qtype = RecordType::from_u16(r.u16());
        q.qclass = Class::from_u16(r.u16());
        m.questions.push_back(std::move(q));
    }

    auto parse_section = [&](std::uint16_t count,
                             std::vector<ResourceRecord> &into) {
        into.reserve(detail::section_capacity(len, r.pos(), count,
                                              limits::kMinRecordWireLen));
        for (std::uint16_t i = 0; i < count; ++i)
            into.push_back(detail::parse_one_record(input, len, r));
    };
    parse_section(m.header.answer_count, m.answers);
    parse_section(m.header.authority_count, m.authorities);
    parse_section(m.header.additional_count, m.additionals);
    return m;
}

inline Message parse_message(const std::vector<std::uint8_t> &input) {
    return parse_message(input.data(), input.size());
}

namespace detail {
inline void write_u16(std::vector<std::uint8_t> &out, std::uint16_t v) {
    out.push_back(static_cast<std::uint8_t>(v >> 8));
    out.push_back(static_cast<std::uint8_t>(v & 0xff));
}
inline void write_u32(std::vector<std::uint8_t> &out, std::uint32_t v) {
    out.push_back(static_cast<std::uint8_t>(v >> 24));
    out.push_back(static_cast<std::uint8_t>((v >> 16) & 0xff));
    out.push_back(static_cast<std::uint8_t>((v >> 8) & 0xff));
    out.push_back(static_cast<std::uint8_t>(v & 0xff));
}
inline void write_name(std::vector<std::uint8_t> &out, const DnsName &name) {
    validate_encoded_name_len(name);
    for (const auto &label : name.labels) {
        validate_label(reinterpret_cast<const std::uint8_t *>(label.data()),
                       label.size());
        out.push_back(static_cast<std::uint8_t>(label.size()));
        out.insert(out.end(), label.begin(), label.end());
    }
    out.push_back(0);
}
inline void write_record(std::vector<std::uint8_t> &out,
                         const ResourceRecord &r) {
    write_name(out, r.name);
    write_u16(out, r.rrtype.to_u16());
    write_u16(out, r.rclass.to_u16());
    write_u32(out, r.ttl);

    std::vector<std::uint8_t> data;
    switch (r.data.kind) {
        case RecordData::A: data.assign(r.data.a.begin(), r.data.a.end()); break;
        case RecordData::AAAA:
            data.assign(r.data.aaaa.begin(), r.data.aaaa.end());
            break;
        case RecordData::CNAME:
        case RecordData::PTR: write_name(data, r.data.name); break;
        case RecordData::SRV:
            write_u16(data, r.data.srv.priority);
            write_u16(data, r.data.srv.weight);
            write_u16(data, r.data.srv.port);
            write_name(data, r.data.srv.target);
            break;
        case RecordData::RAW: data = r.data.raw; break;
    }
    if (data.size() > 0xffff)
        fail(ErrorKind::Unsupported, "record data too large");
    write_u16(out, static_cast<std::uint16_t>(data.size()));
    out.insert(out.end(), data.begin(), data.end());
}
}  // namespace detail

// Serialize a structured message to wire bytes (throws on a structurally
// impossible message). V1 emits uncompressed names.
inline std::vector<std::uint8_t> serialize_message(const Message &m) {
    if (m.questions.size() > 0xffff || m.answers.size() > 0xffff ||
        m.authorities.size() > 0xffff || m.additionals.size() > 0xffff)
        detail::fail(ErrorKind::InvalidSectionCount, "invalid section count");

    std::vector<std::uint8_t> out;
    detail::write_u16(out, m.header.id);
    detail::write_u16(out, m.header.flags.serialize());
    detail::write_u16(out, static_cast<std::uint16_t>(m.questions.size()));
    detail::write_u16(out, static_cast<std::uint16_t>(m.answers.size()));
    detail::write_u16(out, static_cast<std::uint16_t>(m.authorities.size()));
    detail::write_u16(out, static_cast<std::uint16_t>(m.additionals.size()));

    for (const auto &q : m.questions) {
        detail::write_name(out, q.name);
        detail::write_u16(out, q.qtype.to_u16());
        detail::write_u16(out, q.qclass.to_u16());
    }
    for (const auto &r : m.answers) detail::write_record(out, r);
    for (const auto &r : m.authorities) detail::write_record(out, r);
    for (const auto &r : m.additionals) detail::write_record(out, r);
    return out;
}

}  // namespace dns_message
}  // namespace ca

#endif  // DNS_MESSAGE_HPP
