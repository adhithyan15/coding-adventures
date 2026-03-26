-- bytecode_compiler — Compiler translating ASTs to stack-based bytecode instructions
-- ===================================================================================
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- Layer 4 in the computing stack.
--
-- # What is a bytecode compiler?
--
-- A bytecode compiler translates an Abstract Syntax Tree (AST) — the structured
-- representation of source code produced by a parser — into a flat sequence of
-- bytecode instructions that a virtual machine can execute.
--
-- Think of it like translating a recipe (structured with sections, sub-steps,
-- and nested instructions) into a simple numbered list of actions. The chef
-- (VM) just follows the list from top to bottom, occasionally jumping to a
-- different step number.
--
-- # Why bytecode?
--
-- Source code is designed for humans to read. ASTs are designed for programs
-- to analyze. But neither is efficient for execution. Bytecode sits in between:
--
--   Source code  →  [Lexer]  →  Tokens  →  [Parser]  →  AST  →  [Compiler]  →  Bytecode
--        ↑                                                              ↓
--   Human-readable                                              VM-executable
--
-- Real-world examples:
--   - Java: .java → javac → .class files (JVM bytecode)
--   - Python: .py → compile → .pyc files (CPython bytecode)
--   - Lua: .lua → luac → bytecode chunks
--
-- # Three compilers in one package
--
-- This package provides three compilers, each illustrating a different design:
--
-- 1. BytecodeCompiler — A simple, hardcoded compiler that translates our
--    parser's AST (with NumberLiteral, StringLiteral, BinaryOp, Assignment,
--    ExpressionStmt nodes) directly to our VM's instruction set. Tightly
--    coupled to one language.
--
-- 2. JVMCompiler — A compiler targeting JVM-style bytecode (ICONST, BIPUSH,
--    LDC, ILOAD, ISTORE, etc.). Demonstrates how the same AST can compile
--    to a completely different instruction set with different encoding rules.
--
-- 3. GenericCompiler — A pluggable framework where language-specific behavior
--    is provided by registering handler functions for each AST rule name.
--    Like LLVM's approach: the framework handles plumbing, plugins handle
--    language semantics.
--
-- # OOP pattern
--
-- We use the standard Lua metatable OOP pattern throughout:
--
--     local MyClass = {}
--     MyClass.__index = MyClass
--     function MyClass.new() ... end
--
-- Methods are called with the colon operator: obj:method(args).
--
-- # Dependencies
--
-- This package depends on:
--   - parser: provides AST node types (Program, Assignment, BinaryOp, etc.)
--   - virtual-machine: provides Instruction, CodeObject, and OpCode constants

local bytecode_compiler = {}

bytecode_compiler.VERSION = "0.1.0"


-- =========================================================================
-- Chapter 1: OpCodes — The instruction set constants
-- =========================================================================
--
-- These constants mirror the Go virtual-machine package's OpCode values.
-- Each opcode is a numeric identifier telling the VM what operation to
-- perform. We define them here so the compiler can reference them without
-- depending on the VM package at require-time (since the VM is a stub).
--
-- The opcodes are organized by category:
--
--   0x01-0x0F: Stack manipulation (LOAD_CONST, POP, DUP)
--   0x10-0x1F: Variable access (STORE_NAME, LOAD_NAME, STORE_LOCAL, LOAD_LOCAL)
--   0x20-0x2F: Arithmetic (ADD, SUB, MUL, DIV)
--   0x30-0x3F: Comparison (CMP_EQ, CMP_LT, CMP_GT)
--   0x40-0x4F: Control flow (JUMP, JUMP_IF_FALSE, JUMP_IF_TRUE)
--   0x50-0x5F: Functions (CALL, RETURN)
--   0x60-0x6F: I/O (PRINT)
--   0xFF:      Termination (HALT)

bytecode_compiler.OpLoadConst   = 0x01
bytecode_compiler.OpPop         = 0x02
bytecode_compiler.OpDup         = 0x03
bytecode_compiler.OpStoreName   = 0x10
bytecode_compiler.OpLoadName    = 0x11
bytecode_compiler.OpStoreLocal  = 0x12
bytecode_compiler.OpLoadLocal   = 0x13
bytecode_compiler.OpAdd         = 0x20
bytecode_compiler.OpSub         = 0x21
bytecode_compiler.OpMul         = 0x22
bytecode_compiler.OpDiv         = 0x23
bytecode_compiler.OpCmpEq       = 0x30
bytecode_compiler.OpCmpLt       = 0x31
bytecode_compiler.OpCmpGt       = 0x32
bytecode_compiler.OpJump        = 0x40
bytecode_compiler.OpJumpIfFalse = 0x41
bytecode_compiler.OpJumpIfTrue  = 0x42
bytecode_compiler.OpCall        = 0x50
bytecode_compiler.OpReturn      = 0x51
bytecode_compiler.OpPrint       = 0x60
bytecode_compiler.OpHalt        = 0xFF


-- =========================================================================
-- Chapter 2: Instruction and CodeObject — Data containers
-- =========================================================================
--
-- An Instruction is a single bytecode operation: an opcode plus an optional
-- operand. For example:
--
--   { opcode = 0x01, operand = 0 }   -- LOAD_CONST index 0
--   { opcode = 0x20 }                -- ADD (no operand needed)
--
-- A CodeObject bundles everything the VM needs to execute a compiled program:
--   - instructions: the sequence of Instruction tables
--   - constants: literal values (numbers, strings) referenced by index
--   - names: variable/function names referenced by index

