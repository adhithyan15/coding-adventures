// Tests for the C++ garbage-collector library, using the header-only iso_test.h
// harness (pure ISO). Cases mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <memory>
#include <string>
#include <unordered_map>
#include <vector>

#include "garbage_collector.hpp"

namespace gc = ca::garbage_collector;
using gc::ConsCell;
using gc::LispClosure;
using gc::MarkAndSweepGC;
using gc::Symbol;
using gc::Value;

int main() {
    {  // allocate and deref
        MarkAndSweepGC g;
        auto addr = g.allocate(std::make_unique<ConsCell>(42, -1));
        ISO_CHECK(g.is_valid_address(addr));
        ISO_CHECK(std::string(g.deref(addr)->type_name()) == "ConsCell");
    }
    {  // allocate a symbol
        MarkAndSweepGC g;
        auto addr = g.allocate(std::make_unique<Symbol>("foo"));
        ISO_CHECK(g.is_valid_address(addr));
        ISO_CHECK(g.heap_size() == 1);
    }
    {  // collect unreachable
        MarkAndSweepGC g;
        auto a1 = g.allocate(std::make_unique<ConsCell>(42, -1));
        g.allocate(std::make_unique<Symbol>("unreachable"));
        ISO_CHECK(g.heap_size() == 2);
        ISO_CHECK(g.collect({Value::address(a1)}) == 1);
        ISO_CHECK(g.heap_size() == 1);
        ISO_CHECK(g.is_valid_address(a1));
    }
    {  // reachable chain survives
        MarkAndSweepGC g;
        auto a2 = g.allocate(std::make_unique<Symbol>("end"));
        auto a1 = g.allocate(
            std::make_unique<ConsCell>(static_cast<std::int64_t>(a2), -1));
        ISO_CHECK(g.collect({Value::address(a1)}) == 0);
        ISO_CHECK(g.heap_size() == 2);
    }
    {  // unreachable cycle / standalone collected
        MarkAndSweepGC g;
        auto a1 = g.allocate(std::make_unique<ConsCell>(0, 0));
        g.allocate(std::make_unique<ConsCell>(0, 0));
        g.allocate(std::make_unique<Symbol>("standalone"));
        ISO_CHECK(g.collect({Value::address(a1)}) == 2);
    }
    {  // no roots frees everything
        MarkAndSweepGC g;
        g.allocate(std::make_unique<ConsCell>(1, 2));
        g.allocate(std::make_unique<Symbol>("orphan"));
        ISO_CHECK(g.heap_size() == 2);
        ISO_CHECK(g.collect({}) == 2);
        ISO_CHECK(g.heap_size() == 0);
    }
    {  // stats
        MarkAndSweepGC g;
        g.allocate(std::make_unique<Symbol>("a"));
        g.allocate(std::make_unique<Symbol>("b"));
        g.collect({});
        gc::GcStats want{2, 1, 2, 0};
        ISO_CHECK(g.stats() == want);
    }
    {  // address space starts at 0x10000 and increments
        MarkAndSweepGC g;
        auto a1 = g.allocate(std::make_unique<Symbol>("a"));
        auto a2 = g.allocate(std::make_unique<Symbol>("b"));
        ISO_CHECK(a1 == 0x10000);
        ISO_CHECK(a2 == 0x10001);
    }
    {  // closure references only its valid (>= 0) environment addresses
        std::unordered_map<std::string, std::int64_t> env{{"x", 0x10000},
                                                          {"y", -1}};
        LispClosure closure("(lambda (a) (+ a x))", env, {"a"});
        auto refs = closure.references();
        ISO_CHECK(refs.size() == 1 && refs[0] == 0x10000);
    }
    {  // symbol table interns by name
        MarkAndSweepGC g;
        gc::SymbolTable t(g);
        auto a1 = t.intern("foo");
        auto a2 = t.intern("foo");
        ISO_CHECK(a1 == a2);
        ISO_CHECK(t.intern("bar") != a1);
    }
    {  // symbol table lookup
        MarkAndSweepGC g;
        gc::SymbolTable t(g);
        ISO_CHECK(!t.lookup("foo").has_value());
        t.intern("foo");
        ISO_CHECK(t.lookup("foo").has_value());
    }
    {  // symbol table reports all live symbols
        MarkAndSweepGC g;
        gc::SymbolTable t(g);
        t.intern("foo");
        t.intern("bar");
        t.intern("baz");
        auto syms = t.all_symbols();
        ISO_CHECK(syms.size() == 3);
        ISO_CHECK(syms.count("foo") && syms.count("bar") && syms.count("baz"));
    }
    {  // multiple collections keep only the rooted object
        MarkAndSweepGC g;
        auto root = g.allocate(std::make_unique<Symbol>("root"));
        for (int i = 0; i < 5; ++i) g.allocate(std::make_unique<Symbol>("temp"));
        g.collect({Value::address(root)});
        ISO_CHECK(g.heap_size() == 1);
        for (int i = 0; i < 3; ++i)
            g.allocate(std::make_unique<Symbol>("temp2"));
        g.collect({Value::address(root)});
        ISO_CHECK(g.heap_size() == 1);
        gc::GcStats s = g.stats();
        ISO_CHECK(s.total_allocations == 9);
        ISO_CHECK(s.total_collections == 2);
        ISO_CHECK(s.total_freed == 8);
    }
    {  // a list of values works as roots (simulating a VM stack)
        MarkAndSweepGC g;
        auto a1 = g.allocate(std::make_unique<Symbol>("a"));
        auto a2 = g.allocate(std::make_unique<Symbol>("b"));
        g.allocate(std::make_unique<Symbol>("c"));
        ISO_CHECK(g.collect({Value::list(
                      {Value::address(a1), Value::address(a2)})}) == 1);
    }
    {  // deref of a freed object returns nothing
        MarkAndSweepGC g;
        auto addr = g.allocate(std::make_unique<Symbol>("gone"));
        g.collect({});
        ISO_CHECK(g.deref(addr) == nullptr);
        ISO_CHECK(!g.is_valid_address(addr));
    }

    return ISO_TEST_RESULT();
}
