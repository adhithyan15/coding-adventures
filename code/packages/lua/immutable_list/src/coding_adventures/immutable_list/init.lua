-- coding_adventures.immutable_list
-- ============================================================================
--
-- A PERSISTENT IMMUTABLE SINGLY-LINKED LIST
--
-- This module implements the classic functional-programming linked list,
-- often called a "cons list" (from the Lisp function `cons` — short for
-- "construct"). It is the simplest persistent data structure: every operation
-- that "modifies" a list actually returns a *new* list, sharing structure with
-- the old one.
--
-- STRUCTURE
-- ---------
-- A list is either:
--   1. The EMPTY list:   { is_empty = true }
--   2. A CONS CELL:      { head = value, tail = <list> }
--
-- Example — the list [3, 2, 1]:
--
--   cons(3, cons(2, cons(1, empty())))
--
--   In memory:
--     node_a = { head=3, tail=node_b }
--     node_b = { head=2, tail=node_c }
--     node_c = { head=1, tail=empty_sentinel }
--
-- STRUCTURAL SHARING
-- ------------------
-- When we `cons` a new element onto an existing list, we create exactly one
-- new node. The tail points directly to the old list — no copying occurs.
--
--   l1 = cons(1, empty)       → [1]
--   l2 = cons(2, l1)          → [2, 1]
--   l3 = cons(3, l2)          → [3, 2, 1]
--
-- l1, l2, and l3 all share the same cons cell for `1`. This is "structural
-- sharing" — old lists are never modified, so sharing is safe.
--
-- PERFORMANCE
-- -----------
--   cons   : O(1) — create one new node
--   head   : O(1) — read the head field
--   tail   : O(1) — read the tail field
--   length : O(n) — traverse all nodes
--   append : O(n) — must copy the left list
--   reverse: O(n)
--   map    : O(n)
--   filter : O(n)
--   foldl  : O(n)
--
-- Usage:
--   local List = require("coding_adventures.immutable_list")
--   local l = List.from_array({1, 2, 3})
--   print(List.head(l))    -- 1
--   print(List.length(l))  -- 3
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ---------------------------------------------------------------------------
-- The empty-list sentinel
-- ---------------------------------------------------------------------------
--
-- We use a single, unique table as the canonical empty list. All empty lists
-- are the same object, so `is_empty(l)` is a simple field check.

local EMPTY = { is_empty = true }

-- ---------------------------------------------------------------------------
-- Constructors
-- ---------------------------------------------------------------------------

-- List.empty()
--   Return the unique empty-list sentinel.
--   An empty list has no head and no tail — it is the base case.
function M.empty()
    return EMPTY
end

-- List.cons(value, list)
--   Prepend `value` to `list`, returning a new cons cell.
--
--   This is the fundamental building block. Given:
--     l = cons(2, cons(1, empty))   -- [2, 1]
--   then:
--     cons(3, l)                    -- [3, 2, 1]  (l is unchanged)
--
--   Parameters:
--     value — any Lua value to store as the head element
--     list  — the existing list to use as the tail
function M.cons(value, list)
    return { head = value, tail = list }
end

-- ---------------------------------------------------------------------------
-- Predicates
-- ---------------------------------------------------------------------------

-- List.is_empty(list)
--   Return true if `list` is the empty list, false otherwise.
function M.is_empty(list)
    return list.is_empty == true
end

-- ---------------------------------------------------------------------------
-- Accessors
-- ---------------------------------------------------------------------------

-- List.head(list)
--   Return the first element of `list`.
--   Raises an error if the list is empty (there is no head to return).
function M.head(list)
    assert(not M.is_empty(list), "immutable_list.head: cannot take head of empty list")
    return list.head
end

-- List.tail(list)
--   Return everything after the first element.
--   Raises an error if the list is empty.
function M.tail(list)
    assert(not M.is_empty(list), "immutable_list.tail: cannot take tail of empty list")
    return list.tail
end

-- ---------------------------------------------------------------------------
-- Measurement
-- ---------------------------------------------------------------------------

-- List.length(list)
--   Count the number of elements in `list` by traversing it.
--   This is O(n) — linear in the number of elements.
function M.length(list)
    local count = 0
    local node = list
    while not M.is_empty(node) do
        count = count + 1
        node = node.tail
    end
    return count
end

-- ---------------------------------------------------------------------------
-- Conversion
-- ---------------------------------------------------------------------------

