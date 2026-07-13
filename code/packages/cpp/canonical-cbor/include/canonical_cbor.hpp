// canonical_cbor.hpp — deterministic CBOR (RFC 8949) codec, header-only in pure
// ISO C++17 (namespace ca::canonical_cbor). A faithful port of the Rust
// `canonical-cbor` crate.
// ===========================================================================
//
// Encodes and decodes CBOR values in a canonical (deterministic) profile so
// that decode(encode(v)) round-trips and encode(v) is the same bytes on every
// platform. Profile (RFC 8949 §4.2.3, "length-first map key ordering"):
// definite length only, smallest-form integers, map keys sorted length-first
// then bytewise, no floats, opaque tags, no `undefined`.
//
// VALUE MODEL. `CborValue` is a fully value-semantic type (copyable, equality-
// comparable) built from std::vector / std::string — no pointers, no manual
// memory. A Tag stores its number in `u` and its single inner value as the
// one element of `array` (documented below); every other kind uses only its
// natural field.
//
// DIVERGENCE FROM RUST. `decode` throws `CborException` (carrying a
// `CborError`) in place of the Rust `Result::Err`; `encode` returns
// `std::vector<uint8_t>` (Rust `Vec<u8>`).
//
// Pure ISO C++17: compiles under GCC, Clang and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors; no <cmath>, no compiler extensions.
#ifndef CA_CANONICAL_CBOR_HPP
#define CA_CANONICAL_CBOR_HPP

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <exception>
#include <string>
#include <utility>
#include <vector>

namespace ca {
namespace canonical_cbor {

// Maximum recursion depth accepted by decode() — guards against attacker-
// crafted deeply-nested inputs that would otherwise blow the stack.
inline constexpr std::size_t MAX_DECODE_DEPTH = 128;

// Decoder error kinds (mirror the Rust `CborError` variants).
enum class CborError {
    UnexpectedEof,
    TrailingBytes,
    Reserved,
    Indefinite,
    NonMinimalInteger,
    InvalidUtf8,
    NonCanonicalMapOrder,
    UnsupportedSimple,
    FloatNotSupported,
    TooDeep,
    LengthTooLarge,
};

// Thrown by decode() on any violation of the canonical profile.
class CborException : public std::exception {
public:
    explicit CborException(CborError e) : err_(e) {}
    CborError error() const noexcept { return err_; }
    const char* what() const noexcept override { return "canonical-cbor decode error"; }

private:
    CborError err_;
};

// A CBOR value in the canonical profile.
struct CborValue {
    enum class Type { Unsigned, Negative, Bytes, Text, Array, Map, Tag, Bool, Null };

    Type type = Type::Null;
    std::uint64_t u = 0;  // Unsigned / Negative value; tag number for Tag
    bool boolean = false;
    std::vector<std::uint8_t> bytes;                       // Bytes
    std::string text;                                      // Text (UTF-8)
    std::vector<CborValue> array;                          // Array; Tag inner = array[0]
    std::vector<std::pair<CborValue, CborValue>> map;      // Map

    // ── Factories ────────────────────────────────────────────────────────
    static CborValue unsigned_val(std::uint64_t n) {
        CborValue v;
        v.type = Type::Unsigned;
        v.u = n;
        return v;
    }
    static CborValue negative(std::uint64_t n) {  // encodes -1 - n
        CborValue v;
        v.type = Type::Negative;
        v.u = n;
        return v;
    }
    static CborValue boolean_val(bool b) {
        CborValue v;
        v.type = Type::Bool;
        v.boolean = b;
        return v;
    }
    static CborValue null() { return CborValue{}; }  // default is Null
    static CborValue byte_string(std::vector<std::uint8_t> b) {
        CborValue v;
        v.type = Type::Bytes;
        v.bytes = std::move(b);
        return v;
    }
    static CborValue text_string(std::string s) {
        CborValue v;
        v.type = Type::Text;
        v.text = std::move(s);
        return v;
    }
    static CborValue arr(std::vector<CborValue> items) {
        CborValue v;
        v.type = Type::Array;
        v.array = std::move(items);
        return v;
    }
    static CborValue mapping(std::vector<std::pair<CborValue, CborValue>> entries) {
        CborValue v;
        v.type = Type::Map;
        v.map = std::move(entries);
        return v;
    }
    static CborValue tag(std::uint64_t number, CborValue inner) {
        CborValue v;
        v.type = Type::Tag;
        v.u = number;
        v.array.push_back(std::move(inner));  // inner stored as array[0]
        return v;
    }

    const CborValue& tag_inner() const { return array[0]; }

