#include "bounded_json.hpp"
#include "iso_test.h"
#include <cstring>
#include <locale>
#include <string>
#include <type_traits>

namespace bj = ca::bounded_json;
bool second_translation_unit();

template<class F> static bool throws(F fn) {
    try { fn(); } catch (const std::exception&) { return true; }
    return false;
}

// An upstream allocator observes REAL requests and fails the chosen one. The
// document must release every earlier request on both valid and invalid input.
class FaultResource final : public std::pmr::memory_resource {
    void* do_allocate(std::size_t bytes, std::size_t alignment) override {
        ++calls;
        if (calls == fail_at) throw std::bad_alloc();
        void* p = std::pmr::new_delete_resource()->allocate(bytes, alignment);
        live += bytes;
        if (live > peak) peak = live;
        return p;
    }
    void do_deallocate(void* p, std::size_t bytes, std::size_t alignment) override {
        live -= bytes;
        std::pmr::new_delete_resource()->deallocate(p, bytes, alignment);
    }
    bool do_is_equal(const std::pmr::memory_resource& other) const noexcept override {
        return this == &other;
    }
public:
    std::size_t calls = 0, live = 0, peak = 0, fail_at = 0;
};

static void reject(std::string_view text, bj::Status expected = bj::Status::syntax) {
    auto result = bj::parse(text);
    ISO_CHECK(!result);
    ISO_CHECK(result.error.kind == expected);
    ISO_CHECK(result.error.offset <= text.size());
}

static void values() {
    static_assert(!std::is_copy_constructible<bj::Document>::value, "unique owner");
    static_assert(!std::is_move_constructible<bj::Document>::value, "stable views");
    static_assert(!std::is_copy_constructible<bj::Value>::value, "immutable nodes");
    std::string input = " {\"a\":[null,true,false,1,1.0,1e0,\"text\"],\"a\":2,\"a\\u0000b\":3} \r\n\t";
    auto result = bj::parse(input);
    ISO_CHECK(result);
    input.assign(input.size(), 'x');
    auto owner = std::move(result.document);
    const auto& root = owner->root();
    ISO_CHECK(root.kind() == bj::Kind::object);
    ISO_CHECK(root.size() == 3);
    ISO_CHECK(owner->node_count() == 11);
    ISO_CHECK(root.member(0).first == "a");
    ISO_CHECK(root.member(1).first == "a");
    ISO_CHECK(root.member(1).second->integer() == 2);
    ISO_CHECK(root.member(2).first == std::string_view("a\0b", 3));
    const auto& array = *root.member(0).second;
    ISO_CHECK(array.size() == 7);
    ISO_CHECK(array.at(0).kind() == bj::Kind::null);
    ISO_CHECK(array.at(1).boolean());
    ISO_CHECK(!array.at(2).boolean());
    ISO_CHECK(array.at(3).kind() == bj::Kind::integer);
    ISO_CHECK(array.at(3).number() == 1.0);
    ISO_CHECK(array.at(4).kind() == bj::Kind::floating);
    ISO_CHECK(array.at(5).kind() == bj::Kind::floating);
    ISO_CHECK(array.at(6).string() == "text");
    ISO_CHECK(throws([&] { array.at(7); }));
    ISO_CHECK(throws([&] { root.member(3); }));
    ISO_CHECK(throws([&] { root.at(0); }));
    ISO_CHECK(throws([&] { array.member(0); }));
    ISO_CHECK(throws([&] { root.boolean(); }));
    ISO_CHECK(throws([&] { root.integer(); }));
    ISO_CHECK(throws([&] { root.number(); }));
    ISO_CHECK(throws([&] { root.string(); }));
    ISO_CHECK(throws([&] { array.at(3).size(); }));
    ISO_CHECK(throws([&] { array.at(4).integer(); }));
    ISO_CHECK(second_translation_unit());
    ISO_CHECK(bj::parse("[]").document->root().size() == 0);
    ISO_CHECK(bj::parse("{}").document->root().size() == 0);
    ISO_CHECK(bj::parse("\"\"").document->root().string().empty());
}

