-- Tests for lisp_parser
-- ======================
--
-- Comprehensive busted test suite for the Lisp parser package.
--
-- The Lisp grammar has six rules:
--
--   program   = { sexpr }
--   sexpr     = atom | list | quoted
--   atom      = NUMBER | SYMBOL | STRING
--   list      = LPAREN list_body RPAREN
--   list_body = [ sexpr { sexpr } [ DOT sexpr ] ]
--   quoted    = QUOTE sexpr
--
-- Coverage:
--   - Module loads and exposes the public API
--   - parse() returns an ASTNode with rule_name "program"
--   - Atom parsing: NUMBER, SYMBOL, STRING
--   - Empty program (only whitespace/comments)
--   - Empty list ()
--   - Simple lists with atoms
--   - Nested lists (arbitrary depth)
--   - Quoted forms 'x and '(...)
--   - Dotted pairs (a . b)
--   - Multi-expression programs
--   - Real-world Lisp programs
--   - create_parser() returns an uninvoked parser
--   - get_grammar() returns the grammar object
--   - Error cases: unterminated list, garbage input

-- Resolve sibling packages from the monorepo so busted can find them.
package.path = (
    "../src/?.lua;"                                           ..
    "../src/?/init.lua;"                                      ..
    "../../lisp_lexer/src/?.lua;"                             ..
    "../../lisp_lexer/src/?/init.lua;"                        ..
    "../../grammar_tools/src/?.lua;"                          ..
    "../../grammar_tools/src/?/init.lua;"                     ..
    "../../lexer/src/?.lua;"                                  ..
    "../../lexer/src/?/init.lua;"                             ..
    "../../parser/src/?.lua;"                                 ..
    "../../parser/src/?/init.lua;"                            ..
    "../../state_machine/src/?.lua;"                          ..
    "../../state_machine/src/?/init.lua;"                     ..
    "../../directed_graph/src/?.lua;"                         ..
    "../../directed_graph/src/?/init.lua;"                    ..
    package.path
)

local lisp_parser = require("coding_adventures.lisp_parser")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Recursively count ASTNodes with a given rule_name.
-- @param node       ASTNode
-- @param rule_name  string
-- @return number
local function count_nodes(node, rule_name)
    if type(node) ~= "table" then return 0 end
    local n = (node.rule_name == rule_name) and 1 or 0
    if node.children then
        for _, child in ipairs(node.children) do
            n = n + count_nodes(child, rule_name)
        end
    end
    return n
end

--- Find the first ASTNode with the given rule_name (depth-first).
-- @param node       ASTNode
-- @param rule_name  string
-- @return ASTNode|nil
local function find_node(node, rule_name)
    if type(node) ~= "table" then return nil end
    if node.rule_name == rule_name then return node end
    if node.children then
        for _, child in ipairs(node.children) do
            local found = find_node(child, rule_name)
            if found then return found end
        end
    end
    return nil
end

