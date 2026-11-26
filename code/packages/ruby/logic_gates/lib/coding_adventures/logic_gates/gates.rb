# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Logic Gates -- the foundation of all digital computing.
# ---------------------------------------------------------------------------
#
# === What is a logic gate? ===
#
# A logic gate is the simplest possible decision-making element. It takes
# one or two inputs, each either 0 or 1, and produces a single output
# that is also 0 or 1. The output is entirely determined by the inputs --
# there is no randomness, no hidden state, no memory.
#
# In physical hardware, gates are built from transistors -- tiny electronic
# switches etched into silicon. A modern CPU contains billions of transistors
# organized into billions of gates. But conceptually, every computation a
# computer performs -- from adding numbers to rendering video to running AI
# models -- ultimately reduces to combinations of these simple 0-or-1
# operations.
#
# This module implements the seven fundamental gates, proves that all of them
# can be built from a single gate type (NAND), and provides multi-input
# variants.
#
# === Why only 0 and 1? ===
#
# Computers use binary (base-2) because transistors are most reliable as
# on/off switches. A transistor that is "on" (conducting electricity)
# represents 1. A transistor that is "off" (blocking electricity) represents
# 0. You could theoretically build a computer using base-3 or base-10, but
# the error margins for distinguishing between voltage levels would make it
# unreliable. Binary gives us two clean, easily distinguishable states.
#
# === Ruby Implementation Notes ===
#
# Unlike Python, Ruby's booleans (true/false) are NOT integers and are NOT
# subclasses of Integer. This simplifies our validation -- we only need to
# check that the value is an Integer and that it equals 0 or 1. Anything
# else (true, false, nil, floats, strings, or an Integer outside {0, 1})
# raises InvalidBitError, a subclass of ArgumentError, that names the
# offending parameter.
#
# All methods are defined as module_function so they can be called directly:
#   CodingAdventures::LogicGates.and_gate(1, 0)
#
# We use snake_case method names with _gate suffix for the basic gates
# (since AND, OR, NOT are Ruby reserved words), and also provide constant-
# style aliases via class methods where possible.
# ---------------------------------------------------------------------------

