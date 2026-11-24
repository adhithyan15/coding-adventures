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

local type, mtype, sformat, ssub, select, setmetatable, getmetatable, rawget =
    type, math.type, string.format, string.sub, select, setmetatable, getmetatable, rawget

local INVALID_BIT = "INVALID_BIT"
local INVALID_ARITY = "INVALID_ARITY"

local InvalidBit = {}
InvalidBit.__index = InvalidBit
InvalidBit.__tostring = function(e)
    return sformat("logic_gates: %s must be 0 or 1 (integer), got %s", e.param, e.value)
end

local InvalidArity = {}
InvalidArity.__index = InvalidArity
InvalidArity.__tostring = function(e)
    return sformat("logic_gates: %s", e.value)
end

-- Bounded, metamethod-free description of an arbitrary value.
local function describe(v)
    local t = type(v)
    if t == "number" then
        if mtype(v) == "integer" then return sformat("integer %d", v) end
        return sformat("float %.17g", v)
    elseif t == "string" then
        local shown = ssub(v, 1, 32)
        return sformat("string %q%s", shown, #v > 32 and "..." or "")
    end
    return t
end

--- True when `err` is the structured error raised for a non-bit input.
function gates.is_invalid_bit(err)
    return type(err) == "table" and getmetatable(err) == InvalidBit
        and rawget(err, "code") == INVALID_BIT
end

--- True when `err` is the structured error raised for extra arguments.
function gates.is_invalid_arity(err)
    return type(err) == "table" and getmetatable(err) == InvalidArity
        and rawget(err, "code") == INVALID_ARITY
end

local INVALID_WIDTH = "INVALID_WIDTH"
local INVALID_SHAPE = "INVALID_SHAPE"

local InvalidWidth = {}
InvalidWidth.__index = InvalidWidth
InvalidWidth.__tostring = function(e)
    return sformat("logic_gates: %s invalid width: %s", e.param, e.value)
end

local InvalidShape = {}
InvalidShape.__index = InvalidShape
InvalidShape.__tostring = function(e)
    return sformat("logic_gates: %s invalid shape: %s", e.param, e.value)
end

--- True when `err` is the structured error raised for a bad width.
function gates.is_invalid_width(err)
    return type(err) == "table" and getmetatable(err) == InvalidWidth
        and rawget(err, "code") == INVALID_WIDTH
end

--- True when `err` is the structured error raised for a malformed state or
--- data container (not a table, wrong length, missing fields, ...).
function gates.is_invalid_shape(err)
    return type(err) == "table" and getmetatable(err) == InvalidShape
        and rawget(err, "code") == INVALID_SHAPE
end

--- Stable error code of any structured logic_gates error, else nil.
function gates.error_code(err)
    if gates.is_invalid_bit(err) or gates.is_invalid_arity(err)
        or gates.is_invalid_width(err) or gates.is_invalid_shape(err) then
        return rawget(err, "code")
    end
    return nil
end

--- Raise an INVALID_WIDTH error (used by sequential.lua).
function gates.raise_invalid_width(param, detail)
    error(setmetatable({ code = INVALID_WIDTH, param = param, value = detail }, InvalidWidth), 0)
end

--- Raise an INVALID_SHAPE error (used by sequential.lua).
function gates.raise_invalid_shape(param, detail)
    error(setmetatable({ code = INVALID_SHAPE, param = param, value = detail }, InvalidShape), 0)
end

--- Describe an arbitrary value without invoking metamethods.
gates.describe = describe

--- Validate that a value is a valid binary digit: the INTEGER 0 or 1.
--
-- Returns the canonical integer so gates compute only on validated data.
--
-- @param value any The value to check.
-- @param name string The parameter name (carried in the error).
local function validate_bit(value, name)
    if type(value) == "number" and mtype(value) == "integer"
        and (value == 0 or value == 1) then
        return value
    end
    error(setmetatable({ code = INVALID_BIT, param = name, value = describe(value) }, InvalidBit), 0)
end

gates.validate_bit = validate_bit

--- Reject surplus arguments (`select('#', ...)` counts trailing nils too).
local function check_arity(gate, ...)
    local extra = select("#", ...)
    if extra > 0 then
        error(setmetatable({
            code = INVALID_ARITY,
            param = gate,
            value = sformat("%s received %d extra argument(s)", gate, extra),
        }, InvalidArity), 0)
    end
end

--- Validate that all values in a table are valid binary digits.
--
-- @param values table A list of values to check.
-- @param name string The parameter name (for error messages).
local function validate_bits(values, name)
    for i = 1, values.n do
        validate_bit(values[i], sformat("%s[%d]", name, i))
    end
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
function gates.AND(a, b, ...)
    check_arity("AND", ...)
    a = validate_bit(a, "a")
    b = validate_bit(b, "b")
    -- Delegate to the CMOS AND gate (NAND + inverter = 6 transistors).
    local result, err = _cmos_and:evaluate_digital(a, b)
    if err then error("logic_gates: AND CMOS evaluation error: " .. tostring(err)) end
    return result
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
function gates.OR(a, b, ...)
    check_arity("OR", ...)
    a = validate_bit(a, "a")
    b = validate_bit(b, "b")
    -- Delegate to the CMOS OR gate (NOR + inverter = 6 transistors).
    local result, err = _cmos_or:evaluate_digital(a, b)
    if err then error("logic_gates: OR CMOS evaluation error: " .. tostring(err)) end
    return result
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
function gates.NOT(a, ...)
    check_arity("NOT", ...)
    a = validate_bit(a, "a")
    -- Delegate to the CMOS inverter (2 transistors: 1 PMOS + 1 NMOS).
    local result, err = _cmos_not:evaluate_digital(a)
    if err then error("logic_gates: NOT CMOS evaluation error: " .. tostring(err)) end
    return result
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
function gates.XOR(a, b, ...)
    check_arity("XOR", ...)
    a = validate_bit(a, "a")
    b = validate_bit(b, "b")
    -- Delegate to the CMOS XOR gate (4 NAND gates = 16 transistors).
    local result, err = _cmos_xor:evaluate_digital(a, b)
    if err then error("logic_gates: XOR CMOS evaluation error: " .. tostring(err)) end
    return result
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
function gates.NAND(a, b, ...)
    check_arity("NAND", ...)
    a = validate_bit(a, "a")
    b = validate_bit(b, "b")
    -- Delegate to the CMOS NAND gate (4 transistors — the natural CMOS primitive).
    local result, err = _cmos_nand:evaluate_digital(a, b)
    if err then error("logic_gates: NAND CMOS evaluation error: " .. tostring(err)) end
    return result
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
function gates.NOR(a, b, ...)
    check_arity("NOR", ...)
    a = validate_bit(a, "a")
    b = validate_bit(b, "b")
    -- Delegate to the CMOS NOR gate (4 transistors — the other natural CMOS primitive).
    local result, err = _cmos_nor:evaluate_digital(a, b)
    if err then error("logic_gates: NOR CMOS evaluation error: " .. tostring(err)) end
    return result
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
function gates.XNOR(a, b, ...)
    check_arity("XNOR", ...)
    a = validate_bit(a, "a")
    b = validate_bit(b, "b")
    -- Delegate to the dedicated CMOSXnor gate (XOR + Inverter = 8 transistors).
    local result, err = _cmos_xnor:evaluate_digital(a, b)
    if err then error("logic_gates: XNOR CMOS evaluation error: " .. tostring(err)) end
    return result
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
function gates.NAND_NOT(a, ...)
    check_arity("NAND_NOT", ...)
    a = validate_bit(a, "a")
    return gates.NAND(a, a)
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
function gates.NAND_AND(a, b, ...)
    check_arity("NAND_AND", ...)
    a = validate_bit(a, "a")
    b = validate_bit(b, "b")
    return gates.NAND_NOT(gates.NAND(a, b))
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
function gates.NAND_OR(a, b, ...)
    check_arity("NAND_OR", ...)
    a = validate_bit(a, "a")
    b = validate_bit(b, "b")
    return gates.NAND(gates.NAND(a, a), gates.NAND(b, b))
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
function gates.NAND_XOR(a, b, ...)
    check_arity("NAND_XOR", ...)
    a = validate_bit(a, "a")
    b = validate_bit(b, "b")
    local c = gates.NAND(a, b)
    return gates.NAND(gates.NAND(a, c), gates.NAND(b, c))
end

-- =========================================================================
-- Multi-Input Gates
-- =========================================================================
--
-- Real circuits often need to AND or OR more than two signals together.
-- Multi-input gates are built by chaining two-input gates:
--
--   AND(A, B, C, D) = AND(AND(AND(A, B), C), D)

--- ANDn returns 1 only when ALL inputs are 1.
--
-- This is the variadic (multi-input) version of AND. It chains
-- two-input AND operations across all inputs using left folding.
--
-- @param ... number Two or more inputs (each 0 or 1).
-- @return number 1 if all inputs are 1, else 0.
function gates.ANDn(...)
    local inputs = table.pack(...)
    if inputs.n < 2 then
        error("logic_gates: ANDn requires at least 2 inputs")
    end
    validate_bits(inputs, "inputs")
    local result = inputs[1]
    for i = 2, inputs.n do
        result = gates.AND(result, inputs[i])
    end
    return result
end

--- ORn returns 1 when AT LEAST ONE input is 1.
--
-- This is the variadic (multi-input) version of OR. It chains
-- two-input OR operations across all inputs using left folding.
--
-- @param ... number Two or more inputs (each 0 or 1).
-- @return number 1 if any input is 1, else 0.
function gates.ORn(...)
    local inputs = table.pack(...)
    if inputs.n < 2 then
        error("logic_gates: ORn requires at least 2 inputs")
    end
    validate_bits(inputs, "inputs")
    local result = inputs[1]
    for i = 2, inputs.n do
        result = gates.OR(result, inputs[i])
    end
    return result
end

return gates
