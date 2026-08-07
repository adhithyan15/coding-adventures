// static_vector_test.cpp — behavioral tests for ca::static_vector, using the
// header-only iso_test.h harness (pure ISO C++17). Covers the vector-like
// surface: push/pop, full/empty, indexing, checked at() throwing, range-for,
// and clear.
#include "iso_test.h"

#include "static_vector.hpp"

#include <stdexcept>

int main() {
    ca::static_vector<int, 3> v;

    // Fresh vector: empty, capacity 3.
    ISO_CHECK(v.empty());
    ISO_CHECK(!v.full());
    ISO_CHECK_EQ_INT(v.size(), 0);
    ISO_CHECK_EQ_INT(v.capacity(), 3);

    // Fill to capacity; the fourth push_back must fail (full), not overflow.
    ISO_CHECK(v.push_back(10));
    ISO_CHECK(v.push_back(20));
    ISO_CHECK(v.push_back(30));
    ISO_CHECK(v.full());
    ISO_CHECK(!v.push_back(40));
    ISO_CHECK_EQ_INT(v.size(), 3);

    // Indexing and checked access.
    ISO_CHECK_EQ_INT(v[0], 10);
    ISO_CHECK_EQ_INT(v.at(2), 30);

    // at() past the end throws std::out_of_range.
    bool threw = false;
    try {
        (void)v.at(3);
    } catch (const std::out_of_range &) {
        threw = true;
    }
    ISO_CHECK_MSG(threw, "at() must throw std::out_of_range past the end");

    // Range-for visits exactly the live elements, in order: 10 + 20 + 30 = 60.
    int total = 0;
    for (int x : v) {
        total += x;
    }
    ISO_CHECK_EQ_INT(total, 60);

    // pop_back shrinks; a freed slot can be reused.
    v.pop_back();
    ISO_CHECK_EQ_INT(v.size(), 2);
    ISO_CHECK(v.push_back(99));
    ISO_CHECK_EQ_INT(v[2], 99);

    // clear empties without touching capacity.
    v.clear();
    ISO_CHECK(v.empty());
    ISO_CHECK_EQ_INT(v.capacity(), 3);

    return ISO_TEST_RESULT();
}
