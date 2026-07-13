/*
 * Tests for garbage-collector, using the header-only iso_test.h harness (pure
 * ISO). Cases mirror the Rust crate's own unit tests.
 */
#include "iso_test.h"

#include <stdlib.h> /* free */

#include "garbage_collector.h"

int main(void) {
    { /* allocate and deref */
        GcHeap *gc = gc_new();
        size_t addr = gc_allocate(gc, gc_cons_new(42, -1));
        ISO_CHECK(gc_is_valid_address(gc, addr));
        ISO_CHECK_STR_EQ(gc_object_type_name(gc_deref(gc, addr)), "ConsCell");
        gc_free(gc);
    }
    { /* allocate a symbol */
        GcHeap *gc = gc_new();
        size_t addr = gc_allocate(gc, gc_symbol_new("foo"));
        ISO_CHECK(gc_is_valid_address(gc, addr));
        ISO_CHECK(gc_heap_size(gc) == 1);
        gc_free(gc);
    }
    { /* collect unreachable */
        GcHeap *gc = gc_new();
        size_t a1 = gc_allocate(gc, gc_cons_new(42, -1));
        gc_allocate(gc, gc_symbol_new("unreachable"));
        ISO_CHECK(gc_heap_size(gc) == 2);
        GcValue roots[1] = {gc_val_address(a1)};
        ISO_CHECK(gc_collect(gc, roots, 1) == 1);
        ISO_CHECK(gc_heap_size(gc) == 1);
        ISO_CHECK(gc_is_valid_address(gc, a1));
        gc_free(gc);
    }
    { /* reachable chain survives */
        GcHeap *gc = gc_new();
        size_t a2 = gc_allocate(gc, gc_symbol_new("end"));
        size_t a1 = gc_allocate(gc, gc_cons_new((int64_t)a2, -1));
        GcValue roots[1] = {gc_val_address(a1)};
        ISO_CHECK(gc_collect(gc, roots, 1) == 0);
        ISO_CHECK(gc_heap_size(gc) == 2);
        gc_free(gc);
    }
    { /* unreachable cycle / standalone collected */
        GcHeap *gc = gc_new();
        size_t a1 = gc_allocate(gc, gc_cons_new(0, 0));
        gc_allocate(gc, gc_cons_new(0, 0));
        gc_allocate(gc, gc_symbol_new("standalone"));
        /* a1 has car=cdr=0, not valid heap addresses (they start at 0x10000) */
        GcValue roots[1] = {gc_val_address(a1)};
        ISO_CHECK(gc_collect(gc, roots, 1) == 2);
        gc_free(gc);
    }
    { /* no roots frees everything */
        GcHeap *gc = gc_new();
        gc_allocate(gc, gc_cons_new(1, 2));
        gc_allocate(gc, gc_symbol_new("orphan"));
        ISO_CHECK(gc_heap_size(gc) == 2);
        ISO_CHECK(gc_collect(gc, NULL, 0) == 2);
        ISO_CHECK(gc_heap_size(gc) == 0);
        gc_free(gc);
    }
    { /* stats */
        GcHeap *gc = gc_new();
        gc_allocate(gc, gc_symbol_new("a"));
        gc_allocate(gc, gc_symbol_new("b"));
        gc_collect(gc, NULL, 0);
        GcStats s = gc_stats(gc);
        ISO_CHECK(s.total_allocations == 2);
        ISO_CHECK(s.total_collections == 1);
        ISO_CHECK(s.total_freed == 2);
        ISO_CHECK(s.heap_size == 0);
        gc_free(gc);
    }
    { /* address space starts at 0x10000 and increments */
        GcHeap *gc = gc_new();
        size_t a1 = gc_allocate(gc, gc_symbol_new("a"));
        size_t a2 = gc_allocate(gc, gc_symbol_new("b"));
        ISO_CHECK(a1 == 0x10000);
        ISO_CHECK(a2 == 0x10001);
        gc_free(gc);
    }
    { /* closure references only its valid (>= 0) environment addresses */
        const char *keys[2] = {"x", "y"};
        int64_t vals[2] = {0x10000, -1};
        const char *params[1] = {"a"};
        GcObject *closure =
            gc_closure_new("(lambda (a) (+ a x))", keys, vals, 2, params, 1);
        size_t n = 0;
        size_t *refs = gc_object_references(closure, &n);
        ISO_CHECK(n == 1 && refs != NULL && refs[0] == 0x10000);
        free(refs);
        gc_object_free(closure);
    }
    { /* symbol table interns by name */
        GcHeap *gc = gc_new();
        GcSymbolTable *t = gc_symbol_table_new(gc);
        size_t a1 = gc_symbol_table_intern(t, "foo");
        size_t a2 = gc_symbol_table_intern(t, "foo");
        ISO_CHECK(a1 == a2);
        size_t a3 = gc_symbol_table_intern(t, "bar");
        ISO_CHECK(a1 != a3);
        gc_symbol_table_free(t);
        gc_free(gc);
    }
    { /* symbol table lookup */
        GcHeap *gc = gc_new();
        GcSymbolTable *t = gc_symbol_table_new(gc);
        size_t addr;
        ISO_CHECK(!gc_symbol_table_lookup(t, "foo", &addr));
        gc_symbol_table_intern(t, "foo");
        ISO_CHECK(gc_symbol_table_lookup(t, "foo", &addr));
        gc_symbol_table_free(t);
        gc_free(gc);
    }
    { /* symbol table reports all live symbols */
        GcHeap *gc = gc_new();
        GcSymbolTable *t = gc_symbol_table_new(gc);
        gc_symbol_table_intern(t, "foo");
        gc_symbol_table_intern(t, "bar");
        gc_symbol_table_intern(t, "baz");
        ISO_CHECK(gc_symbol_table_count(t) == 3);
        ISO_CHECK(gc_symbol_table_contains(t, "foo"));
        ISO_CHECK(gc_symbol_table_contains(t, "bar"));
        ISO_CHECK(gc_symbol_table_contains(t, "baz"));
        gc_symbol_table_free(t);
        gc_free(gc);
    }
    { /* multiple collections keep only the rooted object */
        GcHeap *gc = gc_new();
        size_t root = gc_allocate(gc, gc_symbol_new("root"));
        for (int i = 0; i < 5; i++) gc_allocate(gc, gc_symbol_new("temp"));
        GcValue roots[1] = {gc_val_address(root)};
        gc_collect(gc, roots, 1);
        ISO_CHECK(gc_heap_size(gc) == 1);
        for (int i = 0; i < 3; i++) gc_allocate(gc, gc_symbol_new("temp2"));
        gc_collect(gc, roots, 1);
        ISO_CHECK(gc_heap_size(gc) == 1);
        GcStats s = gc_stats(gc);
        ISO_CHECK(s.total_allocations == 9);
        ISO_CHECK(s.total_collections == 2);
        ISO_CHECK(s.total_freed == 8);
        gc_free(gc);
    }
    { /* a list of values works as roots (simulating a VM stack) */
        GcHeap *gc = gc_new();
        size_t a1 = gc_allocate(gc, gc_symbol_new("a"));
        size_t a2 = gc_allocate(gc, gc_symbol_new("b"));
        gc_allocate(gc, gc_symbol_new("c"));
        GcValue inner[2] = {gc_val_address(a1), gc_val_address(a2)};
        GcValue roots[1] = {gc_val_list(inner, 2)};
        ISO_CHECK(gc_collect(gc, roots, 1) == 1);
        gc_value_free(&roots[0]);
        gc_free(gc);
    }
    { /* deref of a freed object returns nothing */
        GcHeap *gc = gc_new();
        size_t addr = gc_allocate(gc, gc_symbol_new("gone"));
        gc_collect(gc, NULL, 0);
        ISO_CHECK(gc_deref(gc, addr) == NULL);
        ISO_CHECK(!gc_is_valid_address(gc, addr));
        gc_free(gc);
    }

    return ISO_TEST_RESULT();
}
