-- gates.lua — Combinational Logic Gates
-- =======================================
--
-- This module implements the seven fundamental Boolean gates plus
-- NAND-derived variants and multi-input gates.
--
-- The seven fundamental gates
-- ---------------------------
--
--   Gate  | Symbol | Truth Table (A,B -> Out)
--   ------|--------|-------------------------
--   AND   |  A*B   | 0,0->0  0,1->0  1,0->0  1,1->1
--   OR    |  A+B   | 0,0->0  0,1->1  1,0->1  1,1->1
--   NOT   |  ~A    | 0->1  1->0  (unary gate)
--   XOR   |  A^B   | 0,0->0  0,1->1  1,0->1  1,1->0
--   NAND  | ~(A*B) | 0,0->1  0,1->1  1,0->1  1,1->0
--   NOR   | ~(A+B) | 0,0->1  0,1->0  1,0->0  1,1->0
--   XNOR  | ~(A^B) | 0,0->1  0,1->0  1,0->0  1,1->1
--
-- NAND as the universal gate
-- --------------------------
--
-- NAND is called a "universal gate" because any other gate can be built
-- from NANDs alone. This is not just a theoretical curiosity — real chip
-- fabrication processes (like CMOS) often build everything from NAND or
-- NOR gates because their transistor layouts are simpler and faster.
--
--   NOT from NAND:   NAND(A, A) = ~A
--   AND from NAND:   NAND(NAND(A,B), NAND(A,B)) = A*B
--   OR from NAND:    NAND(NAND(A,A), NAND(B,B)) = A+B
--   XOR from NAND:   NAND(NAND(A, NAND(A,B)), NAND(B, NAND(A,B)))
--
-- Input conventions
-- -----------------
--
-- All inputs must be 0 or 1 (representing low/high voltage in hardware).
-- Functions raise a structured INVALID_BIT error (see validate_bit) on
-- invalid inputs; floats such as 1.0 are rejected too. In real hardware, voltages outside
-- the valid range cause undefined behavior — our error is the software
-- equivalent of "the chip does something unpredictable."

-- The CMOS gate module from the transistors package. Each of the seven
-- primitive gate functions delegates its digital evaluation to a CMOS
-- transistor model here, reflecting the physical reality that logic gates
-- are built from transistor pairs.
local cmos = require("coding_adventures.transistors.cmos_gates")

local gates = {}

-- Module-level CMOS gate instances, shared across all calls to avoid
-- repeated object construction. Uses default circuit parameters (3.3 V Vdd).
local _cmos_not  = cmos.CMOSInverter()
local _cmos_nand = cmos.CMOSNand()
local _cmos_nor  = cmos.CMOSNor()
local _cmos_and  = cmos.CMOSAnd()
local _cmos_or   = cmos.CMOSOr()
local _cmos_xor  = cmos.CMOSXor()
local _cmos_xnor = cmos.CMOSXnor()

-- CMOS calls are bound and guarded
-- --------------------------------
--
-- The transistors package shares its class methods and uses globals at call
-- time.  We capture each gate's `evaluate_digital` FUNCTION now (so patching
-- the class later cannot redirect us), call it through the load-captured
-- `pcall`, and accept only an integer 0/1 result: any raised error, error
-- return or malformed result becomes a structured INTERNAL error.  What
-- remains is documented: the transistors package is inside the trust
-- boundary -- sub-gates it calls internally still go through its own mutable
-- class tables.
local _eval_not  = _cmos_not.evaluate_digital
local _eval_nand = _cmos_nand.evaluate_digital
local _eval_nor  = _cmos_nor.evaluate_digital
local _eval_and  = _cmos_and.evaluate_digital
local _eval_or   = _cmos_or.evaluate_digital
local _eval_xor  = _cmos_xor.evaluate_digital
local _eval_xnor = _cmos_xnor.evaluate_digital

-- =========================================================================
-- Input Validation
-- =========================================================================