--- Create a new Instruction table.
-- @param opcode number The opcode for this instruction.
-- @param operand any Optional operand (index, jump target, etc.).
-- @return table An instruction table with opcode and operand fields.
function bytecode_compiler.Instruction(opcode, operand)
    return { opcode = opcode, operand = operand }
end

--- Create a new CodeObject table.
-- @param instructions table Array of Instruction tables.
-- @param constants table Array of constant values.
-- @param names table Array of name strings.
-- @return table A code object with instructions, constants, and names fields.
function bytecode_compiler.CodeObject(instructions, constants, names)
    return {
        instructions = instructions or {},
        constants = constants or {},
        names = names or {},
    }
end


-- =========================================================================
-- Chapter 3: AST Node constructors — The input format
-- =========================================================================
--
-- These constructors create the AST node types that the BytecodeCompiler
-- expects. They mirror the Go parser's types:
--
--   Program        — A list of statements (the root node).
--   Assignment     — "x = expr" — stores a value in a named variable.
--   ExpressionStmt — A bare expression whose result is discarded.
--   NumberLiteral  — A numeric constant like 42 or 3.14.
--   StringLiteral  — A string constant like "hello".
--   Name           — A variable reference like x or counter.
--   BinaryOp       — An infix operation like "1 + 2" or "a * b".
--
-- Each node has a `type` field for dispatch, matching the Go type names.

--- Create a Program node (root of the AST).
-- @param statements table Array of statement nodes.
-- @return table A program node.
function bytecode_compiler.Program(statements)
    return { type = "Program", statements = statements or {} }
end

--- Create an Assignment node: target = value.
-- @param target table A Name node for the variable being assigned.
-- @param value table An expression node for the value.
-- @return table An assignment node.
function bytecode_compiler.Assignment(target, value)
    return { type = "Assignment", target = target, value = value }
end

--- Create an ExpressionStmt node (expression used as a statement).
-- @param expression table The expression whose result is discarded.
-- @return table An expression statement node.
function bytecode_compiler.ExpressionStmt(expression)
    return { type = "ExpressionStmt", expression = expression }
end

--- Create a NumberLiteral node.
-- @param value number The numeric value.
-- @return table A number literal node.
function bytecode_compiler.NumberLiteral(value)
    return { type = "NumberLiteral", value = value }
end

--- Create a StringLiteral node.
-- @param value string The string value.
-- @return table A string literal node.
function bytecode_compiler.StringLiteral(value)
    return { type = "StringLiteral", value = value }
end

--- Create a Name node (variable reference).
-- @param name string The variable name.
-- @return table A name node.
function bytecode_compiler.Name(name)
    return { type = "Name", name = name }
end

--- Create a BinaryOp node (infix operator).
-- @param left table The left operand expression.
-- @param op string The operator symbol ("+", "-", "*", "/").
-- @param right table The right operand expression.
-- @return table A binary operation node.
function bytecode_compiler.BinaryOp(left, op, right)
    return { type = "BinaryOp", left = left, op = op, right = right }
end