    // Deep structural equality (order-sensitive on arrays/maps, like Rust).
    bool operator==(const CborValue& o) const {
        if (type != o.type) return false;
        switch (type) {
            case Type::Unsigned:
            case Type::Negative:
                return u == o.u;
            case Type::Bool:
                return boolean == o.boolean;
            case Type::Null:
                return true;
            case Type::Bytes:
                return bytes == o.bytes;
            case Type::Text:
                return text == o.text;
            case Type::Array:
                return array == o.array;
            case Type::Map:
                return map == o.map;
            case Type::Tag:
                return u == o.u && array == o.array;
        }
        return false;
    }
    bool operator!=(const CborValue& o) const { return !(*this == o); }
};

namespace detail {

// Append `arg` big-endian in the shortest form under major type `major`.
inline void write_type_and_argument(std::vector<std::uint8_t>& out,
                                    std::uint8_t major, std::uint64_t arg) {
    std::uint8_t mt = static_cast<std::uint8_t>(major << 5);
    auto push_be = [&](std::uint64_t a, int nbytes) {
        for (int i = nbytes - 1; i >= 0; i--)
            out.push_back(static_cast<std::uint8_t>((a >> (8 * i)) & 0xFF));
    };
    if (arg <= 23) {
        out.push_back(static_cast<std::uint8_t>(mt | static_cast<std::uint8_t>(arg)));
    } else if (arg <= 0xFF) {
        out.push_back(static_cast<std::uint8_t>(mt | 24));
        push_be(arg, 1);
    } else if (arg <= 0xFFFF) {
        out.push_back(static_cast<std::uint8_t>(mt | 25));
        push_be(arg, 2);
    } else if (arg <= 0xFFFFFFFFu) {
        out.push_back(static_cast<std::uint8_t>(mt | 26));
        push_be(arg, 4);
    } else {
        out.push_back(static_cast<std::uint8_t>(mt | 27));
        push_be(arg, 8);
    }
}

inline void encode_into(const CborValue& v, std::vector<std::uint8_t>& out);

inline void encode_map(const CborValue& v, std::vector<std::uint8_t>& out) {
    // Encode each key, stable-sort length-first then bytewise, then emit.
    std::vector<std::pair<std::vector<std::uint8_t>, const CborValue*>> ents;
    ents.reserve(v.map.size());
    for (const auto& kv : v.map) {
        std::vector<std::uint8_t> kb;
        encode_into(kv.first, kb);
        ents.emplace_back(std::move(kb), &kv.second);
    }
    std::stable_sort(ents.begin(), ents.end(), [](const auto& a, const auto& b) {
        if (a.first.size() != b.first.size())
            return a.first.size() < b.first.size();
        return a.first < b.first;  // bytewise lex
    });
    write_type_and_argument(out, 5, static_cast<std::uint64_t>(ents.size()));
    for (const auto& e : ents) {
        out.insert(out.end(), e.first.begin(), e.first.end());
        encode_into(*e.second, out);
    }
}

inline void encode_into(const CborValue& v, std::vector<std::uint8_t>& out) {
    using T = CborValue::Type;
    switch (v.type) {
        case T::Unsigned:
            write_type_and_argument(out, 0, v.u);
            break;
        case T::Negative:
            write_type_and_argument(out, 1, v.u);
            break;
        case T::Bytes:
            write_type_and_argument(out, 2, static_cast<std::uint64_t>(v.bytes.size()));
            out.insert(out.end(), v.bytes.begin(), v.bytes.end());
            break;
        case T::Text:
            write_type_and_argument(out, 3, static_cast<std::uint64_t>(v.text.size()));
            out.insert(out.end(), v.text.begin(), v.text.end());
            break;
        case T::Array:
            write_type_and_argument(out, 4, static_cast<std::uint64_t>(v.array.size()));
            for (const auto& item : v.array) encode_into(item, out);
            break;
        case T::Map:
            encode_map(v, out);
            break;
        case T::Tag:
            write_type_and_argument(out, 6, v.u);
            encode_into(v.array[0], out);
            break;
        case T::Bool:
            out.push_back(v.boolean ? 0xF5 : 0xF4);
            break;
        case T::Null:
            out.push_back(0xF6);
            break;
    }
}

// Same acceptance set as Rust std::str::from_utf8: rejects overlong forms,
// surrogates (U+D800..DFFF), and code points above U+10FFFF.
inline bool utf8_valid(const std::uint8_t* s, std::size_t n) {
    std::size_t i = 0;
    auto cont = [&](std::size_t j) { return (s[j] & 0xC0) == 0x80; };
    while (i < n) {
        std::uint8_t b0 = s[i];
        if (b0 < 0x80) {
            i += 1;
        } else if (b0 >= 0xC2 && b0 <= 0xDF) {
            if (i + 1 >= n || !cont(i + 1)) return false;
            i += 2;
        } else if (b0 == 0xE0) {
            if (i + 2 >= n || s[i + 1] < 0xA0 || s[i + 1] > 0xBF || !cont(i + 2))
                return false;
            i += 3;
        } else if (b0 >= 0xE1 && b0 <= 0xEC) {
            if (i + 2 >= n || !cont(i + 1) || !cont(i + 2)) return false;
            i += 3;
        } else if (b0 == 0xED) {
            if (i + 2 >= n || s[i + 1] < 0x80 || s[i + 1] > 0x9F || !cont(i + 2))
                return false;  // excludes surrogates
            i += 3;
        } else if (b0 >= 0xEE && b0 <= 0xEF) {
            if (i + 2 >= n || !cont(i + 1) || !cont(i + 2)) return false;
            i += 3;
        } else if (b0 == 0xF0) {
            if (i + 3 >= n || s[i + 1] < 0x90 || s[i + 1] > 0xBF || !cont(i + 2) ||
                !cont(i + 3))
                return false;
            i += 4;
        } else if (b0 >= 0xF1 && b0 <= 0xF3) {
            if (i + 3 >= n || !cont(i + 1) || !cont(i + 2) || !cont(i + 3))
                return false;
            i += 4;
        } else if (b0 == 0xF4) {
            if (i + 3 >= n || s[i + 1] < 0x80 || s[i + 1] > 0x8F || !cont(i + 2) ||
                !cont(i + 3))
                return false;  // caps at U+10FFFF
            i += 4;
        } else {
            return false;
        }
    }
    return true;
}

// A cursor over the input; each read throws CborException on failure.
class Cursor {
public:
    Cursor(const std::uint8_t* bytes, std::size_t len)
        : bytes_(bytes), len_(len), pos_(0) {}