-- Why a structured error, and why integers only
-- ----------------------------------------------
--
-- Two traps live in the obvious one-liner `if v ~= 0 and v ~= 1 then error(..)`:
--
--   1. In Lua 5.3+ the float `1.0` compares EQUAL to the integer `1`
--      (`1.0 == 1` is true, and so is `-0.0 == 0`).  A check built on `==`
--      therefore silently accepts floats, which are a different TYPE in
--      every other language lane (LG02 makes `1.0` a reject row).  The only
--      honest test is `math.type(v) == "integer"`.
--   2. A free-text message gives callers nothing stable to match on: a
--      binding or test that greps for "must be 0 or 1" breaks the moment
--      someone rewords it.  So we raise a TABLE with a stable `code`
--      ("INVALID_BIT"), the offending `param`, and a human `value`
--      description.  A metatable `__tostring` renders it, so an uncaught
--      error still prints a readable line.  `gates.is_invalid_bit(err)` is
--      the supported predicate.
--
-- We never hand the offending value to `tostring()`: a table or userdata may
-- carry a user-defined `__tostring`/`__index`/`__eq`, which would run
-- arbitrary code during error construction.  `type()` and `math.type()` do
-- not consult metamethods, and numbers/strings are rendered with bounded,
-- metamethod-free formatting.

-- Every global this file needs is captured as a local NOW, at load time,
-- so later replacement of `error`, `string.format`, `select`, ... in the
-- caller's environment cannot make validation fail open.
local select, sformat, pcall, mtype = select, string.format, pcall, math.type

-- The private kit (error minting, bit validation) is claimed once and never
-- re-exported: callers cannot mint genuine errors through the public table.
local kit = require("coding_adventures.logic_gates.internal")()
local MAX_INPUTS = kit.MAX_INPUTS
local INVALID_ARITY, INTERNAL = kit.INVALID_ARITY, kit.INTERNAL
local raise, validate_bit, is_kind = kit.raise, kit.validate_bit, kit.is_kind

--- Largest number of inputs ANDn / ORn accept (checked before any copying).
gates.MAX_INPUTS = MAX_INPUTS

--- True when `err` is the structured error raised for a non-bit input.
function gates.is_invalid_bit(err) return is_kind(err, kit.INVALID_BIT) end
--- True when `err` is the structured error raised for a bad argument count.
function gates.is_invalid_arity(err) return is_kind(err, INVALID_ARITY) end
--- True when `err` is the structured error raised for a bad width / size cap.
function gates.is_invalid_width(err) return is_kind(err, kit.INVALID_WIDTH) end
--- True when `err` is the structured error raised for a malformed container.
function gates.is_invalid_shape(err) return is_kind(err, kit.INVALID_SHAPE) end
--- True when `err` is the structured error raised for an internal failure.
function gates.is_internal(err) return is_kind(err, INTERNAL) end
--- Stable error code of any genuine logic_gates error, else nil.
gates.error_code = kit.error_code

--- Reject surplus arguments (`select('#', ...)` counts trailing nils too).
local function check_arity(gate, ...)
    local extra = select("#", ...)
    if extra > 0 then
        raise(INVALID_ARITY, gate, sformat("%s received %d extra argument(s)", gate, extra))
    end
end

local function run_cmos(gate, f, inst, ...)
    local ok, result, err = pcall(f, inst, ...)
    if not ok or err ~= nil or mtype(result) ~= "integer" or (result ~= 0 and result ~= 1) then
        raise(INTERNAL, "internal", gate .. " CMOS evaluation failed")
    end
    return result
end


-- =========================================================================
-- The Seven Fundamental Gates
-- =========================================================================

