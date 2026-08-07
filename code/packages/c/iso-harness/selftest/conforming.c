/*
 * conforming.c — a pure ISO C17 program that MUST compile and run cleanly under
 * every compiler the harness finds, with -pedantic-errors -Wall -Wextra -Werror
 * (or /permissive- /W4 /WX on MSVC). It also doubles as a smoke test of
 * iso_test.h. If this ever fails, the harness or its flags are broken.
 */
#include "iso_test.h"

static int add(int a, int b) {
    return a + b;
}

int main(void) {
    const unsigned char lhs[3] = {0x01, 0x02, 0x03};
    const unsigned char rhs[3] = {0x01, 0x02, 0x03};

    ISO_CHECK(add(2, 3) == 5);
    ISO_CHECK_EQ_INT(add(10, -4), 6);
    ISO_CHECK_MSG(sizeof(int) >= 2, "ISO C guarantees int is at least 16 bits");

    /* Exercise the extended assertion macros so they are verified under every
     * compiler (GCC, Clang, MSVC), not just the ones on a dev machine. */
    ISO_CHECK_EQ_UINT(sizeof(char), 1u);
    ISO_CHECK_STR_EQ("iso", "iso");
    ISO_CHECK_MEM_EQ(lhs, rhs, 3);
    ISO_CHECK_EQ_DBL(0.1 + 0.2, 0.3, 1e-9);

    return ISO_TEST_RESULT();
}
