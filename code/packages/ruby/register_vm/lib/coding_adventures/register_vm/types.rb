# frozen_string_literal: true

# ==========================================================================
# Types -- Data Structures for the Register VM
# ==========================================================================
#
# Ruby offers two ways to build simple value objects:
#
#   Struct.new(...)   — mutable, flexible, classic Ruby idiom
#   Data.define(...)  — immutable, value-semantic, introduced in Ruby 3.2
#
# We use Struct for things that change during execution (CallFrame, VMObject,
# Context) and simple POJOs where mutation is expected. We use Data for
# things that are true immutable records (here we keep Struct for callframes
# so they can be updated in-place without building a new object each step).
#
# JavaScript type system overview (this VM mimics JS semantics):
#
#   Type        Ruby representation    Notes
#   ----------  ---------------------  ------------------------------------
#   number      Integer or Float       No distinction between int / float
#   string      String                 Immutable in JS; mutable in Ruby
#   boolean     TrueClass / FalseClass
#   null        NilClass               JS null
#   undefined   UNDEFINED sentinel     Distinct from null
#   object      VMObject               { properties: Hash, hidden_class: id }
#   array       Array                  Plain Ruby array (no hidden class)
#   function    VMFunction             { code: CodeObject, context: Context }
#
# The UNDEFINED sentinel (below) is a module-level frozen object — not nil,
# not false — so it can be tested with `obj.equal?(UNDEFINED)`.
#
module CodingAdventures
  module RegisterVM
    # -----------------------------------------------------------------------
    # UNDEFINED — the "undefined" value in JS-land
    # -----------------------------------------------------------------------
    # In JavaScript, `undefined` and `null` are distinct:
    #   typeof undefined === "undefined"
    #   typeof null      === "object"    (famous JS footgun!)
    #
    # We represent `undefined` as a unique singleton object (not nil) so that
    # `registers` that haven't been written yet contain UNDEFINED, not nil.
    # Build and configure the sentinel before freezing so we can add singleton
    # methods. In Ruby 3.3+, calling def on an already-frozen object raises
    # FrozenError, so the methods must be defined while the object is still mutable.
    _undef = Object.new
    def _undef.inspect
      "undefined"
    end

    def _undef.to_s
      "undefined"
    end

    UNDEFINED = _undef.freeze

    # -----------------------------------------------------------------------
    # VMObject — a JS-style Object with hidden class tracking
    # -----------------------------------------------------------------------
    # Hidden classes (also called "shapes" or "maps") are V8's key optimization:
    #
    #   const a = { x: 1 };        // hidden class H0: { x }
    #   const b = { x: 2 };        // also H0 — same shape!
    #   a.y = 3;                   // transition to H1: { x, y }
    #
    # When two objects share a hidden class, the JIT can generate a single
    # fast-path that works for both without looking up the hash. Here we just
    # track the ID; actually transitioning classes on property addition is
    # left as a future exercise.
    VMObject = Struct.new(:hidden_class_id, :properties, keyword_init: true) do
      # Default the properties hash to an empty Hash so callers can skip it.
      def initialize(hidden_class_id:, properties: {})
        super(hidden_class_id: hidden_class_id, properties: properties)
      end
    end

    # -----------------------------------------------------------------------
    # VMFunction — a first-class function value (closure)
    # -----------------------------------------------------------------------
    # A function is two things:
    #   1. A CodeObject  — the compiled instructions + metadata
    #   2. A Context     — the captured lexical scope (the "closure" part)
    #
    # When CALL is executed, we create a new CallFrame using the function's
    # CodeObject and inherit its captured Context as the parent scope.
    VMFunction = Struct.new(:code, :context, keyword_init: true)

    # -----------------------------------------------------------------------
    # CodeObject — compiled bytecode for one function
    # -----------------------------------------------------------------------
    # Maps closely to V8's SharedFunctionInfo + BytecodeArray:
    #
    #   instructions      Array of RegisterInstruction
    #   constants         Array of literal values (numbers, strings, …)
    #   names             Array of name strings (variable names, property keys)
    #   register_count    How many registers this frame needs
    #   feedback_slot_count  Size of the feedback vector
    #   parameter_count   Number of formal parameters
    #   name              Debug name (shown in stack traces)
    CodeObject = Struct.new(
      :instructions,
      :constants,
      :names,
      :register_count,
      :feedback_slot_count,
      :parameter_count,
      :name,
      keyword_init: true
    ) do
      def initialize(instructions:, constants:, names:,
                     register_count:, feedback_slot_count:,
                     parameter_count: 0, name: "anonymous")
        super
      end
    end

    # -----------------------------------------------------------------------
    # RegisterInstruction — one bytecode instruction
    # -----------------------------------------------------------------------
    # Each instruction has:
    #   opcode        — Integer byte (from Opcodes module)
    #   operands      — Array of Integer operands (register indices, etc.)
    #   feedback_slot — Optional Integer index into the frame's feedback vector
    #
    # Example — the instruction "ADD r2" with feedback tracking:
    #   RegisterInstruction.new(
    #     opcode: Opcodes::ADD, operands: [2], feedback_slot: 0
    #   )
    RegisterInstruction = Struct.new(:opcode, :operands, :feedback_slot, keyword_init: true) do
      def initialize(opcode:, operands: [], feedback_slot: nil)
        super
      end
    end

    # -----------------------------------------------------------------------
    # CallFrame — the execution context of one function invocation
    # -----------------------------------------------------------------------
    # Analogous to a stack frame in a native call stack. Fields:
    #
    #   code            CodeObject being executed
    #   ip              Instruction pointer (current index into instructions)
    #   accumulator     The V8-style accumulator register
    #   registers       Array of register values (size = code.register_count)
    #   feedback_vector Array of feedback slot states (size = code.feedback_slot_count)
    #   context         Scope chain head (innermost lexical Context)
    #   caller_frame    The frame that called into this one (nil for top-level)
    CallFrame = Struct.new(
      :code,
      :ip,
      :accumulator,
      :registers,
      :feedback_vector,
      :context,
      :caller_frame,
      keyword_init: true
    )

    # -----------------------------------------------------------------------
    # VMResult — the outcome of running a CodeObject to completion
    # -----------------------------------------------------------------------
    # After execute() returns:
    #   return_value    The final accumulator value
    #   output          Array of Strings emitted by PRINT instructions
    #   error           A VMError if execution raised one, else nil
    VMResult = Struct.new(:return_value, :output, :error, keyword_init: true)

    # -----------------------------------------------------------------------
    # VMError — a runtime error raised by the interpreter
    # -----------------------------------------------------------------------
    # Carries the instruction index and opcode where the fault occurred so
    # error messages can point to the exact bytecode location.
    class VMError < StandardError
      attr_reader :instruction_index, :opcode

      def initialize(message, instruction_index: 0, opcode: 0)
        super(message)
        @instruction_index = instruction_index
        @opcode = opcode
      end
    end

    # -----------------------------------------------------------------------
    # Context — one level of the lexical scope chain
    # -----------------------------------------------------------------------
    # The scope chain is a linked list of Contexts:
    #
    #   global context
    #     └─ function context
    #           └─ block context  (innermost, head of chain)
    #
    # Each Context has:
    #   slots   — Array of values, one per local variable in this scope
    #   parent  — The enclosing Context (nil at the global scope)
    #
    # Variable lookup by (depth, idx) walks `depth` parent pointers then
    # reads/writes slots[idx]. This is how V8 resolves free variables in
    # closures without a heap-allocated dictionary.
    Context = Struct.new(:slots, :parent, keyword_init: true)

    # -----------------------------------------------------------------------
    # TraceStep — one recorded step from execute_with_trace
    # -----------------------------------------------------------------------
    # Useful for visualizers, debuggers, and test assertions. Records the
    # full interpreter state before and after each instruction.
    TraceStep = Struct.new(
      :ip,
      :opcode_name,
      :operands,
      :accumulator_before,
      :accumulator_after,
      :registers_snapshot,
      keyword_init: true
    )

    # -----------------------------------------------------------------------
    # Feedback slot state notes (stored inline as Ruby objects):
    # -----------------------------------------------------------------------
    #
    # :uninitialized
    #   The slot has never been observed. The JIT should not specialize yet.
    #
    # { kind: :monomorphic, types: [["number", "number"]] }
    #   Only one type pair has been observed. The JIT can emit a fast path.
    #
    # { kind: :polymorphic, types: [["number", "number"], ["string", "string"]] }
    #   2–4 different type pairs. The JIT can emit a small dispatch table.
    #
    # :megamorphic
    #   More than 4 type pairs. The JIT gives up and falls back to slow path.
    #
    # This mirrors V8's IC (inline cache) state machine:
    #   uninitialized → monomorphic → polymorphic → megamorphic
    # Each transition is irreversible (deoptimization aside).
  end
end