--- AND returns 1 only when BOTH inputs are 1.
--
-- Circuit diagram (two transistors in series):
--
--       Vcc (+)
--        |
--        R  (pull-up resistor)
--        |
--        +--- Output
--        |
--       [A]  (transistor controlled by input A)
--        |
--       [B]  (transistor controlled by input B)
--        |
--       GND
--
-- Truth table:
--
--   A | B | A AND B
--   --|---|--------
--   0 | 0 |   0
--   0 | 1 |   0
--   1 | 0 |   0
--   1 | 1 |   1
--
-- In Lua 5.4, the & operator performs bitwise AND on integers.
-- Since our inputs are always 0 or 1, this gives the correct result.
--
-- @param a number Input A (0 or 1).
-- @param b number Input B (0 or 1).
-- @return number The AND of A and B.
local function AND(a, b, ...)
    check_arity("AND", ...)
    a = validate_bit(a, "a")
    b = validate_bit(b, "b")
    -- Delegate to the CMOS AND gate (NAND + inverter = 6 transistors).
    return run_cmos("AND", _eval_and, _cmos_and, a, b)
end

--- OR returns 1 when AT LEAST ONE input is 1.
--
-- Circuit diagram (two transistors in parallel):
--
--       Vcc (+)
--        |
--        R  (pull-up resistor)
--        |
--        +--- Output
--       / \
--     [A] [B]  (transistors in parallel)
--       \ /
--        |
--       GND
--
-- Truth table:
--
--   A | B | A OR B
--   --|---|-------
--   0 | 0 |   0
--   0 | 1 |   1
--   1 | 0 |   1
--   1 | 1 |   1
--
-- @param a number Input A (0 or 1).
-- @param b number Input B (0 or 1).
-- @return number The OR of A and B.
local function OR(a, b, ...)
    check_arity("OR", ...)
    a = validate_bit(a, "a")
    b = validate_bit(b, "b")
    -- Delegate to the CMOS OR gate (NOR + inverter = 6 transistors).
    return run_cmos("OR", _eval_or, _cmos_or, a, b)
end

--- NOT inverts its input: 0 becomes 1, 1 becomes 0.
--
-- This is the simplest possible gate — just one transistor.
-- When A = 1, the transistor conducts, pulling output to GND (0).
-- When A = 0, the transistor is off, output floats up to Vcc (1).
--
-- Truth table:
--
--   A | NOT A
--   --|------
--   0 |   1
--   1 |   0
--
-- @param a number Input A (0 or 1).
-- @return number The NOT of A.
local function NOT(a, ...)
    check_arity("NOT", ...)
    a = validate_bit(a, "a")
    -- Delegate to the CMOS inverter (2 transistors: 1 PMOS + 1 NMOS).
    return run_cmos("NOT", _eval_not, _cmos_not, a)
end

