-- Tests for coding_adventures.immutable_list
--
-- Covers: empty, cons, is_empty, head, tail, length, to_array, from_array,
--         append, reverse, map, filter, foldl, nth
--
-- Lua 5.4 busted test suite.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
local List = require("coding_adventures.immutable_list")

describe("immutable_list", function()

    -- -----------------------------------------------------------------------
    -- Version
    -- -----------------------------------------------------------------------

    it("has VERSION", function()
        assert.is_not_nil(List.VERSION)
        assert.equals("0.1.0", List.VERSION)
    end)

    -- -----------------------------------------------------------------------
    -- empty / is_empty
    -- -----------------------------------------------------------------------

    it("empty() returns an empty list", function()
        local e = List.empty()
        assert.is_true(List.is_empty(e))
    end)

    it("empty() always returns the same sentinel object", function()
        assert.equals(List.empty(), List.empty())
    end)

    it("is_empty returns false for a non-empty list", function()
        local l = List.cons(1, List.empty())
        assert.is_false(List.is_empty(l))
    end)

    -- -----------------------------------------------------------------------
    -- cons / head / tail
    -- -----------------------------------------------------------------------

    it("cons creates a list with the given head", function()
        local l = List.cons(42, List.empty())
        assert.equals(42, List.head(l))
    end)

    it("cons creates a list whose tail is the given list", function()
        local e  = List.empty()
        local l  = List.cons(42, e)
        assert.equals(e, List.tail(l))
    end)

    it("head of a one-element list returns that element", function()
        local l = List.cons("hello", List.empty())
        assert.equals("hello", List.head(l))
    end)

    it("tail of a one-element list is empty", function()
        local l = List.cons(1, List.empty())
        assert.is_true(List.is_empty(List.tail(l)))
    end)

    it("cons chains correctly to form [3,2,1]", function()
        local l = List.cons(3, List.cons(2, List.cons(1, List.empty())))
        assert.equals(3, List.head(l))
        assert.equals(2, List.head(List.tail(l)))
        assert.equals(1, List.head(List.tail(List.tail(l))))
        assert.is_true(List.is_empty(List.tail(List.tail(List.tail(l)))))
    end)

    it("head raises an error on empty list", function()
        assert.has_error(function()
            List.head(List.empty())
        end)
    end)

    it("tail raises an error on empty list", function()
        assert.has_error(function()
            List.tail(List.empty())
        end)
    end)

    -- -----------------------------------------------------------------------
    -- length
    -- -----------------------------------------------------------------------

    it("length of empty list is 0", function()
        assert.equals(0, List.length(List.empty()))
    end)

    it("length of single-element list is 1", function()
        local l = List.cons(99, List.empty())
        assert.equals(1, List.length(l))
    end)

    it("length of a three-element list is 3", function()
        local l = List.cons(3, List.cons(2, List.cons(1, List.empty())))
        assert.equals(3, List.length(l))
    end)

    -- -----------------------------------------------------------------------
    -- to_array / from_array
    -- -----------------------------------------------------------------------

    it("to_array of empty list returns empty table", function()
        local arr = List.to_array(List.empty())
        assert.equals(0, #arr)
    end)

    it("to_array preserves element order", function()
        local l = List.cons(3, List.cons(2, List.cons(1, List.empty())))
        local arr = List.to_array(l)
        assert.equals(3, #arr)
        assert.equals(3, arr[1])
        assert.equals(2, arr[2])
        assert.equals(1, arr[3])
    end)

    it("from_array of empty table returns empty list", function()
        local l = List.from_array({})
        assert.is_true(List.is_empty(l))
    end)

    it("from_array puts first array element at the head", function()
        local l = List.from_array({1, 2, 3})
        assert.equals(1, List.head(l))
    end)

    it("from_array then to_array is a round-trip", function()
        local orig = {10, 20, 30, 40}
        local arr  = List.to_array(List.from_array(orig))
        assert.equals(#orig, #arr)
        for i = 1, #orig do
            assert.equals(orig[i], arr[i])
        end
    end)

    it("from_array builds a list of the right length", function()
        local l = List.from_array({5, 6, 7, 8, 9})
        assert.equals(5, List.length(l))
    end)

    -- -----------------------------------------------------------------------
    -- append
    -- -----------------------------------------------------------------------

    it("append of two empty lists is empty", function()
        local result = List.append(List.empty(), List.empty())
        assert.is_true(List.is_empty(result))
    end)

    it("append empty to a list returns the list contents", function()
        local l = List.from_array({1, 2, 3})
        local result = List.append(List.empty(), l)
        assert.equals(3, List.length(result))
        assert.equals(1, List.head(result))
    end)

    it("append a list to empty returns the list contents", function()
        local l = List.from_array({1, 2, 3})
        local result = List.append(l, List.empty())
        assert.equals(3, List.length(result))
        assert.equals(1, List.head(result))
    end)

    it("append concatenates two lists in the correct order", function()
        local a = List.from_array({1, 2})
        local b = List.from_array({3, 4})
        local result = List.to_array(List.append(a, b))
        assert.equals(4, #result)
        assert.equals(1, result[1])
        assert.equals(2, result[2])
        assert.equals(3, result[3])
        assert.equals(4, result[4])
    end)

    -- -----------------------------------------------------------------------
    -- reverse
    -- -----------------------------------------------------------------------

    it("reverse of empty list is empty", function()
        assert.is_true(List.is_empty(List.reverse(List.empty())))
    end)

    it("reverse of single-element list is itself", function()
        local l = List.cons(42, List.empty())
        local r = List.reverse(l)
        assert.equals(42, List.head(r))
        assert.is_true(List.is_empty(List.tail(r)))
    end)

    it("reverse reverses element order", function()
        local l = List.from_array({1, 2, 3})
        local r = List.to_array(List.reverse(l))
        assert.equals(3, r[1])
        assert.equals(2, r[2])
        assert.equals(1, r[3])
    end)

    it("double reverse returns original order", function()
        local orig = {10, 20, 30, 40, 50}
        local l    = List.from_array(orig)
        local arr  = List.to_array(List.reverse(List.reverse(l)))
        for i = 1, #orig do
            assert.equals(orig[i], arr[i])
        end
    end)

    -- -----------------------------------------------------------------------
    -- map
    -- -----------------------------------------------------------------------

    it("map of empty list is empty", function()
        local result = List.map(List.empty(), function(x) return x * 2 end)
        assert.is_true(List.is_empty(result))
    end)

    it("map applies function to each element preserving order", function()
        local l   = List.from_array({1, 2, 3})
        local res = List.to_array(List.map(l, function(x) return x * 2 end))
        assert.equals(2, res[1])
        assert.equals(4, res[2])
        assert.equals(6, res[3])
    end)

    it("map does not modify the original list", function()
        local l = List.from_array({1, 2, 3})
        List.map(l, function(x) return x + 100 end)
        assert.equals(1, List.head(l))
    end)

    -- -----------------------------------------------------------------------
    -- filter
    -- -----------------------------------------------------------------------

    it("filter of empty list is empty", function()
        local result = List.filter(List.empty(), function() return true end)
        assert.is_true(List.is_empty(result))
    end)

    it("filter keeps only matching elements", function()
        local l   = List.from_array({1, 2, 3, 4, 5})
        local res = List.to_array(List.filter(l, function(x) return x % 2 == 0 end))
        assert.equals(2, #res)
        assert.equals(2, res[1])
        assert.equals(4, res[2])
    end)

    it("filter with always-false predicate returns empty list", function()
        local l   = List.from_array({1, 2, 3})
        local res = List.filter(l, function() return false end)
        assert.is_true(List.is_empty(res))
    end)

    it("filter with always-true predicate returns all elements", function()
        local l   = List.from_array({1, 2, 3})
        local res = List.filter(l, function() return true end)
        assert.equals(3, List.length(res))
    end)

    -- -----------------------------------------------------------------------
    -- foldl
    -- -----------------------------------------------------------------------

    it("foldl of empty list returns the initial accumulator", function()
        local result = List.foldl(List.empty(), 99, function(acc, _) return acc end)
        assert.equals(99, result)
    end)

    it("foldl sums a list of numbers", function()
        local l = List.from_array({1, 2, 3, 4, 5})
        local sum = List.foldl(l, 0, function(acc, x) return acc + x end)
        assert.equals(15, sum)
    end)

    it("foldl builds a string left-to-right", function()
        local l   = List.from_array({"a", "b", "c"})
        local str = List.foldl(l, "", function(acc, x) return acc .. x end)
        assert.equals("abc", str)
    end)

    -- -----------------------------------------------------------------------
    -- nth
    -- -----------------------------------------------------------------------

    it("nth(1) returns the head element", function()
        local l = List.from_array({10, 20, 30})
        assert.equals(10, List.nth(l, 1))
    end)

    it("nth(2) returns the second element", function()
        local l = List.from_array({10, 20, 30})
        assert.equals(20, List.nth(l, 2))
    end)

    it("nth(3) returns the last element of a three-element list", function()
        local l = List.from_array({10, 20, 30})
        assert.equals(30, List.nth(l, 3))
    end)

    it("nth raises an error for index out of range", function()
        local l = List.from_array({1, 2, 3})
        assert.has_error(function()
            List.nth(l, 4)
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Structural sharing
    -- -----------------------------------------------------------------------

    it("cons shares the tail without copying it", function()
        -- l1 and l2 share the same tail nodes (structural sharing)
        local shared = List.cons(1, List.empty())
        local l1 = List.cons(2, shared)
        local l2 = List.cons(3, shared)
        -- Both l1 and l2 point to the same `shared` node
        assert.equals(List.tail(l1), List.tail(l2))
    end)

end)