-- List.to_array(list)
--   Convert a list to a Lua array table, preserving order.
--   The first element of the list becomes index 1 of the array.
--
--   Example: to_array(cons(3, cons(2, cons(1, empty)))) → {3, 2, 1}
function M.to_array(list)
    local arr = {}
    local node = list
    while not M.is_empty(node) do
        arr[#arr + 1] = node.head
        node = node.tail
    end
    return arr
end

-- List.from_array(arr)
--   Build a list from a Lua array table.
--   The element at index 1 becomes the head of the resulting list.
--
--   Example: from_array({1, 2, 3}) → cons(1, cons(2, cons(3, empty)))
--     → head = 1, tail.head = 2, tail.tail.head = 3
--
--   We build from the BACK so that arr[1] ends up at the front.
function M.from_array(arr)
    local list = EMPTY
    for i = #arr, 1, -1 do
        list = M.cons(arr[i], list)
    end
    return list
end

-- ---------------------------------------------------------------------------
-- List operations
-- ---------------------------------------------------------------------------

-- List.append(left, right)
--   Concatenate two lists: all elements of `left` followed by all of `right`.
--
--   This is O(n) in the length of `left` — we must copy every node of `left`
--   because each node's tail needs to point to the next element of the result.
--   `right` is shared without copying.
--
--   Example:
--     append([3, 2], [1]) → [3, 2, 1]
--
--   Algorithm: collect left elements into an array, then rebuild prepending
--   each onto `right` in reverse.
function M.append(left, right)
    -- Collect left elements
    local elems = M.to_array(left)
    -- Prepend onto right in reverse order
    local result = right
    for i = #elems, 1, -1 do
        result = M.cons(elems[i], result)
    end
    return result
end

-- List.reverse(list)
--   Return a new list with elements in reverse order.
--
--   Example: reverse([3, 2, 1]) → [1, 2, 3]
--
--   Uses a left-fold: start with empty, then for each element, cons it onto
--   the accumulator. Since cons prepends, the result is reversed.
function M.reverse(list)
    local acc = EMPTY
    local node = list
    while not M.is_empty(node) do
        acc = M.cons(node.head, acc)
        node = node.tail
    end
    return acc
end

-- List.map(list, fn)
--   Apply function `fn` to each element, returning a new list of results.
--   Order is preserved: map([3,2,1], f) → [f(3), f(2), f(1)].
--
--   Because cons prepends, we build the result in reverse and then reverse it.
function M.map(list, fn)
    local acc = EMPTY
    local node = list
    while not M.is_empty(node) do
        acc = M.cons(fn(node.head), acc)
        node = node.tail
    end
    return M.reverse(acc)
end

-- List.filter(list, pred)
--   Keep only elements for which predicate `pred` returns true.
--   Order is preserved.
--
--   Example: filter([1,2,3,4], is_even) → [2, 4]
function M.filter(list, pred)
    local acc = EMPTY
    local node = list
    while not M.is_empty(node) do
        if pred(node.head) then
            acc = M.cons(node.head, acc)
        end
        node = node.tail
    end
    return M.reverse(acc)
end

-- List.foldl(list, init, fn)
--   Left fold: combine elements left-to-right with a binary function.
--
--   foldl([a, b, c], z, f) = f(f(f(z, a), b), c)
--
--   The fold "accumulates" a result by processing elements one by one:
--     acc = init
--     acc = fn(acc, a)
--     acc = fn(acc, b)
--     acc = fn(acc, c)
--     return acc
--
--   Example: foldl([1,2,3], 0, add) = ((0+1)+2)+3 = 6
function M.foldl(list, init, fn)
    local acc = init
    local node = list
    while not M.is_empty(node) do
        acc = fn(acc, node.head)
        node = node.tail
    end
    return acc
end

-- List.nth(list, n)
--   Return the n-th element of the list (1-based indexing).
--   Raises an error if n is out of range.
--
--   Example: nth([3,2,1], 1) → 3
--            nth([3,2,1], 2) → 2
function M.nth(list, n)
    assert(type(n) == "number" and n >= 1, "immutable_list.nth: n must be >= 1")
    local node = list
    local i = 1
    while not M.is_empty(node) do
        if i == n then
            return node.head
        end
        i = i + 1
        node = node.tail
    end
    error("immutable_list.nth: index " .. n .. " out of range (length " .. (i-1) .. ")")
end

-- ---------------------------------------------------------------------------

return M