static void grammar() {
    for (const char* bad : {"", " ", "x", "+1", "01", "-01", "-", "--1", ".1", "1.",
            "1e", "1e+", "1e-", "1E", "NaN", "Infinity", "nullx", "null true",
            "tru", "fals", "n", "[", "{", "[1", "[1,]", "{\"a\":}", "{\"a\" 1}",
            "{\"a\":1,}", "{a:1}", "[1 2]", "{\"a\":1 \"b\":2}", "\"", "\"abc",
            "\"\\", "\"\\q\"", "\"\\u0\"", "\"\\u000x\"", "\"\\ux000\"",
            "true\v", "false\f", "\"\n\""}) reject(bad);
    reject(std::string("null\0", 5));
    for (int c = 0; c < 32; ++c) {
        std::string text = "\""; text.push_back(static_cast<char>(c)); text += '"';
        reject(text);
    }
    for (const char* good : {"0", "-0", "1", "-1", "1.25", "-0.5", "1e1", "1E+1", "1e-1", "1234567890"}) {
        auto result = bj::parse(good); ISO_CHECK(result);
    }
    ISO_CHECK(bj::parse("9223372036854775807").document->root().integer() == (std::numeric_limits<std::int64_t>::max)());
    ISO_CHECK(bj::parse("-9223372036854775808").document->root().integer() == (std::numeric_limits<std::int64_t>::min)());
    ISO_CHECK(bj::parse("9223372036854775808").document->root().kind() == bj::Kind::floating);
    ISO_CHECK(bj::parse("-9223372036854775809").document->root().kind() == bj::Kind::floating);
    reject("1e9999", bj::Status::number);
    reject("1e-9999", bj::Status::number);
    auto zero = bj::parse("-0.0");
    ISO_CHECK(zero && zero.document->root().number() == 0);
    const double negative_zero = zero.document->root().number();
    const double expected_zero = -0.0;
    ISO_CHECK(std::memcmp(&negative_zero, &expected_zero, sizeof(double)) == 0);
    ISO_CHECK(bj::parse("4.9406564584124654e-324"));
    ISO_CHECK(bj::parse("1.7976931348623157e308"));
}

static void unicode() {
    auto escaped = bj::parse("\"\\\"\\\\\\/\\b\\f\\n\\r\\t\\u0000\\u007f\\u0080\\u07FF\\u0800\\uFFFF\\uD800\\uDC00\\uDBFF\\uDFFF\"");
    ISO_CHECK(escaped);
    const auto text = escaped.document->root().string();
    ISO_CHECK(text.substr(0, 9) == std::string_view("\"\\/\b\f\n\r\t\0", 9));
    ISO_CHECK(text.substr(text.size() - 4) == std::string_view("\xf4\x8f\xbf\xbf", 4));
    for (const std::string& raw : {std::string("\xc2\x80"), std::string("\xdf\xbf"),
            std::string("\xe0\xa0\x80"), std::string("\xef\xbf\xbf"),
            std::string("\xf0\x90\x80\x80"), std::string("\xf4\x8f\xbf\xbf")}) {
        auto result = bj::parse("\"" + raw + "\"");
        ISO_CHECK(result); ISO_CHECK(result.document->root().string() == raw);
    }
    for (const std::string& raw : {std::string("\x80"), std::string("\xc0\xaf"), std::string("\xc1\xbf"),
            std::string("\xc2"), std::string("\xc2x"), std::string("\xe0\x80\x80"),
            std::string("\xed\xa0\x80"), std::string("\xf0\x80\x80\x80"),
            std::string("\xf4\x90\x80\x80"), std::string("\xf5\x80\x80\x80"), std::string("\xff")})
        reject("\"" + raw + "\"", bj::Status::utf8);
    reject(std::string("\"\xe0", 2), bj::Status::utf8);
    for (const char* bad : {"\"\\uDC00\"", "\"\\uD800\"", "\"\\uD800x\"", "\"\\uD800\\x\"", "\"\\uD800\\u0041\""})
        reject(bad, bj::Status::utf8);
}

