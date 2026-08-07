/*
 * conforming.cpp — a pure ISO C++17 program that MUST compile and run cleanly
 * under every C++ compiler the harness finds, with the strict conformance
 * flags. It exercises a couple of standard-library facilities (std::vector,
 * range-for) to prove the C++ toolchain is wired up, and doubles as a smoke
 * test of iso_test.h compiled as C++.
 */
#include "iso_test.h"

#include <string>
#include <vector>

static int sum(const std::vector<int>& values) {
    int total = 0;
    for (int value : values) {
        total += value;
    }
    return total;
}

int main() {
    ISO_CHECK(sum({1, 2, 3}) == 6);
    ISO_CHECK_EQ_INT(sum(std::vector<int>{}), 0);

    // The extended macros must also compile and work as C++ — the string one
    // accepts std::string::c_str(); the memory one takes any byte buffer.
    const std::string greeting = "iso";
    const unsigned char bytes[2] = {0xab, 0xcd};
    ISO_CHECK_EQ_UINT(greeting.size(), 3u);
    ISO_CHECK_STR_EQ(greeting.c_str(), "iso");
    ISO_CHECK_MEM_EQ(bytes, bytes, 2);
    ISO_CHECK_EQ_DBL(0.1 + 0.2, 0.3, 1e-9);

    return ISO_TEST_RESULT();
}
