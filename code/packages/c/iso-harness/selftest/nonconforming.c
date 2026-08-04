/*
 * nonconforming.c — deliberately NON-ISO C. It uses a GNU "statement
 * expression" (`({ ... })`), which is a compiler extension the ISO C standard
 * does not permit. Under -pedantic-errors (GCC/Clang) or /permissive- (MSVC)
 * this MUST fail to compile.
 *
 * The harness's self-test compiles this and asserts that every compiler rejects
 * it. If a compiler were to accept it, that would prove our strict flags are not
 * actually enforcing ISO conformance — so this file is the harness testing
 * itself.
 */
int main(void) {
    int x = ({ int t = 41; t + 1; }); /* GNU statement expression — not ISO C */
    return x == 42 ? 0 : 1;
}
