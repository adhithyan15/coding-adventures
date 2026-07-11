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
    ISO_CHECK(add(2, 3) == 5);
    ISO_CHECK_EQ_INT(add(10, -4), 6);
    ISO_CHECK_MSG(sizeof(int) >= 2, "ISO C guarantees int is at least 16 bits");
    return ISO_TEST_RESULT();
}
