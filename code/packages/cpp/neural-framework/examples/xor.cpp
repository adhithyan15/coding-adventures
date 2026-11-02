// A native command with two data sources: trusted fixed argv, untrusted stdin.
// Checkpoint text can supply numbers, never a path, program or working directory.
#include "neural_framework.hpp"
#include <cstdio>
#include <iostream>
#include <string>

namespace nf = ca::neural_framework;
static nf::Network read_checkpoint() {
    std::string text;
    char c;
    while (std::cin.get(c)) {
        if (text.size() == ca::bounded_json::max_bytes) throw nf::Error(nf::Code::capacity);
        text.push_back(c);
    }
    if (!std::cin.eof()) throw nf::Error(nf::Code::argument);
    return nf::Network::from_json(text);
}
int main(int argc, char** argv) {
    try {
        std::size_t epochs = 3000; bool resume = false;
        if (argc == 2) {
            const std::string_view arg(argv[1]);
            if (arg == "--resume") { resume = true; epochs = 1; }
            else if (arg == "--baseline-3001") epochs = 3001;
            else if (arg == "--baseline-3002") epochs = 3002;
            else throw nf::Error(nf::Code::argument);
        } else if (argc != 1) throw nf::Error(nf::Code::argument);
        auto n = resume ? read_checkpoint() : nf::Network::seeded({2, 4, 1}, {nf::Activation::tanh, nf::Activation::sigmoid}, 7);
        const nf::Matrix x({{0, 0}, {0, 1}, {1, 0}, {1, 1}}), y({{0}, {1}, {1}, {0}});
        // Public batch validation verifies BOTH ends of the loaded architecture.
        // Never index predictions using the fixed target shape before this check.
        auto losses = n.fit(x, y, 1, epochs); auto predictions = n.predict(x);
        std::string out = "{\"contract\":\"NN51\",\"implementation\":\"native-cpp\",\"bindings\":0,\"seven_native_exchange\":\"pending\",\"checkpoint\":";
        out += n.to_json(); out += ",\"predictions\":[";
        for (std::size_t i = 0; i < predictions.rows; ++i) {
            if (i != 0) out += ',';
            out += '['; nf::detail::append_number(out, predictions.data[i][0]); out += ']';
        }
        out += "],\"initial_loss\":"; nf::detail::append_number(out, losses.front());
        out += ",\"final_loss\":"; nf::detail::append_number(out, losses.back());
        out += ",\"losses\":[";
        for (std::size_t i = 0; i < losses.size(); ++i) {
            if (i != 0) out += ',';
            nf::detail::append_number(out, losses[i]);
        }
        out += "]}\n";
        std::cout << out;
        if (!std::cout) throw nf::Error(nf::Code::argument);
        return 0;
    } catch (const nf::Error& e) {
        std::fprintf(stderr, "native Cpp XOR rejected (category %d)\n", static_cast<int>(e.code()));
    } catch (const std::bad_alloc&) {
        std::fputs("native Cpp XOR allocation failed\n", stderr);
    } catch (const std::exception&) {
        std::fputs("native Cpp XOR operation failed\n", stderr);
    }
    return 1;
}