    std::uint8_t read_u8() {
        if (pos_ >= len_) throw CborException(CborError::UnexpectedEof);
        return bytes_[pos_++];
    }

    // Read `n` bytes, returning a pointer into the input; checked add.
    const std::uint8_t* read_n(std::size_t n) {
        if (n > static_cast<std::size_t>(-1) - pos_)
            throw CborException(CborError::LengthTooLarge);
        std::size_t end = pos_ + n;
        if (end > len_) throw CborException(CborError::UnexpectedEof);
        const std::uint8_t* p = bytes_ + pos_;
        pos_ = end;
        return p;
    }

    std::size_t remaining() const { return len_ - pos_; }
    std::size_t pos() const { return pos_; }
    const std::uint8_t* base() const { return bytes_; }
    bool at_end() const { return pos_ == len_; }

private:
    const std::uint8_t* bytes_;
    std::size_t len_;
    std::size_t pos_;
};

// Reject a declared length larger than the remaining input before allocating.
inline std::size_t length_within_remaining(std::uint64_t declared,
                                           std::size_t remaining,
                                           std::size_t min_per_unit) {
    if (declared > static_cast<std::uint64_t>(static_cast<std::size_t>(-1)))
        throw CborException(CborError::LengthTooLarge);
    std::size_t d = static_cast<std::size_t>(declared);
    if (d != 0 && min_per_unit > static_cast<std::size_t>(-1) / d)
        throw CborException(CborError::LengthTooLarge);
    if (d * min_per_unit > remaining)
        throw CborException(CborError::LengthTooLarge);
    return d;
}

struct Header {
    std::uint8_t major;
    std::uint8_t info;
    std::uint64_t arg;
};

inline Header read_header(Cursor& c) {
    std::uint8_t b = c.read_u8();
    Header h;
    h.major = static_cast<std::uint8_t>(b >> 5);
    h.info = static_cast<std::uint8_t>(b & 0x1F);
    bool enforce_minimal = (h.major != 7);
    if (h.info <= 23) {
        h.arg = h.info;
        return h;
    }
    if (h.info == 24) {
        std::uint8_t v = c.read_u8();
        if (enforce_minimal && v <= 23)
            throw CborException(CborError::NonMinimalInteger);
        h.arg = v;
        return h;
    }
    if (h.info == 25 || h.info == 26 || h.info == 27) {
        int nbytes = h.info == 25 ? 2 : (h.info == 26 ? 4 : 8);
        const std::uint8_t* bs = c.read_n(static_cast<std::size_t>(nbytes));
        std::uint64_t v = 0;
        for (int i = 0; i < nbytes; i++) v = (v << 8) | bs[i];
        std::uint64_t threshold =
            h.info == 25 ? 0xFFu : (h.info == 26 ? 0xFFFFu : 0xFFFFFFFFu);
        if (enforce_minimal && v <= threshold)
            throw CborException(CborError::NonMinimalInteger);
        h.arg = v;
        return h;
    }
    if (h.info <= 30) throw CborException(CborError::Reserved);
    throw CborException(CborError::Indefinite);
}

// Strict length-first-then-bytewise `<` on key encodings (equal => false).
inline bool key_strictly_less(const std::uint8_t* a, std::size_t alen,
                              const std::uint8_t* b, std::size_t blen) {
    if (alen != blen) return alen < blen;
    for (std::size_t i = 0; i < alen; i++) {
        if (a[i] != b[i]) return a[i] < b[i];
    }
    return false;  // equal
}

inline CborValue read_value(Cursor& c, std::size_t depth);

inline CborValue read_array(Cursor& c, std::size_t depth, std::uint64_t arg) {
    std::size_t count = length_within_remaining(arg, c.remaining(), 1);
    std::vector<CborValue> items;
    items.reserve(count);
    for (std::size_t i = 0; i < count; i++)
        items.push_back(read_value(c, depth + 1));
    return CborValue::arr(std::move(items));
}

inline CborValue read_map(Cursor& c, std::size_t depth, std::uint64_t arg) {
    std::size_t count = length_within_remaining(arg, c.remaining(), 2);
    std::vector<std::pair<CborValue, CborValue>> entries;
    entries.reserve(count);
    std::size_t prev_start = 0, prev_end = 0;
    bool have_prev = false;
    for (std::size_t i = 0; i < count; i++) {
        std::size_t key_start = c.pos();
        CborValue k = read_value(c, depth + 1);
        std::size_t key_end = c.pos();
        CborValue val = read_value(c, depth + 1);
        if (have_prev &&
            !key_strictly_less(c.base() + prev_start, prev_end - prev_start,
                               c.base() + key_start, key_end - key_start))
            throw CborException(CborError::NonCanonicalMapOrder);
        prev_start = key_start;
        prev_end = key_end;
        have_prev = true;
        entries.emplace_back(std::move(k), std::move(val));
    }
    return CborValue::mapping(std::move(entries));
}

inline CborValue read_value(Cursor& c, std::size_t depth) {
    if (depth > MAX_DECODE_DEPTH) throw CborException(CborError::TooDeep);
    Header h = read_header(c);
    switch (h.major) {
        case 0:
            return CborValue::unsigned_val(h.arg);
        case 1:
            return CborValue::negative(h.arg);
        case 2: {
            std::size_t len = length_within_remaining(h.arg, c.remaining(), 1);
            const std::uint8_t* s = c.read_n(len);
            return CborValue::byte_string(std::vector<std::uint8_t>(s, s + len));
        }
        case 3: {
            std::size_t len = length_within_remaining(h.arg, c.remaining(), 1);
            const std::uint8_t* s = c.read_n(len);
            if (!utf8_valid(s, len)) throw CborException(CborError::InvalidUtf8);
            return CborValue::text_string(
                std::string(reinterpret_cast<const char*>(s), len));
        }
        case 4:
            return read_array(c, depth, h.arg);
        case 5:
            return read_map(c, depth, h.arg);
        case 6:
            return CborValue::tag(h.arg, read_value(c, depth + 1));
        case 7:
            switch (h.info) {
                case 20:
                    return CborValue::boolean_val(false);
                case 21:
                    return CborValue::boolean_val(true);
                case 22:
                    return CborValue::null();
                case 25:
                case 26:
                case 27:
                    throw CborException(CborError::FloatNotSupported);
                default:
                    throw CborException(CborError::UnsupportedSimple);
            }
        default:
            throw CborException(CborError::UnsupportedSimple);  // unreachable
    }
}

}  // namespace detail

// Encode a value to canonical CBOR bytes.
inline std::vector<std::uint8_t> encode(const CborValue& v) {
    std::vector<std::uint8_t> out;
    detail::encode_into(v, out);
    return out;
}

// Decode exactly one canonical CBOR item; throws CborException on any
// violation (including trailing bytes).
inline CborValue decode(const std::uint8_t* bytes, std::size_t len) {
    detail::Cursor c(bytes, len);
    CborValue v = detail::read_value(c, 0);
    if (!c.at_end()) throw CborException(CborError::TrailingBytes);
    return v;
}

inline CborValue decode(const std::vector<std::uint8_t>& bytes) {
    return decode(bytes.data(), bytes.size());
}

}  // namespace canonical_cbor
}  // namespace ca

#endif  // CA_CANONICAL_CBOR_HPP