static void limits() {
    bj::Options options;
    options.bytes = 3;
    ISO_CHECK(bj::parse("null", options).error.kind == bj::Status::byte_limit);
    options = {}; options.depth = 0;
    ISO_CHECK(bj::parse("1", options));
    ISO_CHECK(bj::parse("[]", options).error.kind == bj::Status::depth_limit);
    options.depth = 1;
    ISO_CHECK(bj::parse("[1]", options));
    ISO_CHECK(bj::parse("[[]]", options).error.kind == bj::Status::depth_limit);
    options = {}; options.nodes = 0;
    ISO_CHECK(bj::parse("1", options).error.kind == bj::Status::node_limit);
    options.nodes = 2;
    ISO_CHECK(bj::parse("[1]", options));
    ISO_CHECK(bj::parse("[1,2]", options).error.kind == bj::Status::node_limit);
    options = {}; options.memory = 0;
    ISO_CHECK(bj::parse("1", options).error.kind == bj::Status::memory_limit);
    options = {}; options.bytes = bj::max_bytes + 1;
    ISO_CHECK(bj::parse("1", options).error.kind == bj::Status::argument);
    options = {}; options.depth = bj::max_depth + 1;
    ISO_CHECK(bj::parse("1", options).error.kind == bj::Status::argument);
    options = {}; options.nodes = bj::max_nodes + 1;
    ISO_CHECK(bj::parse("1", options).error.kind == bj::Status::argument);
    options = {}; options.memory = bj::max_memory + 1;
    ISO_CHECK(bj::parse("1", options).error.kind == bj::Status::argument);
    options = {}; options.upstream = nullptr;
    ISO_CHECK(bj::parse("1", options).error.kind == bj::Status::argument);
    const std::string deep(128, '['), close(128, ']');
    ISO_CHECK(bj::parse(deep + "0" + close));
    ISO_CHECK(bj::parse("[" + deep + "0" + close + "]").error.kind == bj::Status::depth_limit);
    std::string wide = "[0";
    for (unsigned i = 0; i < 65534; ++i) wide += ",0";
    ISO_CHECK(bj::parse(wide + "]"));
    ISO_CHECK(bj::parse(wide + ",0]").error.kind == bj::Status::node_limit);
    ISO_CHECK(bj::parse(std::string(bj::max_bytes + 1, ' ')).error.kind == bj::Status::byte_limit);
    auto valid = bj::parse("\"" + std::string(5000, 'a') + "\"");
    ISO_CHECK(valid);
    options = {}; options.memory = valid.document->peak_allocation_bytes();
    ISO_CHECK(bj::parse("\"" + std::string(5000, 'a') + "\"", options));
    --options.memory;
    ISO_CHECK(bj::parse("\"" + std::string(5000, 'a') + "\"", options).error.kind == bj::Status::memory_limit);
}

static void allocation_faults() {
    const std::string fixture = "{\"" + std::string(100, 'k') + "\":[\"" + std::string(200, 'x') + "\",1,{\"inner\":true}]}";
    for (bool malformed : {false, true}) {
        const std::string input = fixture + (malformed ? "x" : "");
        FaultResource normal;
        bj::Options options; options.upstream = &normal;
        auto result = bj::parse(input, options);
        ISO_CHECK(static_cast<bool>(result) == !malformed);
        if (result) ISO_CHECK(result.document->peak_allocation_bytes() == normal.peak);
        result.document.reset();
        ISO_CHECK(normal.live == 0);
        for (std::size_t fail = 1; fail <= normal.calls; ++fail) {
            FaultResource failing; failing.fail_at = fail;
            options.upstream = &failing;
            auto failed = bj::parse(input, options);
            ISO_CHECK(!failed);
            ISO_CHECK(failed.error.kind == bj::Status::allocation);
            ISO_CHECK(failed.error.offset <= input.size());
            ISO_CHECK(failing.live == 0);
        }
    }
}

int main() {
    values(); grammar(); unicode(); limits(); allocation_faults();
    return ISO_TEST_RESULT();
}
