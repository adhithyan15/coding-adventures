defmodule CodingAdventures.StarlarkAstToBytecodeCompiler.Opcodes do
  @moduledoc """
  Starlark Opcodes — The instruction set for the Starlark virtual machine.

  ## Chapter 1: Why Starlark Has Its Own Opcodes

  The GenericVM is a blank slate — it has no built-in opcodes. Languages
  register their own opcodes via `GenericVM.register_opcode/3`. This module
  defines the opcode *numbers* and *names* for Starlark.

  These opcodes are Starlark's "machine language." The compiler translates
  Starlark source code into sequences of these opcodes, and the VM executes
  them.

  ## Chapter 2: Opcode Organization

  Opcodes are grouped by category using the high nibble (first hex digit):

      0x0_ = Stack operations      (push, pop, dup, load constants)
      0x1_ = Variable operations   (store/load by name or slot)
      0x2_ = Arithmetic            (add, sub, mul, div, bitwise)
      0x3_ = Comparison & boolean  (==, !=, <, >, in, not)
      0x4_ = Control flow          (jump, branch)
      0x5_ = Functions             (make, call, return)
      0x6_ = Collections           (build list, dict, tuple)
      0x7_ = Subscript & attribute (indexing, slicing, dot access)
      0x8_ = Iteration             (get_iter, for_iter, unpack)
      0x9_ = Module                (load statement)
      0xA_ = I/O                   (print)
      0xF_ = VM control            (halt)

  This grouping mirrors the JVM's organization and makes it easy to tell
  an instruction's category at a glance from its hex value.

  ## Chapter 3: How to Use These Constants

  Each opcode is a module attribute (constant). Use them when emitting
  bytecode instructions:

      alias CodingAdventures.StarlarkAstToBytecodeCompiler.Opcodes, as: Op

      # Emit a LOAD_CONST instruction
      GenericCompiler.emit(compiler, Op.load_const(), 0)

      # Check what category an opcode belongs to
      opcode = Op.add()  # 0x20 — arithmetic category
  """

  # ===========================================================================
  # Stack Operations (0x0_)
  # ===========================================================================
  #
  # Stack operations manage the operand stack — the VM's primary workspace.
  # Values are pushed onto the stack, operated on, and results pushed back.

  @doc "Push a constant from the pool. Operand: pool index. -> value"
  def load_const, do: 0x01

  @doc "Discard top of stack. value ->"
  def pop, do: 0x02

  @doc "Duplicate top of stack. value -> value value"
  def dup, do: 0x03

  @doc "Push nil (None). -> nil"
  def load_none, do: 0x04

  @doc "Push true. -> true"
  def load_true, do: 0x05

  @doc "Push false. -> false"
  def load_false, do: 0x06

  # ===========================================================================
  # Variable Operations (0x1_)
  # ===========================================================================
  #
  # Variable operations move values between the stack and named storage.
  # There are three kinds of storage:
  # - Names: global/module-level variables (looked up by string name)
  # - Locals: function-local variables (looked up by slot index, faster)
  # - Closures: variables captured from enclosing scopes (looked up by cell index)

  @doc "Pop and store in named variable. Operand: name index. value ->"
  def store_name, do: 0x10

  @doc "Push named variable's value. Operand: name index. -> value"
  def load_name, do: 0x11

  @doc "Pop and store in local slot. Operand: slot index. value ->"
  def store_local, do: 0x12

  @doc "Push local slot's value. Operand: slot index. -> value"
  def load_local, do: 0x13

  @doc "Pop and store in closure cell. Operand: cell index. value ->"
  def store_closure, do: 0x14

  @doc "Push closure cell's value. Operand: cell index. -> value"
  def load_closure, do: 0x15

  # ===========================================================================
  # Arithmetic Operations (0x2_)
  # ===========================================================================
  #
  # Arithmetic operations pop operands from the stack, perform the operation,
  # and push the result back. Binary ops pop two values (a and b where b is
  # on top), unary ops pop one.

  @doc "Pop two values, push a + b. Supports int, float, str, list. a b -> result"
  def add, do: 0x20

  @doc "Pop two values, push a - b. a b -> result"
  def sub, do: 0x21

  @doc "Pop two values, push a * b. Also handles str * int. a b -> result"
  def mul, do: 0x22

  @doc "Pop two values, push a / b (float division). a b -> result"
  def div_op, do: 0x23

  @doc "Pop two values, push a // b (floor division). a b -> result"
  def floor_div, do: 0x24

  @doc "Pop two values, push a % b. Also handles str formatting. a b -> result"
  def mod, do: 0x25

  @doc "Pop two values, push a ** b. a b -> result"
  def power, do: 0x26

  @doc "Pop one value, push -a. a -> -a"
  def negate, do: 0x27

  @doc "Pop two values, push a & b. a b -> result"
  def bit_and, do: 0x28

  @doc "Pop two values, push a | b. a b -> result"
  def bit_or, do: 0x29

  @doc "Pop two values, push a ^ b. a b -> result"
  def bit_xor, do: 0x2A

  @doc "Pop one value, push ~a. a -> ~a"
  def bit_not, do: 0x2B

  @doc "Pop two values, push a << b. a b -> result"
  def lshift, do: 0x2C

  @doc "Pop two values, push a >> b. a b -> result"
  def rshift, do: 0x2D

  # ===========================================================================
  # Comparison Operations (0x3_)
  # ===========================================================================
  #
  # Comparison operations pop two values and push a boolean result.
  # They follow Starlark's comparison semantics (similar to Python).

  @doc "Pop two values, push a == b. a b -> bool"
  def cmp_eq, do: 0x30

  @doc "Pop two values, push a != b. a b -> bool"
  def cmp_ne, do: 0x31

  @doc "Pop two values, push a < b. a b -> bool"
  def cmp_lt, do: 0x32

  @doc "Pop two values, push a > b. a b -> bool"
  def cmp_gt, do: 0x33

  @doc "Pop two values, push a <= b. a b -> bool"
  def cmp_le, do: 0x34

  @doc "Pop two values, push a >= b. a b -> bool"
  def cmp_ge, do: 0x35

  @doc "Pop two values, push a in b. a b -> bool"
  def cmp_in, do: 0x36

  @doc "Pop two values, push a not in b. a b -> bool"
  def cmp_not_in, do: 0x37

  # ===========================================================================
  # Boolean Operations (0x38)
  # ===========================================================================

  @doc "Pop one value, push logical not. a -> !a"
  def logical_not, do: 0x38

  # ===========================================================================
  # Control Flow (0x4_)
  # ===========================================================================
  #
  # Control flow operations change the program counter (PC) to alter the
  # order of execution. They implement if/else, loops, and short-circuit
  # boolean evaluation.

  @doc "Unconditional jump. Operand: target index."
  def jump, do: 0x40

  @doc "Pop value, jump if falsy. Operand: target. value ->"
  def jump_if_false, do: 0x41

  @doc "Pop value, jump if truthy. Operand: target. value ->"
  def jump_if_true, do: 0x42

  @doc """
  If top is falsy, jump (keep value); else pop. For `and` short-circuit.
  Operand: target. value -> value? (if jump) or -> (if no jump)
  """
  def jump_if_false_or_pop, do: 0x43

  @doc """
  If top is truthy, jump (keep value); else pop. For `or` short-circuit.
  Operand: target. value -> value? (if jump) or -> (if no jump)
  """
  def jump_if_true_or_pop, do: 0x44

  # ===========================================================================
  # Function Operations (0x5_)
  # ===========================================================================
  #
  # Function operations handle creating function objects, calling them,
  # and returning from them.

  @doc "Create a function object. Operand: flags. code defaults -> func"
  def make_function, do: 0x50

  @doc "Call function with N positional args. Operand: arg count. func args -> result"
  def call_function, do: 0x51

  @doc "Call function with keyword args. Operand: total arg count. func args kw_names -> result"
  def call_function_kw, do: 0x52

  @doc "Return from function. value ->"
  def return_op, do: 0x53

  # ===========================================================================
  # Collection Operations (0x6_)
  # ===========================================================================
  #
  # Collection operations create and modify lists, dicts, and tuples.

  @doc "Create list from N stack items. Operand: count. items -> list"
  def build_list, do: 0x60

  @doc "Create dict from N key-value pairs. Operand: pair count. k1 v1 k2 v2 ... -> dict"
  def build_dict, do: 0x61

  @doc "Create tuple from N stack items. Operand: count. items -> tuple"
  def build_tuple, do: 0x62

  @doc "Append value to list (for comprehensions). list value -> list"
  def list_append, do: 0x63

  @doc "Set dict entry (for comprehensions). dict key value -> dict"
  def dict_set, do: 0x64

  # ===========================================================================
  # Subscript & Attribute Operations (0x7_)
  # ===========================================================================
  #
  # These operations handle indexing (obj[key]), attribute access (obj.attr),
  # and slicing (obj[start:stop:step]).

  @doc "obj[key]. obj key -> value"
  def load_subscript, do: 0x70

  @doc "obj[key] = value. obj key value ->"
  def store_subscript, do: 0x71

  @doc "obj.attr. Operand: attr name index. obj -> value"
  def load_attr, do: 0x72

  @doc "obj.attr = value. Operand: attr name index. obj value ->"
  def store_attr, do: 0x73

  @doc "obj[start:stop:step]. Operand: flags for which are present. obj start? stop? step? -> value"
  def load_slice, do: 0x74

  # ===========================================================================
  # Iteration Operations (0x8_)
  # ===========================================================================
  #
  # Iteration operations implement for loops and sequence unpacking.

  @doc "Get iterator from iterable. iterable -> iterator"
  def get_iter, do: 0x80

  @doc """
  Get next from iterator, or jump to end. Operand: target.
  iterator -> iterator value (if has next)
  iterator -> (if exhausted, jumps to target)
  """
  def for_iter, do: 0x81

  @doc "Unpack N items from sequence. Operand: count. seq -> items"
  def unpack_sequence, do: 0x82

  # ===========================================================================
  # Module Operations (0x9_)
  # ===========================================================================
  #
  # Module operations implement the load() statement for importing symbols
  # from other Starlark files.

  @doc "Load a module (for load() statement). Operand: module name index. -> module"
  def load_module, do: 0x90

  @doc "Extract symbol from module. Operand: symbol name index. module -> value"
  def import_from, do: 0x91

  # ===========================================================================
  # I/O Operations (0xA_)
  # ===========================================================================

  @doc "Pop and print value, capture in output. value ->"
  def print_op, do: 0xA0

  # ===========================================================================
  # VM Control (0xF_)
  # ===========================================================================

  @doc "Stop execution."
  def halt, do: 0xFF

  # ===========================================================================
  # Operator-to-Opcode Mappings
  # ===========================================================================
  #
  # These maps are used by the compiler when it encounters operator expressions.
  # Instead of a big case statement, the compiler looks up the operator symbol
  # in the appropriate map to find the corresponding opcode.

  @doc """
  Maps binary operator symbols to their bytecode opcodes.

  Used by the compiler when it encounters `arith`, `term`, `shift`,
  or other binary-expression grammar rules.
  """
  def binary_op_map do
    %{
      "+" => add(),
      "-" => sub(),
      "*" => mul(),
      "/" => div_op(),
      "//" => floor_div(),
      "%" => mod(),
      "**" => power(),
      "&" => bit_and(),
      "|" => bit_or(),
      "^" => bit_xor(),
      "<<" => lshift(),
      ">>" => rshift()
    }
  end

  @doc "Maps comparison operator symbols to their bytecode opcodes."
  def compare_op_map do
    %{
      "==" => cmp_eq(),
      "!=" => cmp_ne(),
      "<" => cmp_lt(),
      ">" => cmp_gt(),
      "<=" => cmp_le(),
      ">=" => cmp_ge(),
      "in" => cmp_in(),
      "not in" => cmp_not_in()
    }
  end

  @doc "Maps augmented assignment operators to their underlying arithmetic opcodes."
  def augmented_assign_map do
    %{
      "+=" => add(),
      "-=" => sub(),
      "*=" => mul(),
      "/=" => div_op(),
      "//=" => floor_div(),
      "%=" => mod(),
      "&=" => bit_and(),
      "|=" => bit_or(),
      "^=" => bit_xor(),
      "<<=" => lshift(),
      ">>=" => rshift(),
      "**=" => power()
    }
  end

  @doc """
  Maps unary operator symbols to their bytecode opcodes.

  Note: unary `+` doesn't have a dedicated opcode. It evaluates the
  expression (for type checking) but doesn't change the value.
  """
  def unary_op_map do
    %{
      "-" => negate(),
      "+" => pop(),   # unary + is a no-op on valid numeric types
      "~" => bit_not()
    }
  end

  @doc "Returns a list of all 46 opcode values."
  def all_opcodes do
    [
      load_const(), pop(), dup(), load_none(), load_true(), load_false(),
      store_name(), load_name(), store_local(), load_local(),
      store_closure(), load_closure(),
      add(), sub(), mul(), div_op(), floor_div(), mod(), power(), negate(),
      bit_and(), bit_or(), bit_xor(), bit_not(), lshift(), rshift(),
      cmp_eq(), cmp_ne(), cmp_lt(), cmp_gt(), cmp_le(), cmp_ge(),
      cmp_in(), cmp_not_in(),
      logical_not(),
      jump(), jump_if_false(), jump_if_true(),
      jump_if_false_or_pop(), jump_if_true_or_pop(),
      make_function(), call_function(), call_function_kw(), return_op(),
      build_list(), build_dict(), build_tuple(), list_append(), dict_set(),
      load_subscript(), store_subscript(), load_attr(), store_attr(), load_slice(),
      get_iter(), for_iter(), unpack_sequence(),
      load_module(), import_from(),
      print_op(),
      halt()
    ]
  end
end
