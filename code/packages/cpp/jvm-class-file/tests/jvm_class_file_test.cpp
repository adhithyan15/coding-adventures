// Tests for jvm-class-file, using the header-only iso_test.h harness (pure ISO).
// Vectors mirror the Rust crate's unit tests.
#include "iso_test.h"

#include <cstdint>
#include <optional>
#include <string>
#include <vector>

#include "jvm_class_file.hpp"

namespace jc = ca::jvm_class_file;
using Bytes = std::vector<std::uint8_t>;

// Hand-assemble a class file containing a Fieldref and a Methodref, using a
// simple sequential (non-deduplicated) constant pool. The parser+resolvers only
// care that refs point at consistent entries.
static Bytes synthetic_class_with_member_refs() {
    auto u2 = [](Bytes& b, std::uint16_t v) {
        b.push_back(static_cast<std::uint8_t>(v >> 8));
        b.push_back(static_cast<std::uint8_t>(v & 0xFF));
    };
    auto u4 = [](Bytes& b, std::uint32_t v) {
        b.push_back(static_cast<std::uint8_t>(v >> 24));
        b.push_back(static_cast<std::uint8_t>((v >> 16) & 0xFF));
        b.push_back(static_cast<std::uint8_t>((v >> 8) & 0xFF));
        b.push_back(static_cast<std::uint8_t>(v & 0xFF));
    };
    auto utf8 = [&](Bytes& b, const std::string& s) {
        b.push_back(1);
        u2(b, static_cast<std::uint16_t>(s.size()));
        b.insert(b.end(), s.begin(), s.end());
    };

    // Constant pool (1-indexed):
    //  1 Utf8 "demo/Refs"      2 Class->1
    //  3 Utf8 "java/lang/Object" 4 Class->3
    //  5 Utf8 "VALUE"   6 Utf8 "I"   7 NameAndType(5,6)   8 Fieldref(2,7)
    //  9 Utf8 "helper" 10 Utf8 "()I" 11 NameAndType(9,10) 12 Methodref(2,11)
    // 13 Utf8 "main"   14 Utf8 "()V" 15 Utf8 "Code"
    Bytes pool;
    utf8(pool, "demo/Refs");
    pool.push_back(7);
    u2(pool, 1);
    utf8(pool, "java/lang/Object");
    pool.push_back(7);
    u2(pool, 3);
    utf8(pool, "VALUE");
    utf8(pool, "I");
    pool.push_back(12);
    u2(pool, 5);
    u2(pool, 6);
    pool.push_back(9);
    u2(pool, 2);
    u2(pool, 7);
    utf8(pool, "helper");
    utf8(pool, "()I");
    pool.push_back(12);
    u2(pool, 9);
    u2(pool, 10);
    pool.push_back(10);
    u2(pool, 2);
    u2(pool, 11);
    utf8(pool, "main");
    utf8(pool, "()V");
    utf8(pool, "Code");

    // Code attribute body: max_stack 0, max_locals 1, code [0xB1], exc 0, attrs 0
    Bytes code_body;
    u2(code_body, 0);
    u2(code_body, 1);
    u4(code_body, 1);
    code_body.push_back(0xB1);
    u2(code_body, 0);
    u2(code_body, 0);
    Bytes code_attr;
    u2(code_attr, 15);  // "Code"
    u4(code_attr, static_cast<std::uint32_t>(code_body.size()));
    code_attr.insert(code_attr.end(), code_body.begin(), code_body.end());

    Bytes method;
    u2(method, jc::kAccPublic | jc::kAccStatic);
    u2(method, 13);  // "main"
    u2(method, 14);  // "()V"
    u2(method, 1);   // 1 attribute
    method.insert(method.end(), code_attr.begin(), code_attr.end());

    Bytes out;
    u4(out, 0xCAFEBABE);
    u2(out, 0);   // minor
    u2(out, 61);  // major
    u2(out, 16);  // constant_pool_count (15 entries + 1)
    out.insert(out.end(), pool.begin(), pool.end());
    u2(out, jc::kAccPublic | jc::kAccSuper);
    u2(out, 2);  // this_class
    u2(out, 4);  // super_class
    u2(out, 0);  // interfaces
    u2(out, 0);  // fields
    u2(out, 1);  // methods
    out.insert(out.end(), method.begin(), method.end());
    u2(out, 0);  // class attributes
    return out;
}

