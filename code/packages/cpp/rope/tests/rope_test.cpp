// Tests for the C++ rope, using the iso_test.h harness. Pinned to the Rust
// crate's own assertions plus edge cases; value semantics (copyable).
#include "iso_test.h"

#include <string>

#include "rope.hpp"

int main() {
    // concat / index / split.
    {
        auto r = ca::rope::concat(ca::rope::from_string("hello"),
                                  ca::rope::from_string(" world"));
        ISO_CHECK_EQ_UINT(r.len(), 11);
        ISO_CHECK(r.index(1) == std::optional<char>('e'));
        auto parts = r.split(5);
        ISO_CHECK(parts.first.to_string() == "hello");
        ISO_CHECK(parts.second.to_string() == " world");
    }

    // insert / erase / rebalance.
    {
        auto r = ca::rope::from_string("ace").insert(1, "b");
        r = r.insert(3, "d");
        ISO_CHECK(r.to_string() == "abcde");
        r = r.erase(1, 2);
        ISO_CHECK(r.to_string() == "ade");

        auto b = ca::rope::concat(ca::rope::from_string("ab"),
                                  ca::rope::from_string("cdef"))
                     .rebalance();
        ISO_CHECK(b.is_balanced());
        ISO_CHECK(b.depth() <= 3);
        ISO_CHECK(b.substring(1, 4) == "bcd");
    }

    // Empty rope.
    {
        ca::rope e;
        ISO_CHECK(e.empty());
        ISO_CHECK_EQ_UINT(e.len(), 0);
        ISO_CHECK_EQ_UINT(e.depth(), 0);
        ISO_CHECK(e.is_balanced());
        ISO_CHECK(!e.index(0).has_value());
        ISO_CHECK(e.to_string().empty());
    }

    // Concat with an empty operand returns the other side.
    {
        auto a = ca::rope::concat(ca::rope::from_string("hi"), ca::rope());
        auto b = ca::rope::concat(ca::rope(), ca::rope::from_string("yo"));
        ISO_CHECK(a.to_string() == "hi");
        ISO_CHECK(b.to_string() == "yo");
    }

    // Value semantics: copies are independent (structural sharing is safe
    // because nodes are immutable).
    {
        auto a = ca::rope::from_string("abc");
        auto c = a;                 // copy
        auto d = c.insert(1, "XY"); // does not disturb a or c
        ISO_CHECK(a.to_string() == "abc");
        ISO_CHECK(c.to_string() == "abc");
        ISO_CHECK(d.to_string() == "aXYbc");
    }

    // substring / erase clamping.
    {
        auto s = ca::rope::from_string("abcdef");
        ISO_CHECK(s.substring(4, 100) == "ef");
        ISO_CHECK(s.substring(3, 3).empty());
        ISO_CHECK(s.erase(2, 100).to_string() == "ab");
    }

    // Weighted index into a concatenated rope.
    {
        auto r = ca::rope::concat(
            ca::rope::concat(ca::rope::from_string("ab"),
                             ca::rope::from_string("cd")),
            ca::rope::from_string("ef"));
        ISO_CHECK(r.index(0) == std::optional<char>('a'));
        ISO_CHECK(r.index(3) == std::optional<char>('d'));
        ISO_CHECK(r.index(5) == std::optional<char>('f'));
        ISO_CHECK(!r.index(6).has_value());
    }

    return ISO_TEST_RESULT();
}
