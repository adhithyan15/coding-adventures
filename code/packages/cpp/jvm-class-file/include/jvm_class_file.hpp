// jvm_class_file.hpp — a small JVM class-file parser + builder, header-only C++17.
// ==============================================================================
//
// A faithful port of the Rust `jvm-class-file` crate, in namespace
// `ca::jvm_class_file`. It does two jobs:
//
//   1. parse a deliberately small, boring subset of the JVM class-file format
//   2. build a minimal one-method class file for tests and bootstrap tooling
//
// The parser is intentionally CONSERVATIVE: when the bytes ask for something it
// does not understand — or an attacker-controlled length runs past the end of
// the buffer — it throws `Error` instead of guessing. Every read goes through a
// bounds-checked cursor (`ClassReader`), so malformed input can never read out
// of bounds.
//
// Pure ISO C++17: <cstdint>, <optional>, <stdexcept>, <string>, <vector>.

#ifndef JVM_CLASS_FILE_HPP
#define JVM_CLASS_FILE_HPP

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace ca {
namespace jvm_class_file {

// ── Access flags & opcode constants (re-exported from the crate) ─────────────
constexpr std::uint16_t kAccPublic = 0x0001;
constexpr std::uint16_t kAccStatic = 0x0008;
constexpr std::uint16_t kAccSuper = 0x0020;

constexpr std::uint8_t kAconstNull = 0x01;
constexpr std::uint8_t kAload = 0x19;
constexpr std::uint8_t kAstore = 0x3A;
constexpr std::uint8_t kDup = 0x59;
constexpr std::uint8_t kAaload = 0x32;
constexpr std::uint8_t kAastore = 0x53;
constexpr std::uint8_t kAnewarray = 0xBD;
constexpr std::uint8_t kIfnull = 0xC6;
constexpr std::uint8_t kIfnonnull = 0xC7;
constexpr std::uint8_t kGoto = 0xA7;
constexpr std::uint8_t kSwap = 0x5F;

// ── Error ────────────────────────────────────────────────────────────────────
class Error : public std::runtime_error {
  public:
    explicit Error(const std::string& message) : std::runtime_error(message) {}
};

// ── Version ──────────────────────────────────────────────────────────────────
struct Version {
    std::uint16_t major = 0;
    std::uint16_t minor = 0;
    bool operator==(const Version& o) const noexcept {
        return major == o.major && minor == o.minor;
    }
};

// ── Constant-pool entry (a tagged union mirroring the Rust enum) ─────────────
struct ConstantPoolEntry {
    enum class Kind {
        Utf8,
        Integer,
        Long,
        Double,
        Class,
        String,
        NameAndType,
        Fieldref,
        Methodref
    };
    Kind kind;
    std::string utf8;         // Utf8
    std::int32_t integer = 0; // Integer
    std::int64_t long_v = 0;  // Long
    double double_v = 0.0;    // Double
    std::uint16_t a = 0;      // Class:name / String:string / NameAndType:name /
                              // Fieldref/Methodref:class
    std::uint16_t b = 0;      // NameAndType:descriptor / *ref:name_and_type

    static ConstantPoolEntry Utf8(std::string s) {
        ConstantPoolEntry e;
        e.kind = Kind::Utf8;
        e.utf8 = std::move(s);
        return e;
    }
    static ConstantPoolEntry Integer(std::int32_t v) {
        ConstantPoolEntry e;
        e.kind = Kind::Integer;
        e.integer = v;
        return e;
    }
    static ConstantPoolEntry Long(std::int64_t v) {
        ConstantPoolEntry e;
        e.kind = Kind::Long;
        e.long_v = v;
        return e;
    }
    static ConstantPoolEntry Double(double v) {
        ConstantPoolEntry e;
        e.kind = Kind::Double;
        e.double_v = v;
        return e;
    }
    static ConstantPoolEntry Class(std::uint16_t name_index) {
        ConstantPoolEntry e;
        e.kind = Kind::Class;
        e.a = name_index;
        return e;
    }
    static ConstantPoolEntry String(std::uint16_t string_index) {
        ConstantPoolEntry e;
        e.kind = Kind::String;
        e.a = string_index;
        return e;
    }
    static ConstantPoolEntry NameAndType(std::uint16_t name_index,
                                         std::uint16_t descriptor_index) {
        ConstantPoolEntry e;
        e.kind = Kind::NameAndType;
        e.a = name_index;
        e.b = descriptor_index;
        return e;
    }
    static ConstantPoolEntry Fieldref(std::uint16_t class_index,
                                      std::uint16_t nat_index) {
        ConstantPoolEntry e;
        e.kind = Kind::Fieldref;
        e.a = class_index;
        e.b = nat_index;
        return e;
    }
    static ConstantPoolEntry Methodref(std::uint16_t class_index,
                                       std::uint16_t nat_index) {
        ConstantPoolEntry e;
        e.kind = Kind::Methodref;
        e.a = class_index;
        e.b = nat_index;
        return e;
    }
};

// ── Resolved-constant (the "loadable" projection) ────────────────────────────
struct ResolvedConstant {
    enum class Kind { Utf8, Integer, Long, Double, String } kind;
    std::string text;         // Utf8 / String
    std::int32_t integer = 0; // Integer
    std::int64_t long_v = 0;  // Long
    double double_v = 0.0;    // Double

    static ResolvedConstant Utf8(std::string s) {
        return {Kind::Utf8, std::move(s), 0, 0, 0.0};
    }
    static ResolvedConstant Integer(std::int32_t v) {
        return {Kind::Integer, "", v, 0, 0.0};
    }
    static ResolvedConstant Long(std::int64_t v) {
        return {Kind::Long, "", 0, v, 0.0};
    }
    static ResolvedConstant Double(double v) {
        return {Kind::Double, "", 0, 0, v};
    }
    static ResolvedConstant String(std::string s) {
        return {Kind::String, std::move(s), 0, 0, 0.0};
    }
    bool operator==(const ResolvedConstant& o) const noexcept {
        if (kind != o.kind) {
            return false;
        }
        switch (kind) {
            case Kind::Utf8:
            case Kind::String: return text == o.text;
            case Kind::Integer: return integer == o.integer;
            case Kind::Long: return long_v == o.long_v;
            case Kind::Double: return double_v == o.double_v;
        }
        return false;
    }
};

struct FieldReference {
    std::string class_name, name, descriptor;
    bool operator==(const FieldReference& o) const noexcept {
        return class_name == o.class_name && name == o.name &&
               descriptor == o.descriptor;
    }
};
struct MethodReference {
    std::string class_name, name, descriptor;
    bool operator==(const MethodReference& o) const noexcept {
        return class_name == o.class_name && name == o.name &&
               descriptor == o.descriptor;
    }
};

struct AttributeInfo {
    std::string name;
    std::vector<std::uint8_t> info;
};
struct CodeAttribute {
    std::string name;
    std::uint16_t max_stack = 0;
    std::uint16_t max_locals = 0;
    std::vector<std::uint8_t> code;
    std::vector<AttributeInfo> nested_attributes;
};
struct MethodAttribute {
    bool is_code = false;
    CodeAttribute code;  // when is_code
    AttributeInfo raw;   // otherwise
};
struct MethodInfo {
    std::uint16_t access_flags = 0;
    std::string name;
    std::string descriptor;
    std::vector<MethodAttribute> attributes;

    const CodeAttribute* code_attribute() const {
        for (const auto& a : attributes) {
            if (a.is_code) {
                return &a.code;
            }
        }
        return nullptr;
    }
};
struct FieldInfo {
    std::uint16_t access_flags = 0;
    std::string name;
    std::string descriptor;
};

// ── ClassReader — a bounds-checked big-endian cursor ─────────────────────────
namespace detail {
class ClassReader {
  public:
    ClassReader(const std::uint8_t* data, std::size_t len)
        : data_(data), len_(len), offset_(0) {}

    std::size_t remaining() const { return len_ - offset_; }

    const std::uint8_t* read(std::size_t length) {
        if (length > remaining()) {
            throw Error("Unexpected end of class file: need " +
                        std::to_string(length) + " bytes, have " +
                        std::to_string(remaining()));
        }
        const std::uint8_t* p = data_ + offset_;
        offset_ += length;
        return p;
    }
    std::uint8_t u1() { return read(1)[0]; }
    std::uint16_t u2() {
        const std::uint8_t* p = read(2);
        return static_cast<std::uint16_t>((std::uint16_t(p[0]) << 8) | p[1]);
    }
    std::uint32_t u4() {
        const std::uint8_t* p = read(4);
        return (std::uint32_t(p[0]) << 24) | (std::uint32_t(p[1]) << 16) |
               (std::uint32_t(p[2]) << 8) | std::uint32_t(p[3]);
    }
    std::int32_t i4() { return static_cast<std::int32_t>(u4()); }
    std::int64_t i8() {
        std::uint64_t hi = u4();
        std::uint64_t lo = u4();
        return static_cast<std::int64_t>((hi << 32) | lo);
    }
    double f8() {
        std::int64_t bits = i8();
        double d;
        std::memcpy(&d, &bits, sizeof d);
        return d;
    }

  private:
    const std::uint8_t* data_;
    std::size_t len_;
    std::size_t offset_;
};
}  // namespace detail

// ── The class file ───────────────────────────────────────────────────────────
class ClassFile {
  public:
    Version version;
    std::uint16_t access_flags = 0;
    std::string this_class_name;
    std::string super_class_name;
    std::vector<std::optional<ConstantPoolEntry>> constant_pool;
    std::vector<FieldInfo> fields;
    std::vector<MethodInfo> methods;

    const std::string& get_utf8(std::uint16_t index) const {
        const ConstantPoolEntry& e = entry(index);
        if (e.kind != ConstantPoolEntry::Kind::Utf8) {
            throw Error("Constant pool entry " + std::to_string(index) +
                        " is not a UTF-8 string");
        }
        return e.utf8;
    }

    std::string resolve_class_name(std::uint16_t index) const {
        const ConstantPoolEntry& e = entry(index);
        if (e.kind != ConstantPoolEntry::Kind::Class) {
            throw Error("Constant pool entry " + std::to_string(index) +
                        " is not a Class entry");
        }
        return get_utf8(e.a);
    }

    std::pair<std::string, std::string> resolve_name_and_type(
        std::uint16_t index) const {
        const ConstantPoolEntry& e = entry(index);
        if (e.kind != ConstantPoolEntry::Kind::NameAndType) {
            throw Error("Constant pool entry " + std::to_string(index) +
                        " is not a NameAndType entry");
        }
        return {get_utf8(e.a), get_utf8(e.b)};
    }

    ResolvedConstant resolve_constant(std::uint16_t index) const {
        const ConstantPoolEntry& e = entry(index);
        switch (e.kind) {
            case ConstantPoolEntry::Kind::Utf8:
                return ResolvedConstant::Utf8(e.utf8);
            case ConstantPoolEntry::Kind::Integer:
                return ResolvedConstant::Integer(e.integer);
            case ConstantPoolEntry::Kind::Long:
                return ResolvedConstant::Long(e.long_v);
            case ConstantPoolEntry::Kind::Double:
                return ResolvedConstant::Double(e.double_v);
            case ConstantPoolEntry::Kind::String:
                return ResolvedConstant::String(get_utf8(e.a));
            default:
                throw Error("Constant pool entry " + std::to_string(index) +
                            " is not a loadable constant");
        }
    }

    FieldReference resolve_fieldref(std::uint16_t index) const {
        const ConstantPoolEntry& e = entry(index);
        if (e.kind != ConstantPoolEntry::Kind::Fieldref) {
            throw Error("Constant pool entry " + std::to_string(index) +
                        " is not a Fieldref entry");
        }
        auto nt = resolve_name_and_type(e.b);
        return {resolve_class_name(e.a), nt.first, nt.second};
    }

    MethodReference resolve_methodref(std::uint16_t index) const {
        const ConstantPoolEntry& e = entry(index);
        if (e.kind != ConstantPoolEntry::Kind::Methodref) {
            throw Error("Constant pool entry " + std::to_string(index) +
                        " is not a Methodref entry");
        }
        auto nt = resolve_name_and_type(e.b);
        return {resolve_class_name(e.a), nt.first, nt.second};
    }

    const MethodInfo* find_method(
        const std::string& name,
        const std::optional<std::string>& descriptor = std::nullopt) const {
        for (const auto& m : methods) {
            if (m.name == name &&
                (!descriptor.has_value() || *descriptor == m.descriptor)) {
                return &m;
            }
        }
        return nullptr;
    }

  private:
    const ConstantPoolEntry& entry(std::uint16_t index) const {
        if (index == 0 ||
            static_cast<std::size_t>(index) >= constant_pool.size()) {
            throw Error("Constant pool index " + std::to_string(index) +
                        " is out of range");
        }
        const auto& slot = constant_pool[index];
        if (!slot.has_value()) {
            throw Error("Constant pool index " + std::to_string(index) +
                        " points at a reserved wide slot");
        }
        return *slot;
    }
};

// ── Parser ───────────────────────────────────────────────────────────────────
namespace detail {

inline const std::string& get_utf8_pool(
    const std::vector<std::optional<ConstantPoolEntry>>& pool,
    std::uint16_t index) {
    if (static_cast<std::size_t>(index) >= pool.size() ||
        !pool[index].has_value()) {
        throw Error("Constant pool entry " + std::to_string(index) +
                    " is out of range");
    }
    const ConstantPoolEntry& e = *pool[index];
    if (e.kind != ConstantPoolEntry::Kind::Utf8) {
        throw Error("Constant pool entry " + std::to_string(index) +
                    " is not a UTF-8 string");
    }
    return e.utf8;
}

inline MethodAttribute parse_attribute(
    ClassReader& reader,
    const std::vector<std::optional<ConstantPoolEntry>>& pool,
    bool allow_code) {
    std::string name = get_utf8_pool(pool, reader.u2());
    std::uint32_t attribute_length = reader.u4();

    if (name == "Code" && allow_code) {
        const std::uint8_t* body = reader.read(attribute_length);
        ClassReader nested(body, attribute_length);
        CodeAttribute code;
        code.name = name;
        code.max_stack = nested.u2();
        code.max_locals = nested.u2();
        std::uint32_t code_length = nested.u4();
        const std::uint8_t* code_bytes = nested.read(code_length);
        code.code.assign(code_bytes, code_bytes + code_length);
        std::uint16_t exception_table_count = nested.u2();
        for (std::uint16_t i = 0; i < exception_table_count; ++i) {
            nested.read(8);
        }
        std::uint16_t nested_count = nested.u2();
        for (std::uint16_t i = 0; i < nested_count; ++i) {
            MethodAttribute inner = parse_attribute(nested, pool, false);
            if (inner.is_code) {
                throw Error("nested Code attributes are not supported");
            }
            code.nested_attributes.push_back(inner.raw);
        }
        if (nested.remaining() != 0) {
            throw Error("trailing bytes inside Code attribute");
        }
        MethodAttribute out;
        out.is_code = true;
        out.code = std::move(code);
        return out;
    }

    const std::uint8_t* info = reader.read(attribute_length);
    MethodAttribute out;
    out.is_code = false;
    out.raw.name = std::move(name);
    out.raw.info.assign(info, info + attribute_length);
    return out;
}

inline MethodInfo parse_method(
    ClassReader& reader,
    const std::vector<std::optional<ConstantPoolEntry>>& pool) {
    MethodInfo m;
    m.access_flags = reader.u2();
    m.name = get_utf8_pool(pool, reader.u2());
    m.descriptor = get_utf8_pool(pool, reader.u2());
    std::uint16_t attributes_count = reader.u2();
    for (std::uint16_t i = 0; i < attributes_count; ++i) {
        m.attributes.push_back(parse_attribute(reader, pool, true));
    }
    return m;
}

}  // namespace detail

inline ClassFile parse_class_file(const std::vector<std::uint8_t>& data) {
    detail::ClassReader reader(data.data(), data.size());
    std::uint32_t magic = reader.u4();
    if (magic != 0xCAFEBABEu) {
        char buf[64];
        std::snprintf(buf, sizeof buf,
                      "Invalid class-file magic: expected 0xCAFEBABE, got "
                      "0x%08X",
                      magic);
        throw Error(buf);
    }

    ClassFile cf;
    cf.version.minor = reader.u2();
    cf.version.major = reader.u2();

    std::size_t constant_pool_count = reader.u2();
    cf.constant_pool.assign(constant_pool_count, std::nullopt);
    std::size_t index = 1;
    while (index < constant_pool_count) {
        std::uint8_t tag = reader.u1();
        switch (tag) {
            case 1: {  // Utf8
                std::uint16_t length = reader.u2();
                const std::uint8_t* bytes = reader.read(length);
                cf.constant_pool[index] = ConstantPoolEntry::Utf8(
                    std::string(reinterpret_cast<const char*>(bytes), length));
                break;
            }
            case 3:  // Integer
                cf.constant_pool[index] =
                    ConstantPoolEntry::Integer(reader.i4());
                break;
            case 5:  // Long — occupies two slots
                cf.constant_pool[index] = ConstantPoolEntry::Long(reader.i8());
                index += 2;
                continue;
            case 6:  // Double — occupies two slots
                cf.constant_pool[index] = ConstantPoolEntry::Double(reader.f8());
                index += 2;
                continue;
            case 7:  // Class
                cf.constant_pool[index] = ConstantPoolEntry::Class(reader.u2());
                break;
            case 8:  // String
                cf.constant_pool[index] = ConstantPoolEntry::String(reader.u2());
                break;
            case 9: {  // Fieldref
                std::uint16_t ci = reader.u2();
                std::uint16_t nt = reader.u2();
                cf.constant_pool[index] = ConstantPoolEntry::Fieldref(ci, nt);
                break;
            }
            case 10: {  // Methodref
                std::uint16_t ci = reader.u2();
                std::uint16_t nt = reader.u2();
                cf.constant_pool[index] = ConstantPoolEntry::Methodref(ci, nt);
                break;
            }
            case 12: {  // NameAndType
                std::uint16_t ni = reader.u2();
                std::uint16_t di = reader.u2();
                cf.constant_pool[index] =
                    ConstantPoolEntry::NameAndType(ni, di);
                break;
            }
            default:
                throw Error("Unsupported constant-pool tag: " +
                            std::to_string(tag));
        }
        index += 1;
    }

    cf.access_flags = reader.u2();
    std::uint16_t this_class_index = reader.u2();
    std::uint16_t super_class_index = reader.u2();
    std::uint16_t interfaces_count = reader.u2();
    for (std::uint16_t i = 0; i < interfaces_count; ++i) {
        reader.u2();
    }

    std::uint16_t fields_count = reader.u2();
    for (std::uint16_t i = 0; i < fields_count; ++i) {
        FieldInfo f;
        f.access_flags = reader.u2();
        f.name = detail::get_utf8_pool(cf.constant_pool, reader.u2());
        f.descriptor = detail::get_utf8_pool(cf.constant_pool, reader.u2());
        std::uint16_t attributes_count = reader.u2();
        for (std::uint16_t a = 0; a < attributes_count; ++a) {
            detail::parse_attribute(reader, cf.constant_pool, false);
        }
        cf.fields.push_back(std::move(f));
    }

    std::uint16_t methods_count = reader.u2();
    for (std::uint16_t i = 0; i < methods_count; ++i) {
        cf.methods.push_back(detail::parse_method(reader, cf.constant_pool));
    }

    std::uint16_t class_attributes_count = reader.u2();
    for (std::uint16_t i = 0; i < class_attributes_count; ++i) {
        detail::parse_attribute(reader, cf.constant_pool, false);
    }

    if (reader.remaining() != 0) {
        throw Error("Trailing bytes after class-file parse: " +
                    std::to_string(reader.remaining()));
    }

    cf.this_class_name = cf.resolve_class_name(this_class_index);
    cf.super_class_name = cf.resolve_class_name(super_class_index);
    return cf;
}

// ── Minimal builder ──────────────────────────────────────────────────────────
struct MinimalClassConstant {
    enum class Kind { Integer, String } kind;
    std::int32_t integer = 0;
    std::string text;
    static MinimalClassConstant Integer(std::int32_t v) {
        return {Kind::Integer, v, ""};
    }
    static MinimalClassConstant String(std::string s) {
        return {Kind::String, 0, std::move(s)};
    }
};

struct BuildMinimalClassFileParams {
    std::string class_name;
    std::string method_name;
    std::string descriptor;
    std::vector<std::uint8_t> code;
    std::uint16_t max_stack = 0;
    std::uint16_t max_locals = 0;
    std::vector<MinimalClassConstant> constants;
    std::uint16_t major_version = 61;
    std::uint16_t minor_version = 0;
    std::uint16_t class_access_flags = kAccPublic | kAccSuper;
    std::uint16_t method_access_flags = kAccPublic | kAccStatic;
    std::string super_class_name = "java/lang/Object";
};

namespace detail {

inline void append_u2(std::vector<std::uint8_t>& b, std::uint16_t v) {
    b.push_back(static_cast<std::uint8_t>(v >> 8));
    b.push_back(static_cast<std::uint8_t>(v & 0xFF));
}
inline void append_u4(std::vector<std::uint8_t>& b, std::uint32_t v) {
    b.push_back(static_cast<std::uint8_t>(v >> 24));
    b.push_back(static_cast<std::uint8_t>((v >> 16) & 0xFF));
    b.push_back(static_cast<std::uint8_t>((v >> 8) & 0xFF));
    b.push_back(static_cast<std::uint8_t>(v & 0xFF));
}
inline void append_i4(std::vector<std::uint8_t>& b, std::int32_t v) {
    append_u4(b, static_cast<std::uint32_t>(v));
}

// The constant-pool builder deduplicates by a string key but emits entries in
// insertion order (so the output is deterministic).
class ConstantPoolBuilder {
  public:
    std::size_t count() const { return entries_.size() + 1; }

    std::vector<std::uint8_t> encode() const {
        std::vector<std::uint8_t> out;
        for (const auto& e : entries_) {
            out.insert(out.end(), e.begin(), e.end());
        }
        return out;
    }

    std::uint16_t utf8(const std::string& value) {
        if (value.size() > 0xFFFF) {
            throw Error("UTF-8 constant exceeds 65535 bytes");
        }
        std::vector<std::uint8_t> payload{1};
        append_u2(payload, static_cast<std::uint16_t>(value.size()));
        payload.insert(payload.end(), value.begin(), value.end());
        return add("Utf8:" + value, payload);
    }
    std::uint16_t integer(std::int32_t value) {
        std::vector<std::uint8_t> payload{3};
        append_i4(payload, value);
        return add("Integer:" + std::to_string(value), payload);
    }
    std::uint16_t class_ref(const std::string& value) {
        std::uint16_t name_index = utf8(value);
        std::vector<std::uint8_t> payload{7};
        append_u2(payload, name_index);
        return add("Class:" + value, payload);
    }
    std::uint16_t string(const std::string& value) {
        std::uint16_t string_index = utf8(value);
        std::vector<std::uint8_t> payload{8};
        append_u2(payload, string_index);
        return add("String:" + value, payload);
    }

  private:
    std::uint16_t add(const std::string& key,
                      const std::vector<std::uint8_t>& payload) {
        for (std::size_t i = 0; i < keys_.size(); ++i) {
            if (keys_[i] == key) {
                return index_[i];
            }
        }
        entries_.push_back(payload);
        // The emitted constant_pool_count is entries + 1, so the entry count
        // must stay <= 0xFFFE for the count to fit in a u16.
        if (entries_.size() > 0xFFFE) {
            throw Error("constant pool exceeds u16 count");
        }
        std::uint16_t idx = static_cast<std::uint16_t>(entries_.size());
        keys_.push_back(key);
        index_.push_back(idx);
        return idx;
    }

    std::vector<std::vector<std::uint8_t>> entries_;
    std::vector<std::string> keys_;
    std::vector<std::uint16_t> index_;
};

}  // namespace detail

inline std::vector<std::uint8_t> build_minimal_class_file(
    const BuildMinimalClassFileParams& params) {
    using detail::append_u2;
    using detail::append_u4;
    if (params.class_name.empty()) {
        throw Error("class name must not be empty");
    }
    if (params.method_name.empty()) {
        throw Error("method name must not be empty");
    }
    if (params.descriptor.empty()) {
        throw Error("descriptor must not be empty");
    }

    detail::ConstantPoolBuilder pool;
    std::uint16_t this_class_index = pool.class_ref(params.class_name);
    std::uint16_t super_class_index = pool.class_ref(
        params.super_class_name.empty() ? "java/lang/Object"
                                        : params.super_class_name);
    std::uint16_t method_name_index = pool.utf8(params.method_name);
    std::uint16_t descriptor_index = pool.utf8(params.descriptor);
    std::uint16_t code_name_index = pool.utf8("Code");

    for (const auto& c : params.constants) {
        if (c.kind == MinimalClassConstant::Kind::Integer) {
            pool.integer(c.integer);
        } else {
            pool.string(c.text);
        }
    }

    std::vector<std::uint8_t> code_body;
    append_u2(code_body, params.max_stack);
    append_u2(code_body, params.max_locals);
    if (params.code.size() > 0xFFFFFFFFu) {
        throw Error("method code exceeds 4 GiB");
    }
    append_u4(code_body, static_cast<std::uint32_t>(params.code.size()));
    code_body.insert(code_body.end(), params.code.begin(), params.code.end());
    append_u2(code_body, 0);
    append_u2(code_body, 0);

    std::vector<std::uint8_t> code_attribute;
    append_u2(code_attribute, code_name_index);
    append_u4(code_attribute, static_cast<std::uint32_t>(code_body.size()));
    code_attribute.insert(code_attribute.end(), code_body.begin(),
                          code_body.end());

    std::vector<std::uint8_t> method_info;
    append_u2(method_info, params.method_access_flags);
    append_u2(method_info, method_name_index);
    append_u2(method_info, descriptor_index);
    append_u2(method_info, 1);
    method_info.insert(method_info.end(), code_attribute.begin(),
                       code_attribute.end());

    std::vector<std::uint8_t> out;
    append_u4(out, 0xCAFEBABEu);
    append_u2(out, params.minor_version);
    append_u2(out, params.major_version);
    append_u2(out, static_cast<std::uint16_t>(pool.count()));
    std::vector<std::uint8_t> pool_bytes = pool.encode();
    out.insert(out.end(), pool_bytes.begin(), pool_bytes.end());
    append_u2(out, params.class_access_flags);
    append_u2(out, this_class_index);
    append_u2(out, super_class_index);
    append_u2(out, 0);  // interfaces
    append_u2(out, 0);  // fields
    append_u2(out, 1);  // methods
    out.insert(out.end(), method_info.begin(), method_info.end());
    append_u2(out, 0);  // class attributes
    return out;
}

}  // namespace jvm_class_file
}  // namespace ca

#endif  // JVM_CLASS_FILE_HPP
