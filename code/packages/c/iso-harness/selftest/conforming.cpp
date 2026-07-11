/*
 * conforming.cpp — a pure ISO C++17 program that MUST compile and run cleanly
 * under every C++ compiler the harness finds, with the strict conformance
 * flags. It exercises a couple of standard-library facilities (std::vector,
 * range-for) to prove the C++ toolchain is wired up, and doubles as a smoke
 * test of iso_test.h compiled as C++.
 */
#include "iso_test.h"

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
    return ISO_TEST_RESULT();
}
