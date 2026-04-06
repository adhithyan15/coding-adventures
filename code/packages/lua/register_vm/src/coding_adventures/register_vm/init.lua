-- Module: CodingAdventures.RegisterVM
-- ============================================================================
-- Implements a generic register-based virtual machine with an accumulator
-- model and feedback vectors, modeled after V8's Ignition interpreter.
--
-- ## What is a Register VM?
--
-- A virtual machine (VM) is software that mimics a real CPU. It executes a
-- sequence of instructions stored in memory, maintaining state in registers.
--
-- This VM uses an "accumulator model": most operations load a value INTO the
-- accumulator, operate on it, and then optionally store it back to a numbered
-- register. This mirrors how real CPUs work — x86 originally had an AX
-- "accumulator" register that many instructions implicitly used.
--
-- ## Accumulator vs. Stack VM
--
-- Stack VMs (like the JVM and .NET CLR) keep operands on a stack:
--   PUSH a, PUSH b, ADD  =>  result on top of stack
--
-- Register VMs (like Lua's own VM, Dalvik on Android, and V8's Ignition) use
-- named registers instead:
--   LDAR r0    ; load register 0 into accumulator
--   ADD r1     ; accumulator = accumulator + register 1
--   STAR r2    ; store accumulator to register 2
--
-- Register VMs generally produce larger bytecode (register indices take space)
-- but execute faster because there's no push/pop overhead.
--
-- ## Feedback Vectors (Inline Caches)
--
-- Real JS engines like V8 observe what TYPES of values flow through each
-- operation at runtime. A slot starts "uninitialized", sees its first type
-- and becomes "monomorphic" (one type), sees a second distinct type and
-- becomes "polymorphic" (a few types), then eventually "megamorphic" (give up
-- optimizing — too many types). This information drives JIT compilation.
--
-- We simulate this with feedback slots attached to each instruction.
--
-- ## Hidden Classes
--
-- V8 assigns a "hidden class" (also called "shape" or "map") to each object.
-- Two objects have the same hidden class if they have the same set of
-- properties in the same insertion order. This allows the JIT to compile
-- property accesses into simple offset loads instead of hash-table lookups.
--
-- We simulate hidden classes using a global registry keyed by sorted property
-- names.
--
-- ## References
-- - V8's Ignition design: https://v8.dev/blog/ignition-interpreter
-- - "A No-Frills Introduction to Lua 5.1 VM Instructions" by Kein-Hong Man
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- ## Opcodes
-- ============================================================================
-- Each opcode is a byte-sized integer constant. We group them by function:
--
--   0x00–0x0F  Load immediate values into the accumulator
--   0x10–0x1F  Move values between registers and accumulator
--   0x20–0x2F  Global and context (scope) variable access
--   0x30–0x3F  Arithmetic and bitwise operations
--   0x40–0x4F  Comparison and logical operations
--   0x50–0x5F  Control flow (jumps)
--   0x60–0x6F  Function calls and returns
--   0x70–0x7F  Property access (named and keyed)
--   0x80–0x8F  Object/array/closure creation
--   0x90–0x9F  Iterator protocol
--   0xA0–0xAF  Exception handling
--   0xB0–0xBF  Context and module variable access
--   0xF0–0xFF  VM meta-instructions (stack check, debugger, halt)

