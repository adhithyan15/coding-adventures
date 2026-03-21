defmodule CodingAdventures.VirtualMachine.Errors do
  @moduledoc """
  Custom exception types for virtual machine errors.

  ## Why Custom Exceptions?

  When something goes wrong inside a VM, generic error messages like
  "something broke" are useless. Each exception type here represents a
  specific category of failure, making it easy to:

  - **Pattern match** on the error type in rescue/catch blocks
  - **Provide clear messages** that explain what went wrong and why
  - **Distinguish** between programmer errors (invalid opcode) and
    runtime errors (stack underflow, division by zero)

  ## Exception Hierarchy

  All VM exceptions use Elixir's `defexception` macro, which means they
  are proper structs with a `message` field and they integrate with
  Elixir's standard error handling (`raise`, `rescue`, `catch`).

      try do
        GenericVM.pop(empty_vm)
      rescue
        e in StackUnderflowError ->
          IO.puts("Oops: \#{e.message}")
      end

  ## Error Types

  | Exception             | When it happens                                      |
  |-----------------------|------------------------------------------------------|
  | VMError               | Generic VM error (catch-all)                         |
  | StackUnderflowError   | Pop/peek on an empty stack                           |
  | UndefinedNameError    | Variable or function name not found                  |
  | DivisionByZeroError   | Division or modulo by zero                           |
  | InvalidOpcodeError    | Instruction with no registered handler               |
  | InvalidOperandError   | Instruction operand is wrong type or out of range     |
  | VMTypeError           | Type mismatch (e.g., adding a string to a number)    |
  | MaxRecursionError     | Call stack depth exceeds the configured limit         |
  """

  # ---------------------------------------------------------------------------
  # VMError — the generic catch-all
  # ---------------------------------------------------------------------------

  defmodule VMError do
    @moduledoc """
    Generic virtual machine error.

    Used for errors that do not fit a more specific category, such as
    attempting to return from an empty call stack or executing in an
    invalid VM state.
    """
    defexception message: "An error occurred in the virtual machine."
  end

  # ---------------------------------------------------------------------------
  # StackUnderflowError
  # ---------------------------------------------------------------------------

  defmodule StackUnderflowError do
    @moduledoc """
    Raised when trying to pop or peek at an empty stack.

    ## What causes this?

    A stack underflow means the program tried to consume a value that
    does not exist. Common causes:

    - An ADD instruction when fewer than two values are on the stack
    - A POP instruction on an empty stack
    - A function that returns a value but the caller did not push arguments

    In a well-formed program, every pop is balanced by a prior push.
    A stack underflow indicates a bug in the compiled bytecode.
    """
    defexception message: "Stack underflow — cannot pop from an empty stack."
  end

  # ---------------------------------------------------------------------------
  # UndefinedNameError
  # ---------------------------------------------------------------------------

  defmodule UndefinedNameError do
    @moduledoc """
    Raised when a variable or function name is not found.

    This is the bytecode equivalent of Python's `NameError` or
    JavaScript's `ReferenceError`. It means the program tried to
    read a variable that was never assigned, or call a function
    that was never defined.
    """
    defexception message: "Undefined name — variable or function not found."
  end

  # ---------------------------------------------------------------------------
  # DivisionByZeroError
  # ---------------------------------------------------------------------------

  defmodule DivisionByZeroError do
    @moduledoc """
    Raised when dividing or taking modulo by zero.

    Division by zero is undefined in mathematics and produces an error in
    virtually every programming language. The VM catches it explicitly
    rather than letting the BEAM crash with an arithmetic error, so the
    error message is clear and VM-specific.
    """
    defexception message: "Division by zero."
  end

  # ---------------------------------------------------------------------------
  # InvalidOpcodeError
  # ---------------------------------------------------------------------------

  defmodule InvalidOpcodeError do
    @moduledoc """
    Raised when the VM encounters an instruction with no registered handler.

    This means the bytecode contains an opcode number that the VM does not
    know how to execute. Possible causes:

    - Corrupted bytecode
    - Bytecode compiled for a different VM version
    - A plugin was not registered before execution

    The GenericVM's pluggable design means opcodes must be explicitly
    registered via `register_opcode/3` before they can be used.
    """
    defexception message: "Invalid opcode — no handler registered."
  end

  # ---------------------------------------------------------------------------
  # InvalidOperandError
  # ---------------------------------------------------------------------------

  defmodule InvalidOperandError do
    @moduledoc """
    Raised when an instruction's operand is invalid.

    Examples:
    - LOAD_CONST with an operand index that exceeds the constants pool
    - JUMP with a negative target address
    - An instruction that requires an operand but has `nil`
    """
    defexception message: "Invalid operand for instruction."
  end

  # ---------------------------------------------------------------------------
  # VMTypeError
  # ---------------------------------------------------------------------------

  defmodule VMTypeError do
    @moduledoc """
    Raised when an operation encounters incompatible types.

    For example, trying to ADD a string and a number, or using a
    non-boolean value in a conditional jump. This is the VM's equivalent
    of a type error — the values exist, but they cannot be combined
    in the way the instruction requires.
    """
    defexception message: "Type error in virtual machine operation."
  end

  # ---------------------------------------------------------------------------
  # MaxRecursionError
  # ---------------------------------------------------------------------------

  defmodule MaxRecursionError do
    @moduledoc """
    Raised when the call stack exceeds the configured maximum depth.

    ## Why limit recursion?

    Without a limit, an infinite recursion (function A calls A calls A...)
    would consume memory until the system crashes. By setting a maximum
    recursion depth, the VM detects runaway recursion early and raises
    a clear error instead of crashing mysteriously.

    This is analogous to Python's `RecursionError` (default limit: 1000)
    or a stack overflow in C/C++.
    """
    defexception message: "Maximum recursion depth exceeded."
  end
end