-- =========================================================================
-- Chapter 4: BytecodeCompiler — The simple, hardcoded compiler
-- =========================================================================
--
-- The BytecodeCompiler is the most straightforward compiler design: it
-- knows exactly what AST node types to expect (from our parser) and
-- what instruction set to target (our VM's opcodes).
--
-- It walks the AST recursively, emitting instructions as it goes:
--
--   NumberLiteral → LOAD_CONST (add number to constant pool)
--   StringLiteral → LOAD_CONST (add string to constant pool)
--   Name          → LOAD_NAME (look up variable by name index)
--   BinaryOp      → compile left, compile right, emit operator
--   Assignment    → compile value, STORE_NAME
--   ExpressionStmt → compile expression, POP (discard result)
--   Program       → compile each statement, HALT
--
-- The operator map translates operator symbols to opcodes:
--
--   "+" → OpAdd (0x20)
--   "-" → OpSub (0x21)
--   "*" → OpMul (0x22)
--   "/" → OpDiv (0x23)

local BytecodeCompiler = {}
BytecodeCompiler.__index = BytecodeCompiler

--- The mapping from operator symbols to VM opcodes.
-- When the compiler encounters a BinaryOp node with operator "+", it
-- looks up "+" in this table to find OpAdd (0x20).
BytecodeCompiler.OPERATOR_MAP = {
    ["+"] = bytecode_compiler.OpAdd,
    ["-"] = bytecode_compiler.OpSub,
    ["*"] = bytecode_compiler.OpMul,
    ["/"] = bytecode_compiler.OpDiv,
}

--- Create a new BytecodeCompiler.
--
-- The compiler starts with empty instruction, constant, and name lists.
-- As it compiles an AST, it populates these lists. After compilation,
-- they are bundled into a CodeObject.
--
-- @return BytecodeCompiler A new compiler instance.
function BytecodeCompiler.new()
    local self = setmetatable({}, BytecodeCompiler)

    -- The growing list of bytecode instructions.
    self.instructions = {}

    -- The constant pool — literal values referenced by LOAD_CONST.
    -- Deduplicated: the same value always gets the same index.
    self.constants = {}

    -- The name pool — variable names referenced by STORE_NAME / LOAD_NAME.
    -- Deduplicated: the same name always gets the same index.
    self.names = {}

    return self
end

--- Compile a Program AST into a CodeObject.
--
-- This is the main entry point. It:
--   1. Compiles each statement in the program.
--   2. Appends a HALT instruction.
--   3. Returns a self-contained CodeObject.
--
-- @param program table A Program node with a statements array.
-- @return table A CodeObject with instructions, constants, and names.
function BytecodeCompiler:compile(program)
    for _, stmt in ipairs(program.statements) do
        self:compile_statement(stmt)
    end

    -- Every program must end with HALT. Without this, the VM would try
    -- to read past the end of the instruction array — like a car driving
    -- off the end of a road.
    table.insert(self.instructions,
        bytecode_compiler.Instruction(bytecode_compiler.OpHalt))

    return bytecode_compiler.CodeObject(
        self.instructions,
        self.constants,
        self.names
    )
end

--- Compile a single statement.
--
-- Dispatches to the appropriate method based on the statement's type field.
-- Currently supports:
--   - Assignment: "x = expr"
--   - ExpressionStmt: bare expression (result discarded with POP)
--
-- @param stmt table A statement node.
function BytecodeCompiler:compile_statement(stmt)
    if stmt.type == "Assignment" then
        self:compile_assignment(stmt)
    elseif stmt.type == "ExpressionStmt" then
        self:compile_expression(stmt.expression)
        -- The expression leaves a value on the stack. Since this is a
        -- statement (not part of a larger expression), we discard it.
        table.insert(self.instructions,
            bytecode_compiler.Instruction(bytecode_compiler.OpPop))
    else
        error("Unknown statement type: " .. tostring(stmt.type))
    end
end

--- Compile an assignment statement.
--
-- An assignment "x = expr" compiles to:
--   1. Compile the value expression (pushes result onto stack).
--   2. STORE_NAME with the index of "x" in the name pool.
--
-- @param node table An Assignment node.
function BytecodeCompiler:compile_assignment(node)
    self:compile_expression(node.value)
    local name_index = self:add_name(node.target.name)
    table.insert(self.instructions,
        bytecode_compiler.Instruction(bytecode_compiler.OpStoreName, name_index))
end

--- Compile an expression node.
--
-- Dispatches based on the expression's type field:
--
--   NumberLiteral → Add to constants, emit LOAD_CONST.
--   StringLiteral → Add to constants, emit LOAD_CONST.
--   Name          → Add to names, emit LOAD_NAME.
--   BinaryOp      → Compile left, compile right, emit operator.
--
-- @param expr table An expression node.
function BytecodeCompiler:compile_expression(expr)
    if expr.type == "NumberLiteral" then
        local idx = self:add_constant(expr.value)
        table.insert(self.instructions,
            bytecode_compiler.Instruction(bytecode_compiler.OpLoadConst, idx))

    elseif expr.type == "StringLiteral" then
        local idx = self:add_constant(expr.value)
        table.insert(self.instructions,
            bytecode_compiler.Instruction(bytecode_compiler.OpLoadConst, idx))

    elseif expr.type == "Name" then
        local idx = self:add_name(expr.name)
        table.insert(self.instructions,
            bytecode_compiler.Instruction(bytecode_compiler.OpLoadName, idx))

    elseif expr.type == "BinaryOp" then
        -- Post-order traversal: compile operands first, then the operator.
        -- This ensures the operands are on the stack when the operator runs.
        --
        -- For "1 + 2":
        --   LOAD_CONST 0  (pushes 1)
        --   LOAD_CONST 1  (pushes 2)
        --   ADD            (pops 2 and 1, pushes 3)
        self:compile_expression(expr.left)
        self:compile_expression(expr.right)

        local opcode = BytecodeCompiler.OPERATOR_MAP[expr.op]
        if not opcode then
            error("Unknown operator: " .. tostring(expr.op))
        end
        table.insert(self.instructions,
            bytecode_compiler.Instruction(opcode))

    else
        error("Unknown expression type: " .. tostring(expr.type))
    end
end

--- Add a constant to the constant pool, returning its index.
--
-- Constants are deduplicated: if the same value already exists in the
-- pool, its existing index is returned. This saves memory and ensures
-- that "LOAD_CONST 0" always means the same thing.
--
-- Note: Lua indices are 0-based in the returned index (matching the Go
-- implementation), even though Lua tables are 1-based internally. The
-- constant at index 0 is stored at self.constants[1].
--
-- @param value any The constant value (number, string, etc.).
-- @return number The 0-based index of the constant in the pool.
function BytecodeCompiler:add_constant(value)
    for i, v in ipairs(self.constants) do
        if v == value then
            return i - 1  -- Convert 1-based Lua index to 0-based
        end
    end
    table.insert(self.constants, value)
    return #self.constants - 1  -- Convert 1-based length to 0-based index
end

--- Add a name to the name pool, returning its index.
--
-- Names are deduplicated just like constants. The same variable name
-- used in multiple places gets the same index.
--
-- @param name string The variable or function name.
-- @return number The 0-based index of the name in the pool.
function BytecodeCompiler:add_name(name)
    for i, n in ipairs(self.names) do
        if n == name then
            return i - 1
        end
    end
    table.insert(self.names, name)
    return #self.names - 1
end

bytecode_compiler.BytecodeCompiler = BytecodeCompiler


-- =========================================================================
-- Chapter 5: JVM Bytecode Constants
-- =========================================================================
--
-- The JVM (Java Virtual Machine) uses a completely different instruction
-- encoding than our custom VM. Where our VM uses high-level opcodes like
-- LOAD_CONST with an operand, the JVM uses specialized instructions:
--
--   ICONST_0 through ICONST_5: Push integers 0-5 (no operand needed).
--   BIPUSH: Push a byte-sized integer (-128 to 127).
--   LDC: Load a constant from the constant pool.
--   ILOAD_0 through ILOAD_3: Load local variable from slots 0-3.
--   ISTORE_0 through ISTORE_3: Store to local variable slots 0-3.
--
-- This specialization reduces bytecode size: "push 0" is a single byte
-- (ICONST_0 = 0x03) instead of two bytes (BIPUSH 0x00).
--
-- Real JVM bytecode uses exactly these opcodes. You can verify with
-- "javap -c" on any .class file.

bytecode_compiler.ICONST_0 = 0x03
bytecode_compiler.ICONST_1 = 0x04
bytecode_compiler.ICONST_2 = 0x05
bytecode_compiler.ICONST_3 = 0x06
bytecode_compiler.ICONST_4 = 0x07
bytecode_compiler.ICONST_5 = 0x08
bytecode_compiler.BIPUSH   = 0x10
bytecode_compiler.LDC      = 0x12
bytecode_compiler.ILOAD    = 0x15
bytecode_compiler.ILOAD_0  = 0x1A
bytecode_compiler.ILOAD_1  = 0x1B
bytecode_compiler.ILOAD_2  = 0x1C
bytecode_compiler.ILOAD_3  = 0x1D
bytecode_compiler.ISTORE   = 0x36
bytecode_compiler.ISTORE_0 = 0x3B
bytecode_compiler.ISTORE_1 = 0x3C
bytecode_compiler.ISTORE_2 = 0x3D
bytecode_compiler.ISTORE_3 = 0x3E
bytecode_compiler.JVM_POP  = 0x57
bytecode_compiler.IADD     = 0x60
bytecode_compiler.ISUB     = 0x64
bytecode_compiler.IMUL     = 0x68
bytecode_compiler.IDIV     = 0x6C
bytecode_compiler.RETURN   = 0xB1


-- =========================================================================
-- Chapter 6: JVMCodeObject — Output format for JVM compilation
-- =========================================================================
--
-- A JVMCodeObject differs from our CodeObject in that the bytecode is a
-- flat array of raw bytes (not structured Instruction tables). This matches
-- how real JVM .class files store method bytecode.

--- Create a JVMCodeObject.
-- @param bytecodes table Array of byte values (numbers 0-255).
-- @param constants table Array of constant values.
-- @param num_locals number Total number of local variable slots.
-- @param local_names table Array of local variable names.
-- @return table A JVM code object.
function bytecode_compiler.JVMCodeObject(bytecodes, constants, num_locals, local_names)
    return {
        bytecode = bytecodes or {},
        constants = constants or {},
        num_locals = num_locals or 0,
        local_names = local_names or {},
    }
end


-- =========================================================================
-- Chapter 7: JVMCompiler — Targeting JVM-style bytecode
-- =========================================================================
--
-- The JVMCompiler translates the same AST as BytecodeCompiler, but emits
-- JVM-compatible bytecode. The key differences:
--
-- 1. Raw bytes instead of structured instructions.
--    Our VM: { opcode = 0x01, operand = 0 }
--    JVM:    { 0x12, 0x00 }  (LDC followed by constant pool index)
--
-- 2. Specialized instructions for small values.
--    Our VM: LOAD_CONST 0  (always 2 "units")
--    JVM:    ICONST_0       (1 byte for values 0-5)
--            BIPUSH n       (2 bytes for -128 to 127)
--            LDC idx        (2 bytes for larger values)
--
-- 3. Indexed local variable slots instead of named variables.
--    Our VM: STORE_NAME "x"  (looks up by name)
--    JVM:    ISTORE_0         (stores to slot 0)
--
-- This demonstrates how the same high-level program compiles very
-- differently depending on the target architecture.

local JVMCompiler = {}
JVMCompiler.__index = JVMCompiler

--- The mapping from operator symbols to JVM arithmetic opcodes.
JVMCompiler.OPERATOR_MAP = {
    ["+"] = bytecode_compiler.IADD,
    ["-"] = bytecode_compiler.ISUB,
    ["*"] = bytecode_compiler.IMUL,
    ["/"] = bytecode_compiler.IDIV,
}

--- Create a new JVMCompiler.
-- @return JVMCompiler A new compiler instance.
function JVMCompiler.new()
    local self = setmetatable({}, JVMCompiler)
    self.bytecode = {}
    self.constants = {}
    self.locals = {}
    return self
end

--- Compile a Program AST into a JVMCodeObject.
-- @param program table A Program node with a statements array.
-- @return table A JVMCodeObject with bytecode, constants, and local info.
function JVMCompiler:compile(program)
    for _, stmt in ipairs(program.statements) do
        self:compile_statement(stmt)
    end

    -- The JVM requires every method to end with a return instruction.
    -- RETURN (0xB1) is the void-return opcode.
    table.insert(self.bytecode, bytecode_compiler.RETURN)

    return bytecode_compiler.JVMCodeObject(
        self.bytecode,
        self.constants,
        #self.locals,
        self.locals
    )
end

--- Compile a single statement.
-- @param stmt table A statement node.
function JVMCompiler:compile_statement(stmt)
    if stmt.type == "Assignment" then
        self:compile_assignment(stmt)
    elseif stmt.type == "ExpressionStmt" then
        self:compile_expression(stmt.expression)
        table.insert(self.bytecode, bytecode_compiler.JVM_POP)
    else
        error("Unknown statement type: " .. tostring(stmt.type))
    end
end

--- Compile an assignment statement for the JVM.
--
-- JVM variables are stored in numbered "local variable slots" (0, 1, 2, ...),
-- not by name. The compiler assigns slot numbers as variables are first seen.
--
-- @param node table An Assignment node.
function JVMCompiler:compile_assignment(node)
    self:compile_expression(node.value)
    local slot = self:get_local_slot(node.target.name)
    self:emit_istore(slot)
end

--- Compile an expression for JVM bytecode.
-- @param expr table An expression node.
function JVMCompiler:compile_expression(expr)
    if expr.type == "NumberLiteral" then
        self:emit_number(expr.value)

    elseif expr.type == "StringLiteral" then
        local idx = self:add_constant(expr.value)
        table.insert(self.bytecode, bytecode_compiler.LDC)
        -- LDC operand is the constant pool index (single byte).
        table.insert(self.bytecode, idx)

    elseif expr.type == "Name" then
        local slot = self:get_local_slot(expr.name)
        self:emit_iload(slot)

    elseif expr.type == "BinaryOp" then
        self:compile_expression(expr.left)
        self:compile_expression(expr.right)

        local opcode = JVMCompiler.OPERATOR_MAP[expr.op]
        if not opcode then
            error("Unknown operator: " .. tostring(expr.op))
        end
        table.insert(self.bytecode, opcode)

    else
        error("Unknown expression type: " .. tostring(expr.type))
    end
end

--- Emit the most efficient instruction for loading an integer.
--
-- The JVM has three ways to push an integer, chosen by value range:
--
--   0-5:       ICONST_n  (1 byte) — most compact
--   -128..127: BIPUSH n  (2 bytes)
--   else:      LDC idx   (2 bytes, uses constant pool)
--
-- This optimization matters in real JVM bytecode: class files are often
-- shipped over networks, so smaller is better.
--
-- @param value number The integer value to push.
function JVMCompiler:emit_number(value)
    if value >= 0 and value <= 5 then
        -- ICONST_0 (0x03) through ICONST_5 (0x08).
        -- The opcode for value n is ICONST_0 + n.
        table.insert(self.bytecode, bytecode_compiler.ICONST_0 + value)

    elseif value >= -128 and value <= 127 then
        -- BIPUSH pushes a signed byte.
        table.insert(self.bytecode, bytecode_compiler.BIPUSH)
        -- Mask to unsigned byte representation (handles negative values).
        table.insert(self.bytecode, value & 0xFF)

    else
        -- Fall back to constant pool for larger values.
        local idx = self:add_constant(value)
        table.insert(self.bytecode, bytecode_compiler.LDC)
        table.insert(self.bytecode, idx)
    end
end

--- Emit an ISTORE instruction for the given local variable slot.
--
-- Slots 0-3 have dedicated single-byte opcodes (ISTORE_0 through ISTORE_3).
-- Higher slots use the two-byte ISTORE + slot form.
--
-- @param slot number The local variable slot index (0-based).
function JVMCompiler:emit_istore(slot)
    if slot <= 3 then
        table.insert(self.bytecode, bytecode_compiler.ISTORE_0 + slot)
    else
        table.insert(self.bytecode, bytecode_compiler.ISTORE)
        table.insert(self.bytecode, slot)
    end
end

--- Emit an ILOAD instruction for the given local variable slot.
--
-- Like ISTORE, slots 0-3 have dedicated single-byte opcodes.
--
-- @param slot number The local variable slot index (0-based).
function JVMCompiler:emit_iload(slot)
    if slot <= 3 then
        table.insert(self.bytecode, bytecode_compiler.ILOAD_0 + slot)
    else
        table.insert(self.bytecode, bytecode_compiler.ILOAD)
        table.insert(self.bytecode, slot)
    end
end

--- Add a constant to the JVM constant pool, returning its 0-based index.
-- Deduplicated: same value returns same index.
-- @param value any The constant value.
-- @return number The 0-based index.
function JVMCompiler:add_constant(value)
    for i, v in ipairs(self.constants) do
        if v == value then
            return i - 1
        end
    end
    table.insert(self.constants, value)
    return #self.constants - 1
end

--- Get the local variable slot for a name, allocating a new one if needed.
--
-- Local variable slots are assigned sequentially: the first variable seen
-- gets slot 0, the second gets slot 1, and so on. If the same variable
-- name is referenced again, it returns the existing slot.
--
-- @param name string The variable name.
-- @return number The 0-based slot index.
function JVMCompiler:get_local_slot(name)
    for i, n in ipairs(self.locals) do
        if n == name then
            return i - 1
        end
    end
    table.insert(self.locals, name)
    return #self.locals - 1
end

bytecode_compiler.JVMCompiler = JVMCompiler


-- =========================================================================
-- Chapter 8: GenericCompiler AST types — ASTNode and TokenNode
-- =========================================================================
--
-- For the GenericCompiler to walk *any* language's AST, we need a common
-- tree shape different from the BytecodeCompiler's AST:
--
-- ASTNode — A non-terminal (interior) node. It has:
--   - rule_name: identifies what grammar rule produced this node.
--   - children: an ordered list of child nodes (ASTNode or TokenNode).
--
-- TokenNode — A terminal (leaf) node. It has:
--   - token_type: the token category (e.g., "NUMBER", "IDENTIFIER").
--   - value: the actual text from the source code.
--
-- Example: the expression "1 + 2" might parse into:
--
--   ASTNode{ rule_name = "addition", children = {
--       TokenNode{ token_type = "NUMBER", value = "1" },
--       TokenNode{ token_type = "PLUS", value = "+" },
--       TokenNode{ token_type = "NUMBER", value = "2" },
--   }}

--- Create an ASTNode (non-terminal / interior node).
-- @param rule_name string The grammar rule that produced this node.
-- @param children table Array of child nodes (ASTNode or TokenNode tables).
-- @return table An AST node table with node_kind = "ast".
function bytecode_compiler.ASTNode(rule_name, children)
    return {
        node_kind = "ast",
        rule_name = rule_name,
        children = children or {},
    }
end

--- Create a TokenNode (terminal / leaf node).
-- @param token_type string The token category (e.g., "NUMBER").
-- @param value string The token text from source code.
-- @return table A token node table with node_kind = "token".
function bytecode_compiler.TokenNode(token_type, value)
    return {
        node_kind = "token",
        token_type = token_type,
        value = value,
    }
end


-- =========================================================================
-- Chapter 9: CompilerScope — Local variable tracking for nested scopes
-- =========================================================================
--
-- Languages with functions or block scoping need to track which variables
-- are "local" to each scope. CompilerScope provides a scope stack.
--
-- Each CompilerScope maintains a locals map from variable names to slot
-- indices. Scopes form a linked list via the parent pointer, enabling
-- lexical scoping lookups.
--
-- Real VMs do this too:
--   - The JVM uses a "local variable array" per stack frame, indexed by slot.
--   - CPython uses a co_varnames tuple, indexed by slot.
--   - Our scope's locals map serves the same purpose.

local CompilerScope = {}
CompilerScope.__index = CompilerScope

--- Create a new CompilerScope linked to the given parent.
-- If parent is nil, this is the outermost (global) scope.
-- @param parent CompilerScope|nil The enclosing scope.
-- @return CompilerScope A new scope instance.
function CompilerScope.new(parent)
    local self = setmetatable({}, CompilerScope)
    self.locals = {}
    self.parent = parent
    self._count = 0  -- Track number of locals for slot assignment.
    return self
end

--- Register a new local variable and return its slot index.
-- If the name already exists, returns the existing slot (deduplication).
-- @param name string The variable name.
-- @return number The 0-based slot index.
function CompilerScope:add_local(name)
    if self.locals[name] then
        return self.locals[name]
    end
    local slot = self._count
    self.locals[name] = slot
    self._count = self._count + 1
    return slot
end

--- Look up a variable's slot index by name.
-- Returns the slot and true if found, or nil and false if not.
-- Does NOT walk up the parent chain — that's intentional. Different
-- languages handle scope lookup differently.
-- @param name string The variable name.
-- @return number|nil, boolean The slot index (or nil) and whether found.
function CompilerScope:get_local(name)
    local slot = self.locals[name]
    if slot then
        return slot, true
    end
    return nil, false
end

--- Return the total number of local variables in this scope.
-- Needed for function metadata — the VM needs to know how many local
-- slots to allocate.
-- @return number The count of local variables.
function CompilerScope:num_locals()
    return self._count
end

bytecode_compiler.CompilerScope = CompilerScope


-- =========================================================================
-- Chapter 10: GenericCompiler — The pluggable compilation framework
-- =========================================================================
--
-- The GenericCompiler provides the infrastructure for compilation:
-- instruction emission, constant/name pool management, scope tracking,
-- jump patching, and nested code object compilation. Language-specific
-- behavior is provided by registering handler functions for each AST
-- rule name.
--
-- Think of it like a kitchen (GenericCompiler) with cooking equipment
-- (emit, add_constant, enter_scope, etc.) — and the chef (language
-- plugin) decides what dish to make by registering recipes (handlers).
--
-- Usage:
--
--   local compiler = GenericCompiler.new()
--
--   compiler:register_rule("number", function(c, node)
--       local token = node.children[1]
--       local value = tonumber(token.value)
--       local idx = c:add_constant(value)
--       c:emit(bc.OpLoadConst, idx)
--   end)
--
--   local code = compiler:compile(ast)
--
-- This is exactly how real compiler frameworks work:
--   - LLVM has a generic IR that many language front-ends compile to.
--   - GraalVM's Truffle framework lets languages register AST interpreters.
--   - .NET's Roslyn has a common compilation pipeline with language-specific
--     syntax analyzers plugged in.

local GenericCompiler = {}
GenericCompiler.__index = GenericCompiler

--- Create a fresh GenericCompiler with empty state and no registered handlers.
-- @return GenericCompiler A new compiler instance.
function GenericCompiler.new()
    local self = setmetatable({}, GenericCompiler)

    -- The growing list of bytecode instructions emitted so far.
    self.instructions = {}

    -- The constant pool — literal values referenced by LOAD_CONST.
    self.constants = {}

    -- The name pool — variable/function names referenced by index.
    self.names = {}

    -- The current local variable scope, or nil if not inside a scope.
    self.scope = nil

    -- Handler dispatch table: rule_name → handler function.
    self._dispatch = {}

    -- Accumulated code objects from compile_nested calls.
    self._code_objects = {}

    return self
end


-- =========================================================================
-- Plugin registration
-- =========================================================================

--- Register a compile handler for a specific AST rule name.
-- This is how language plugins teach the compiler about their syntax.
-- If a handler was already registered for the same rule name, it is
-- silently replaced.
-- @param rule_name string The AST rule name to handle.
-- @param handler function A function(compiler, node) that compiles the node.
function GenericCompiler:register_rule(rule_name, handler)
    self._dispatch[rule_name] = handler
end


-- =========================================================================
-- Instruction emission
-- =========================================================================

--- Emit a single bytecode instruction and return its 0-based index.
--
-- Call with no operand for instructions like ADD, POP, HALT:
--   c:emit(bc.OpAdd)
--
-- Call with one operand for instructions like LOAD_CONST:
--   c:emit(bc.OpLoadConst, 0)
--
-- The returned index is useful for jump patching — you might emit a
-- JUMP_IF_FALSE now and patch its target later when you know where
-- the else-branch starts.
--
-- @param opcode number The opcode to emit.
-- @param operand any Optional operand.
-- @return number The 0-based index of the emitted instruction.
function GenericCompiler:emit(opcode, operand)
    local instr = bytecode_compiler.Instruction(opcode, operand)
    table.insert(self.instructions, instr)
    return #self.instructions - 1  -- 0-based index
end

--- Emit a jump instruction with a placeholder operand (0).
--
-- Jump instructions need a target address, but at emit time we often
-- don't know the target yet. The solution is backpatching:
--
--   1. emit_jump(opcode) — emit with operand=0 (placeholder).
--   2. patch_jump(index) — later, fill in the real target.
--
-- This two-step process is used by every real compiler.
--
-- @param opcode number The jump opcode (OpJump, OpJumpIfFalse, etc.).
-- @return number The 0-based index of the emitted jump instruction.
function GenericCompiler:emit_jump(opcode)
    return self:emit(opcode, 0)
end

--- Patch a previously emitted jump instruction with the real target.
--
-- If target is provided, the jump goes to that instruction index.
-- If omitted, the jump targets current_offset — the next instruction.
--
-- Example — compiling "if (cond) { then } else { else }":
--
--   compile_condition()
--   local jump_to_else = c:emit_jump(bc.OpJumpIfFalse)
--   compile_then_branch()
--   local jump_over_else = c:emit_jump(bc.OpJump)
--   c:patch_jump(jump_to_else)   -- else starts here
--   compile_else_branch()
--   c:patch_jump(jump_over_else) -- after else
--
-- @param index number The 0-based index of the jump instruction to patch.
-- @param target number|nil The target instruction index. Defaults to current_offset.
function GenericCompiler:patch_jump(index, target)
    -- Convert 0-based index to 1-based Lua index.
    local lua_index = index + 1
    if lua_index < 1 or lua_index > #self.instructions then
        error(string.format(
            "CompilerError: Cannot patch jump at index %d: instruction does not exist",
            index))
    end
    local t = target or self:current_offset()
    self.instructions[lua_index] = bytecode_compiler.Instruction(
        self.instructions[lua_index].opcode,
        t
    )
end

--- Return the 0-based index where the next emitted instruction will go.
-- Used for jump target calculations.
-- @return number The current instruction count (0-based next-index).
function GenericCompiler:current_offset()
    return #self.instructions
end


-- =========================================================================
-- Pool management — constants and names
-- =========================================================================

--- Add a constant to the constant pool, returning its 0-based index.
-- Constants are deduplicated: if the value already exists, the existing
-- index is returned.
-- @param value any The constant value.
-- @return number The 0-based index.
function GenericCompiler:add_constant(value)
    for i, v in ipairs(self.constants) do
        if v == value then
            return i - 1
        end
    end
    table.insert(self.constants, value)
    return #self.constants - 1
end

--- Add a name to the name pool, returning its 0-based index.
-- Names are deduplicated just like constants.
-- @param name string The variable or function name.
-- @return number The 0-based index.
function GenericCompiler:add_name(name)
    for i, n in ipairs(self.names) do
        if n == name then
            return i - 1
        end
    end
    table.insert(self.names, name)
    return #self.names - 1
end


-- =========================================================================
-- Scope management
-- =========================================================================

--- Push a new local variable scope. If params are provided, they are
-- pre-assigned to local slots (slot 0 for the first, slot 1 for the
-- second, etc.).
--
-- Scopes are linked: the new scope's parent points to the previous scope.
--
-- @param ... string Parameter names to pre-assign to slots.
-- @return CompilerScope The new scope.
function GenericCompiler:enter_scope(...)
    local params = { ... }
    local new_scope = CompilerScope.new(self.scope)
    for _, name in ipairs(params) do
        new_scope:add_local(name)
    end
    self.scope = new_scope
    return new_scope
end

--- Pop the current scope and restore the parent scope. Returns the scope
-- that was just exited, so the caller can inspect its num_locals or
-- other properties.
-- @return CompilerScope The exited scope.
function GenericCompiler:exit_scope()
    if self.scope == nil then
        error("CompilerError: Cannot exit scope: not currently inside a scope. " ..
              "Did you call exit_scope() without a matching enter_scope()?")
    end
    local exited = self.scope
    self.scope = exited.parent
    return exited
end


-- =========================================================================
-- Node compilation — the recursive dispatch engine
-- =========================================================================

--- Compile a nested code object (e.g., a function body).
--
-- Saves the compiler's current state, compiles the AST node into a fresh
-- code unit, then restores the original state. The nested code object is
-- returned and also stored in the code_objects list.
--
-- This is how real compilers handle functions-within-functions:
--   - CPython compiles each function body as a separate code object.
--   - The JVM compiles inner classes and lambdas as separate .class files.
--
-- @param node table An ASTNode to compile in isolation.
-- @return table A CodeObject for the nested code.
function GenericCompiler:compile_nested(node)
    -- Save current state.
    local saved_instructions = self.instructions
    local saved_constants = self.constants
    local saved_names = self.names

    -- Start fresh for the nested code unit.
    self.instructions = {}
    self.constants = {}
    self.names = {}

    -- Compile the nested AST.
    self:compile_node(node)

    -- Package the result.
    local code_object = bytecode_compiler.CodeObject(
        self.instructions,
        self.constants,
        self.names
    )

    -- Store for later reference.
    table.insert(self._code_objects, code_object)

    -- Restore outer state.
    self.instructions = saved_instructions
    self.constants = saved_constants
    self.names = saved_names

    return code_object
end

--- Compile a single AST node or token node. This is the main dispatch
-- method — the recursive heart of the compiler.
--
-- The decision tree:
--
--   1. TokenNode (leaf): Call compile_token(), which is a no-op by default.
--   2. ASTNode with a registered handler: Call the handler.
--   3. ASTNode with one child and no handler: Pass through to the child.
--   4. ASTNode with multiple children and no handler: Error.
--
-- The pass-through behavior (case 3) is important. In a real grammar,
-- many rules exist purely for precedence or grouping:
--
--   expression -> comparison -> addition -> multiplication -> primary
--
-- When parsing "42", all of these rules fire, each producing a single-
-- child node. The pass-through rule means we don't need handlers for
-- these "wrapper" rules.
--
-- @param node table An ASTNode or TokenNode table.
function GenericCompiler:compile_node(node)
    -- Case 1: Token nodes (leaves) are handled by compile_token.
    if node.node_kind == "token" then
        self:compile_token(node)
        return
    end

    -- Must be an ASTNode.
    if node.node_kind ~= "ast" then
        error(string.format(
            "CompilerError: compile_node received unexpected node_kind %q",
            tostring(node.node_kind)))
    end

    -- Case 2: Look for a registered handler for this rule name.
    local handler = self._dispatch[node.rule_name]
    if handler then
        handler(self, node)
        return
    end

    -- Case 3: No handler, but single child — pass through.
    if #node.children == 1 then
        self:compile_node(node.children[1])
        return
    end

    -- Case 4: No handler and multiple (or zero) children — error.
    error(string.format(
        "UnhandledRuleError: No handler registered for rule %q and node has %d children. " ..
        "Register a handler with compiler:register_rule(%q, handler).",
        node.rule_name, #node.children, node.rule_name))
end

--- Compile a token node. By default, this is a no-op — tokens are
-- typically handled by their parent ASTNode's handler, which knows the
-- context (is this number a literal? is this identifier a variable
-- reference? a function name?).
-- @param token table A TokenNode table.
function GenericCompiler:compile_token(token)
    -- No-op by default. Tokens are consumed by their parent node's handler.
    _ = token
end


-- =========================================================================
-- Top-level compilation
-- =========================================================================

--- Compile an entire AST into a CodeObject. This is the main entry point.
--
-- It:
--   1. Compiles the root AST node (recursively compiling all children).
--   2. Appends a HALT instruction (or a custom halt opcode).
--   3. Returns a self-contained CodeObject.
--
-- @param ast table An ASTNode (the root of the parse tree).
-- @param halt_opcode number|nil Optional halt opcode. Defaults to OpHalt (0xFF).
-- @return table A CodeObject with instructions, constants, and names.
function GenericCompiler:compile(ast, halt_opcode)
    -- Compile the entire tree, emitting instructions as we go.
    self:compile_node(ast)

    -- Determine the halt opcode — default is OpHalt (0xFF).
    local halt = halt_opcode or bytecode_compiler.OpHalt

    -- Append the halt instruction. Every program must end with HALT,
    -- just like every real CPU program must eventually stop.
    self:emit(halt)

    -- Package everything into a self-contained CodeObject.
    return bytecode_compiler.CodeObject(
        self.instructions,
        self.constants,
        self.names
    )
end

bytecode_compiler.GenericCompiler = GenericCompiler


return bytecode_compiler
