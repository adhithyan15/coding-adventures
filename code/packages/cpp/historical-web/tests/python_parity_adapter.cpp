#include <cstdint>
#include <iostream>
#include <string>
#include <string_view>

#include "historical_web.hpp"

namespace hw = coding_adventures::historical_web;

namespace {
int nibble(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}
std::string from_hex(std::string_view line) {
    if ((line.size() & 1u) != 0) throw std::runtime_error("odd hex input");
    std::string out;
    out.reserve(line.size() / 2);
    for (std::size_t i = 0; i < line.size(); i += 2) {
        const int a = nibble(line[i]), b = nibble(line[i + 1]);
        if (a < 0 || b < 0) throw std::runtime_error("invalid hex input");
        out.push_back(static_cast<char>((a << 4) | b));
    }
    return out;
}
void quote(std::string_view value) {
    std::cout << '"';
    for (unsigned char c : value) {
        if (c == '"' || c == '\\') std::cout << '\\' << static_cast<char>(c);
        else if (c == '\n') std::cout << "\\n";
        else if (c == '\r') std::cout << "\\r";
        else if (c == '\t') std::cout << "\\t";
        else if (c < 0x20) {
            static constexpr char digits[] = "0123456789abcdef";
            std::cout << "\\u00" << digits[c >> 4] << digits[c & 15];
        } else std::cout << static_cast<char>(c);
    }
    std::cout << '"';
}
void write_node(const hw::HtmlDocument& doc, std::size_t index) {
    const hw::HtmlNode& node = doc.node_at(index);
    if (node.kind == hw::NodeKind::text) { quote(node.text); return; }
    std::cout << "{\"tag\":"; quote(node.tag); std::cout << ",\"attributes\":{";
    for (std::size_t i = 0; i < node.attribute_count; ++i) {
        if (i != 0) std::cout << ',';
        const auto& attr = doc.attribute_at(node.first_attribute_index + i);
        quote(attr.name); std::cout << ':';
        if (attr.value) quote(*attr.value); else std::cout << "null";
    }
    std::cout << "},\"children\":[";
    std::size_t child = node.first_child_index;
    bool first = true;
    while (child != hw::no_index) {
        if (!first) std::cout << ',';
        write_node(doc, child);
        first = false;
        child = doc.node_at(child).next_sibling_index;
    }
    std::cout << "]}";
}
}

int main() {
    std::string line;
    while (std::getline(std::cin, line)) {
        try {
            const auto source = from_hex(line);
            const auto doc = hw::parse_html_tags_1992(source);
            std::size_t root = doc.first_root();
            bool first = true;
            std::cout << '[';
            while (root != hw::no_index) {
                if (!first) std::cout << ',';
                write_node(doc, root);
                first = false;
                root = doc.node_at(root).next_sibling_index;
            }
            std::cout << "]\n";
        } catch (...) {
            std::cout << "null\n";
        }
    }
}
