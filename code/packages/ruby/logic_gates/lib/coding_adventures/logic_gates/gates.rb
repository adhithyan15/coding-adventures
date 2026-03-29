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
# check that the value is an Integer and that it equals 0 or 1. Passing
# true, false, floats, strings, or any non-Integer type will raise a
# TypeError. Passing integers outside {0, 1} will raise a ValueError
# (ArgumentError in Ruby).
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
    CMOS_NAND     = CodingAdventures::Transistors::CMOSNand.new
    CMOS_NOR      = CodingAdventures::Transistors::CMOSNor.new
    CMOS_AND      = CodingAdventures::Transistors::CMOSAnd.new
    CMOS_OR       = CodingAdventures::Transistors::CMOSOr.new
    CMOS_XOR      = CodingAdventures::Transistors::CMOSXor.new
    CMOS_XNOR     = CodingAdventures::Transistors::CMOSXnor.new

    # -----------------------------------------------------------------------
    # Input validation
    # -----------------------------------------------------------------------
    # Every gate checks that its inputs are valid binary values (0 or 1).
    # In Ruby, booleans are NOT integers (unlike Python where bool is a
    # subclass of int), so we simply check is_a?(Integer).
    # We reject anything that is not an Integer, and any Integer outside
    # {0, 1}.

    # Ensure a value is a binary bit: the integer 0 or the integer 1.
    #
    # We explicitly reject:
    # - Non-Integer types (true, false, "1", 1.0, nil, etc.)
    # - Integers outside {0, 1} -- not valid binary digits.
    #
    # @param value [Object] the value to validate
    # @param name [String] the parameter name (for error messages)
    # @raise [TypeError] if value is not an Integer
    # @raise [ArgumentError] if value is not 0 or 1
    # @return [void]
    def self.validate_bit(value, name = "input")
      unless value.is_a?(Integer)
        raise TypeError, "#{name} must be an Integer, got #{value.class}"
      end

      unless value == 0 || value == 1
        raise ArgumentError, "#{name} must be 0 or 1, got #{value}"
      end
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
    # @raise [TypeError] if a is not an Integer
    # @raise [ArgumentError] if a is not 0 or 1
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
    # @param inputs [Array<Integer>] two or more binary inputs
    # @return [Integer] 1 if all inputs are 1, else 0
    # @raise [ArgumentError] if fewer than 2 inputs are provided
    def self.and_n(*inputs)
      if inputs.length < 2
        raise ArgumentError, "and_n requires at least 2 inputs"
      end

      inputs.reduce { |acc, bit| and_gate(acc, bit) }
    end

    # OR with N inputs. Returns 1 if ANY input is 1.
    #
    # This chains 2-input OR gates together using reduce:
    #     or_n(a, b, c, d) = OR(OR(OR(a, b), c), d)
    #
    # @param inputs [Array<Integer>] two or more binary inputs
    # @return [Integer] 1 if any input is 1, else 0
    # @raise [ArgumentError] if fewer than 2 inputs are provided
    def self.or_n(*inputs)
      if inputs.length < 2
        raise ArgumentError, "or_n requires at least 2 inputs"
      end

      inputs.reduce { |acc, bit| or_gate(acc, bit) }
    end
  end
end
