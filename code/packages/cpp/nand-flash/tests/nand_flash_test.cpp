#include "bounded_json.hpp"
#include "erase_limit.hpp"
#include "independent_model.hpp"
#include "iso_test.h"
#include "model_test.hpp"
#include "nand_flash.hpp"

#include <cstdint>
#include <exception>
#include <fstream>
#include <iterator>
#include <limits>
#include <string>
#include <string_view>
#include <vector>

using namespace ca::nand_flash;
using ca::bounded_json::Kind;
using ca::bounded_json::Value;

namespace {
struct FixtureError {};
const Value &key(const Value &value, std::string_view name) {
    if (value.kind() != Kind::object)
        throw FixtureError{};
    const Value *found = nullptr;
    for (std::size_t i = 0; i < value.size(); ++i) {
        auto item = value.member(i);
        if (item.first == name) {
            if (found)
                throw FixtureError{};
            found = item.second;
        }
    }
    if (!found)
        throw FixtureError{};
    return *found;
}
bool has_key(const Value &value, std::string_view name) {
    if (value.kind() != Kind::object)
        return false;
    for (std::size_t i = 0; i < value.size(); ++i)
        if (value.member(i).first == name)
            return true;
    return false;
}
void exact(const Value &value, std::initializer_list<std::string_view> names) {
    if (value.kind() != Kind::object || value.size() != names.size())
        throw FixtureError{};
    for (auto name : names)
        (void)key(value, name);
}
std::int64_t integer(const Value &v) {
    if (v.kind() != Kind::integer)
        throw FixtureError{};
    return v.integer();
}
bool boolean(const Value &v) {
    if (v.kind() != Kind::boolean)
        throw FixtureError{};
    return v.boolean();
}
std::string string(const Value &v) {
    if (v.kind() != Kind::string)
        throw FixtureError{};
    return std::string(v.string());
}
std::vector<std::uint8_t> unhex(const Value &v) {
    std::string s = string(v);
    if (s.size() % 2)
        throw FixtureError{};
    auto digit = [](char c) -> int {
        if (c >= '0' && c <= '9')
            return c - '0';
        if (c >= 'a' && c <= 'f')
            return c - 'a' + 10;
        return -1;
    };
    std::vector<std::uint8_t> bytes;
    for (std::size_t i = 0; i < s.size(); i += 2) {
        int a = digit(s[i]), b = digit(s[i + 1]);
        if (a < 0 || b < 0)
            throw FixtureError{};
        bytes.push_back(static_cast<std::uint8_t>(a * 16 + b));
    }
    return bytes;
}
std::string hex(const std::vector<std::uint8_t> &data) {
    static constexpr char digits[] = "0123456789abcdef";
    std::string result;
    for (auto b : data) {
        result.push_back(digits[b >> 4]);
        result.push_back(digits[b & 15]);
    }
    return result;
}
Snapshot snapshot_value(const Value &v, Geometry g) {
    exact(v, {"pages", "erase_counts", "programmed"});
    const auto &pages = key(v, "pages");
    const auto &counts = key(v, "erase_counts");
    const auto &markers = key(v, "programmed");
    if (pages.kind() != Kind::array || counts.kind() != Kind::array ||
        markers.kind() != Kind::array || pages.size() != static_cast<std::size_t>(g.blocks) ||
        counts.size() != pages.size() || markers.size() != pages.size())
        throw FixtureError{};
    Snapshot out;
    for (std::size_t b = 0; b < pages.size(); ++b) {
        const auto &row = pages.at(b);
        const auto &mr = markers.at(b);
        if (row.kind() != Kind::array || mr.kind() != Kind::array ||
            row.size() != static_cast<std::size_t>(g.pages_per_block) || mr.size() != row.size())
            throw FixtureError{};
        const auto count = integer(counts.at(b));
        if (count < 0)
            throw FixtureError{};
        out.erase_counts.push_back(static_cast<std::uint64_t>(count));
        std::vector<std::string> hexrow;
        std::vector<bool> markrow;
        for (std::size_t p = 0; p < row.size(); ++p) {
            auto bytes = unhex(row.at(p));
            if (bytes.size() != static_cast<std::size_t>(g.page_size_bytes))
                throw FixtureError{};
            hexrow.push_back(hex(bytes));
            markrow.push_back(boolean(mr.at(p)));
        }
        out.pages.push_back(std::move(hexrow));
        out.programmed.push_back(std::move(markrow));
    }
    return out;
}
struct Op {
    std::string name;
    std::int64_t block = 0, page = 0;
    std::vector<std::uint8_t> data;
    bool fail = false;
    std::string error_type, error_message;
    std::string result;
    Snapshot after;
};
struct Fixture {
    Config config;
    Snapshot initial;
    std::vector<Op> ops;
};
void validate_fixture_geometry(const Config &config) {
    const auto g = config.geometry;
    if (g.blocks < 1 || g.blocks > max_blocks || g.pages_per_block < 1 ||
        g.pages_per_block > max_pages_per_block || g.page_size_bytes < 1 ||
        g.page_size_bytes > max_page_size_bytes ||
        (config.max_erase_cycles &&
         (*config.max_erase_cycles < 0 ||
          static_cast<std::uint64_t>(*config.max_erase_cycles) > max_safe_erase_cycles)))
        throw FixtureError{};

    // The bounded dimensions are positive and individually small, so these
    // products are representable before converting into allocation sizes.
    const auto page_count = static_cast<std::uint64_t>(g.blocks) *
                            static_cast<std::uint64_t>(g.pages_per_block);
    const auto page_size = static_cast<std::uint64_t>(g.page_size_bytes);
    if (page_count > max_storage_bytes / page_size)
        throw FixtureError{};
}

void admit_fixture_with_model(const Fixture &fixture) {
    nand_test::Model model(fixture.config);
    if (!(model.snapshot() == fixture.initial))
        throw FixtureError{};

    for (const auto &op : fixture.ops) {
        std::string actual_error;
        std::string actual_result;
        try {
            if (op.name == "read") {
                actual_result = hex(model.read(op.block, op.page));
            } else if (op.name == "program") {
                model.program(op.block, op.page, op.data);
            } else {
                model.erase_block(op.block);
            }
        } catch (nand_test::ModelError error) {
            actual_error = nand_test::model_error_text(error);
        }

        const std::string declared_error =
            op.fail ? op.error_type + ":" + op.error_message : std::string{};
        if (actual_error != declared_error)
            throw FixtureError{};
        if (op.name == "read" && actual_error.empty() && actual_result != op.result)
            throw FixtureError{};
        if (!(model.snapshot() == op.after))
            throw FixtureError{};
    }
}
Fixture parse_fixture(
    std::string_view text, ca::bounded_json::Options options = [] {
        ca::bounded_json::Options o;
        o.bytes = 65536;
        o.depth = 64;
        o.nodes = 16384;
        o.memory = 8u * 1024u * 1024u;
        return o;
    }()) {
    auto parsed = ca::bounded_json::parse(text, options);
    if (!parsed)
        throw FixtureError{};
    const auto &root = parsed.document->root();
    exact(root, {"format", "description", "geometry", "initial_state", "operations",
                 "states_after_operations"});
    if (string(key(root, "format")) != "coding-adventures-nand-flash-v1")
        throw FixtureError{};
    (void)string(key(root, "description"));
    const auto &gj = key(root, "geometry");
    exact(gj, {"block_count", "pages_per_block", "page_size_bytes", "max_erase_cycles"});
    Fixture f;
    f.config.geometry = {integer(key(gj, "block_count")), integer(key(gj, "pages_per_block")),
                         integer(key(gj, "page_size_bytes"))};
    const auto &max = key(gj, "max_erase_cycles");
    if (max.kind() == Kind::integer)
        f.config.max_erase_cycles = max.integer();
    else if (max.kind() != Kind::null)
        throw FixtureError{};
    validate_fixture_geometry(f.config);
    f.initial = snapshot_value(key(root, "initial_state"), f.config.geometry);
    const auto &operations = key(root, "operations");
    const auto &states = key(root, "states_after_operations");
    if (operations.kind() != Kind::array || states.kind() != Kind::array ||
        operations.size() == 0 || states.size() != operations.size())
        throw FixtureError{};
    for (std::size_t i = 0; i < operations.size(); ++i) {
        const auto &oj = operations.at(i);
        if (oj.kind() != Kind::object)
            throw FixtureError{};
        Op op;
        op.name = string(key(oj, "op"));
        if (op.name == "read") {
            exact(oj, {"op", "block", "page", "result"});
            op.block = integer(key(oj, "block"));
            op.page = integer(key(oj, "page"));
            op.result = string(key(oj, "result"));
            if (unhex(key(oj, "result")).size() !=
                static_cast<std::size_t>(f.config.geometry.page_size_bytes))
                throw FixtureError{};
        } else if (op.name == "program") {
            op.block = integer(key(oj, "block"));
            op.page = integer(key(oj, "page"));
            op.data = unhex(key(oj, "data"));
            if (op.data.size() != static_cast<std::size_t>(f.config.geometry.page_size_bytes))
                throw FixtureError{};
            if (has_key(oj, "result")) {
                exact(oj, {"op", "block", "page", "data", "result"});
                if (key(oj, "result").kind() != Kind::null)
                    throw FixtureError{};
            } else {
                exact(oj, {"op", "block", "page", "data", "error"});
                const auto &e = key(oj, "error");
                exact(e, {"type", "message"});
                op.fail = true;
                op.error_type = string(key(e, "type"));
                op.error_message = string(key(e, "message"));
            }
        } else if (op.name == "erase_block") {
            op.block = integer(key(oj, "block"));
            if (has_key(oj, "result")) {
                exact(oj, {"op", "block", "result"});
                if (key(oj, "result").kind() != Kind::null)
                    throw FixtureError{};
            } else {
                exact(oj, {"op", "block", "error"});
                const auto &e = key(oj, "error");
                exact(e, {"type", "message"});
                op.fail = true;
                op.error_type = string(key(e, "type"));
                op.error_message = string(key(e, "message"));
            }
        } else
            throw FixtureError{};
        if (op.fail &&
            !((op.name == "program" && op.error_type == "ProgramError" &&
               op.error_message == ProgramError().what()) ||
              (op.name == "erase_block" && op.error_type == "EraseLimitError" &&
               op.error_message == EraseLimitError().what())))
            throw FixtureError{};
        op.after = snapshot_value(states.at(i), f.config.geometry);
        f.ops.push_back(std::move(op));
    }
    admit_fixture_with_model(f);
    return f;
}
std::string load_fixture() {
    std::ifstream in("../../../specs/fixtures/database-media-v1/nand-flash.json", std::ios::binary);
    if (!in)
        throw FixtureError{};
    in.seekg(0, std::ios::end);
    const auto n = in.tellg();
    if (n < 0 || n > 65536)
        throw FixtureError{};
    in.seekg(0);
    std::string data(static_cast<std::size_t>(n), '\0');
    in.read(data.data(), static_cast<std::streamsize>(data.size()));
    if (in.gcount() != static_cast<std::streamsize>(data.size()))
        throw FixtureError{};
    return data;
}
void exercise(const Fixture &f, std::size_t *replayed_operations = nullptr) {
    Device d(f.config);
    ISO_CHECK(d.snapshot() == f.initial);
    for (const auto &op : f.ops) {
        if (replayed_operations)
            ++*replayed_operations;
        bool saw = false;
        try {
            if (op.name == "read") {
                auto out = d.read(op.block, op.page);
                const auto actual_hex = hex(out);
                ISO_CHECK_STR_EQ(actual_hex.c_str(), op.result.c_str());
            } else if (op.name == "program")
                d.program(op.block, op.page, op.data);
            else
                d.erase_block(op.block);
        } catch (const ProgramError &e) {
            saw = true;
            ISO_CHECK(op.fail && op.error_type == "ProgramError");
            ISO_CHECK_STR_EQ(e.what(), op.error_message.c_str());
        } catch (const EraseLimitError &e) {
            saw = true;
            ISO_CHECK(op.fail && op.error_type == "EraseLimitError");
            ISO_CHECK_STR_EQ(e.what(), op.error_message.c_str());
        }
        ISO_CHECK(saw == op.fail);
        ISO_CHECK(d.snapshot() == op.after);
    }
}
template <typename Replay>
void validate_then_replay(std::string_view text, Replay replay) {
    const auto fixture = parse_fixture(text);
    replay(fixture);
}
} // namespace
namespace {
void fixture_tests() {
    try {
        const auto text = load_fixture();
        std::size_t replayed_operations = 0;
        validate_then_replay(text, [&](const Fixture &fixture) {
            exercise(fixture, &replayed_operations);
        });
        ISO_CHECK(replayed_operations == 12);
    } catch (const FixtureError &) {
        ISO_CHECK_MSG(false, "canonical fixture rejected");
    } catch (const GeometryError &) {
        ISO_CHECK_MSG(false, "canonical fixture geometry rejected");
    }
    const auto canonical = load_fixture();
    auto rejected = [](const std::string &bad_text) {
        bool refused = false;
        std::size_t replay_calls = 0;
        try {
            validate_then_replay(bad_text, [&](const Fixture &) { ++replay_calls; });
        } catch (const FixtureError &) {
            refused = true;
        } catch (const GeometryError &) {
            refused = true;
        }
        return refused && replay_calls == 0;
    };
    auto replace_once = [](std::string text, const std::string &from, const std::string &to) {
        const auto position = text.find(from);
        if (position == std::string::npos)
            throw FixtureError{};
        text.replace(position, from.size(), to);
        return text;
    };
    auto replace_last_once = [](std::string text, const std::string &from,
                                const std::string &to) {
        const auto position = text.rfind(from);
        if (position == std::string::npos)
            throw FixtureError{};
        text.replace(position, from.size(), to);
        return text;
    };
    ISO_CHECK(rejected(replace_once(canonical, "\"format\":", "\"extra\": 1, \"format\":")));
    ISO_CHECK(rejected(replace_once(
        canonical, "\"format\":", "\"format\": \"coding-adventures-nand-flash-v1\", \"format\":")));
    ISO_CHECK(rejected(replace_once(canonical, "\"op\": \"read\"", "\"op\": \"unknown\"")));
    ISO_CHECK(rejected(replace_once(canonical, "\"type\": \"ProgramError\"", "\"type\": \"OtherError\"")));
    auto wrong_late_read = replace_once(canonical, "\"result\": \"0000\"", "\"result\": \"ffff\"");
    ISO_CHECK(wrong_late_read != canonical);
    ISO_CHECK(rejected(wrong_late_read));
    auto program_error_on_erase = replace_once(
        canonical,
        "\"op\": \"erase_block\",\n      \"block\": 1,\n      \"result\": null",
        "\"op\": \"erase_block\",\n      \"block\": 1,\n      \"error\": {\n        \"type\": \"ProgramError\",\n        \"message\": \"page has already been programmed since erase\"\n      }");
    ISO_CHECK(program_error_on_erase != canonical);
    ISO_CHECK(rejected(program_error_on_erase));
    auto wrong_final_erase_count = replace_last_once(
        canonical,
        "\"erase_counts\": [\n        1,\n        1\n      ]",
        "\"erase_counts\": [\n        1,\n        0\n      ]");
    ISO_CHECK(wrong_final_erase_count != canonical);
    ISO_CHECK(rejected(wrong_final_erase_count));
    ISO_CHECK(rejected(replace_once(canonical, "\"block_count\": 2", "\"block_count\": true")));
    ISO_CHECK(rejected(replace_once(canonical, "\"block_count\": 2", "\"block_count\": 2.0")));
    ISO_CHECK(rejected(replace_once(
        canonical, "\"block_count\": 2", "\"block_count\": -9223372036854775808")));
    auto over_storage = replace_once(canonical, "\"block_count\": 2", "\"block_count\": 1024");
    over_storage = replace_once(over_storage, "\"pages_per_block\": 2", "\"pages_per_block\": 256");
    over_storage = replace_once(over_storage, "\"page_size_bytes\": 2", "\"page_size_bytes\": 65536");
    ISO_CHECK(rejected(over_storage));
    ISO_CHECK(rejected(replace_once(
        canonical,
        "  \"description\": \"Portable logical SLC NAND page and block behavior for DB01.\",\n",
        "")));
    ISO_CHECK(rejected(canonical.substr(0, canonical.size() - 2)));
    ISO_CHECK(rejected(canonical + " trailing"));
}
void parser_tests() {
    auto duplicate = ca::bounded_json::parse("{\"x\":1,\"x\":2}");
    ISO_CHECK(duplicate);
    ISO_CHECK(duplicate.document->root().size() == 2);
    ISO_CHECK(!ca::bounded_json::parse("true false"));
    auto floating = ca::bounded_json::parse("1.0");
    ISO_CHECK(floating && floating.document->root().kind() == Kind::floating);
    ca::bounded_json::Options tiny;
    tiny.bytes = 2;
    ISO_CHECK(ca::bounded_json::parse("123", tiny).error.kind ==
              ca::bounded_json::Status::byte_limit);
    ca::bounded_json::Options shallow;
    shallow.depth = 1;
    ISO_CHECK(ca::bounded_json::parse("[[0]]", shallow).error.kind ==
              ca::bounded_json::Status::depth_limit);
    ca::bounded_json::Options nodes;
    nodes.nodes = 1;
    ISO_CHECK(ca::bounded_json::parse("[0]", nodes).error.kind ==
              ca::bounded_json::Status::node_limit);
    ca::bounded_json::Options memory;
    memory.memory = 1;
    ISO_CHECK(ca::bounded_json::parse("[0]", memory).error.kind ==
              ca::bounded_json::Status::memory_limit);
    ISO_CHECK(ca::bounded_json::parse(std::string("\"\xff\"", 3)).error.kind ==
              ca::bounded_json::Status::utf8);
}
void geometry_tests() {
    const std::vector<Geometry> invalid{{0, 1, 1},   {-1, 1, 1},    {1025, 1, 1},
                                        {1, 257, 1}, {1, 1, 65537}, {1024, 256, 256}};
    for (const auto geometry : invalid) {
        bool refused = false;
        try {
            Device device{{geometry, std::nullopt}};
        } catch (const GeometryError &) {
            refused = true;
        }
        ISO_CHECK(refused);
    }
    Device max_storage{{{1024, 256, 64}, std::nullopt}};
    const Geometry max_storage_geometry{1024, 256, 64};
    ISO_CHECK(max_storage.geometry() == max_storage_geometry);
    Device max_page{{{1, 1, 65536}, max_safe_erase_cycles}};
    const Geometry max_page_geometry{1, 1, 65536};
    ISO_CHECK(max_page.geometry() == max_page_geometry);
    bool negative_cycle_rejected = false;
    try {
        Device negative_limit{{{1, 1, 1}, -1}};
    } catch (const GeometryError &) {
        negative_cycle_rejected = true;
    }
    ISO_CHECK(negative_cycle_rejected);
    bool max_cycle_rejected = false;
    try {
        Device over_limit{{{1, 1, 1}, 9007199254740992LL}};
    } catch (const GeometryError &) {
        max_cycle_rejected = true;
    }
    ISO_CHECK(max_cycle_rejected);
    bool stable = false;
    try {
        Device invalid_device{{{0, 1, 1}, std::nullopt}};
    } catch (const GeometryError &error) {
        stable = std::string(error.what()) == "geometry values are outside supported bounds";
    }
    ISO_CHECK(stable);
}
void ownership_and_refusal_tests() {
    Device device{{{2, 2, 2}, 1}};
    std::vector<std::uint8_t> source{0x12, 0x34};
    device.program(0, 0, source);
    source[0] = 0;
    ISO_CHECK(device.is_programmed(0, 0));
    ISO_CHECK(!device.is_programmed(0, 1));
    ISO_CHECK(device.erase_count(0) == 0);
    bool bad_count = false;
    try {
        (void)device.erase_count(-1);
    } catch (const AddressError &error) {
        bad_count = std::string(error.what()) == "block index out of range";
    }
    ISO_CHECK(bad_count);
    bool bad_marker = false;
    try {
        (void)device.is_programmed(0, 2);
    } catch (const AddressError &error) {
        bad_marker = std::string(error.what()) == "page index out of range";
    }
    ISO_CHECK(bad_marker);
    auto readback = device.read(0, 0);
    ISO_CHECK(readback[0] == 0x12);
    readback[0] = 0;
    ISO_CHECK(device.read(0, 0)[0] == 0x12);
    const auto before_detach = device.snapshot();
    auto detached = before_detach;
    detached.pages[0][0] = "0000";
    detached.programmed[0][0] = false;
    detached.erase_counts[0] = 99;
    ISO_CHECK(device.snapshot() == before_detach);

    const auto before_short = device.snapshot();
    bool short_payload = false;
    try {
        device.program(0, 0, {1});
    } catch (const PayloadError &) {
        short_payload = true;
    }
    ISO_CHECK(short_payload);
    ISO_CHECK(device.snapshot() == before_short);
    bool repeat_program = false;
    try {
        device.program(0, 0, {0, 0});
    } catch (const ProgramError &) {
        repeat_program = true;
    }
    ISO_CHECK(repeat_program);
    ISO_CHECK(device.snapshot() == before_short);
    bool bad_page = false;
    try {
        (void)device.read(0, 2);
    } catch (const AddressError &) {
        bad_page = true;
    }
    ISO_CHECK(bad_page);
    device.program(0, 1, {0xff, 0xff});
    repeat_program = false;
    try {
        device.program(0, 1, {0xff, 0xff});
    } catch (const ProgramError &) {
        repeat_program = true;
    }
    ISO_CHECK(repeat_program);

    bool block_first = false;
    try {
        device.program(-1, 9, {});
    } catch (const AddressError &error) {
        block_first = std::string(error.what()) == "block index out of range";
    }
    ISO_CHECK(block_first);
    bool page_first = false;
    try {
        device.program(0, 9, {});
    } catch (const AddressError &error) {
        page_first = std::string(error.what()) == "page index out of range";
    }
    ISO_CHECK(page_first);
}
void erase_limit_tests() {
    ISO_CHECK(ca::nand_flash::detail::erase_allowed(max_safe_erase_cycles - 1, false, 0));
    ISO_CHECK(!ca::nand_flash::detail::erase_allowed(max_safe_erase_cycles, false, 0));
    ISO_CHECK(ca::nand_flash::detail::erase_allowed(0, true, 1));
    ISO_CHECK(!ca::nand_flash::detail::erase_allowed(1, true, 1));
}
} // namespace
int main() {
    fixture_tests();
    parser_tests();
    geometry_tests();
    ownership_and_refusal_tests();
    erase_limit_tests();
    nand_test::seeded_model();
    nand_test::bounded_histories();
    return ISO_TEST_RESULT();
}