M.Opcodes = {
  -- Load immediates into accumulator
  LDA_CONSTANT              = 0x00,  -- acc = constants[operand[0]]
  LDA_ZERO                  = 0x01,  -- acc = 0
  LDA_SMI                   = 0x02,  -- acc = operand[0] (small integer)
  LDA_UNDEFINED             = 0x03,  -- acc = nil
  LDA_NULL                  = 0x04,  -- acc = nil (same in Lua; we tag it)
  LDA_TRUE                  = 0x05,  -- acc = true
  LDA_FALSE                 = 0x06,  -- acc = false

  -- Register ↔ accumulator moves
  LDAR                      = 0x10,  -- acc = registers[operand[0]]
  STAR                      = 0x11,  -- registers[operand[0]] = acc
  MOV                       = 0x12,  -- registers[dst] = registers[src]

  -- Global and scope variable access
  LDA_GLOBAL                = 0x20,  -- acc = globals[names[operand[0]]]
  STA_GLOBAL                = 0x21,  -- globals[names[operand[0]]] = acc
  LDA_CONTEXT_SLOT          = 0x22,  -- acc = context at depth/idx
  STA_CONTEXT_SLOT          = 0x23,  -- store acc into context at depth/idx
  LDA_CURRENT_CONTEXT_SLOT  = 0x24,  -- acc = frame.context.slots[operand[0]]
  STA_CURRENT_CONTEXT_SLOT  = 0x25,  -- frame.context.slots[operand[0]] = acc

  -- Arithmetic
  ADD                       = 0x30,  -- acc = acc + registers[operand[0]]
  SUB                       = 0x31,  -- acc = acc - registers[operand[0]]
  MUL                       = 0x32,  -- acc = acc * registers[operand[0]]
  DIV                       = 0x33,  -- acc = acc / registers[operand[0]]
  MOD                       = 0x34,  -- acc = acc % registers[operand[0]]
  POW                       = 0x35,  -- acc = acc ^ registers[operand[0]]
  ADD_SMI                   = 0x36,  -- acc = acc + operand[0]
  SUB_SMI                   = 0x37,  -- acc = acc - operand[0]
  NEGATE                    = 0x38,  -- acc = -acc

  -- Bitwise operations (integers in Lua 5.3+)
  BITWISE_AND               = 0x39,  -- acc = acc & registers[operand[0]]
  BITWISE_OR                = 0x3A,  -- acc = acc | registers[operand[0]]
  BITWISE_XOR               = 0x3B,  -- acc = acc ~ registers[operand[0]]
  BITWISE_NOT               = 0x3C,  -- acc = ~acc
  SHIFT_LEFT                = 0x3D,  -- acc = acc << registers[operand[0]]
  SHIFT_RIGHT               = 0x3E,  -- acc = acc >> registers[operand[0]]
  SHIFT_RIGHT_LOGICAL        = 0x3F,  -- acc = acc >> operand (unsigned)

  -- Comparisons — each puts true/false in accumulator
  TEST_EQUAL                = 0x40,  -- acc = (acc == registers[op[0]])
  TEST_NOT_EQUAL            = 0x41,  -- acc = (acc ~= registers[op[0]])
  TEST_STRICT_EQUAL         = 0x42,  -- acc = strict equal (type + value)
  TEST_STRICT_NOT_EQUAL     = 0x43,  -- acc = strict not equal
  TEST_LESS_THAN            = 0x44,  -- acc = (acc < registers[op[0]])
  TEST_GREATER_THAN         = 0x45,  -- acc = (acc > registers[op[0]])
  TEST_LE                   = 0x46,  -- acc = (acc <= registers[op[0]])
  TEST_GE                   = 0x47,  -- acc = (acc >= registers[op[0]])
  TEST_IN                   = 0x48,  -- acc = key in object (property exists)
  TEST_INSTANCE_OF          = 0x49,  -- acc = instanceof check
  TEST_UNDETECTABLE         = 0x4A,  -- acc = (acc is nil/undefined/null)
  LOGICAL_NOT               = 0x4B,  -- acc = not acc (boolean negation)
  TYPE_OF                   = 0x4C,  -- acc = typeof acc (string)

  -- Control flow
  -- Jump operands are signed RELATIVE offsets added to the current ip.
  -- ip advances by 1 each step, then the offset is applied, so JUMP 0
  -- would re-execute the current instruction (infinite loop). JUMP 1
  -- skips the next instruction.
  JUMP                            = 0x50,
  JUMP_IF_TRUE                    = 0x51,
  JUMP_IF_FALSE                   = 0x52,
  JUMP_IF_NULL                    = 0x53,
  JUMP_IF_UNDEFINED               = 0x54,
  JUMP_IF_NULL_OR_UNDEFINED       = 0x55,
  JUMP_IF_TO_BOOLEAN_TRUE         = 0x56,
  JUMP_IF_TO_BOOLEAN_FALSE        = 0x57,
  JUMP_LOOP                       = 0x58,  -- like JUMP but hints optimizer

  -- Function calls
  CALL_ANY_RECEIVER        = 0x60,  -- call acc as function
  CALL_PROPERTY            = 0x61,  -- call method on object
  CALL_UNDEFINED_RECEIVER  = 0x62,  -- call function with undefined 'this'
  CONSTRUCT                = 0x63,  -- new acc(args)
  RETURN                   = 0x64,  -- return acc to caller
  SUSPEND_GENERATOR        = 0x65,  -- yield from generator
  RESUME_GENERATOR         = 0x66,  -- resume a suspended generator

  -- Property access
  LDA_NAMED_PROPERTY            = 0x70,  -- acc = obj[name]
  STA_NAMED_PROPERTY            = 0x71,  -- obj[name] = acc
  LDA_KEYED_PROPERTY            = 0x72,  -- acc = obj[key_reg]
  STA_KEYED_PROPERTY            = 0x73,  -- obj[key_reg] = acc
  LDA_NAMED_PROPERTY_NO_FEEDBACK= 0x74,  -- like LDA_NAMED_PROPERTY, no slot
  STA_NAMED_PROPERTY_NO_FEEDBACK= 0x75,
  DELETE_PROPERTY_STRICT        = 0x76,  -- delete obj[name] (strict mode)
  DELETE_PROPERTY_SLOPPY        = 0x77,  -- delete obj[name] (sloppy mode)

  -- Object / array / closure creation
  CREATE_OBJECT_LITERAL  = 0x80,
  CREATE_ARRAY_LITERAL   = 0x81,
  CREATE_REGEXP_LITERAL  = 0x82,
  CREATE_CLOSURE         = 0x83,  -- create function closure
  CREATE_CONTEXT         = 0x84,  -- create a new scope context
  CLONE_OBJECT           = 0x85,

  -- Iterator protocol (like `for...of` in JS)
  GET_ITERATOR       = 0x90,
  CALL_ITERATOR_STEP = 0x91,
  GET_ITERATOR_DONE  = 0x92,
  GET_ITERATOR_VALUE = 0x93,

  -- Exception handling
  THROW   = 0xA0,
  RETHROW = 0xA1,

  -- Context / module
  PUSH_CONTEXT       = 0xB0,
  POP_CONTEXT        = 0xB1,
  LDA_MODULE_VARIABLE = 0xB2,
  STA_MODULE_VARIABLE = 0xB3,

  -- VM meta
  STACK_CHECK = 0xF0,  -- check call depth hasn't overflowed
  DEBUGGER    = 0xFE,  -- breakpoint (no-op in our implementation)
  HALT        = 0xFF,  -- stop the VM unconditionally
}

-- ============================================================================
-- ## Feedback Slot State Machine
-- ============================================================================
-- Feedback slots track which types have been seen at a given operation site.
-- The state machine has four states:
--
--   uninitialized → monomorphic → polymorphic → megamorphic
--
-- Once megamorphic, there's no going back — we've given up trying to
-- specialize this operation for a particular type.
--
-- The "type string" for arithmetic is "typeA:typeB" (e.g., "number:number").
-- For property access it might be a hidden class ID.
--
-- This simulates V8's inline caches (ICs), which are small pieces of
-- machine code that get patched based on observed types.

-- Create a fresh uninitialized feedback slot.
local function new_feedback_slot()
  return { kind = "uninitialized" }
end

-- Build the feedback_vector for a CodeObject.
-- Each slot starts uninitialized.
local function init_feedback_vector(slot_count)
  local fv = {}
  for i = 1, slot_count do
    fv[i] = new_feedback_slot()
  end
  return fv
end