--- XOR (exclusive OR) returns 1 when inputs DIFFER.
--
-- XOR answers the question: "Are these two bits different?"
-- This makes it invaluable for comparison, parity checking,
-- and arithmetic (it's the core of binary addition).
--
-- Truth table:
--
--   A | B | A XOR B
--   --|---|--------
--   0 | 0 |   0
--   0 | 1 |   1
--   1 | 0 |   1
--   1 | 1 |   0
--
-- In Lua 5.4, the ~ operator (binary) performs bitwise XOR.
--
-- @param a number Input A (0 or 1).
-- @param b number Input B (0 or 1).
-- @return number The XOR of A and B.
local function XOR(a, b, ...)
    check_arity("XOR", ...)
    a = validate_bit(a, "a")
    b = validate_bit(b, "b")
    -- Delegate to the CMOS XOR gate (4 NAND gates = 16 transistors).
    return run_cmos("XOR", _eval_xor, _cmos_xor, a, b)
end

--- NAND returns 0 only when BOTH inputs are 1.
--
-- NAND = NOT(AND). It is the "universal gate" — you can build ANY
-- other Boolean function using only NAND gates.
--
-- Truth table:
--
--   A | B | A NAND B
--   --|---|--------
--   0 | 0 |   1
--   0 | 1 |   1
--   1 | 0 |   1
--   1 | 1 |   0
--
-- @param a number Input A (0 or 1).
-- @param b number Input B (0 or 1).
-- @return number The NAND of A and B.
local function NAND(a, b, ...)
    check_arity("NAND", ...)
    a = validate_bit(a, "a")
    b = validate_bit(b, "b")
    -- Delegate to the CMOS NAND gate (4 transistors — the natural CMOS primitive).
    return run_cmos("NAND", _eval_nand, _cmos_nand, a, b)
end

--- NOR returns 1 only when BOTH inputs are 0.
--
-- NOR = NOT(OR). Like NAND, NOR is also a universal gate.
-- The Apollo Guidance Computer used about 5,600 NOR gates
-- as its only logic element.
--
-- Truth table:
--
--   A | B | A NOR B
--   --|---|--------
--   0 | 0 |   1
--   0 | 1 |   0
--   1 | 0 |   0
--   1 | 1 |   0
--
-- @param a number Input A (0 or 1).
-- @param b number Input B (0 or 1).
-- @return number The NOR of A and B.
local function NOR(a, b, ...)
    check_arity("NOR", ...)
    a = validate_bit(a, "a")
    b = validate_bit(b, "b")
    -- Delegate to the CMOS NOR gate (4 transistors — the other natural CMOS primitive).
    return run_cmos("NOR", _eval_nor, _cmos_nor, a, b)
end

--- XNOR returns 1 when inputs are the SAME.
--
-- XNOR = NOT(XOR). It is the "equivalence" gate — it answers
-- "are these two bits equal?"
--
-- Truth table:
--
--   A | B | A XNOR B
--   --|---|--------
--   0 | 0 |   1
--   0 | 1 |   0
--   1 | 0 |   0
--   1 | 1 |   1
--
-- @param a number Input A (0 or 1).
-- @param b number Input B (0 or 1).
-- @return number The XNOR of A and B.
local function XNOR(a, b, ...)
    check_arity("XNOR", ...)
    a = validate_bit(a, "a")
    b = validate_bit(b, "b")
    -- Delegate to the dedicated CMOSXnor gate (XOR + Inverter = 8 transistors).
    return run_cmos("XNOR", _eval_xnor, _cmos_xnor, a, b)
end

-- =========================================================================
-- NAND-Derived Gates (Proving Functional Completeness)
-- =========================================================================
--
-- The following functions rebuild each fundamental gate using ONLY NAND.
-- This proves that NAND is functionally complete — it alone can express
-- any Boolean function.

--- NAND_NOT implements NOT using only NAND gates.
--
-- The trick: feed the same input to both sides of a NAND.
-- NAND(A, A) = NOT(A AND A) = NOT(A)
--
-- @param a number Input A (0 or 1).
-- @return number NOT(A), computed using only NAND.
local function NAND_NOT(a, ...)
    check_arity("NAND_NOT", ...)
    a = validate_bit(a, "a")
    return NAND(a, a)
end

--- NAND_AND implements AND using only NAND gates.
--
-- AND = NOT(NAND), and NOT = NAND(x,x), so:
-- NAND_AND(A, B) = NAND_NOT(NAND(A, B))
--
-- Gate count: 2 NANDs.
--
-- @param a number Input A (0 or 1).
-- @param b number Input B (0 or 1).
-- @return number A AND B, computed using only NAND.
local function NAND_AND(a, b, ...)
    check_arity("NAND_AND", ...)
    a = validate_bit(a, "a")
    b = validate_bit(b, "b")
    return NAND_NOT(NAND(a, b))
end

--- NAND_OR implements OR using only NAND gates.
--
-- By De Morgan's law: A OR B = NAND(NOT(A), NOT(B))
--                             = NAND(NAND(A,A), NAND(B,B))
--
-- Gate count: 3 NANDs.
--
-- @param a number Input A (0 or 1).
-- @param b number Input B (0 or 1).
-- @return number A OR B, computed using only NAND.
local function NAND_OR(a, b, ...)
    check_arity("NAND_OR", ...)
    a = validate_bit(a, "a")
    b = validate_bit(b, "b")
    return NAND(NAND(a, a), NAND(b, b))
end

--- NAND_XOR implements XOR using only NAND gates.
--
-- Let C = NAND(A, B), then:
-- XOR = NAND(NAND(A, C), NAND(B, C))
--
-- Gate count: 4 NANDs (the minimum possible).
--
-- @param a number Input A (0 or 1).
-- @param b number Input B (0 or 1).
-- @return number A XOR B, computed using only NAND.
local function NAND_XOR(a, b, ...)
    check_arity("NAND_XOR", ...)
    a = validate_bit(a, "a")
    b = validate_bit(b, "b")
    local c = NAND(a, b)
    return NAND(NAND(a, c), NAND(b, c))
end

-- =========================================================================
-- Multi-Input Gates
-- =========================================================================
--
-- Real circuits often need to AND or OR more than two signals together.
-- Multi-input gates are built by chaining two-input gates:
--
--   AND(A, B, C, D) = AND(AND(AND(A, B), C), D)

--- Validate a packed argument list: every element must be integer 0 or 1.
local function validate_inputs(inputs, n)
    for i = 1, n do
        inputs[i] = validate_bit(inputs[i], "inputs[" .. i .. "]")
    end
end

-- The arity and the cap are checked on `select("#", ...)` BEFORE the
-- arguments are copied into a table, so the structured cap error fires first.
-- (Calling any Lua vararg function with N arguments costs the VM about 2N
-- stack slots against a 1,000,000-slot limit, so a caller passing several
-- hundred thousand arguments can overflow the stack before this code runs;
-- that failure belongs to the caller's own call and cannot be intercepted.
-- Inputs beyond MAX_INPUTS that do reach us are rejected cleanly.)

--- ANDn returns 1 only when ALL inputs are 1.
--
-- Variadic AND: 2..MAX_INPUTS inputs.
--
-- @param ... number Two or more inputs (each 0 or 1).
-- @return number 1 if all inputs are 1, else 0.
local function ANDn(...)
    local n = select("#", ...)
    if n < 2 then raise(INVALID_ARITY, "ANDn", "ANDn requires at least 2 inputs") end
    if n > MAX_INPUTS then
        raise(INVALID_ARITY, "ANDn", sformat("ANDn accepts at most %d inputs", MAX_INPUTS))
    end
    local inputs = { ... }
    validate_inputs(inputs, n)
    local result = inputs[1]
    for i = 2, n do
        result = AND(result, inputs[i])
    end
    return result
end

--- ORn returns 1 when AT LEAST ONE input is 1.
--
-- @param ... number Two or more inputs (each 0 or 1).
-- @return number 1 if any input is 1, else 0.
local function ORn(...)
    local n = select("#", ...)
    if n < 2 then raise(INVALID_ARITY, "ORn", "ORn requires at least 2 inputs") end
    if n > MAX_INPUTS then
        raise(INVALID_ARITY, "ORn", sformat("ORn accepts at most %d inputs", MAX_INPUTS))
    end
    local inputs = { ... }
    validate_inputs(inputs, n)
    local result = inputs[1]
    for i = 2, n do
        result = OR(result, inputs[i])
    end
    return result
end

-- Exports
-- -------
--
-- Every function above calls its siblings through the LOCAL upvalues, bound
-- when this file loaded, and the sequential circuits receive the same
-- originals through the private kit.  The table returned below is a
-- READ-ONLY proxy: `gates.NAND = f` raises.  `rawset` and the debug library
-- can still defeat that; code that can call them is inside the host's trust
-- boundary and could replace the whole Lua environment anyway.
gates.AND, gates.OR, gates.NOT, gates.XOR = AND, OR, NOT, XOR
gates.NAND, gates.NOR, gates.XNOR = NAND, NOR, XNOR
gates.NAND_NOT, gates.NAND_AND, gates.NAND_OR, gates.NAND_XOR =
    NAND_NOT, NAND_AND, NAND_OR, NAND_XOR
gates.ANDn, gates.ORn = ANDn, ORn

-- Build the sequential circuits from the ORIGINAL gates and the private kit.
-- The kit lives only in this chunk's locals; it is handed over once, here.
kit.NOT, kit.AND, kit.XOR, kit.NOR = NOT, AND, XOR, NOR
gates.sequential = require("coding_adventures.logic_gates.sequential_impl")(kit)

return kit.readonly(gates)