int main() {
    // ── builds and parses a minimal class file ─────────────────────────────
    {
        jc::BuildMinimalClassFileParams params;
        params.class_name = "demo/Example";
        params.method_name = "main";
        params.descriptor = "([Ljava/lang/String;)V";
        params.code = {0xB1};
        params.max_stack = 0;
        params.max_locals = 1;
        params.constants = {jc::MinimalClassConstant::Integer(7),
                            jc::MinimalClassConstant::String("hello")};

        Bytes bytes = jc::build_minimal_class_file(params);
        jc::ClassFile parsed = jc::parse_class_file(bytes);
        ISO_CHECK(parsed.this_class_name == "demo/Example");
        ISO_CHECK(parsed.super_class_name == "java/lang/Object");
        ISO_CHECK_EQ_INT(parsed.version.major, 61);

        const jc::MethodInfo* method = parsed.find_method(
            "main", std::optional<std::string>("([Ljava/lang/String;)V"));
        ISO_CHECK(method != nullptr);
        const jc::CodeAttribute* code = method->code_attribute();
        ISO_CHECK(code != nullptr);
        ISO_CHECK((code->code == Bytes{0xB1}));

        bool has_int7 = false, has_hello = false;
        for (std::size_t i = 0; i < parsed.constant_pool.size(); ++i) {
            const auto& slot = parsed.constant_pool[i];
            if (!slot.has_value()) {
                continue;
            }
            using K = jc::ConstantPoolEntry::Kind;
            if (slot->kind == K::Integer || slot->kind == K::String ||
                slot->kind == K::Utf8) {
                jc::ResolvedConstant rc =
                    parsed.resolve_constant(static_cast<std::uint16_t>(i));
                if (rc == jc::ResolvedConstant::Integer(7)) has_int7 = true;
                if (rc == jc::ResolvedConstant::String("hello")) has_hello = true;
            }
        }
        ISO_CHECK(has_int7);
        ISO_CHECK(has_hello);
    }

    // ── rejects invalid magic ──────────────────────────────────────────────
    {
        bool threw = false;
        try {
            jc::parse_class_file(Bytes{0, 1, 2, 3});
        } catch (const jc::Error& e) {
            threw = true;
            ISO_CHECK(std::string(e.what()).find("Invalid class-file magic") !=
                      std::string::npos);
        }
        ISO_CHECK(threw);
    }

    // ── resolves fieldrefs and methodrefs ──────────────────────────────────
    {
        Bytes bytes = synthetic_class_with_member_refs();
        jc::ClassFile parsed = jc::parse_class_file(bytes);
        std::uint16_t fieldref_index = 0, methodref_index = 0;
        for (std::size_t i = 0; i < parsed.constant_pool.size(); ++i) {
            const auto& slot = parsed.constant_pool[i];
            if (!slot.has_value()) continue;
            if (slot->kind == jc::ConstantPoolEntry::Kind::Fieldref)
                fieldref_index = static_cast<std::uint16_t>(i);
            if (slot->kind == jc::ConstantPoolEntry::Kind::Methodref)
                methodref_index = static_cast<std::uint16_t>(i);
        }
        ISO_CHECK(fieldref_index != 0 && methodref_index != 0);
        jc::FieldReference fr = parsed.resolve_fieldref(fieldref_index);
        ISO_CHECK((fr == jc::FieldReference{"demo/Refs", "VALUE", "I"}));
        jc::MethodReference mr = parsed.resolve_methodref(methodref_index);
        ISO_CHECK((mr == jc::MethodReference{"demo/Refs", "helper", "()I"}));
    }

    // ── a Code attribute may hold a nested (Raw) attribute named "Code" ────
    {
        // Build via the minimal builder, then hand-check we can parse a Code
        // attribute whose body carries one nested attribute.
        auto u2 = [](Bytes& b, std::uint16_t v) {
            b.push_back(static_cast<std::uint8_t>(v >> 8));
            b.push_back(static_cast<std::uint8_t>(v & 0xFF));
        };
        auto u4 = [](Bytes& b, std::uint32_t v) {
            b.push_back(static_cast<std::uint8_t>(v >> 24));
            b.push_back(static_cast<std::uint8_t>((v >> 16) & 0xFF));
            b.push_back(static_cast<std::uint8_t>((v >> 8) & 0xFF));
            b.push_back(static_cast<std::uint8_t>(v & 0xFF));
        };
        auto utf8 = [&](Bytes& b, const std::string& s) {
            b.push_back(1);
            u2(b, static_cast<std::uint16_t>(s.size()));
            b.insert(b.end(), s.begin(), s.end());
        };
        // pool: 1 Utf8 "demo/Nested" 2 Class->1 3 Utf8 "java/lang/Object"
        //       4 Class->3 5 Utf8 "main" 6 Utf8 "()V" 7 Utf8 "Code"
        Bytes pool;
        utf8(pool, "demo/Nested");
        pool.push_back(7);
        u2(pool, 1);
        utf8(pool, "java/lang/Object");
        pool.push_back(7);
        u2(pool, 3);
        utf8(pool, "main");
        utf8(pool, "()V");
        utf8(pool, "Code");

        Bytes nested_body;  // an inner "Code"-named Raw attribute body
        u2(nested_body, 0);
        u2(nested_body, 0);
        u4(nested_body, 1);
        nested_body.push_back(0xB1);
        u2(nested_body, 0);
        u2(nested_body, 0);

        Bytes outer_body;
        u2(outer_body, 0);
        u2(outer_body, 1);
        u4(outer_body, 1);
        outer_body.push_back(0xB1);
        u2(outer_body, 0);
        u2(outer_body, 1);   // 1 nested attribute
        u2(outer_body, 7);   // name "Code"
        u4(outer_body, static_cast<std::uint32_t>(nested_body.size()));
        outer_body.insert(outer_body.end(), nested_body.begin(),
                          nested_body.end());

        Bytes code_attr;
        u2(code_attr, 7);
        u4(code_attr, static_cast<std::uint32_t>(outer_body.size()));
        code_attr.insert(code_attr.end(), outer_body.begin(), outer_body.end());

        Bytes method;
        u2(method, jc::kAccPublic | jc::kAccStatic);
        u2(method, 5);
        u2(method, 6);
        u2(method, 1);
        method.insert(method.end(), code_attr.begin(), code_attr.end());

        Bytes bytes;
        u4(bytes, 0xCAFEBABE);
        u2(bytes, 0);
        u2(bytes, 61);
        u2(bytes, 8);  // 7 entries + 1
        bytes.insert(bytes.end(), pool.begin(), pool.end());
        u2(bytes, jc::kAccPublic | jc::kAccSuper);
        u2(bytes, 2);
        u2(bytes, 4);
        u2(bytes, 0);
        u2(bytes, 0);
        u2(bytes, 1);
        bytes.insert(bytes.end(), method.begin(), method.end());
        u2(bytes, 0);

        jc::ClassFile parsed = jc::parse_class_file(bytes);
        const jc::MethodInfo* m =
            parsed.find_method("main", std::optional<std::string>("()V"));
        ISO_CHECK(m != nullptr);
        const jc::CodeAttribute* code = m->code_attribute();
        ISO_CHECK(code != nullptr);
        ISO_CHECK_EQ_UINT(code->nested_attributes.size(), 1u);
        ISO_CHECK(code->nested_attributes[0].name == "Code");
    }

    // ── a truncated file is rejected, not read out of bounds ───────────────
    {
        bool threw = false;
        try {
            // valid magic, then nothing.
            jc::parse_class_file(Bytes{0xCA, 0xFE, 0xBA, 0xBE});
        } catch (const jc::Error&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    return ISO_TEST_RESULT();
}
