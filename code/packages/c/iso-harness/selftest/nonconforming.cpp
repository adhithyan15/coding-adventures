/*
 * nonconforming.cpp — deliberately NON-ISO C++. Like nonconforming.c it uses a
 * GNU statement expression (`({ ... })`), which ISO C++ does not permit. Under
 * -pedantic-errors (GCC/Clang) or /permissive- (MSVC) this MUST fail to
 * compile. The harness self-test asserts every C++ compiler rejects it.
 */
int main() {
    int x = ({ int t = 41; t + 1; }); /* GNU statement expression — not ISO C++ */
    return x == 42 ? 0 : 1;
}