--- Collect token values of all leaf nodes (depth-first).
-- @param node  ASTNode
-- @return table  Array of value strings.
local function leaf_values(node)
    if type(node) ~= "table" then return {} end
    local out = {}
    if type(node.is_leaf) == "function" and node:is_leaf() then
        local tok = type(node.token) == "function" and node:token() or nil
        if tok then out[#out + 1] = tok.value end
    elseif node.children then
        for _, child in ipairs(node.children) do
            local sub = leaf_values(child)
            for _, v in ipairs(sub) do out[#out + 1] = v end
        end
    end
    return out
end

-- =========================================================================
-- Module surface
-- =========================================================================

describe("lisp_parser module", function()
    it("loads successfully", function()
        assert.is_not_nil(lisp_parser)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(lisp_parser.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", lisp_parser.VERSION)
    end)

    it("exposes parse as a function", function()
        assert.is_function(lisp_parser.parse)
    end)

    it("exposes create_parser as a function", function()
        assert.is_function(lisp_parser.create_parser)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(lisp_parser.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = lisp_parser.get_grammar()
        assert.is_not_nil(g)
    end)
end)

-- =========================================================================
-- Root node structure
-- =========================================================================

describe("root node", function()
    it("parse returns an ASTNode table", function()
        local ast = lisp_parser.parse("42")
        assert.is_table(ast)
    end)

    it("root rule_name is 'program'", function()
        local ast = lisp_parser.parse("42")
        assert.are.equal("program", ast.rule_name)
    end)

    it("root has a children table", function()
        local ast = lisp_parser.parse("42")
        assert.is_table(ast.children)
    end)
end)

-- =========================================================================
-- Atom parsing
-- =========================================================================
--
-- Atoms are the leaves of the Lisp AST.  They are self-evaluating values
-- (numbers, strings) or symbolic references (symbols).

describe("atom: NUMBER", function()
    it("parses a positive integer", function()
        local ast = lisp_parser.parse("42")
        local atom = find_node(ast, "atom")
        assert.is_not_nil(atom)
    end)

    it("parses zero", function()
        local ast = lisp_parser.parse("0")
        local atom = find_node(ast, "atom")
        assert.is_not_nil(atom)
    end)

    it("parses a negative integer", function()
        local ast = lisp_parser.parse("-7")
        local atom = find_node(ast, "atom")
        assert.is_not_nil(atom)
    end)
end)

describe("atom: SYMBOL", function()
    it("parses a simple symbol", function()
        local ast = lisp_parser.parse("define")
        local atom = find_node(ast, "atom")
        assert.is_not_nil(atom)
    end)

    it("parses an operator symbol", function()
        local ast = lisp_parser.parse("+")
        local atom = find_node(ast, "atom")
        assert.is_not_nil(atom)
    end)

    it("parses a predicate symbol", function()
        local ast = lisp_parser.parse("null?")
        local atom = find_node(ast, "atom")
        assert.is_not_nil(atom)
    end)
end)

describe("atom: STRING", function()
    it("parses a simple string", function()
        local ast = lisp_parser.parse('"hello"')
        local atom = find_node(ast, "atom")
        assert.is_not_nil(atom)
    end)

    it("parses an empty string", function()
        local ast = lisp_parser.parse('""')
        local atom = find_node(ast, "atom")
        assert.is_not_nil(atom)
    end)
end)

-- =========================================================================
-- Empty program
-- =========================================================================

describe("empty program", function()
    it("empty input parses to a program node", function()
        local ast = lisp_parser.parse("")
        assert.are.equal("program", ast.rule_name)
    end)

    it("whitespace-only parses to a program node", function()
        local ast = lisp_parser.parse("   \n\t  ")
        assert.are.equal("program", ast.rule_name)
    end)

    it("comment-only parses to a program node", function()
        local ast = lisp_parser.parse("; just a comment\n;; another")
        assert.are.equal("program", ast.rule_name)
    end)
end)

-- =========================================================================
-- List parsing
-- =========================================================================
--
-- A list is LPAREN list_body RPAREN.
-- An empty list () has an empty list_body.

describe("list parsing", function()
    it("parses an empty list ()", function()
        local ast = lisp_parser.parse("()")
        local list_node = find_node(ast, "list")
        assert.is_not_nil(list_node)
    end)

    it("parses a one-element list (42)", function()
        local ast = lisp_parser.parse("(42)")
        local list_node = find_node(ast, "list")
        assert.is_not_nil(list_node)
        -- list has: LPAREN, list_body, RPAREN
        assert.are.equal(3, #list_node.children)
    end)

    it("parses (+ 1 2) — a function call", function()
        -- In Lisp, (+ 1 2) means: call the function + with arguments 1 and 2.
        -- This is the fundamental syntactic unit: (operator operand operand ...).
        local ast = lisp_parser.parse("(+ 1 2)")
        local list_node = find_node(ast, "list")
        assert.is_not_nil(list_node)
        -- Should contain the SYMBOL + and NUMBER tokens
        local atom_count = count_nodes(ast, "atom")
        assert.are.equal(3, atom_count)  -- +, 1, 2
    end)

    it("parses (define x 42)", function()
        local ast = lisp_parser.parse("(define x 42)")
        assert.are.equal("program", ast.rule_name)
        local list_node = find_node(ast, "list")
        assert.is_not_nil(list_node)
        -- The list_body contains three sexprs: define, x, 42
        local atom_count = count_nodes(ast, "atom")
        assert.are.equal(3, atom_count)
    end)

    it("parses a list of strings", function()
        local ast = lisp_parser.parse('("a" "b" "c")')
        local list_node = find_node(ast, "list")
        assert.is_not_nil(list_node)
        assert.are.equal(3, count_nodes(ast, "atom"))
    end)
end)

-- =========================================================================
-- Nested lists
-- =========================================================================
--
-- Lisp lists can be nested to arbitrary depth.  Each nested list is just
-- another S-expression in the parent list's list_body.

describe("nested lists", function()
    it("parses a singly nested list", function()
        local ast = lisp_parser.parse("((1))")
        -- Outer list contains inner list
        local lists = count_nodes(ast, "list")
        assert.are.equal(2, lists)
    end)

    it("parses (+ (* 2 3) 4)", function()
        -- (+ (* 2 3) 4) — nested arithmetic
        -- Evaluates to (+ 6 4) = 10.
        local ast = lisp_parser.parse("(+ (* 2 3) 4)")
        local lists = count_nodes(ast, "list")
        assert.are.equal(2, lists)
        -- atoms: +, *, 2, 3, 4 = five atoms
        local atoms  = count_nodes(ast, "atom")
        assert.are.equal(5, atoms)
    end)

    it("parses a lambda expression", function()
        -- (lambda (x y) (+ x y))
        -- Three-layer nesting: outer list → param list and body list
        local ast = lisp_parser.parse("(lambda (x y) (+ x y))")
        local lists = count_nodes(ast, "list")
        assert.are.equal(3, lists)  -- (lambda...), (x y), (+ x y)
    end)

    it("parses (car (cdr '(1 2 3)))", function()
        local ast = lisp_parser.parse("(car (cdr '(1 2 3)))")
        -- Three lists: (car...), (cdr...), '(1 2 3)
        assert.are.equal(3, count_nodes(ast, "list"))
        -- One quoted form
        assert.are.equal(1, count_nodes(ast, "quoted"))
    end)

    it("parses deeply nested (a (b (c (d))))", function()
        local ast = lisp_parser.parse("(a (b (c (d))))")
        -- 4 lists: outer, (b...), (c...), (d)
        assert.are.equal(4, count_nodes(ast, "list"))
        -- 4 atoms: a, b, c, d
        assert.are.equal(4, count_nodes(ast, "atom"))
    end)
end)

-- =========================================================================
-- Quoted forms
-- =========================================================================
--
-- 'x is syntactic sugar for (quote x).
-- The grammar rule: quoted = QUOTE sexpr
-- This captures the ' token followed by any S-expression.

describe("quoted forms", function()
    it("parses 'x as a quoted sexpr", function()
        local ast = lisp_parser.parse("'x")
        local q = find_node(ast, "quoted")
        assert.is_not_nil(q)
    end)

    it("'x quoted node has two children (QUOTE and sexpr)", function()
        local ast = lisp_parser.parse("'x")
        local q = find_node(ast, "quoted")
        assert.is_not_nil(q)
        assert.are.equal(2, #q.children)
    end)

    it("parses '42 — quoted number", function()
        local ast = lisp_parser.parse("'42")
        local q = find_node(ast, "quoted")
        assert.is_not_nil(q)
    end)

    it("parses '(1 2 3) — quoted list", function()
        -- '(1 2 3) is a literal data list, not a function call.
        -- The parser should produce a quoted node wrapping a list node.
        local ast = lisp_parser.parse("'(1 2 3)")
        local q = find_node(ast, "quoted")
        assert.is_not_nil(q)
        local l = find_node(ast, "list")
        assert.is_not_nil(l)
    end)

    it("parses nested quote ''x", function()
        local ast = lisp_parser.parse("''x")
        local quotes = count_nodes(ast, "quoted")
        assert.are.equal(2, quotes)
    end)

    it("parses quoted string '\"hello\"", function()
        local ast = lisp_parser.parse("'\"hello\"")
        local q = find_node(ast, "quoted")
        assert.is_not_nil(q)
    end)
end)

-- =========================================================================
-- Dotted pairs
-- =========================================================================
--
-- The DOT notation represents cons cells directly.
-- Grammar: list_body = [ sexpr { sexpr } [ DOT sexpr ] ]
--
-- The DOT must appear after at least one sexpr in the list_body.

describe("dotted pairs", function()
    it("parses (a . b) — a dotted pair", function()
        local ast = lisp_parser.parse("(a . b)")
        local l = find_node(ast, "list")
        assert.is_not_nil(l)
    end)

    it("(a . b) contains a list_body", function()
        local ast = lisp_parser.parse("(a . b)")
        local lb = find_node(ast, "list_body")
        assert.is_not_nil(lb)
    end)

    it("parses (1 2 . 3) — improper list", function()
        -- An improper list: the cdr of the last pair is 3, not nil.
        -- This means the list is: cons(1, cons(2, 3))
        local ast = lisp_parser.parse("(1 2 . 3)")
        local l = find_node(ast, "list")
        assert.is_not_nil(l)
        local atoms = count_nodes(ast, "atom")
        assert.are.equal(3, atoms)  -- 1, 2, 3
    end)

    it("parses an alist '((a . 1) (b . 2))", function()
        -- Association lists are classic Lisp data structures.
        -- Each pair is (key . value).
        local ast = lisp_parser.parse("'((a . 1) (b . 2))")
        assert.is_not_nil(find_node(ast, "quoted"))
        assert.are.equal(2, count_nodes(ast, "list") - 1)  -- two inner lists + outer
        -- Actually the outer list is inside the quoted sexpr:
        -- quoted → list((a.1)(b.2)) which contains list(a.1) and list(b.2)
        local lists = count_nodes(ast, "list")
        assert.are.equal(3, lists)  -- outer + two dotted pairs
    end)
end)

-- =========================================================================
-- Multi-expression programs
-- =========================================================================
--
-- Grammar: program = { sexpr }
-- A program is a sequence of zero or more S-expressions.
-- Lisp source files typically contain many top-level definitions.

describe("multi-expression programs", function()
    it("parses two sequential atoms", function()
        local ast = lisp_parser.parse("1 2")
        assert.are.equal("program", ast.rule_name)
        local sexprs = count_nodes(ast, "sexpr")
        -- sexprs are nested — direct children of program should be 2
        assert.is_true(sexprs >= 2)
    end)

    it("parses (define x 42) (display x)", function()
        local ast = lisp_parser.parse("(define x 42) (display x)")
        assert.are.equal("program", ast.rule_name)
        assert.are.equal(2, count_nodes(ast, "list"))
    end)

    it("parses three top-level expressions", function()
        local src = "(define x 1)\n(define y 2)\n(+ x y)"
        local ast = lisp_parser.parse(src)
        assert.are.equal(3, count_nodes(ast, "list"))
    end)

    it("parses a program with comments between expressions", function()
        local src = [[
;; Define a variable
(define x 42)
;; Display it
(display x)
]]
        local ast = lisp_parser.parse(src)
        assert.are.equal("program", ast.rule_name)
        assert.are.equal(2, count_nodes(ast, "list"))
    end)

    it("parses a realistic fibonacci program", function()
        -- The classic recursive Fibonacci definition in Scheme.
        -- This is one of the most frequently cited Lisp programs:
        --
        --   (define (fib n)
        --     (if (< n 2)
        --         n
        --         (+ (fib (- n 1))
        --            (fib (- n 2)))))
        --
        -- In Scheme, (define (f x) body) is shorthand for
        -- (define f (lambda (x) body)).
        local src = [[
(define (fib n)
  (if (< n 2)
      n
      (+ (fib (- n 1))
         (fib (- n 2)))))
]]
        local ast = lisp_parser.parse(src)
        assert.are.equal("program", ast.rule_name)
        -- Multiple nested lists in the fibonacci function
        local lists = count_nodes(ast, "list")
        assert.is_true(lists >= 7)
        -- Many atoms: define, fib, n, if, <, 2, +, -, 1, 2
        local atoms = count_nodes(ast, "atom")
        assert.is_true(atoms >= 10)
    end)

    it("parses a program with quoted data", function()
        local src = [[
(define colors '(red green blue))
(car colors)
]]
        local ast = lisp_parser.parse(src)
        assert.are.equal(2, count_nodes(ast, "list"))
        assert.are.equal(1, count_nodes(ast, "quoted"))
    end)

    it("parses let bindings", function()
        -- (let ((x 1) (y 2)) (+ x y))
        -- let introduces local bindings.  The first argument is a list of
        -- (name value) pairs; the body is evaluated with those bindings.
        local ast = lisp_parser.parse("(let ((x 1) (y 2)) (+ x y))")
        -- Outer let list + (x 1) + (y 2) + (+ x y) + binding list
        local lists = count_nodes(ast, "list")
        assert.is_true(lists >= 4)
    end)
end)

-- =========================================================================
-- create_parser API
-- =========================================================================

describe("create_parser", function()
    it("returns a non-nil parser object", function()
        local p = lisp_parser.create_parser("(+ 1 2)")
        assert.is_not_nil(p)
    end)

    it("parser has a parse method", function()
        local p = lisp_parser.create_parser("(+ 1 2)")
        assert.is_function(p.parse)
    end)

    it("calling parse on the created parser works", function()
        local p = lisp_parser.create_parser("42")
        local ast, err = p:parse()
        assert.is_nil(err)
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error on an unterminated list", function()
        assert.has_error(function()
            lisp_parser.parse("(define x")
        end)
    end)

    it("raises an error on an unexpected character", function()
        assert.has_error(function()
            lisp_parser.parse("@bad")
        end)
    end)

    it("raises an error on a lone RPAREN", function()
        assert.has_error(function()
            lisp_parser.parse(")")
        end)
    end)
end)
