// Fixture-only trace adapter, compiled temporarily by the C++ package BUILD.
#include "unix_history.hpp"
#include <iostream>

static void quote(std::string_view value) {
    constexpr std::string_view hex = "0123456789abcdef";
    std::cout << '"';
    for (unsigned char c : value) {
        if (c < 32 || c == '"' || c == '\\') std::cout << "\\u00" << hex[c >> 4] << hex[c & 15];
        else std::cout << c;
    }
    std::cout << '"';
}

static void number(std::string_view name, std::optional<std::size_t> value) {
    if (value) std::cout << ",\"" << name << "\":" << *value;
}

static void event(const unix_history::Event& item) {
    std::cout << "{\"call\":";
    quote(item.call);
    if (item.path) {
        std::cout << ",\"path\":";
        quote(*item.path);
    }
    if (!item.error.empty()) {
        std::cout << ",\"error\":";
        quote(item.error);
    }
    number("fd", item.fd);
    number("inode", item.inode);
    number("offset", item.offset);
    number("bytes", item.bytes);
    number("nextOffset", item.next_offset);
    if (item.names) {
        std::cout << ",\"names\":[";
        bool first = true;
        for (const auto& name : *item.names) {
            if (!std::exchange(first, false)) std::cout << ',';
            quote(name);
        }
        std::cout << ']';
    }
    std::cout << '}';
}

int main(int argc, char** argv) {
    std::string command = argc == 1 ? "cat /hello" : argv[1];
    for (int i = 2; i < argc; ++i) command.append(" ").append(argv[i]);
    auto result = unix_history::run(command);
    std::cout << "{\"output\":";
    quote(result.output);
    std::cout << ",\"events\":[";
    bool first = true;
    for (const auto& item : result.events) {
        if (!std::exchange(first, false)) std::cout << ',';
        event(item);
    }
    std::cout << "]}\n";
}