module CodingAdventures
  module LogicGates
    # -----------------------------------------------------------------------
    # CMOS gate instances — shared, allocated once at module load time.
    # -----------------------------------------------------------------------
    # Each of the seven primitive gates delegates its digital evaluation to
    # a CMOS transistor model from CodingAdventures::Transistors. Using a
    # module-level constant avoids allocating a new object on every call
    # while still exercising the full transistor simulation.
    #
    # Default circuit parameters: 3.3 V Vdd, 180 nm CMOS process node.
    CMOS_INVERTER = CodingAdventures::Transistors::CMOSInverter.new
    CMOS_NAND = CodingAdventures::Transistors::CMOSNand.new
    CMOS_NOR = CodingAdventures::Transistors::CMOSNor.new
    CMOS_AND = CodingAdventures::Transistors::CMOSAnd.new
    CMOS_OR = CodingAdventures::Transistors::CMOSOr.new
    CMOS_XOR = CodingAdventures::Transistors::CMOSXor.new
    CMOS_XNOR = CodingAdventures::Transistors::CMOSXnor.new

    # Deep-freeze every gate model so the constants are immutable and
    # Ractor-shareable (a gate can then run inside a non-main Ractor).
    [CMOS_INVERTER, CMOS_NAND, CMOS_NOR, CMOS_AND, CMOS_OR, CMOS_XOR, CMOS_XNOR]
      .each { |g| Ractor.make_shareable(g) }

    # -----------------------------------------------------------------------
    # Input validation (LG02 sections 7 and 9)
    # -----------------------------------------------------------------------
    # A bit is valid if and only if it is an Integer whose value is 0 or 1.
    #
    # Ruby makes the "exact type" half of that rule easy: true, false and nil
    # are not Integers, and Integer cannot be meaningfully subclassed (its
    # instances are immediate values with no allocator), so `Integer === v`
    # is an exact test. We use `Integer === v` rather than `v.class == Integer`
    # on purpose: Module#=== inspects the real class and never calls a method
    # on the value, so an object that overrides `class`, `is_a?`, `==`,
    # `to_int` or `coerce` cannot talk its way past the check. 1.0, Rational(1),
    # Complex(1, 0), BigDecimal("1"), "1", :one, [1] and {1 => 1} all fail it.
    #
    # Every rejection raises ONE identity, InvalidBitError, so a caller has a
    # single thing to rescue. (Earlier versions raised TypeError for a wrong
    # type and ArgumentError for a wrong value; InvalidBitError < ArgumentError,
    # so `rescue ArgumentError` still works, but `rescue TypeError` no longer
    # catches a rejected bit. That is the breaking change of 0.4.0.)

    # Raised when a gate argument is not the Integer 0 or the Integer 1.
    #
    # @!attribute [r] param
    #   @return [String] the name of the offending parameter ("a", "b", ...)
    # @!attribute [r] value_description
    #   @return [String] a bounded description of the value: the integer
    #     itself for an Integer of at most 64 bits ("an Integer of N bits"
    #     beyond that), otherwise a fixed literal ("a String", "a value of
    #     another type"), never the caller's class name. The value's own
    #     #to_s / #inspect is never called, because for an arbitrary object
    #     that could run hostile code or produce unbounded output.
    class InvalidBitError < ArgumentError
      attr_reader :param, :value_description

      def initialize(param, value_description)
        @param = param
        @value_description = value_description
        super("#{param} must be the Integer 0 or 1, got #{value_description}")
      end
    end

    # Most inputs an n-ary gate (and_n / or_n) accepts. Checked before any
    # validation or gate work so a huge argument list costs O(1) to refuse.
    MAX_INPUTS = 65_536

    # Unbound primitives used for identity checks. A caller-controlled class
    # object (reached through any object's #class) can define singleton
    # methods such as `equal?`, so the receiver of every comparison must be
    # OUR class and the caller's class only ever an argument.
    BASIC_EQUAL = BasicObject.instance_method(:equal?)
    KERNEL_CLASS = Kernel.instance_method(:class)
    private_constant :BASIC_EQUAL, :KERNEL_CLASS

    # Copy a caller-supplied container WITHOUT calling any method on it.
    #
    # `Array === x` and `x.is_a?(Array)` accept subclasses, and every later
    # call (length, each, [], dup) would then dispatch to the caller's
    # overrides. So we ask the real class through Kernel#class (bound, never
    # the object's own) and accept only EXACT Array / Hash. The copy is made
    # with Array.new / Hash[], which on an exact core container run no user
    # code, and all later work uses the copy only. Returns nil if x is not an
    # exact Array (or Hash).
    def self.copy_array(x, cap)
      return nil unless BASIC_EQUAL.bind_call(Array, KERNEL_CLASS.bind_call(x))

      len = Array.instance_method(:length).bind_call(x)
      raise ArgumentError, "container length #{len} exceeds the limit of #{cap}" if len > cap

      Array.new(x)
    rescue TypeError
      nil # BasicObject: no #class to bind
    end

    # Hash variant. It must not hash or compare any caller key, so it never
    # re-inserts caller keys into a new Hash. Instead: bound Hash#keys (a fresh
    # Array, no hashing), every key must be a Symbol (Module#=== never
    # dispatches) and a member of the frozen `allowed` Symbol list, and the copy
    # is built by reading ONLY those keys through bound Hash#[] (each is known
    # present, so a default proc can never fire). Returns nil on any violation.
    def self.copy_hash(x, allowed)
      return nil unless BASIC_EQUAL.bind_call(Hash, KERNEL_CLASS.bind_call(x))

      return nil if Hash.instance_method(:size).bind_call(x) > allowed.size

      keys = Hash.instance_method(:keys).bind_call(x)
      reader = Hash.instance_method(:[])
      out = {}
      keys.each do |k|
        return nil unless Symbol === k && allowed.include?(k)
        out[k] = reader.bind_call(x, k)
      end
      out
    rescue TypeError
      nil
    end
    private_class_method :copy_array, :copy_hash

    # Fixed descriptions by exact built-in class identity. The caller's own
    # class name, strings and methods NEVER reach a message: a class name can
    # be set to anything (newlines, ANSI escapes, odd encodings) with
    # Module#set_temporary_name, so we only ever emit one of these literals.
    TYPE_DESCRIPTIONS = {
      NilClass => "nil",
      TrueClass => "a boolean",
      FalseClass => "a boolean",
      Float => "a Float",
      String => "a String",
      Symbol => "a Symbol",
      Array => "an Array",
      Hash => "a Hash",
      Rational => "a Rational",
      Complex => "a Complex"
    }.freeze
    private_constant :TYPE_DESCRIPTIONS

    # Describe a rejected value without invoking any method defined by the
    # value itself. Integers are echoed (up to 64 bits, else by bit length);
    # everything else maps, by exact class identity read through the bound
    # Kernel#class, to a fixed literal or "a value of another type".
    def self.describe_value(value)
      if Integer === value
        # Integer#to_s on a huge bignum is superlinear and builds a huge string,
        # so only integers up to 64 bits are echoed; larger ones are summarised.
        return (value.bit_length > 64) ? "an Integer of #{value.bit_length} bits" : value.to_s
      end

      klass = begin
        KERNEL_CLASS.bind_call(value)
      rescue TypeError
        return "a value of another type" # BasicObject has no #class to bind
      end
      TYPE_DESCRIPTIONS.each do |known, text|
        return text if BASIC_EQUAL.bind_call(known, klass)
      end
      "a value of another type"
    end
    private_class_method :describe_value

    # Ensure a value is a binary bit: the Integer 0 or the Integer 1.
    #
    # @param value [Object] the value to validate
    # @param name [String] the parameter name reported in the error
    # @raise [InvalidBitError] for anything else
    # @return [void]
    def self.validate_bit(value, name = "input")
      return if Integer === value && (value == 0 || value == 1)

      raise InvalidBitError.new(name, describe_value(value))
    end

    # Make validate_bit private -- it's an internal helper
    private_class_method :validate_bit

    # =====================================================================
    # THE FOUR FUNDAMENTAL GATES
    # =====================================================================
    # These are the building blocks. NOT, AND, OR, and XOR are the four
    # gates from which all other gates (and all of digital logic) can be
    # constructed.
    #
    # Each gate is defined by its "truth table" -- an exhaustive listing of
    # every possible input combination and the corresponding output. Since
    # each input can only be 0 or 1, a two-input gate has exactly 4
    # possible input combinations (2 x 2 = 4), making it easy to verify
    # correctness.

    # The NOT gate (also called an "inverter").
    #
    # NOT is the simplest gate -- it has one input and flips it.
    # If the input is 0, the output is 1. If the input is 1, the output
    # is 0.
    #
    # Think of it like a light switch: if the light is off (0), flipping
    # the switch turns it on (1), and vice versa.
    #
    # Truth table:
    #     Input | Output
    #     ------+-------
    #       0   |   1
    #       1   |   0
    #
    # Circuit symbol:
    #     a -->o-- output
    #     (the small circle o means "invert")
    #
    # @param a [Integer] input bit (0 or 1)
    # @return [Integer] the inverted bit
    # @raise [InvalidBitError] if a is not the Integer 0 or 1
    def self.not_gate(a)
      validate_bit(a, "a")
      # Delegate to the CMOS inverter (2 transistors: 1 PMOS + 1 NMOS).
      CMOS_INVERTER.evaluate_digital(a)
    end

    # The AND gate.
    #
    # AND takes two inputs and outputs 1 ONLY if BOTH inputs are 1.
    # If either input is 0, the output is 0.
    #
    # Think of two switches wired in series (one after the other):
    # electric current can only flow through if both switches are closed
    # (both = 1).
    #
    # Truth table:
    #     A  B  | Output
    #     ------+-------
    #     0  0  |   0      Neither is 1 -> 0
    #     0  1  |   0      Only B is 1  -> 0
    #     1  0  |   0      Only A is 1  -> 0
    #     1  1  |   1      Both are 1   -> 1
    #
    # Circuit symbol:
    #     a --+
    #         |D---- output
    #     b --+
    #
    # @param a [Integer] first input bit (0 or 1)
    # @param b [Integer] second input bit (0 or 1)
    # @return [Integer] 1 if both inputs are 1, else 0
    def self.and_gate(a, b)
      validate_bit(a, "a")
      validate_bit(b, "b")
      # Delegate to the CMOS AND gate (NAND + inverter = 6 transistors).
      CMOS_AND.evaluate_digital(a, b)
    end

    # The OR gate.
    #
    # OR takes two inputs and outputs 1 if EITHER input is 1 (or both).
    # The output is 0 only when both inputs are 0.
    #
    # Think of two switches wired in parallel (side by side): current
    # flows if either switch is closed.
    #
    # Truth table:
    #     A  B  | Output
    #     ------+-------
    #     0  0  |   0      Neither is 1 -> 0
    #     0  1  |   1      B is 1       -> 1
    #     1  0  |   1      A is 1       -> 1
    #     1  1  |   1      Both are 1   -> 1
    #
    # Circuit symbol:
    #     a --\
    #          \---- output
    #     b --/
    #
    # @param a [Integer] first input bit (0 or 1)
    # @param b [Integer] second input bit (0 or 1)
    # @return [Integer] 1 if either input is 1, else 0
    def self.or_gate(a, b)
      validate_bit(a, "a")
      validate_bit(b, "b")
      # Delegate to the CMOS OR gate (NOR + inverter = 6 transistors).
      CMOS_OR.evaluate_digital(a, b)
    end

    # The XOR gate (Exclusive OR).
    #
    # XOR outputs 1 if the inputs are DIFFERENT. Unlike OR, XOR outputs 0
    # when both inputs are 1.
    #
    # The name "exclusive" means: one or the other, but NOT both.
    #
    # Truth table:
    #     A  B  | Output
    #     ------+-------
    #     0  0  |   0      Same      -> 0
    #     0  1  |   1      Different -> 1
    #     1  0  |   1      Different -> 1
    #     1  1  |   0      Same      -> 0
    #
    # Why XOR matters for arithmetic:
    #     In binary addition, 1 + 1 = 10 (that's "one-zero" in binary,
    #     which equals 2 in decimal). The sum digit is 0 and the carry
    #     is 1. Notice that the sum digit (0) is exactly what XOR(1, 1)
    #     produces!
    #
    #     0 + 0 = 0  ->  XOR(0, 0) = 0
    #     0 + 1 = 1  ->  XOR(0, 1) = 1
    #     1 + 0 = 1  ->  XOR(1, 0) = 1
    #     1 + 1 = 0  ->  XOR(1, 1) = 0  (carry the 1 separately)
    #
    #     This is why XOR is the key gate in building adder circuits.
    #
    # @param a [Integer] first input bit (0 or 1)
    # @param b [Integer] second input bit (0 or 1)
    # @return [Integer] 1 if inputs differ, else 0
    def self.xor_gate(a, b)
      validate_bit(a, "a")
      validate_bit(b, "b")
      # Delegate to the CMOS XOR gate (4 NAND gates = 16 transistors).
      CMOS_XOR.evaluate_digital(a, b)
    end

    # =====================================================================
    # COMPOSITE GATES
    # =====================================================================
    # These gates are built by combining fundamental gates. They are
    # included because they appear frequently in digital circuits and have
    # useful properties.

    # The NAND gate (NOT AND).
    #
    # NAND is the inverse of AND: it outputs 1 in every case EXCEPT when
    # both inputs are 1.
    #
    # Truth table:
    #     A  B  | Output
    #     ------+-------
    #     0  0  |   1
    #     0  1  |   1
    #     1  0  |   1
    #     1  1  |   0      <- the only 0 output
    #
    # Why NAND is special -- Functional Completeness:
    #     NAND has a remarkable property: you can build EVERY other gate
    #     using only NAND gates. This means if you had a factory that
    #     could only produce one type of gate, you'd pick NAND -- because
    #     from NAND alone, you can construct NOT, AND, OR, XOR, and any
    #     other logic function.
    #
    #     This property is called "functional completeness" and it's why
    #     real chip manufacturers often build entire processors from NAND
    #     gates -- they're the cheapest and simplest to manufacture.
    #
    #     See the nand_* methods below for proofs of how each gate is
    #     built from NAND.
    #
    # Implementation:
    #     NAND(a, b) = NOT(AND(a, b))
    #
    # @param a [Integer] first input bit (0 or 1)
    # @param b [Integer] second input bit (0 or 1)
    # @return [Integer] 0 if both inputs are 1, else 1
    def self.nand_gate(a, b)
      validate_bit(a, "a")
      validate_bit(b, "b")
      # Delegate to the CMOS NAND gate (4 transistors — the natural CMOS primitive).
      CMOS_NAND.evaluate_digital(a, b)
    end

    # The NOR gate (NOT OR).
    #
    # NOR is the inverse of OR: it outputs 1 ONLY when both inputs are 0.
    #
    # Truth table:
    #     A  B  | Output
    #     ------+-------
    #     0  0  |   1      <- the only 1 output
    #     0  1  |   0
    #     1  0  |   0
    #     1  1  |   0
    #
    # Like NAND, NOR is also functionally complete -- you can build every
    # other gate from NOR alone. (We don't demonstrate this here, but it's
    # a fun exercise!)
    #
    # Implementation:
    #     NOR(a, b) = NOT(OR(a, b))
    #
    # @param a [Integer] first input bit (0 or 1)
    # @param b [Integer] second input bit (0 or 1)
    # @return [Integer] 1 if both inputs are 0, else 0
    def self.nor_gate(a, b)
      validate_bit(a, "a")
      validate_bit(b, "b")
      # Delegate to the CMOS NOR gate (4 transistors — the other natural CMOS primitive).
      CMOS_NOR.evaluate_digital(a, b)
    end

    # The XNOR gate (Exclusive NOR, also called "equivalence gate").
    #
    # XNOR is the inverse of XOR: it outputs 1 when the inputs are the
    # SAME.
    #
    # Truth table:
    #     A  B  | Output
    #     ------+-------
    #     0  0  |   1      Same      -> 1
    #     0  1  |   0      Different -> 0
    #     1  0  |   0      Different -> 0
    #     1  1  |   1      Same      -> 1
    #
    # Use case:
    #     XNOR is used as an equality comparator. If you want to check
    #     whether two bits are equal, XNOR gives you the answer directly:
    #     XNOR(a, b) = 1 means a and b have the same value.
    #
    # Implementation:
    #     XNOR(a, b) = NOT(XOR(a, b))
    #
    # @param a [Integer] first input bit (0 or 1)
    # @param b [Integer] second input bit (0 or 1)
    # @return [Integer] 1 if inputs are the same, else 0
    def self.xnor_gate(a, b)
      validate_bit(a, "a")
      validate_bit(b, "b")
      # Delegate to the dedicated CMOSXnor gate (XOR + Inverter = 8 transistors).
      CMOS_XNOR.evaluate_digital(a, b)
    end

    # =====================================================================
    # NAND-DERIVED GATES -- Proving Functional Completeness
    # =====================================================================
    # The methods below prove that NAND is functionally complete by
    # building NOT, AND, OR, and XOR using ONLY the NAND gate. No other
    # gate is used.
    #
    # This is not just an academic exercise. In real chip manufacturing,
    # the ability to build everything from one gate type dramatically
    # simplifies the fabrication process. The first commercially
    # successful logic family (TTL 7400 series, introduced in 1966) was
    # built around NAND gates.
    #
    # For each derived gate, we show:
    # 1. The construction formula
    # 2. A circuit diagram showing how NAND gates are wired
    # 3. A proof by truth table that it matches the original gate

    # NOT built entirely from NAND gates.
    #
    # Construction:
    #     NOT(a) = NAND(a, a)
    #
    # Why this works:
    #     NAND outputs 0 only when both inputs are 1.
    #     If we feed the same value to both inputs:
    #     - NAND(0, 0) = 1  (neither is 1, so NOT 0 = 1)
    #     - NAND(1, 1) = 0  (both are 1, so NOT 1 = 0)
    #
    # Circuit:
    #     a --+--+
    #         |  |D--o-- output
    #         +--+
    #     (both inputs of the NAND come from the same wire)
    #
    # @param a [Integer] input bit (0 or 1)
    # @return [Integer] the inverted bit
    def self.nand_not(a)
      validate_bit(a, "a")
      nand_gate(a, a)
    end

    # AND built entirely from NAND gates.
    #
    # Construction:
    #     AND(a, b) = NOT(NAND(a, b)) = NAND(NAND(a, b), NAND(a, b))
    #
    # Why this works:
    #     NAND is the opposite of AND. So if we invert NAND's output
    #     (using our nand_not trick above), we get AND back.
    #
    # Circuit (2 NAND gates):
    #     a --+
    #         |D--o--+--+
    #     b --+      |  |D--o-- output
    #                +--+
    #     Gate 1: NAND(a, b)
    #     Gate 2: NAND(result, result) = NOT(result) = AND(a, b)
    #
    # @param a [Integer] first input bit (0 or 1)
    # @param b [Integer] second input bit (0 or 1)
    # @return [Integer] 1 if both inputs are 1, else 0
    def self.nand_and(a, b)
      # Validate here so the error names a/b, not an inner NAND's parameters.
      validate_bit(a, "a")
      validate_bit(b, "b")
      nand_not(nand_gate(a, b))
    end

    # OR built entirely from NAND gates.
    #
    # Construction:
    #     OR(a, b) = NAND(NOT(a), NOT(b)) = NAND(NAND(a,a), NAND(b,b))
    #
    # Why this works (De Morgan's Law):
    #     De Morgan's Law states: NOT(A AND B) = (NOT A) OR (NOT B)
    #     Rearranging: A OR B = NOT(NOT(A) AND NOT(B))
    #                        = NAND(NOT(A), NOT(B))
    #
    #     This is a fundamental identity in Boolean algebra, discovered
    #     by Augustus De Morgan in the 1800s -- long before electronic
    #     computers existed!
    #
    # Circuit (3 NAND gates):
    #     a --+--+
    #         |  |D--o--+
    #         +--+      |
    #                   |D--o-- output
    #     b --+--+      |
    #         |  |D--o--+
    #         +--+
    #     Gate 1: NAND(a, a) = NOT(a)
    #     Gate 2: NAND(b, b) = NOT(b)
    #     Gate 3: NAND(NOT(a), NOT(b)) = OR(a, b)
    #
    # @param a [Integer] first input bit (0 or 1)
    # @param b [Integer] second input bit (0 or 1)
    # @return [Integer] 1 if either input is 1, else 0
    def self.nand_or(a, b)
      # Validate here so the error names a/b, not an inner NAND's parameters.
      validate_bit(a, "a")
      validate_bit(b, "b")
      nand_gate(nand_not(a), nand_not(b))
    end

    # XOR built entirely from NAND gates.
    #
    # Construction:
    #     Let n = NAND(a, b)
    #     XOR(a, b) = NAND(NAND(a, n), NAND(b, n))
    #
    # Why this works:
    #     This is the most complex NAND construction. It uses 4 NAND
    #     gates. The intermediate value n = NAND(a, b) is reused twice,
    #     which is why XOR is more "expensive" in hardware than AND or OR.
    #
    #     Proof by truth table:
    #     a=0, b=0: n=NAND(0,0)=1, NAND(0,1)=1, NAND(0,1)=1, NAND(1,1)=0
    #     a=0, b=1: n=NAND(0,1)=1, NAND(0,1)=1, NAND(1,1)=0, NAND(1,0)=1
    #     a=1, b=0: n=NAND(1,0)=1, NAND(1,1)=0, NAND(0,1)=1, NAND(0,1)=1
    #     a=1, b=1: n=NAND(1,1)=0, NAND(1,0)=1, NAND(1,0)=1, NAND(1,1)=0
    #
    # Circuit (4 NAND gates):
    #     a --+--------+
    #         |        |D--o-- wire1 --+
    #         |   +--+ |               |D--o-- output
    #         |   |  |D--o-- n --+     |
    #     b --+---+              |     |
    #         |                  |D--o-+
    #         +------------------+
    #                          wire2
    #
    #     Gate 1: n = NAND(a, b)
    #     Gate 2: wire1 = NAND(a, n)
    #     Gate 3: wire2 = NAND(b, n)
    #     Gate 4: output = NAND(wire1, wire2)
    #
    # @param a [Integer] first input bit (0 or 1)
    # @param b [Integer] second input bit (0 or 1)
    # @return [Integer] 1 if inputs differ, else 0
    def self.nand_xor(a, b)
      # Validate here so the error names a/b, not an inner NAND's parameters.
      validate_bit(a, "a")
      validate_bit(b, "b")
      nand_ab = nand_gate(a, b)
      nand_gate(nand_gate(a, nand_ab), nand_gate(b, nand_ab))
    end

    # =====================================================================
    # MULTI-INPUT GATES
    # =====================================================================
    # In practice, you often need to AND or OR more than two values
    # together. For example, "are ALL four conditions true?" requires a
    # 4-input AND.
    #
    # Multi-input gates work by chaining 2-input gates. For AND:
    #   AND_N(a, b, c, d) = AND(AND(AND(a, b), c), d)
    #
    # Ruby's Enumerable#reduce (also called #inject) does exactly this:
    # it takes an array and repeatedly applies a 2-argument operation
    # from left to right.

    # AND with N inputs. Returns 1 only if ALL inputs are 1.
    #
    # This chains 2-input AND gates together using reduce:
    #     and_n(a, b, c, d) = AND(AND(AND(a, b), c), d)
    #
    # In hardware, this would be a chain of AND gates:
    #     a --+
    #         |D-- r1 --+
    #     b --+         |D-- r2 --+
    #              c ---+         |D-- output
    #                       d ---+
    #
    # @param inputs [Array<Integer>] two to MAX_INPUTS binary inputs
    # @return [Integer] 1 if all inputs are 1, else 0
    # @raise [ArgumentError] if fewer than 2 or more than MAX_INPUTS inputs
    # @raise [InvalidBitError] if any input is not the Integer 0 or 1
    def self.and_n(*inputs)
      bits = checked_n_inputs("and_n", inputs)
      bits.reduce { |acc, bit| and_gate(acc, bit) }
    end

    # OR with N inputs. Returns 1 if ANY input is 1.
    #
    # This chains 2-input OR gates together using reduce:
    #     or_n(a, b, c, d) = OR(OR(OR(a, b), c), d)
    #
    # @param inputs [Array<Integer>] two to MAX_INPUTS binary inputs
    # @return [Integer] 1 if any input is 1, else 0
    # @raise [ArgumentError] if fewer than 2 or more than MAX_INPUTS inputs
    # @raise [InvalidBitError] if any input is not the Integer 0 or 1
    def self.or_n(*inputs)
      bits = checked_n_inputs("or_n", inputs)
      bits.reduce { |acc, bit| or_gate(acc, bit) }
    end

    # Shared front door of the n-ary gates.
    #
    # The `*inputs` splat hands us a fresh, exact Array that no caller holds
    # a reference to, so it is already a private snapshot: a lazy Enumerator,
    # Set or Range passed positionally arrives as ONE element and is rejected
    # by the bit check (use `and_n(*list)` to spread a list). The count is
    # checked BEFORE any per-element work; every element is validated up
    # front (so `and_n(0, bad)` raises even though AND short-circuits to 0)
    # and the parameter name "inputs[i]" is only built on failure.
    def self.checked_n_inputs(fn_name, inputs)
      n = inputs.length
      raise ArgumentError, "#{fn_name} requires at least 2 inputs" if n < 2
      if n > MAX_INPUTS
        raise ArgumentError, "#{fn_name} accepts at most #{MAX_INPUTS} inputs, got #{n}"
      end

      i = 0
      while i < n
        bit = inputs[i]
        # validate_bit is the one exact validator; the name is built lazily.
        valid = (Integer === bit) && (bit == 0 || bit == 1)
        validate_bit(bit, "inputs[#{i}]") unless valid
        i += 1
      end
      inputs
    end
    private_class_method :checked_n_inputs
  end
end
