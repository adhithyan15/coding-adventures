#include <array>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <iterator>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

#include "bounded_json.hpp"
#include "fuzz_bootstrap.hpp"
#include "historical_web.hpp"
#include "iso_test.h"
#include "test_bootstrap.hpp"

namespace hw = coding_adventures::historical_web;
namespace json = ca::bounded_json;
namespace pbt = ca::pbt;
namespace fz = ca::fuzz;

namespace {
const json::Value& member(const json::Value& value, std::string_view name) {
    for (std::size_t i = 0; i < value.size(); ++i) {
        const auto pair = value.member(i);
        if (pair.first == name) return *pair.second;
    }
    throw std::runtime_error("missing JSON fixture member");
}

std::string read_fixture() {
    std::filesystem::path current = std::filesystem::current_path();
    for (;;) {
        const auto path = current / "code/specs/fixtures/historical-web-v1/vectors.json";
        std::ifstream input(path, std::ios::binary);
        if (input) return std::string(std::istreambuf_iterator<char>(input), {});
        if (current == current.root_path()) break;
        current = current.parent_path();
    }
    throw std::runtime_error("could not locate historical-web-v1/vectors.json");
}

int hex_digit(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

std::string decode_hex(std::string_view hex) {
    if ((hex.size() & 1u) != 0) throw std::runtime_error("odd-length fixture hex");
    std::string bytes;
    bytes.reserve(hex.size() / 2);
    for (std::size_t i = 0; i < hex.size(); i += 2) {
        const int hi = hex_digit(hex[i]);
        const int lo = hex_digit(hex[i + 1]);
        if (hi < 0 || lo < 0) throw std::runtime_error("invalid fixture hex");
        bytes.push_back(static_cast<char>((hi << 4) | lo));
    }
    return bytes;
}

const json::Value& at(const json::Value& value, std::size_t index) { return value.at(index); }
std::string_view text(const json::Value& value) { return value.string(); }

void compare_node(const hw::HtmlDocument& doc, std::size_t actual_index, const json::Value& expected) {
    const hw::HtmlNode& actual = doc.node_at(actual_index);
    if (expected.kind() == json::Kind::string) {
        ISO_CHECK(actual.kind == hw::NodeKind::text);
        ISO_CHECK(actual.text == text(expected));
        return;
    }
    ISO_CHECK(expected.kind() == json::Kind::object);
    ISO_CHECK(actual.kind == hw::NodeKind::element);
    ISO_CHECK(actual.tag == text(member(expected, "tag")));
    const json::Value& attrs = member(expected, "attributes");
    std::size_t attr_index = actual.first_attribute_index;
    std::size_t attr_count = 0;
    for (std::size_t i = 0; i < attrs.size(); ++i) {
        const auto pair = attrs.member(i);
        const hw::HtmlAttribute& attr = doc.attribute_at(attr_index++);
        ISO_CHECK(attr.name == pair.first);
        ISO_CHECK(pair.second->kind() == json::Kind::string
                      ? (attr.value.has_value() && *attr.value == pair.second->string())
                      : !attr.value.has_value());
        ++attr_count;
    }
    ISO_CHECK_EQ_UINT(actual.attribute_count, attr_count);
    const json::Value& children = member(expected, "children");
    std::size_t child = actual.first_child_index;
    for (std::size_t i = 0; i < children.size(); ++i) {
        ISO_CHECK(child != hw::no_index);
        if (child == hw::no_index) return;
        compare_node(doc, child, at(children, i));
        child = doc.node_at(child).next_sibling_index;
    }
    ISO_CHECK(child == hw::no_index);
}

void compare_tree(const hw::HtmlDocument& doc, const json::Value& expected_roots) {
    std::size_t root = doc.first_root();
    ISO_CHECK_EQ_UINT(doc.root_count(), expected_roots.size());
    for (std::size_t i = 0; i < expected_roots.size(); ++i) {
        ISO_CHECK(root != hw::no_index);
        if (root == hw::no_index) return;
        ISO_CHECK(doc.node_at(root).parent_index == hw::no_index);
        compare_node(doc, root, at(expected_roots, i));
        root = doc.node_at(root).next_sibling_index;
    }
    ISO_CHECK(root == hw::no_index);
}

bool arena_valid(const hw::HtmlDocument& doc) {
    if (doc.node_count() != doc.nodes().size() || doc.attribute_count() != doc.attributes().size()) return false;
    std::vector<bool> seen(doc.node_count(), false);
    std::vector<std::size_t> pending;
    std::size_t root = doc.first_root();
    std::size_t roots = 0;
    while (root != hw::no_index) {
        if (root >= doc.node_count() || ++roots > doc.node_count() || doc.node_at(root).parent_index != hw::no_index)
            return false;
        pending.push_back(root);
        root = doc.node_at(root).next_sibling_index;
    }
    if (roots != doc.root_count()) return false;
    while (!pending.empty()) {
        const std::size_t parent = pending.back(); pending.pop_back();
        if (parent >= doc.node_count() || seen[parent]) return false;
        seen[parent] = true;
        const auto& node = doc.node_at(parent);
        if (node.parent_index != hw::no_index && node.parent_index >= parent) return false;
        if (node.kind == hw::NodeKind::element) {
            if (node.attribute_count == 0) {
                if (node.first_attribute_index != hw::no_index && node.first_attribute_index > doc.attribute_count()) return false;
            } else if (node.first_attribute_index == hw::no_index ||
                       node.first_attribute_index > doc.attribute_count() ||
                       node.attribute_count > doc.attribute_count() - node.first_attribute_index) return false;
        } else if (node.attribute_count != 0 || node.first_attribute_index != hw::no_index) return false;
        std::size_t child = node.first_child_index;
        std::size_t children = 0;
        while (child != hw::no_index) {
            if (child >= doc.node_count() || child <= parent || ++children > doc.node_count() ||
                doc.node_at(child).parent_index != parent)
                return false;
            pending.push_back(child);
            const std::size_t next = doc.node_at(child).next_sibling_index;
            if (next != hw::no_index && next <= child) return false;
            child = next;
        }
    }
    for (bool value : seen) if (!value) return false;
    return true;
}

bool valid_address(std::string_view s) {
    if (s.empty()) return false;
    for (unsigned char c : s) if (c < 0x21 || c > 0x7e) return false;
    return true;
}

bool expected_request(std::string_view wire, std::string& address) {
    if (wire.size() < 6 || wire.substr(0, 4) != "GET ") return false;
    std::size_t end = wire.size();
    if (end >= 2 && wire.substr(end - 2) == "\r\n") end -= 2;
    else if (end >= 1 && wire.back() == '\n') --end;
    else return false;
    const std::string_view candidate = wire.substr(4, end - 4);
    if (!valid_address(candidate)) return false;
    address.assign(candidate.data(), candidate.size());
    return true;
}

void test_http_fixtures() {
    const auto parsed = json::parse(read_fixture());
    ISO_CHECK(static_cast<bool>(parsed));
    if (!parsed) return;
    const json::Value& http = member(parsed.document->root(), "http-0.9");
    const json::Value& requests = member(http, "requests");
    ISO_CHECK_EQ_UINT(requests.size(), 3u);
    for (std::size_t i = 0; i < requests.size(); ++i) {
        const auto& v = at(requests, i);
        const auto addr = text(member(v, "address"));
        const auto wire = decode_hex(text(member(v, "wire_hex")));
        ISO_CHECK(hw::encode_http09_request(addr) == wire ||
                  (wire.size() > 1 && wire.back() == '\n' && wire[wire.size() - 2] != '\r' &&
                   hw::encode_http09_request(addr) == wire.substr(0, wire.size() - 1) + "\r\n"));
        ISO_CHECK(hw::parse_http09_request(wire) == addr);
    }
    const auto& invalid_addresses = member(http, "invalid_addresses");
    ISO_CHECK_EQ_UINT(invalid_addresses.size(), 6u);
    for (std::size_t i = 0; i < invalid_addresses.size(); ++i) {
        bool failed = false;
        try { (void)hw::encode_http09_request(text(at(invalid_addresses, i))); }
        catch (const hw::ProfileError& e) { failed = e.kind() == hw::ErrorKind::invalid_address; }
        ISO_CHECK(failed);
    }
    const auto& invalid_requests = member(http, "invalid_request_hex");
    ISO_CHECK_EQ_UINT(invalid_requests.size(), 8u);
    for (std::size_t i = 0; i < invalid_requests.size(); ++i) {
        bool failed = false;
        try { (void)hw::parse_http09_request(decode_hex(text(at(invalid_requests, i)))); }
        catch (const hw::ProfileError& e) { failed = e.kind() == hw::ErrorKind::invalid_request; }
        ISO_CHECK(failed);
    }
    const auto& responses = member(http, "responses");
    ISO_CHECK_EQ_UINT(responses.size(), 3u);
    for (std::size_t i = 0; i < responses.size(); ++i) {
        const auto& v = at(responses, i);
        ISO_CHECK(hw::decode_http09_response(decode_hex(text(member(v, "wire_hex")))) == text(member(v, "body")));
    }
    const auto& invalid_responses = member(http, "invalid_response_hex");
    ISO_CHECK_EQ_UINT(invalid_responses.size(), 2u);
    for (std::size_t i = 0; i < invalid_responses.size(); ++i) {
        bool failed = false;
        try { (void)hw::decode_http09_response(decode_hex(text(at(invalid_responses, i)))); }
        catch (const hw::ProfileError& e) { failed = e.kind() == hw::ErrorKind::invalid_response; }
        ISO_CHECK(failed);
    }
}

void test_html_fixtures() {
    const auto parsed = json::parse(read_fixture());
    ISO_CHECK(static_cast<bool>(parsed));
    if (!parsed) return;
    const json::Value& cases = member(parsed.document->root(), "html-tags-1992");
    ISO_CHECK_EQ_UINT(cases.size(), 13u);
    for (std::size_t i = 0; i < cases.size(); ++i) {
        const auto& v = at(cases, i);
        const hw::HtmlDocument doc = hw::parse_html_tags_1992(text(member(v, "source")));
        compare_tree(doc, member(v, "tree"));
    }
    const json::Value& exchanges = member(parsed.document->root(), "exchanges");
    ISO_CHECK_EQ_UINT(exchanges.size(), 1u);
    for (std::size_t i = 0; i < exchanges.size(); ++i) {
        const auto& v = at(exchanges, i);
        const auto address = text(member(v, "address"));
        const std::string request = decode_hex(text(member(v, "request_hex")));
        ISO_CHECK(hw::encode_http09_request(address) == request);
        ISO_CHECK(hw::parse_http09_request(request) == address);
        const std::string html = decode_hex(text(member(v, "response_hex")));
        ISO_CHECK(hw::decode_http09_response(html) == html);
        const hw::HtmlDocument doc = hw::parse_html_tags_1992(html);
        compare_tree(doc, member(v, "tree"));
    }
}

void test_http_exhaustive_short_wire() {
    std::size_t tested = 0;
    for (std::size_t len = 0; len <= 2; ++len) {
        const std::size_t total = len == 0 ? 1 : (len == 1 ? 256 : 65536);
        for (std::size_t word = 0; word < total; ++word) {
            std::string payload(len, '\0');
            std::size_t n = word;
            for (std::size_t j = 0; j < len; ++j) { payload[j] = static_cast<char>(n & 0xffu); n >>= 8; }
            for (const std::string_view ending : {"\n", "\r\n"}) {
                const std::string wire = "GET " + payload + std::string(ending);
                std::string expected;
                const bool accepts = expected_request(wire, expected);
                try {
                    const auto actual = hw::parse_http09_request(wire);
                    ISO_CHECK(accepts);
                    if (accepts) ISO_CHECK(actual == expected);
                } catch (const hw::ProfileError& error) {
                    ISO_CHECK(!accepts);
                    if (!accepts) ISO_CHECK(error.kind() == hw::ErrorKind::invalid_request);
                }
                ++tested;
            }
        }
    }
    ISO_CHECK_EQ_UINT(tested, 131586u);
}

void test_http_diagnostics_and_transactions() {
    const std::string mismatch = "GXT /\r\n";
    bool seen = false;
    try { (void)hw::parse_http09_request(mismatch); }
    catch (const hw::ProfileError& error) {
        seen = error.kind() == hw::ErrorKind::invalid_request && error.byte_offset() == 1;
    }
    ISO_CHECK(seen);
    const std::string truncated = "GET /";
    seen = false;
    try { (void)hw::parse_http09_request(truncated); }
    catch (const hw::ProfileError& error) {
        seen = error.kind() == hw::ErrorKind::invalid_request && error.byte_offset() == truncated.size();
    }
    ISO_CHECK(seen);
    const std::string invalid_address("/x\0y", 4);
    seen = false;
    try { (void)hw::encode_http09_request(invalid_address); }
    catch (const hw::ProfileError& error) {
        seen = error.kind() == hw::ErrorKind::invalid_address && error.byte_offset() == 2;
    }
    ISO_CHECK(seen);
    seen = false;
    try { (void)hw::encode_http09_request(""); }
    catch (const hw::ProfileError& error) {
        seen = error.kind() == hw::ErrorKind::invalid_address && error.byte_offset() == hw::no_index;
    }
    ISO_CHECK(seen);
    const std::string invalid_response("a\x80", 2);
    seen = false;
    try { (void)hw::decode_http09_response(invalid_response); }
    catch (const hw::ProfileError& error) {
        seen = error.kind() == hw::ErrorKind::invalid_response && error.byte_offset() == 1;
    }
    ISO_CHECK(seen);
    ISO_CHECK(hw::decode_http09_response("").empty());
    ISO_CHECK(hw::decode_http09_response(std::string_view{}).empty());
}

void test_http_properties() {
    pbt::Options opts; opts.ci = pbt::CiMode::Off; opts.num_tests = 500;
    const auto verdict = pbt::for_all(pbt::gen::bytes(1, 24), [](pbt::ValueView value) {
        const auto bytes = value.as_bytes();
        std::string address(bytes.begin(), bytes.end());
        bool encodable = true;
        for (std::uint8_t b : bytes) if (b < 0x21 || b > 0x7e) encodable = false;
        try {
            const std::string wire = hw::encode_http09_request(address);
            if (!encodable || wire != "GET " + address + "\r\n") return false;
            return hw::parse_http09_request(wire) == address;
        } catch (const hw::ProfileError& e) { return !encodable && e.kind() == hw::ErrorKind::invalid_address; }
    }, 0x485745423039ull, opts);
    ISO_CHECK(verdict.passed());
    ISO_CHECK_EQ_INT(verdict.cases_run(), 500);
}

void test_html_properties() {
    pbt::Options opts; opts.ci = pbt::CiMode::Off; opts.num_tests = 400;
    const auto verdict = pbt::for_all(pbt::gen::bytes(0, 64), [](pbt::ValueView value) {
        const auto bytes = value.as_bytes();
        const std::string source(bytes.begin(), bytes.end());
        try { return arena_valid(hw::parse_html_tags_1992(source)); }
        catch (const hw::ProfileError& error) { return error.kind() == hw::ErrorKind::invalid_utf8; }
    }, 0x485745423039ull, opts);
    ISO_CHECK(verdict.passed());
    ISO_CHECK_EQ_INT(verdict.cases_run(), 400);
}

void test_generated_printable_address_property() {
    pbt::Options opts; opts.ci = pbt::CiMode::Off; opts.num_tests = 300;
    auto addresses = pbt::gen::bytes(1, 32).map([](pbt::ValueView value) {
        auto bytes = value.as_bytes();
        std::string address;
        address.reserve(bytes.size());
        for (std::uint8_t byte : bytes) address.push_back(static_cast<char>(0x21u + byte % 94u));
        return pbt::Value::string(address);
    });
    const auto verdict = pbt::for_all(addresses, [](pbt::ValueView value) {
        const std::string address = value.as_string();
        const std::string wire = hw::encode_http09_request(address);
        return wire == "GET " + address + "\r\n" && hw::parse_http09_request(wire) == address;
    }, 0x485745423039ull, opts);
    ISO_CHECK(verdict.passed());
    ISO_CHECK_EQ_INT(verdict.cases_run(), 300);
}

void test_fuzz_public_oracles() {
    std::vector<fz::Bytes> seeds = {{}, {'G','E','T',' ','/','\r','\n'}, {'G','E','T',' ','/','\n'},
        {'G','E','T',' ','/',' ','H','T','T','P'}, {'<','A',' ','H','R','E','F','=','x','>'},
        {'<','X','M','P','>','a','<','/','X','M','P','>'}, {0xff, 0x80, '<', 'P', '>'}};
    fz::Options options; options.seed = 0x485745423039ull; options.max_iterations = 1200;
    options.max_input_len = 96; options.step_budget = 500000; options.mem_budget = 64u * 1024u * 1024u;
    const auto report = fz::run(seeds, options, [](fz::ByteView input, fz::Handle& handle) {
        handle.edge(static_cast<std::uint32_t>(input.size()));
        const std::string bytes = input.size() == 0
            ? std::string()
            : std::string(reinterpret_cast<const char*>(input.data()), input.size());
        std::string expected;
        const bool accepts = expected_request(bytes, expected);
        handle.edge(accepts ? "http09/accepted" : "http09/rejected");
        try {
            const std::string actual = hw::parse_http09_request(bytes);
            if (!accepts || actual != expected) throw std::runtime_error("HTTP oracle mismatch");
        } catch (const hw::ProfileError& error) {
            if (accepts || error.kind() != hw::ErrorKind::invalid_request) throw;
        }
        try {
            const auto doc = hw::parse_html_tags_1992(bytes);
            handle.edge("html1992/valid-utf8-recovered");
            if (doc.root_count() != 0) handle.edge("html1992/has-root");
            if (doc.attribute_count() != 0) handle.edge("html1992/has-recognized-attribute");
            if (doc.node_count() > doc.root_count()) handle.edge("html1992/has-child");
            if (!arena_valid(doc)) throw std::runtime_error("arena invariant mismatch");
        } catch (const hw::ProfileError& error) {
            if (error.kind() != hw::ErrorKind::invalid_utf8) throw;
            handle.edge("html1992/rejected-invalid-utf8");
        }
    });
    ISO_CHECK(report.is_clean());
    ISO_CHECK_EQ_UINT(report.iterations(), options.max_iterations);
}

void test_html_arena_edges_ownership_and_utf8() {
    std::string source = "<ADDRESS><ADDRESS>nested</ADDRESS></ADDRESS></A></A>plain";
    auto doc = hw::parse_html_tags_1992(source);
    source.assign("destroyed");
    ISO_CHECK_EQ_UINT(doc.root_count(), 2u);
    ISO_CHECK(doc.node_at(doc.first_root()).tag == "address");
    ISO_CHECK(doc.nodes().size() >= 4);
    ISO_CHECK(doc.nodes().back().text == "plain");
    bool out_of_range = false;
    try { (void)doc.node_at(doc.node_count()); } catch (const std::out_of_range&) { out_of_range = true; }
    ISO_CHECK(out_of_range);
    out_of_range = false;
    try { (void)doc.attribute_at(0); } catch (const std::out_of_range&) { out_of_range = true; }
    ISO_CHECK(out_of_range);

    const auto attrs = hw::parse_html_tags_1992("<A HREF HREF='' NAME=x NAME=y TYPE>z</A>");
    const auto& a = attrs.node_at(attrs.first_root());
    ISO_CHECK_EQ_UINT(a.attribute_count, 3u);
    ISO_CHECK(attrs.attribute_at(a.first_attribute_index).name == "href");
    ISO_CHECK(!attrs.attribute_at(a.first_attribute_index).value.has_value());
    ISO_CHECK(attrs.attribute_at(a.first_attribute_index + 1).name == "name");
    ISO_CHECK(attrs.attribute_at(a.first_attribute_index + 1).value.has_value());
    ISO_CHECK(*attrs.attribute_at(a.first_attribute_index + 1).value == "x");
    ISO_CHECK(attrs.attribute_at(a.first_attribute_index + 2).name == "type");
    ISO_CHECK(!attrs.attribute_at(a.first_attribute_index + 2).value.has_value());
    for (const std::string& source : {std::string("<A HREF=>x</A>"), std::string("<A HREF==x>y</A>")}) {
        const auto malformed_value = hw::parse_html_tags_1992(source);
        const auto& anchor = malformed_value.node_at(malformed_value.first_root());
        ISO_CHECK(anchor.tag == "a");
        ISO_CHECK_EQ_UINT(anchor.attribute_count, 1u);
        ISO_CHECK(!malformed_value.attribute_at(anchor.first_attribute_index).value.has_value());
    }
    const auto malformed_tag = hw::parse_html_tags_1992("<1 malformed");
    ISO_CHECK(malformed_tag.node_at(malformed_tag.first_root()).text == "<1 malformed");

    const std::string raw_text = u8"aaaaa\u00e9";
    const std::string raw_source = "<xmp>" + raw_text + "</XMP><P>after";
    const auto raw = hw::parse_html_tags_1992(raw_source);
    ISO_CHECK(raw.node_at(raw.first_root()).kind == hw::NodeKind::element);
    ISO_CHECK(raw.node_at(raw.node_at(raw.first_root()).first_child_index).text == raw_text);
    const std::size_t marker = raw.node_at(raw.first_root()).next_sibling_index;
    ISO_CHECK(raw.node_at(marker).tag == "p");
    ISO_CHECK(raw.node_at(raw.node_at(marker).next_sibling_index).text == "after");
    const std::string utf8 = u8"\u00e9\u6c34\U0001f6f0";
    const auto unicode = hw::parse_html_tags_1992(utf8);
    ISO_CHECK(unicode.node_at(unicode.first_root()).text == utf8);
    const std::string nul("a\0b", 3);
    ISO_CHECK(hw::parse_html_tags_1992(nul).node_at(0).text == nul);

    const std::vector<std::pair<std::string, std::size_t>> invalid = {
        {std::string("\xc3", 1), 1}, {std::string("\xe2\x82", 2), 2},
        {std::string("\xf0\x9f\x8c", 3), 3}, {std::string("\xe2X\x82", 3), 1},
        {std::string("\xf0\x9fX\x8c", 4), 2}, {std::string("\xf0\x9f\x8cX", 4), 3}};
    for (const auto& pair : invalid) {
        bool failed = false;
        try { (void)hw::parse_html_tags_1992(pair.first); }
        catch (const hw::ProfileError& error) {
            failed = error.kind() == hw::ErrorKind::invalid_utf8 && error.byte_offset() == pair.second;
        }
        ISO_CHECK(failed);
    }
}

void test_html_scope_boundaries_copy_and_move() {
    {
        const auto doc = hw::parse_html_tags_1992(
            "<UL><LI>one<MENU><LI>inner</MENU>continued</UL>");
        const auto& ul = doc.node_at(doc.first_root());
        const auto& outer = doc.node_at(ul.first_child_index);
        ISO_CHECK(ul.tag == "ul" && outer.tag == "li");
        std::size_t c = outer.first_child_index;
        ISO_CHECK(doc.node_at(c).text == "one");
        c = doc.node_at(c).next_sibling_index;
        ISO_CHECK(doc.node_at(c).tag == "menu");
        const auto& menu = doc.node_at(c);
        ISO_CHECK(doc.node_at(menu.first_child_index).tag == "li");
        c = doc.node_at(c).next_sibling_index;
        ISO_CHECK(doc.node_at(c).text == "continued");
    }
    {
        const auto doc = hw::parse_html_tags_1992(
            "<UL><LI>outer<MENU><LI>middle<DIR><LI>inner</DIR>middle-end</MENU>outer-end</UL>");
        const auto& outer = doc.node_at(doc.node_at(doc.first_root()).first_child_index);
        const auto& menu = doc.node_at(doc.node_at(outer.first_child_index).next_sibling_index);
        const auto& middle = doc.node_at(menu.first_child_index);
        std::size_t c = middle.first_child_index;
        ISO_CHECK(doc.node_at(c).text == "middle");
        c = doc.node_at(c).next_sibling_index;
        ISO_CHECK(doc.node_at(c).tag == "dir");
        c = doc.node_at(c).next_sibling_index;
        ISO_CHECK(doc.node_at(c).text == "middle-end");
        c = doc.node_at(outer.first_child_index).next_sibling_index;
        c = doc.node_at(c).next_sibling_index;
        ISO_CHECK(doc.node_at(c).text == "outer-end");
    }
    {
        const auto doc = hw::parse_html_tags_1992(
            "<DL><DT>outer<DL><DT>inner</DL>outer-tail<DD>definition</DL>");
        const auto& dl = doc.node_at(doc.first_root());
        const auto& dt = doc.node_at(dl.first_child_index);
        std::size_t c = dt.first_child_index;
        ISO_CHECK(doc.node_at(c).text == "outer");
        c = doc.node_at(c).next_sibling_index;
        ISO_CHECK(doc.node_at(c).tag == "dl");
        c = doc.node_at(c).next_sibling_index;
        ISO_CHECK(doc.node_at(c).text == "outer-tail");
        ISO_CHECK(doc.node_at(dt.next_sibling_index).tag == "dd");
    }
    hw::HtmlDocument original = hw::parse_html_tags_1992("<TITLE>owned</TITLE>");
    hw::HtmlDocument copy = hw::parse_html_tags_1992("<P>old</P>");
    copy = original;
    ISO_CHECK(copy.node_count() == original.node_count());
    ISO_CHECK(copy.node_at(copy.first_root()).tag == "title");
    hw::HtmlDocument moved = hw::parse_html_tags_1992("<P>before</P>");
    moved = std::move(copy);
    ISO_CHECK(moved.node_count() == original.node_count());
    ISO_CHECK(copy.node_count() == 0u);
    ISO_CHECK(copy.first_root() == hw::no_index);

    std::string deep;
    constexpr std::size_t depth = 12000;
    constexpr std::size_t misses = 4096;
    deep.reserve(depth * 18 + misses * 4);
    for (std::size_t i = 0; i < depth; ++i) deep += "<ADDRESS>";
    for (std::size_t i = 0; i < misses; ++i) deep += "</A>";
    deep += 'x';
    for (std::size_t i = 0; i < depth; ++i) deep += "</ADDRESS>";
    {
        const auto deep_doc = hw::parse_html_tags_1992(deep);
        ISO_CHECK_EQ_UINT(deep_doc.node_count(), depth + 1);
        ISO_CHECK(deep_doc.node_at(deep_doc.first_root()).tag == "address");
    }  // destruction is also iterative at this nesting depth
}

}  // namespace

int main() {
    test_http_fixtures();
    test_html_fixtures();
    test_http_exhaustive_short_wire();
    test_http_diagnostics_and_transactions();
    test_http_properties();
    test_html_properties();
    test_generated_printable_address_property();
    test_fuzz_public_oracles();
    test_html_arena_edges_ownership_and_utf8();
    test_html_scope_boundaries_copy_and_move();
    return ISO_TEST_RESULT();
}
