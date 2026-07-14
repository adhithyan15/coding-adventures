// posix_selftest.cpp — the C++ half of the platform-harness self-test.
//
// Uses the portable C++17 standard library <thread> (allowed: core-language and
// supported by GCC, Clang, and MSVC). On Linux/macOS std::thread is a thin
// wrapper over pthreads and still needs -pthread at link, so this also proves
// the harness's PLATFORM_LIBS link path for C++. Compiles under -Wall -Wextra
// -Werror without -pedantic-errors.
#include "iso_test.h"

#include <thread>

int main() {
    int counter = 0;
    std::thread t([&counter] { counter += 1; });
    t.join(); // happens-before edge: no data race on `counter`
    ISO_CHECK_EQ_INT(counter, 1);
    return ISO_TEST_RESULT();
}