-- Record a type pair into a feedback slot and advance the state machine.
-- type_pair is a string like "number:number" or "string:number".
-- Returns the (possibly updated) slot — slots are mutated in place.
local function record_feedback(slot, type_pair)
  if slot.kind == "uninitialized" then
    -- First observation: go monomorphic.
    slot.kind = "monomorphic"
    slot.types = { type_pair }
  elseif slot.kind == "monomorphic" then
    -- Check if we've already seen this pair (deduplication).
    if slot.types[1] ~= type_pair then
      -- New type pair: upgrade to polymorphic.
      slot.kind = "polymorphic"
      slot.types = { slot.types[1], type_pair }
    end
    -- Same pair seen again → no state change.
  elseif slot.kind == "polymorphic" then
    -- Check if the pair is already recorded.
    local found = false
    for _, t in ipairs(slot.types) do
      if t == type_pair then found = true; break end
    end
    if not found then
      if #slot.types >= 4 then
        -- Too many distinct types: give up and go megamorphic.
        slot.kind = "megamorphic"
        slot.types = nil
      else
        -- Still manageable: add the new type.
        slot.types[#slot.types + 1] = type_pair
      end
    end
  end
  -- megamorphic: no transitions out
end

-- ============================================================================
-- ## Hidden Class Registry
-- ============================================================================
-- When we create an object or add a property to it, we compute the "hidden
-- class" by sorting the property names and joining them. Two objects with
-- identical property sets share a hidden class ID.
--
-- This global table maps sorted-key-strings to integer IDs.
local hidden_class_registry = {}
local hidden_class_counter  = 0

-- Get (or create) the hidden class ID for a given set of property names.
-- keys_table is a Lua table used as a set (keys are property names).
local function get_hidden_class_id(keys_table)
  local keys = {}
  for k in pairs(keys_table) do
    keys[#keys + 1] = tostring(k)
  end
  table.sort(keys)
  local key_str = table.concat(keys, ",")

  if hidden_class_registry[key_str] == nil then
    hidden_class_counter = hidden_class_counter + 1
    hidden_class_registry[key_str] = hidden_class_counter
  end
  return hidden_class_registry[key_str]
end

-- Create a new VMObject (simulates a JS object / heap value).
-- We represent it as a Lua table with:
--   __hidden_class_id : integer — the hidden class this object currently has
--   properties        : table  — the actual property key→value map
local function new_vm_object(initial_props)
  local props = initial_props or {}
  local hcid = get_hidden_class_id(props)
  return {
    __hidden_class_id = hcid,
    properties = props,
  }
end

-- Update the hidden class after adding/removing a property.
-- Call this whenever properties is mutated.
local function update_hidden_class(obj)
  obj.__hidden_class_id = get_hidden_class_id(obj.properties)
end

-- ============================================================================
-- ## Scope / Context
-- ============================================================================
-- A "context" is a linked list of scope frames. Each frame holds a `slots`
-- table (numerically indexed) and a `parent` pointer.
--
-- This mirrors how closures capture variables: each function's activation
-- record points to the enclosing scope. To look up a variable N levels up,
-- we walk N parent links.
--
-- Example:
--   global scope: { slots = {10, 20} }
--       ↑ parent
--   function scope: { slots = {42} }
--
-- LDA_CONTEXT_SLOT [2, 0] would walk 2 parents to reach the global scope
-- and read slots[1] (0-based index 0 → 1-based index 1 in Lua).

local function new_context(parent, slot_count)
  local slots = {}
  for i = 1, (slot_count or 0) do
    slots[i] = nil  -- uninitialized
  end
  return { slots = slots, parent = parent }
end

-- Walk `depth` parent links from context and return that context.
local function walk_context(context, depth)
  local ctx = context
  for _ = 1, depth do
    if ctx == nil then
      error("context walk exceeded scope chain depth")
    end
    ctx = ctx.parent
  end
  return ctx
end

-- ============================================================================
-- ## Type Utilities
-- ============================================================================
-- These helpers classify Lua values into the type strings used by feedback.

-- Return a short type tag for a Lua value (for feedback type-pair strings).
local function type_tag(v)
  local t = type(v)
  if t == "number" then
    -- Distinguish integer from float (important for JIT specialization).
    if math.type and math.type(v) == "integer" then
      return "int"
    else
      return "float"
    end
  elseif t == "table" then
    if v.__hidden_class_id ~= nil then
      return "object:" .. tostring(v.__hidden_class_id)
    elseif v.kind == "function" then
      return "function"
    else
      return "table"
    end
  else
    return t  -- "nil", "boolean", "string"
  end
end

-- JS-style "ToBoolean" conversion: everything is truthy except false and nil.
-- (In JS, 0, "", and NaN are also falsy — we simplify here.)
local function to_boolean(v)
  if v == nil or v == false then return false end
  return true
end

-- Return a string representing the typeof a value (JS semantics).
local function js_typeof(v)
  local t = type(v)
  if t == "nil"     then return "undefined" end
  if t == "boolean" then return "boolean" end
  if t == "number"  then return "number" end
  if t == "string"  then return "string" end
  if t == "table"   then
    if v.kind == "function" then return "function" end
    return "object"
  end
  return "unknown"
end

-- ============================================================================
-- ## Instruction Constructors
-- ============================================================================
-- These are convenience constructors for building CodeObjects in tests.

-- Create a RegisterInstruction.
function M.make_instruction(opcode, operands, feedback_slot)
  return {
    opcode       = opcode,
    operands     = operands or {},
    feedback_slot = feedback_slot ~= nil and feedback_slot or -1,
  }
end

-- Create a CodeObject.
function M.make_code_object(opts)
  opts = opts or {}
  return {
    name               = opts.name or "<anonymous>",
    instructions       = opts.instructions or {},
    constants          = opts.constants or {},
    names              = opts.names or {},
    register_count     = opts.register_count or 0,
    feedback_slot_count= opts.feedback_slot_count or 0,
    parameter_count    = opts.parameter_count or 0,
  }
end

-- ============================================================================
-- ## VM State
-- ============================================================================
-- The VM itself is a table holding:
--   globals    : table  — global variable scope (caller-supplied)
--   call_depth : int    — current call stack depth
--   max_depth  : int    — stack overflow threshold (default 500)
--   trace      : array  — if tracing, collects TraceStep tables

local function new_vm(globals, options)
  options = options or {}
  return {
    globals    = globals or {},
    call_depth = 0,
    max_depth  = options.max_depth or 500,
    trace      = nil,  -- nil means no tracing; set to {} to enable
  }
end

-- Create a fresh CallFrame for a CodeObject.
-- registers is pre-allocated to register_count length (all nil).
local function new_call_frame(code, context, caller_frame)
  local registers = {}
  for i = 1, code.register_count do
    registers[i] = nil
  end

  local fv = init_feedback_vector(code.feedback_slot_count)

  return {
    code         = code,
    ip           = 1,     -- instruction pointer, 1-based (Lua table index)
    accumulator  = nil,
    registers    = registers,
    feedback_vector = fv,
    context      = context,
    caller_frame = caller_frame,
  }
end

-- ============================================================================
-- ## Opcode Handlers (Dispatch Table)
-- ============================================================================
-- Rather than a giant if/elseif chain, we use a dispatch table: a Lua table
-- keyed by opcode integer, with handler functions as values.
--
-- Each handler receives (vm, frame, instr) and:
--   - Mutates frame.accumulator, frame.registers, vm.globals, etc.
--   - Returns nil normally, "halt" to stop, or a string error message.
--
-- The main execution loop (run_frame) calls handlers via:
--   local result = handlers[instr.opcode](vm, frame, instr)

-- Helper: get a register value by 0-based index.
local function get_reg(frame, idx)
  return frame.registers[idx + 1]
end

-- Helper: set a register value by 0-based index.
local function set_reg(frame, idx, value)
  frame.registers[idx + 1] = value
end

-- Helper: read operand[n] (1-indexed in Lua).
local function op(instr, n)
  return instr.operands[n]
end

-- Helper: maybe record arithmetic feedback.
-- slot_idx is the 0-based feedback slot index (-1 means skip).
local function maybe_record_arith_feedback(frame, slot_idx, lhs, rhs)
  if slot_idx >= 0 and slot_idx < #frame.feedback_vector then
    local pair = type_tag(lhs) .. ":" .. type_tag(rhs)
    record_feedback(frame.feedback_vector[slot_idx + 1], pair)
  end
end

local handlers = {}

-- ── Load Immediates ──────────────────────────────────────────────────────────

-- LDA_CONSTANT: Load a constant from the constant pool into the accumulator.
-- Operand[1] is the 0-based index into code.constants (so we add 1 for Lua).
handlers[M.Opcodes.LDA_CONSTANT] = function(vm, frame, instr)
  frame.accumulator = frame.code.constants[op(instr,1) + 1]
end

-- LDA_ZERO: Load the integer 0. Faster than LDA_CONSTANT for the common case.
handlers[M.Opcodes.LDA_ZERO] = function(vm, frame, instr)
  frame.accumulator = 0
end

-- LDA_SMI: Load a "small integer" (SMI) encoded directly in the operand.
handlers[M.Opcodes.LDA_SMI] = function(vm, frame, instr)
  frame.accumulator = op(instr, 1)
end

-- LDA_UNDEFINED: Load the undefined value (nil in Lua).
handlers[M.Opcodes.LDA_UNDEFINED] = function(vm, frame, instr)
  frame.accumulator = nil
end

-- LDA_NULL: Load null (also nil in our Lua implementation).
-- In a real JS engine, null and undefined are distinct; here we unify them.
handlers[M.Opcodes.LDA_NULL] = function(vm, frame, instr)
  frame.accumulator = nil
end

-- LDA_TRUE / LDA_FALSE: Load boolean literals.
handlers[M.Opcodes.LDA_TRUE] = function(vm, frame, instr)
  frame.accumulator = true
end

handlers[M.Opcodes.LDA_FALSE] = function(vm, frame, instr)
  frame.accumulator = false
end

-- ── Register Moves ───────────────────────────────────────────────────────────

-- LDAR (Load Accumulator from Register): acc = registers[operand[1]]
handlers[M.Opcodes.LDAR] = function(vm, frame, instr)
  frame.accumulator = get_reg(frame, op(instr, 1))
end

-- STAR (Store Accumulator to Register): registers[operand[1]] = acc
handlers[M.Opcodes.STAR] = function(vm, frame, instr)
  set_reg(frame, op(instr, 1), frame.accumulator)
end

-- MOV: Copy from one register to another. operand[1]=src, operand[2]=dst.
handlers[M.Opcodes.MOV] = function(vm, frame, instr)
  local src_val = get_reg(frame, op(instr, 1))
  set_reg(frame, op(instr, 2), src_val)
end

-- ── Global and Context Variable Access ──────────────────────────────────────

-- LDA_GLOBAL: Load a global variable by name index.
-- names[operand[1]+1] gives the variable name string.
handlers[M.Opcodes.LDA_GLOBAL] = function(vm, frame, instr)
  local name = frame.code.names[op(instr, 1) + 1]
  frame.accumulator = vm.globals[name]
end

-- STA_GLOBAL: Store accumulator into a global variable.
handlers[M.Opcodes.STA_GLOBAL] = function(vm, frame, instr)
  local name = frame.code.names[op(instr, 1) + 1]
  vm.globals[name] = frame.accumulator
end

-- LDA_CONTEXT_SLOT: Read from a scope variable N levels up.
-- operand[1]=depth (0 = current frame's context), operand[2]=slot_index.
handlers[M.Opcodes.LDA_CONTEXT_SLOT] = function(vm, frame, instr)
  local depth = op(instr, 1)
  local idx   = op(instr, 2)
  local ctx   = walk_context(frame.context, depth)
  frame.accumulator = ctx.slots[idx + 1]
end

-- STA_CONTEXT_SLOT: Write to a scope variable N levels up.
handlers[M.Opcodes.STA_CONTEXT_SLOT] = function(vm, frame, instr)
  local depth = op(instr, 1)
  local idx   = op(instr, 2)
  local ctx   = walk_context(frame.context, depth)
  ctx.slots[idx + 1] = frame.accumulator
end

-- LDA_CURRENT_CONTEXT_SLOT: Read from current frame's scope (depth=0).
handlers[M.Opcodes.LDA_CURRENT_CONTEXT_SLOT] = function(vm, frame, instr)
  local idx = op(instr, 1)
  frame.accumulator = frame.context.slots[idx + 1]
end

-- STA_CURRENT_CONTEXT_SLOT: Write to current frame's scope.
handlers[M.Opcodes.STA_CURRENT_CONTEXT_SLOT] = function(vm, frame, instr)
  local idx = op(instr, 1)
  frame.context.slots[idx + 1] = frame.accumulator
end

-- ── Arithmetic ───────────────────────────────────────────────────────────────

-- ADD: acc = acc + registers[operand[1]]
-- Also records feedback about operand types (for JIT specialization).
handlers[M.Opcodes.ADD] = function(vm, frame, instr)
  local rhs = get_reg(frame, op(instr, 1))
  maybe_record_arith_feedback(frame, instr.feedback_slot, frame.accumulator, rhs)
  frame.accumulator = frame.accumulator + rhs
end

-- SUB: acc = acc - registers[operand[1]]
handlers[M.Opcodes.SUB] = function(vm, frame, instr)
  local rhs = get_reg(frame, op(instr, 1))
  maybe_record_arith_feedback(frame, instr.feedback_slot, frame.accumulator, rhs)
  frame.accumulator = frame.accumulator - rhs
end

-- MUL: acc = acc * registers[operand[1]]
handlers[M.Opcodes.MUL] = function(vm, frame, instr)
  local rhs = get_reg(frame, op(instr, 1))
  maybe_record_arith_feedback(frame, instr.feedback_slot, frame.accumulator, rhs)
  frame.accumulator = frame.accumulator * rhs
end

-- DIV: acc = acc / registers[operand[1]]
handlers[M.Opcodes.DIV] = function(vm, frame, instr)
  local rhs = get_reg(frame, op(instr, 1))
  maybe_record_arith_feedback(frame, instr.feedback_slot, frame.accumulator, rhs)
  frame.accumulator = frame.accumulator / rhs
end

-- MOD: acc = acc % registers[operand[1]]
handlers[M.Opcodes.MOD] = function(vm, frame, instr)
  local rhs = get_reg(frame, op(instr, 1))
  maybe_record_arith_feedback(frame, instr.feedback_slot, frame.accumulator, rhs)
  frame.accumulator = frame.accumulator % rhs
end

-- POW: acc = acc ^ registers[operand[1]]
handlers[M.Opcodes.POW] = function(vm, frame, instr)
  local rhs = get_reg(frame, op(instr, 1))
  maybe_record_arith_feedback(frame, instr.feedback_slot, frame.accumulator, rhs)
  frame.accumulator = frame.accumulator ^ rhs
end

-- ADD_SMI: acc = acc + immediate (operand[1]).
-- No register lookup — the number is embedded in the instruction.
handlers[M.Opcodes.ADD_SMI] = function(vm, frame, instr)
  frame.accumulator = frame.accumulator + op(instr, 1)
end

-- SUB_SMI: acc = acc - immediate.
handlers[M.Opcodes.SUB_SMI] = function(vm, frame, instr)
  frame.accumulator = frame.accumulator - op(instr, 1)
end

-- NEGATE: acc = -acc
handlers[M.Opcodes.NEGATE] = function(vm, frame, instr)
  frame.accumulator = -frame.accumulator
end

-- ── Bitwise ──────────────────────────────────────────────────────────────────
-- Lua 5.3+ has native integer bitwise operators.

handlers[M.Opcodes.BITWISE_AND] = function(vm, frame, instr)
  local rhs = get_reg(frame, op(instr, 1))
  frame.accumulator = frame.accumulator & rhs
end

handlers[M.Opcodes.BITWISE_OR] = function(vm, frame, instr)
  local rhs = get_reg(frame, op(instr, 1))
  frame.accumulator = frame.accumulator | rhs
end

handlers[M.Opcodes.BITWISE_XOR] = function(vm, frame, instr)
  local rhs = get_reg(frame, op(instr, 1))
  frame.accumulator = frame.accumulator ~ rhs
end

handlers[M.Opcodes.BITWISE_NOT] = function(vm, frame, instr)
  frame.accumulator = ~frame.accumulator
end

handlers[M.Opcodes.SHIFT_LEFT] = function(vm, frame, instr)
  local rhs = get_reg(frame, op(instr, 1))
  frame.accumulator = frame.accumulator << rhs
end

handlers[M.Opcodes.SHIFT_RIGHT] = function(vm, frame, instr)
  local rhs = get_reg(frame, op(instr, 1))
  frame.accumulator = frame.accumulator >> rhs
end

-- SHIFT_RIGHT_LOGICAL: unsigned (logical) right shift.
-- In Lua, >> on integers is arithmetic; we mask to simulate unsigned.
handlers[M.Opcodes.SHIFT_RIGHT_LOGICAL] = function(vm, frame, instr)
  local rhs = get_reg(frame, op(instr, 1))
  -- Convert to unsigned 32-bit, shift, then convert back.
  local v = frame.accumulator & 0xFFFFFFFF
  frame.accumulator = (v >> rhs) & 0xFFFFFFFF
end

-- ── Comparisons ──────────────────────────────────────────────────────────────

handlers[M.Opcodes.TEST_EQUAL] = function(vm, frame, instr)
  local rhs = get_reg(frame, op(instr, 1))
  frame.accumulator = (frame.accumulator == rhs)
end

handlers[M.Opcodes.TEST_NOT_EQUAL] = function(vm, frame, instr)
  local rhs = get_reg(frame, op(instr, 1))
  frame.accumulator = (frame.accumulator ~= rhs)
end

-- Strict equal: both type AND value must match (Lua == already does this).
handlers[M.Opcodes.TEST_STRICT_EQUAL] = function(vm, frame, instr)
  local rhs = get_reg(frame, op(instr, 1))
  frame.accumulator = (frame.accumulator == rhs)
end

handlers[M.Opcodes.TEST_STRICT_NOT_EQUAL] = function(vm, frame, instr)
  local rhs = get_reg(frame, op(instr, 1))
  frame.accumulator = (frame.accumulator ~= rhs)
end

handlers[M.Opcodes.TEST_LESS_THAN] = function(vm, frame, instr)
  local rhs = get_reg(frame, op(instr, 1))
  frame.accumulator = (frame.accumulator < rhs)
end

handlers[M.Opcodes.TEST_GREATER_THAN] = function(vm, frame, instr)
  local rhs = get_reg(frame, op(instr, 1))
  frame.accumulator = (frame.accumulator > rhs)
end

handlers[M.Opcodes.TEST_LE] = function(vm, frame, instr)
  local rhs = get_reg(frame, op(instr, 1))
  frame.accumulator = (frame.accumulator <= rhs)
end

handlers[M.Opcodes.TEST_GE] = function(vm, frame, instr)
  local rhs = get_reg(frame, op(instr, 1))
  frame.accumulator = (frame.accumulator >= rhs)
end

-- TEST_IN: Check if a property name (in acc) exists on object (in register).
-- operand[1] is the register holding the object.
handlers[M.Opcodes.TEST_IN] = function(vm, frame, instr)
  local obj = get_reg(frame, op(instr, 1))
  local key = frame.accumulator
  if type(obj) == "table" and obj.properties ~= nil then
    frame.accumulator = (obj.properties[key] ~= nil)
  elseif type(obj) == "table" then
    frame.accumulator = (obj[key] ~= nil)
  else
    frame.accumulator = false
  end
end

-- TEST_INSTANCE_OF: Simplified — checks if obj.__class == constructor.
handlers[M.Opcodes.TEST_INSTANCE_OF] = function(vm, frame, instr)
  local constructor = get_reg(frame, op(instr, 1))
  local obj = frame.accumulator
  if type(obj) == "table" and type(constructor) == "table" then
    frame.accumulator = (obj.__class == constructor)
  else
    frame.accumulator = false
  end
end

-- TEST_UNDETECTABLE: true if acc is nil (undefined or null in JS).
handlers[M.Opcodes.TEST_UNDETECTABLE] = function(vm, frame, instr)
  frame.accumulator = (frame.accumulator == nil)
end

-- LOGICAL_NOT: Flip the truthiness of acc.
handlers[M.Opcodes.LOGICAL_NOT] = function(vm, frame, instr)
  frame.accumulator = not to_boolean(frame.accumulator)
end

-- TYPE_OF: Return a string describing the type of acc (JS semantics).
handlers[M.Opcodes.TYPE_OF] = function(vm, frame, instr)
  frame.accumulator = js_typeof(frame.accumulator)
end

-- ── Control Flow ─────────────────────────────────────────────────────────────
-- Jump instructions modify frame.ip.
--
-- IMPORTANT: The execution loop increments ip BEFORE dispatching, so to
-- "skip" N instructions we set ip += N. To jump backwards (loop), N is
-- negative. JUMP 0 would re-execute the jump itself (infinite loop).
--
-- We apply the offset AFTER the loop's automatic ip increment, so the
-- offset is relative to the instruction AFTER the jump instruction.

handlers[M.Opcodes.JUMP] = function(vm, frame, instr)
  -- Unconditional jump: offset relative to next instruction.
  local offset = op(instr, 1)
  frame.ip = frame.ip + offset
end

handlers[M.Opcodes.JUMP_IF_TRUE] = function(vm, frame, instr)
  if frame.accumulator == true then
    frame.ip = frame.ip + op(instr, 1)
  end
end

handlers[M.Opcodes.JUMP_IF_FALSE] = function(vm, frame, instr)
  if frame.accumulator == false then
    frame.ip = frame.ip + op(instr, 1)
  end
end

handlers[M.Opcodes.JUMP_IF_NULL] = function(vm, frame, instr)
  if frame.accumulator == nil then
    frame.ip = frame.ip + op(instr, 1)
  end
end

handlers[M.Opcodes.JUMP_IF_UNDEFINED] = function(vm, frame, instr)
  if frame.accumulator == nil then
    frame.ip = frame.ip + op(instr, 1)
  end
end

handlers[M.Opcodes.JUMP_IF_NULL_OR_UNDEFINED] = function(vm, frame, instr)
  if frame.accumulator == nil then
    frame.ip = frame.ip + op(instr, 1)
  end
end

handlers[M.Opcodes.JUMP_IF_TO_BOOLEAN_TRUE] = function(vm, frame, instr)
  if to_boolean(frame.accumulator) then
    frame.ip = frame.ip + op(instr, 1)
  end
end

handlers[M.Opcodes.JUMP_IF_TO_BOOLEAN_FALSE] = function(vm, frame, instr)
  if not to_boolean(frame.accumulator) then
    frame.ip = frame.ip + op(instr, 1)
  end
end

-- JUMP_LOOP: Like JUMP but signals to a real optimizer that this is a
-- backward branch (loop edge). In our interpreter it behaves identically.
handlers[M.Opcodes.JUMP_LOOP] = function(vm, frame, instr)
  frame.ip = frame.ip + op(instr, 1)
end

-- ── Function Calls ───────────────────────────────────────────────────────────
-- Calling a function pushes a new CallFrame onto a conceptual call stack.
-- In our implementation we do it recursively (Lua's own call stack handles it).
--
-- CALL_ANY_RECEIVER: Call the function in acc with arguments from registers.
--   operand[1] = first argument register (0-based)
--   operand[2] = argument count
-- The called function must be a VMFunction (table with kind="function").

local function call_vm_function(vm, fn_table, args)
  -- Increment call depth for STACK_CHECK purposes.
  vm.call_depth = vm.call_depth + 1
  if vm.call_depth > vm.max_depth then
    vm.call_depth = vm.call_depth - 1
    return nil, "stack overflow: call depth exceeded " .. tostring(vm.max_depth)
  end

  -- Create a new context as a child of the function's closure context.
  local fn_context = new_context(fn_table.context, fn_table.code.parameter_count)
  -- Install arguments into context slots.
  for i, arg_val in ipairs(args) do
    fn_context.slots[i] = arg_val
  end

  local fn_frame = new_call_frame(fn_table.code, fn_context, nil)
  local result_val, err = run_frame(vm, fn_frame)
  vm.call_depth = vm.call_depth - 1
  return result_val, err
end

-- Forward-declared below; defined after run_frame is defined.
local run_frame

handlers[M.Opcodes.CALL_ANY_RECEIVER] = function(vm, frame, instr)
  local fn_table = frame.accumulator
  local first_arg_reg = op(instr, 1)
  local arg_count     = op(instr, 2)

  if type(fn_table) ~= "table" or fn_table.kind ~= "function" then
    return "TypeError: callee is not a function"
  end

  local args = {}
  for i = 0, arg_count - 1 do
    args[i + 1] = get_reg(frame, first_arg_reg + i)
  end

  local result_val, err = call_vm_function(vm, fn_table, args)
  if err then return err end
  frame.accumulator = result_val
end

-- CALL_PROPERTY: Call a method on an object.
--   operand[1] = register holding the receiver object
--   operand[2] = name index
--   operand[3] = first argument register
--   operand[4] = argument count
handlers[M.Opcodes.CALL_PROPERTY] = function(vm, frame, instr)
  local obj      = get_reg(frame, op(instr, 1))
  local name     = frame.code.names[op(instr, 2) + 1]
  local first_reg = op(instr, 3)
  local arg_count = op(instr, 4)

  local method
  if type(obj) == "table" and obj.properties ~= nil then
    method = obj.properties[name]
  elseif type(obj) == "table" then
    method = obj[name]
  end

  if type(method) ~= "table" or method.kind ~= "function" then
    return "TypeError: method '" .. tostring(name) .. "' is not a function"
  end

  local args = {}
  for i = 0, arg_count - 1 do
    args[i + 1] = get_reg(frame, first_reg + i)
  end

  local result_val, err = call_vm_function(vm, method, args)
  if err then return err end
  frame.accumulator = result_val
end

-- CALL_UNDEFINED_RECEIVER: Like CALL_ANY_RECEIVER but "this" is undefined.
-- For our purposes identical to CALL_ANY_RECEIVER.
handlers[M.Opcodes.CALL_UNDEFINED_RECEIVER] = handlers[M.Opcodes.CALL_ANY_RECEIVER]

-- CONSTRUCT: Call acc as a constructor (new acc(args)).
--   Creates a new VMObject and calls the function with it as context.
handlers[M.Opcodes.CONSTRUCT] = function(vm, frame, instr)
  local fn_table  = frame.accumulator
  local first_reg = op(instr, 1)
  local arg_count = op(instr, 2)

  if type(fn_table) ~= "table" or fn_table.kind ~= "function" then
    return "TypeError: constructor is not a function"
  end

  local new_obj = new_vm_object({})

  local args = { new_obj }
  for i = 0, arg_count - 1 do
    args[i + 2] = get_reg(frame, first_reg + i)
  end

  local result_val, err = call_vm_function(vm, fn_table, args)
  if err then return err end
  -- Constructors conventionally return the new object (or their explicit return).
  if result_val == nil then
    frame.accumulator = new_obj
  else
    frame.accumulator = result_val
  end
end

-- RETURN: Return acc to the caller. The run_frame loop detects this signal.
handlers[M.Opcodes.RETURN] = function(vm, frame, instr)
  return "return"
end

-- SUSPEND_GENERATOR / RESUME_GENERATOR: Stub — generators not fully implemented.
handlers[M.Opcodes.SUSPEND_GENERATOR] = function(vm, frame, instr)
  return "not_implemented: SUSPEND_GENERATOR"
end

handlers[M.Opcodes.RESUME_GENERATOR] = function(vm, frame, instr)
  return "not_implemented: RESUME_GENERATOR"
end

-- ── Property Access ──────────────────────────────────────────────────────────

-- LDA_NAMED_PROPERTY: Load a named property from an object.
--   acc = object in accumulator
--   operand[1] = register holding the object (receiver)
--   operand[2] = name index
--   operand[3] = feedback slot index
-- V8 convention: acc holds receiver, but we use the register operand.
-- We support both: if operand[1] is -1, use acc as receiver.
handlers[M.Opcodes.LDA_NAMED_PROPERTY] = function(vm, frame, instr)
  local obj_reg  = op(instr, 1)
  local name_idx = op(instr, 2)
  local slot_idx = instr.feedback_slot

  local obj
  if obj_reg == -1 then
    obj = frame.accumulator
  else
    obj = get_reg(frame, obj_reg)
  end

  local name = frame.code.names[name_idx + 1]

  -- Record hidden-class feedback for property access.
  if slot_idx >= 0 and slot_idx < #frame.feedback_vector and
     type(obj) == "table" and obj.__hidden_class_id ~= nil then
    local hcid_str = "hclass:" .. tostring(obj.__hidden_class_id)
    record_feedback(frame.feedback_vector[slot_idx + 1], hcid_str .. ":" .. hcid_str)
  end

  if type(obj) == "table" and obj.properties ~= nil then
    frame.accumulator = obj.properties[name]
  elseif type(obj) == "table" then
    frame.accumulator = obj[name]
  else
    frame.accumulator = nil
  end
end

-- STA_NAMED_PROPERTY: Store acc into a named property of an object.
--   operand[1] = register holding the object
--   operand[2] = name index
handlers[M.Opcodes.STA_NAMED_PROPERTY] = function(vm, frame, instr)
  local obj  = get_reg(frame, op(instr, 1))
  local name = frame.code.names[op(instr, 2) + 1]

  if type(obj) == "table" and obj.properties ~= nil then
    obj.properties[name] = frame.accumulator
    update_hidden_class(obj)
  elseif type(obj) == "table" then
    obj[name] = frame.accumulator
  end
end

-- LDA_KEYED_PROPERTY: Load obj[key] where key is in a register.
--   operand[1] = register holding the object
--   operand[2] = register holding the key
handlers[M.Opcodes.LDA_KEYED_PROPERTY] = function(vm, frame, instr)
  local obj = get_reg(frame, op(instr, 1))
  local key = get_reg(frame, op(instr, 2))

  if type(obj) == "table" and obj.properties ~= nil then
    frame.accumulator = obj.properties[key]
  elseif type(obj) == "table" then
    frame.accumulator = obj[key]
  else
    frame.accumulator = nil
  end
end

-- STA_KEYED_PROPERTY: obj[key] = acc.
--   operand[1] = register holding the object
--   operand[2] = register holding the key
handlers[M.Opcodes.STA_KEYED_PROPERTY] = function(vm, frame, instr)
  local obj = get_reg(frame, op(instr, 1))
  local key = get_reg(frame, op(instr, 2))

  if type(obj) == "table" and obj.properties ~= nil then
    obj.properties[key] = frame.accumulator
    update_hidden_class(obj)
  elseif type(obj) == "table" then
    obj[key] = frame.accumulator
  end
end

-- No-feedback variants: identical but skip the feedback recording.
handlers[M.Opcodes.LDA_NAMED_PROPERTY_NO_FEEDBACK] = function(vm, frame, instr)
  local obj  = get_reg(frame, op(instr, 1))
  local name = frame.code.names[op(instr, 2) + 1]
  if type(obj) == "table" and obj.properties ~= nil then
    frame.accumulator = obj.properties[name]
  elseif type(obj) == "table" then
    frame.accumulator = obj[name]
  else
    frame.accumulator = nil
  end
end

handlers[M.Opcodes.STA_NAMED_PROPERTY_NO_FEEDBACK] = function(vm, frame, instr)
  local obj  = get_reg(frame, op(instr, 1))
  local name = frame.code.names[op(instr, 2) + 1]
  if type(obj) == "table" and obj.properties ~= nil then
    obj.properties[name] = frame.accumulator
    update_hidden_class(obj)
  elseif type(obj) == "table" then
    obj[name] = frame.accumulator
  end
end

-- DELETE_PROPERTY_STRICT / DELETE_PROPERTY_SLOPPY: Delete a property.
handlers[M.Opcodes.DELETE_PROPERTY_STRICT] = function(vm, frame, instr)
  local obj  = get_reg(frame, op(instr, 1))
  local name = frame.code.names[op(instr, 2) + 1]
  if type(obj) == "table" and obj.properties ~= nil then
    obj.properties[name] = nil
    update_hidden_class(obj)
    frame.accumulator = true
  else
    frame.accumulator = false
  end
end

handlers[M.Opcodes.DELETE_PROPERTY_SLOPPY] = handlers[M.Opcodes.DELETE_PROPERTY_STRICT]

-- ── Object / Array / Closure Creation ────────────────────────────────────────

-- CREATE_OBJECT_LITERAL: Create a new empty VM object.
handlers[M.Opcodes.CREATE_OBJECT_LITERAL] = function(vm, frame, instr)
  frame.accumulator = new_vm_object({})
end

-- CREATE_ARRAY_LITERAL: Create a new array (VM object with numeric keys).
--   We store it as a VMObject with a `length` property and numeric indices.
handlers[M.Opcodes.CREATE_ARRAY_LITERAL] = function(vm, frame, instr)
  local arr = new_vm_object({})
  arr.is_array = true
  arr.items = {}
  frame.accumulator = arr
end

-- CREATE_REGEXP_LITERAL: Create a regexp placeholder.
handlers[M.Opcodes.CREATE_REGEXP_LITERAL] = function(vm, frame, instr)
  local pattern = frame.code.constants[op(instr, 1) + 1]
  frame.accumulator = { kind = "regexp", pattern = pattern }
end

-- CREATE_CLOSURE: Create a new function closure capturing the current context.
--   operand[1] = index into constants where the CodeObject is stored
handlers[M.Opcodes.CREATE_CLOSURE] = function(vm, frame, instr)
  local code_obj = frame.code.constants[op(instr, 1) + 1]
  frame.accumulator = {
    kind    = "function",
    code    = code_obj,
    context = frame.context,  -- capture current scope
  }
end

-- CREATE_CONTEXT: Create a new child scope context and push it.
--   operand[1] = number of slots in the new context
handlers[M.Opcodes.CREATE_CONTEXT] = function(vm, frame, instr)
  local slot_count = op(instr, 1) or 0
  frame.context = new_context(frame.context, slot_count)
end

-- CLONE_OBJECT: Shallow-copy a VMObject.
handlers[M.Opcodes.CLONE_OBJECT] = function(vm, frame, instr)
  local src = frame.accumulator
  if type(src) == "table" and src.properties ~= nil then
    local new_props = {}
    for k, v in pairs(src.properties) do
      new_props[k] = v
    end
    frame.accumulator = new_vm_object(new_props)
  else
    frame.accumulator = src
  end
end

-- ── Iterator Protocol ─────────────────────────────────────────────────────────
-- These opcodes implement the ES6 iterator protocol:
--   obj[Symbol.iterator]() → iterator
--   iterator.next()        → {value, done}
--
-- We simulate this with a simple Lua iterator table.

handlers[M.Opcodes.GET_ITERATOR] = function(vm, frame, instr)
  local obj = frame.accumulator
  -- Expect obj to have an `__iterator` key holding a function or a table.
  if type(obj) == "table" and obj.properties ~= nil and obj.properties.__iterator ~= nil then
    frame.accumulator = {
      kind    = "iterator",
      target  = obj,
      index   = 0,
      items   = obj.items or {},
    }
  elseif type(obj) == "table" and obj.items ~= nil then
    frame.accumulator = {
      kind   = "iterator",
      target = obj,
      index  = 0,
      items  = obj.items,
    }
  else
    frame.accumulator = nil
  end
end

handlers[M.Opcodes.CALL_ITERATOR_STEP] = function(vm, frame, instr)
  local iter = frame.accumulator
  if type(iter) ~= "table" or iter.kind ~= "iterator" then
    frame.accumulator = { done = true, value = nil }
    return
  end
  iter.index = iter.index + 1
  local val = iter.items[iter.index]
  frame.accumulator = { done = (val == nil), value = val }
end

handlers[M.Opcodes.GET_ITERATOR_DONE] = function(vm, frame, instr)
  local step = frame.accumulator
  if type(step) == "table" then
    frame.accumulator = step.done
  else
    frame.accumulator = true
  end
end

handlers[M.Opcodes.GET_ITERATOR_VALUE] = function(vm, frame, instr)
  local step = frame.accumulator
  if type(step) == "table" then
    frame.accumulator = step.value
  else
    frame.accumulator = nil
  end
end

-- ── Exception Handling ───────────────────────────────────────────────────────

-- THROW: Raise an error. Propagates up as a Lua error string.
handlers[M.Opcodes.THROW] = function(vm, frame, instr)
  local msg = tostring(frame.accumulator)
  return "throw:" .. msg
end

-- RETHROW: Re-throw the current error (same mechanism).
handlers[M.Opcodes.RETHROW] = function(vm, frame, instr)
  local msg = tostring(frame.accumulator)
  return "throw:" .. msg
end

-- ── Context and Module Variables ─────────────────────────────────────────────

-- PUSH_CONTEXT: Push a new child context.
handlers[M.Opcodes.PUSH_CONTEXT] = function(vm, frame, instr)
  local slot_count = op(instr, 1) or 0
  frame.context = new_context(frame.context, slot_count)
end

-- POP_CONTEXT: Pop back to the parent context.
handlers[M.Opcodes.POP_CONTEXT] = function(vm, frame, instr)
  if frame.context and frame.context.parent then
    frame.context = frame.context.parent
  end
end

-- LDA_MODULE_VARIABLE / STA_MODULE_VARIABLE: Use globals as module namespace.
handlers[M.Opcodes.LDA_MODULE_VARIABLE] = function(vm, frame, instr)
  local name = frame.code.names[op(instr, 1) + 1]
  frame.accumulator = vm.globals[name]
end

handlers[M.Opcodes.STA_MODULE_VARIABLE] = function(vm, frame, instr)
  local name = frame.code.names[op(instr, 1) + 1]
  vm.globals[name] = frame.accumulator
end

-- ── VM Meta Instructions ──────────────────────────────────────────────────────

-- STACK_CHECK: Verify call depth hasn't exceeded the limit.
-- This is emitted by the compiler at function entry to guard against
-- unbounded recursion. (In real V8 this also guards the C++ stack.)
handlers[M.Opcodes.STACK_CHECK] = function(vm, frame, instr)
  if vm.call_depth > vm.max_depth then
    return "stack overflow: call depth exceeded " .. tostring(vm.max_depth)
  end
end

-- DEBUGGER: No-op in our interpreter (would trigger a debugger breakpoint).
handlers[M.Opcodes.DEBUGGER] = function(vm, frame, instr)
  -- No-op
end

-- HALT: Stop execution immediately.
handlers[M.Opcodes.HALT] = function(vm, frame, instr)
  return "halt"
end

-- ============================================================================
-- ## Execution Loop
-- ============================================================================
-- run_frame executes a single CallFrame until RETURN or HALT (or error).
-- Returns (accumulator_value, error_string_or_nil).

run_frame = function(vm, frame)
  local instrs = frame.code.instructions

  while frame.ip >= 1 and frame.ip <= #instrs do
    local instr = instrs[frame.ip]

    -- Advance ip BEFORE dispatch so that jump handlers can modify it freely.
    -- A jump sets frame.ip = frame.ip + offset, where offset is relative to
    -- the instruction AFTER the jump.
    frame.ip = frame.ip + 1

    -- Record a trace step if tracing is enabled.
    if vm.trace ~= nil then
      vm.trace[#vm.trace + 1] = {
        ip          = frame.ip - 1,  -- the instruction we're executing
        opcode      = instr.opcode,
        accumulator = frame.accumulator,
        registers   = { table.unpack(frame.registers) },
      }
    end

    -- Look up the handler. Unknown opcodes produce an error.
    local handler = handlers[instr.opcode]
    if handler == nil then
      return nil, "unknown opcode: " .. tostring(instr.opcode)
    end

    -- Dispatch.
    local signal = handler(vm, frame, instr)

    -- Interpret the signal returned by the handler.
    if signal == "return" then
      -- Normal function return: propagate accumulator to caller.
      return frame.accumulator, nil
    elseif signal == "halt" then
      -- Halt: stop and return accumulator.
      return frame.accumulator, nil
    elseif signal ~= nil then
      -- Any other non-nil signal is treated as an error string.
      -- Strip "throw:" prefix if present, for cleaner error messages.
      local err_msg = signal
      if err_msg:sub(1, 6) == "throw:" then
        err_msg = err_msg:sub(7)
      end
      return nil, err_msg
    end
  end

  -- Fell off the end of the instruction stream: return accumulator.
  return frame.accumulator, nil
end

-- ============================================================================
-- ## Public API
-- ============================================================================

-- execute(code_object, globals) → {value=..., error=nil} | {value=nil, error="..."}
--
-- Execute a CodeObject in a fresh VM with the given globals table.
-- Returns a result table (never throws).
--
-- Example:
--   local result = VM.execute(code, {print = function(...) ... end})
--   if result.error then
--     io.stderr:write("Error: " .. result.error .. "\n")
--   else
--     print("Result:", result.value)
--   end
function M.execute(code_object, globals)
  local vm      = new_vm(globals)
  local context = new_context(nil, 0)
  local frame   = new_call_frame(code_object, context, nil)

  local value, err = run_frame(vm, frame)
  return { value = value, error = err }
end

-- execute_with_trace(code_object, globals) → result_table, trace_array
--
-- Like execute(), but also returns an array of TraceStep tables:
--   { ip=N, opcode=N, accumulator=..., registers={...} }
-- Useful for debugging and testing.
function M.execute_with_trace(code_object, globals)
  local vm      = new_vm(globals)
  vm.trace      = {}
  local context = new_context(nil, 0)
  local frame   = new_call_frame(code_object, context, nil)

  local value, err = run_frame(vm, frame)
  return { value = value, error = err }, vm.trace
end

-- ============================================================================
-- ## Exported Helpers (for tests and advanced users)
-- ============================================================================

-- new_vm_object is exported so tests can build object fixtures.
M.new_vm_object = new_vm_object

-- new_context is exported for tests that need custom scope chains.
M.new_context = new_context

-- record_feedback exported for unit-testing the state machine.
M.record_feedback = record_feedback

-- new_feedback_slot exported for creating test slots.
M.new_feedback_slot = new_feedback_slot

return M
