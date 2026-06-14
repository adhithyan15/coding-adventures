// conduit_test.h — tiny zero-dependency assertion harness.
//
// We avoid GTest/Catch2 so the package has no test-time deps. Each test file
// defines CONDUIT_TEST(name) { ... } blocks; CONDUIT_MAIN() runs them all and
// reports failures to stderr.

#ifndef CONDUIT_TEST_H
#define CONDUIT_TEST_H

#include <iostream>
#include <string>
#include <vector>

namespace conduit_test {

struct TestCase {
    std::string name;
    void (*fn)();
};

inline std::vector<TestCase>& registry() {
    static std::vector<TestCase> r;
    return r;
}

inline int run_all() {
    int failed = 0;
    for (const auto& tc : registry()) {
        try {
            tc.fn();
            std::cout << "[PASS] " << tc.name << '\n';
        } catch (const std::exception& e) {
            std::cerr << "[FAIL] " << tc.name << ": " << e.what() << '\n';
            ++failed;
        } catch (...) {
            std::cerr << "[FAIL] " << tc.name << ": unknown exception\n";
            ++failed;
        }
    }
    std::cout << registry().size() << " tests, " << failed << " failures\n";
    return failed == 0 ? 0 : 1;
}

}  // namespace conduit_test

#define CONDUIT_CONCAT_INNER(a, b) a##b
#define CONDUIT_CONCAT(a, b) CONDUIT_CONCAT_INNER(a, b)

#define CONDUIT_TEST(name)                                                   \
    static void CONDUIT_CONCAT(test_fn_, __LINE__)();                        \
    namespace {                                                             \
    struct CONDUIT_CONCAT(test_reg_, __LINE__) {                            \
        CONDUIT_CONCAT(test_reg_, __LINE__)() {                             \
            ::conduit_test::registry().push_back(                          \
                {#name, &CONDUIT_CONCAT(test_fn_, __LINE__)});              \
        }                                                                  \
    } CONDUIT_CONCAT(test_reg_instance_, __LINE__);                         \
    }                                                                      \
    static void CONDUIT_CONCAT(test_fn_, __LINE__)()

#define CONDUIT_ASSERT(cond)                                                 \
    do {                                                                    \
        if (!(cond)) {                                                       \
            throw std::runtime_error(std::string("assertion failed: ") +    \
                                     #cond + " at " + __FILE__ + ":" +       \
                                     std::to_string(__LINE__));              \
        }                                                                   \
    } while (0)

#define CONDUIT_ASSERT_EQ(a, b)                                             \
    do {                                                                    \
        auto _av = (a);                                                     \
        auto _bv = (b);                                                     \
        if (!(_av == _bv)) {                                                 \
            throw std::runtime_error(std::string("assertion failed: ") +    \
                                     #a + " == " + #b + " at " + __FILE__ +  \
                                     ":" + std::to_string(__LINE__));        \
        }                                                                   \
    } while (0)

#define CONDUIT_MAIN() \
    int main() { return ::conduit_test::run_all(); }

#endif  // CONDUIT_TEST_H
