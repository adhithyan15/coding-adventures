# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Pipelined floating-point arithmetic -- the bridge to GPU architecture.
# ---------------------------------------------------------------------------
#
# === Why Pipelining? ===
#
# Imagine a car factory with a single worker who does everything: welds the
# frame, installs the engine, paints the body, mounts the wheels, inspects
# the result. One car takes 5 hours. Want 100 cars? That's 500 hours.
#
# Now imagine a factory with 5 stations, each doing one step. The first car
# still takes 5 hours to pass through all 5 stations. But while it moves to
# station 2, a NEW car enters station 1. After the initial 5-hour fill-up
# time, a finished car rolls off the line every HOUR -- 5x throughput!
#
# This is pipelining, and it's exactly how GPUs achieve massive throughput.
#
# === Latency vs Throughput ===
#
#     Latency:     Time for ONE operation to complete start-to-finish.
#     Throughput:  How many operations complete per unit time.
#
# For a 5-stage pipeline:
#     Latency = 5 clock cycles (one operation still takes 5 cycles)
#     Throughput = 1 result per clock cycle (after pipeline fills up)
#
# === Clock-Driven Pipeline Registers ===
#
# Between each stage, there's a set of "pipeline registers" -- flip-flops
# that capture the intermediate results on the rising edge of the clock.
# In our simulation, the Clock object fires its listeners on each edge,
# and our pipeline's on_clock_edge method shifts data between stages.

module CodingAdventures
  module FpArithmetic
    # -----------------------------------------------------------------------
    # PipelinedFPAdder -- 5-stage pipelined floating-point adder
    # -----------------------------------------------------------------------
    #
    # Pipeline Stages:
    #   Stage 1: UNPACK    -- Extract sign, exponent, mantissa; handle specials
    #   Stage 2: ALIGN     -- Shift smaller mantissa to align exponents
    #   Stage 3: ADD/SUB   -- Add or subtract aligned mantissas
    #   Stage 4: NORMALIZE -- Shift result so leading 1 is in correct position
    #   Stage 5: ROUND & PACK -- Apply round-to-nearest-even, pack result
    class PipelinedFPAdder
      NUM_STAGES = 5

      attr_reader :results, :cycle_count

      # @param clock [CodingAdventures::Clock::ClockGenerator] The clock to drive the pipeline.
      # @param fmt [FloatFormat] The floating-point format (default FP32).
      def initialize(clock, fmt = FP32)
        @clock = clock
        @fmt = fmt
        @stages = Array.new(NUM_STAGES)
        @inputs_pending = []
        @results = []
        @cycle_count = 0

        # Register with the clock to advance on each edge.
        @edge_handler = method(:on_clock_edge)
        clock.register_listener(@edge_handler)
      end

      # Submit a new addition to the pipeline.
      #
      # @param a [FloatBits] First operand.
      # @param b [FloatBits] Second operand.
      def submit(a, b)
        @inputs_pending << [a, b]
      end

      private

      # Advance the pipeline on rising clock edges.
      def on_clock_edge(edge)
        return unless edge.rising?

        @cycle_count += 1

        # Shift pipeline forward (from end to start to avoid overwriting)
        (NUM_STAGES - 1).downto(1) do |i|
          @stages[i] = process_stage(i, @stages[i - 1])
        end

        # Load new input into stage 0
        if @inputs_pending.any?
          a, b = @inputs_pending.shift
          @stages[0] = process_stage(0, [a, b])
        else
          @stages[0] = nil
        end

        # Collect output from last stage
        if @stages[NUM_STAGES - 1]
          @results << @stages[NUM_STAGES - 1]
          @stages[NUM_STAGES - 1] = nil
        end
      end

      def process_stage(stage_num, input_data)
        return nil if input_data.nil?

        case stage_num
        when 0 then stage_unpack(input_data)
        when 1 then stage_align(input_data)
        when 2 then stage_add(input_data)
        when 3 then stage_normalize(input_data)
        when 4 then stage_round_pack(input_data)
        end
      end

      # Stage 1: Unpack operands
      def stage_unpack(inputs)
        a, b = inputs
        fmt = @fmt

        a_nan = FpArithmetic.nan?(a)
        b_nan = FpArithmetic.nan?(b)
        a_inf = FpArithmetic.inf?(a)
        b_inf = FpArithmetic.inf?(b)
        a_zero = FpArithmetic.zero?(a)
        b_zero = FpArithmetic.zero?(b)

        if a_nan || b_nan
          return {special: FloatBits.new(sign: 0, exponent: Array.new(fmt.exponent_bits, 1),
            mantissa: [1] + Array.new(fmt.mantissa_bits - 1, 0), fmt: fmt)}
        end

        if a_inf && b_inf
          if a.sign == b.sign
            return {special: FloatBits.new(sign: a.sign, exponent: Array.new(fmt.exponent_bits, 1),
              mantissa: Array.new(fmt.mantissa_bits, 0), fmt: fmt)}
          else
            return {special: FloatBits.new(sign: 0, exponent: Array.new(fmt.exponent_bits, 1),
              mantissa: [1] + Array.new(fmt.mantissa_bits - 1, 0), fmt: fmt)}
          end
        end
        return {special: a} if a_inf
        return {special: b} if b_inf

        if a_zero && b_zero
          result_sign = LogicGates.and_gate(a.sign, b.sign)
          return {special: FloatBits.new(sign: result_sign, exponent: Array.new(fmt.exponent_bits, 0),
            mantissa: Array.new(fmt.mantissa_bits, 0), fmt: fmt)}
        end
        return {special: b} if a_zero
        return {special: a} if b_zero

        exp_a = FpArithmetic.bits_msb_to_int(a.exponent)
        exp_b = FpArithmetic.bits_msb_to_int(b.exponent)
        mant_a = FpArithmetic.bits_msb_to_int(a.mantissa)
        mant_b = FpArithmetic.bits_msb_to_int(b.mantissa)

        if exp_a != 0
          mant_a = (1 << fmt.mantissa_bits) | mant_a
        else
          exp_a = 1
        end
        if exp_b != 0
          mant_b = (1 << fmt.mantissa_bits) | mant_b
        else
          exp_b = 1
        end

        guard_bits = 3
        mant_a <<= guard_bits
        mant_b <<= guard_bits

        {sign_a: a.sign, sign_b: b.sign, exp_a: exp_a, exp_b: exp_b,
         mant_a: mant_a, mant_b: mant_b, guard_bits: guard_bits}
      end

      # Stage 2: Align mantissas
      def stage_align(data)
        return data if data.key?(:special)

        fmt = @fmt
        exp_a = data[:exp_a]
        exp_b = data[:exp_b]
        mant_a = data[:mant_a]
        mant_b = data[:mant_b]
        guard_bits = data[:guard_bits]

        if exp_a >= exp_b
          exp_diff = exp_a - exp_b
          if exp_diff > 0 && exp_diff < (fmt.mantissa_bits + 1 + guard_bits)
            shifted_out = mant_b & ((1 << exp_diff) - 1)
            sticky = shifted_out != 0 ? 1 : 0
          else
            sticky = (mant_b != 0 && exp_diff > 0) ? 1 : 0
          end
          mant_b >>= exp_diff
          mant_b |= 1 if sticky == 1 && exp_diff > 0
          result_exp = exp_a
        else
          exp_diff = exp_b - exp_a
          if exp_diff > 0 && exp_diff < (fmt.mantissa_bits + 1 + guard_bits)
            shifted_out = mant_a & ((1 << exp_diff) - 1)
            sticky = shifted_out != 0 ? 1 : 0
          else
            sticky = (mant_a != 0 && exp_diff > 0) ? 1 : 0
          end
          mant_a >>= exp_diff
          mant_a |= 1 if sticky == 1 && exp_diff > 0
          result_exp = exp_b
        end

        {sign_a: data[:sign_a], sign_b: data[:sign_b],
         mant_a: mant_a, mant_b: mant_b, result_exp: result_exp, guard_bits: guard_bits}
      end

      # Stage 3: Add/subtract mantissas
      def stage_add(data)
        return data if data.key?(:special)

        mant_a = data[:mant_a]
        mant_b = data[:mant_b]
        sign_a = data[:sign_a]
        sign_b = data[:sign_b]

        if sign_a == sign_b
          result_mant = mant_a + mant_b
          result_sign = sign_a
        elsif mant_a >= mant_b
          result_mant = mant_a - mant_b
          result_sign = sign_a
        else
          result_mant = mant_b - mant_a
          result_sign = sign_b
        end

        if result_mant == 0
          fmt = @fmt
          return {special: FloatBits.new(sign: 0, exponent: Array.new(fmt.exponent_bits, 0),
            mantissa: Array.new(fmt.mantissa_bits, 0), fmt: fmt)}
        end

        {result_sign: result_sign, result_mant: result_mant,
         result_exp: data[:result_exp], guard_bits: data[:guard_bits]}
      end

      # Stage 4: Normalize
      def stage_normalize(data)
        return data if data.key?(:special)

        fmt = @fmt
        result_mant = data[:result_mant]
        result_exp = data[:result_exp]
        guard_bits = data[:guard_bits]
        normal_pos = fmt.mantissa_bits + guard_bits
        leading_pos = result_mant.bit_length - 1

        if leading_pos > normal_pos
          shift_amount = leading_pos - normal_pos
          lost_bits = result_mant & ((1 << shift_amount) - 1)
          result_mant >>= shift_amount
          result_mant |= 1 if lost_bits != 0
          result_exp += shift_amount
        elsif leading_pos < normal_pos
          shift_amount = normal_pos - leading_pos
          if result_exp - shift_amount >= 1
            result_mant <<= shift_amount
            result_exp -= shift_amount
          else
            actual_shift = result_exp - 1
            result_mant <<= actual_shift if actual_shift > 0
            result_exp = 0
          end
        end

        {result_sign: data[:result_sign], result_mant: result_mant,
         result_exp: result_exp, guard_bits: guard_bits}
      end

      # Stage 5: Round & pack
      def stage_round_pack(data)
        return data[:special] if data.key?(:special)

        fmt = @fmt
        result_mant = data[:result_mant]
        result_exp = data[:result_exp]
        result_sign = data[:result_sign]
        guard_bits = data[:guard_bits]

        guard = (result_mant >> (guard_bits - 1)) & 1
        round_bit = (result_mant >> (guard_bits - 2)) & 1
        sticky_bit = result_mant & ((1 << (guard_bits - 2)) - 1)
        sticky_bit = sticky_bit != 0 ? 1 : 0

        result_mant >>= guard_bits

        if guard == 1
          if round_bit == 1 || sticky_bit == 1
            result_mant += 1
          elsif (result_mant & 1) == 1
            result_mant += 1
          end
        end

        if result_mant >= (1 << (fmt.mantissa_bits + 1))
          result_mant >>= 1
          result_exp += 1
        end

        max_exp = (1 << fmt.exponent_bits) - 1
        if result_exp >= max_exp
          return FloatBits.new(sign: result_sign, exponent: Array.new(fmt.exponent_bits, 1),
            mantissa: Array.new(fmt.mantissa_bits, 0), fmt: fmt)
        end

        if result_exp <= 0
          if result_exp < -(fmt.mantissa_bits)
            return FloatBits.new(sign: result_sign, exponent: Array.new(fmt.exponent_bits, 0),
              mantissa: Array.new(fmt.mantissa_bits, 0), fmt: fmt)
          end
          shift = 1 - result_exp
          result_mant >>= shift
          result_exp = 0
        end

        result_mant &= (1 << fmt.mantissa_bits) - 1 if result_exp > 0

        FloatBits.new(
          sign: result_sign,
          exponent: FpArithmetic.int_to_bits_msb(result_exp, fmt.exponent_bits),
          mantissa: FpArithmetic.int_to_bits_msb(result_mant, fmt.mantissa_bits),
          fmt: fmt
        )
      end
    end

    # -----------------------------------------------------------------------
    # PipelinedFPMultiplier -- 4-stage pipelined floating-point multiplier
    # -----------------------------------------------------------------------
    class PipelinedFPMultiplier
      NUM_STAGES = 4

      attr_reader :results, :cycle_count

      def initialize(clock, fmt = FP32)
        @clock = clock
        @fmt = fmt
        @stages = Array.new(NUM_STAGES)
        @inputs_pending = []
        @results = []
        @cycle_count = 0
        @edge_handler = method(:on_clock_edge)
        clock.register_listener(@edge_handler)
      end

      def submit(a, b)
        @inputs_pending << [a, b]
      end

      private

      def on_clock_edge(edge)
        return unless edge.rising?

        @cycle_count += 1

        (NUM_STAGES - 1).downto(1) do |i|
          @stages[i] = process_stage(i, @stages[i - 1])
        end

        if @inputs_pending.any?
          a, b = @inputs_pending.shift
          @stages[0] = process_stage(0, [a, b])
        else
          @stages[0] = nil
        end

        if @stages[NUM_STAGES - 1]
          @results << @stages[NUM_STAGES - 1]
          @stages[NUM_STAGES - 1] = nil
        end
      end

      def process_stage(stage_num, input_data)
        return nil if input_data.nil?

        case stage_num
        when 0 then stage_unpack_exp(input_data)
        when 1 then stage_multiply(input_data)
        when 2 then stage_normalize(input_data)
        when 3 then stage_round_pack(input_data)
        end
      end

      def stage_unpack_exp(inputs)
        a, b = inputs
        fmt = @fmt
        result_sign = LogicGates.xor_gate(a.sign, b.sign)

        if FpArithmetic.nan?(a) || FpArithmetic.nan?(b)
          return {special: FloatBits.new(sign: 0, exponent: Array.new(fmt.exponent_bits, 1),
            mantissa: [1] + Array.new(fmt.mantissa_bits - 1, 0), fmt: fmt)}
        end

        a_inf = FpArithmetic.inf?(a)
        b_inf = FpArithmetic.inf?(b)
        a_zero = FpArithmetic.zero?(a)
        b_zero = FpArithmetic.zero?(b)

        if (a_inf && b_zero) || (b_inf && a_zero)
          return {special: FloatBits.new(sign: 0, exponent: Array.new(fmt.exponent_bits, 1),
            mantissa: [1] + Array.new(fmt.mantissa_bits - 1, 0), fmt: fmt)}
        end

        if a_inf || b_inf
          return {special: FloatBits.new(sign: result_sign, exponent: Array.new(fmt.exponent_bits, 1),
            mantissa: Array.new(fmt.mantissa_bits, 0), fmt: fmt)}
        end

        if a_zero || b_zero
          return {special: FloatBits.new(sign: result_sign, exponent: Array.new(fmt.exponent_bits, 0),
            mantissa: Array.new(fmt.mantissa_bits, 0), fmt: fmt)}
        end

        exp_a = FpArithmetic.bits_msb_to_int(a.exponent)
        exp_b = FpArithmetic.bits_msb_to_int(b.exponent)
        mant_a = FpArithmetic.bits_msb_to_int(a.mantissa)
        mant_b = FpArithmetic.bits_msb_to_int(b.mantissa)

        mant_a = exp_a != 0 ? ((1 << fmt.mantissa_bits) | mant_a) : (exp_a = 1; mant_a)
        mant_b = exp_b != 0 ? ((1 << fmt.mantissa_bits) | mant_b) : (exp_b = 1; mant_b)

        result_exp = exp_a + exp_b - fmt.bias

        {result_sign: result_sign, result_exp: result_exp, mant_a: mant_a, mant_b: mant_b}
      end

      def stage_multiply(data)
        return data if data.key?(:special)
        {result_sign: data[:result_sign], result_exp: data[:result_exp],
         product: data[:mant_a] * data[:mant_b]}
      end

      def stage_normalize(data)
        return data if data.key?(:special)

        fmt = @fmt
        product = data[:product]
        result_exp = data[:result_exp]
        product_leading = product.bit_length - 1
        normal_pos = 2 * fmt.mantissa_bits

        if product_leading > normal_pos
          result_exp += product_leading - normal_pos
        elsif product_leading < normal_pos
          result_exp -= normal_pos - product_leading
        end

        {result_sign: data[:result_sign], result_exp: result_exp,
         product: product, product_leading: product_leading}
      end

      def stage_round_pack(data)
        return data[:special] if data.key?(:special)

        fmt = @fmt
        result_sign = data[:result_sign]
        result_exp = data[:result_exp]
        product = data[:product]
        product_leading = data[:product_leading]
        round_pos = product_leading - fmt.mantissa_bits

        if round_pos > 0
          guard = (product >> (round_pos - 1)) & 1
          if round_pos >= 2
            round_bit = (product >> (round_pos - 2)) & 1
            sticky = (product & ((1 << (round_pos - 2)) - 1)) != 0 ? 1 : 0
          else
            round_bit = 0
            sticky = 0
          end
          result_mant = product >> round_pos
          if guard == 1
            if round_bit == 1 || sticky == 1
              result_mant += 1
            elsif (result_mant & 1) == 1
              result_mant += 1
            end
          end
          if result_mant >= (1 << (fmt.mantissa_bits + 1))
            result_mant >>= 1
            result_exp += 1
          end
        elsif round_pos == 0
          result_mant = product
        else
          result_mant = product << (-round_pos)
        end

        max_exp = (1 << fmt.exponent_bits) - 1
        if result_exp >= max_exp
          return FloatBits.new(sign: result_sign, exponent: Array.new(fmt.exponent_bits, 1),
            mantissa: Array.new(fmt.mantissa_bits, 0), fmt: fmt)
        end

        if result_exp <= 0
          if result_exp < -(fmt.mantissa_bits)
            return FloatBits.new(sign: result_sign, exponent: Array.new(fmt.exponent_bits, 0),
              mantissa: Array.new(fmt.mantissa_bits, 0), fmt: fmt)
          end
          shift = 1 - result_exp
          result_mant >>= shift
          result_exp = 0
        end

        result_mant &= (1 << fmt.mantissa_bits) - 1 if result_exp > 0

        FloatBits.new(
          sign: result_sign,
          exponent: FpArithmetic.int_to_bits_msb(result_exp, fmt.exponent_bits),
          mantissa: FpArithmetic.int_to_bits_msb(result_mant, fmt.mantissa_bits),
          fmt: fmt
        )
      end
    end

    # -----------------------------------------------------------------------
    # PipelinedFMA -- 6-stage pipelined fused multiply-add
    # -----------------------------------------------------------------------
    class PipelinedFMA
      NUM_STAGES = 6

      attr_reader :results, :cycle_count

      def initialize(clock, fmt = FP32)
        @clock = clock
        @fmt = fmt
        @stages = Array.new(NUM_STAGES)
        @inputs_pending = []
        @results = []
        @cycle_count = 0
        @edge_handler = method(:on_clock_edge)
        clock.register_listener(@edge_handler)
      end

      # Submit a new FMA operation (a * b + c).
      def submit(a, b, c)
        @inputs_pending << [a, b, c]
      end

      private

      def on_clock_edge(edge)
        return unless edge.rising?

        @cycle_count += 1

        (NUM_STAGES - 1).downto(1) do |i|
          @stages[i] = process_stage(i, @stages[i - 1])
        end

        if @inputs_pending.any?
          a, b, c = @inputs_pending.shift
          @stages[0] = process_stage(0, [a, b, c])
        else
          @stages[0] = nil
        end

        if @stages[NUM_STAGES - 1]
          @results << @stages[NUM_STAGES - 1]
          @stages[NUM_STAGES - 1] = nil
        end
      end

      def process_stage(stage_num, input_data)
        return nil if input_data.nil?

        case stage_num
        when 0 then stage_unpack(input_data)
        when 1 then stage_multiply(input_data)
        when 2 then stage_align(input_data)
        when 3 then stage_add(input_data)
        when 4 then stage_normalize(input_data)
        when 5 then stage_round_pack(input_data)
        end
      end

      def stage_unpack(inputs)
        a, b, c = inputs
        fmt = @fmt

        if FpArithmetic.nan?(a) || FpArithmetic.nan?(b) || FpArithmetic.nan?(c)
          return {special: FloatBits.new(sign: 0, exponent: Array.new(fmt.exponent_bits, 1),
            mantissa: [1] + Array.new(fmt.mantissa_bits - 1, 0), fmt: fmt)}
        end

        a_inf = FpArithmetic.inf?(a)
        b_inf = FpArithmetic.inf?(b)
        c_inf = FpArithmetic.inf?(c)
        a_zero = FpArithmetic.zero?(a)
        b_zero = FpArithmetic.zero?(b)
        product_sign = LogicGates.xor_gate(a.sign, b.sign)

        if (a_inf && b_zero) || (b_inf && a_zero)
          return {special: FloatBits.new(sign: 0, exponent: Array.new(fmt.exponent_bits, 1),
            mantissa: [1] + Array.new(fmt.mantissa_bits - 1, 0), fmt: fmt)}
        end

        if a_inf || b_inf
          if c_inf && product_sign != c.sign
            return {special: FloatBits.new(sign: 0, exponent: Array.new(fmt.exponent_bits, 1),
              mantissa: [1] + Array.new(fmt.mantissa_bits - 1, 0), fmt: fmt)}
          end
          return {special: FloatBits.new(sign: product_sign, exponent: Array.new(fmt.exponent_bits, 1),
            mantissa: Array.new(fmt.mantissa_bits, 0), fmt: fmt)}
        end

        if a_zero || b_zero
          if FpArithmetic.zero?(c)
            result_sign = LogicGates.and_gate(product_sign, c.sign)
            return {special: FloatBits.new(sign: result_sign, exponent: Array.new(fmt.exponent_bits, 0),
              mantissa: Array.new(fmt.mantissa_bits, 0), fmt: fmt)}
          end
          return {special: c}
        end

        return {special: c} if c_inf

        exp_a = FpArithmetic.bits_msb_to_int(a.exponent)
        exp_b = FpArithmetic.bits_msb_to_int(b.exponent)
        mant_a = FpArithmetic.bits_msb_to_int(a.mantissa)
        mant_b = FpArithmetic.bits_msb_to_int(b.mantissa)
        exp_c = FpArithmetic.bits_msb_to_int(c.exponent)
        mant_c = FpArithmetic.bits_msb_to_int(c.mantissa)

        mant_a = exp_a != 0 ? ((1 << fmt.mantissa_bits) | mant_a) : (exp_a = 1; mant_a)
        mant_b = exp_b != 0 ? ((1 << fmt.mantissa_bits) | mant_b) : (exp_b = 1; mant_b)
        mant_c = exp_c != 0 ? ((1 << fmt.mantissa_bits) | mant_c) : (exp_c = 1; mant_c)

        {product_sign: product_sign, c_sign: c.sign,
         exp_a: exp_a, exp_b: exp_b, mant_a: mant_a, mant_b: mant_b,
         exp_c: exp_c, mant_c: mant_c}
      end

      def stage_multiply(data)
        return data if data.key?(:special)

        fmt = @fmt
        product = data[:mant_a] * data[:mant_b]
        product_exp = data[:exp_a] + data[:exp_b] - fmt.bias
        product_leading = product.bit_length - 1
        normal_product_pos = 2 * fmt.mantissa_bits

        if product_leading > normal_product_pos
          product_exp += product_leading - normal_product_pos
        elsif product_leading < normal_product_pos
          product_exp -= normal_product_pos - product_leading
        end

        {product_sign: data[:product_sign], c_sign: data[:c_sign],
         product: product, product_exp: product_exp, product_leading: product_leading,
         exp_c: data[:exp_c], mant_c: data[:mant_c]}
      end

      def stage_align(data)
        return data if data.key?(:special)

        fmt = @fmt
        product = data[:product]
        product_exp = data[:product_exp]
        product_leading = data[:product_leading]
        exp_c = data[:exp_c]
        mant_c = data[:mant_c]
        exp_diff = product_exp - exp_c

        c_scale_shift = product_leading - fmt.mantissa_bits
        c_aligned = if c_scale_shift >= 0
          mant_c << c_scale_shift
        else
          mant_c >> (-c_scale_shift)
        end

        if exp_diff >= 0
          c_aligned >>= exp_diff
          result_exp = product_exp
        else
          product >>= (-exp_diff)
          result_exp = exp_c
        end

        {product_sign: data[:product_sign], c_sign: data[:c_sign],
         product: product, c_aligned: c_aligned, result_exp: result_exp,
         product_leading: product_leading}
      end

      def stage_add(data)
        return data if data.key?(:special)

        fmt = @fmt
        product = data[:product]
        c_aligned = data[:c_aligned]
        product_sign = data[:product_sign]
        c_sign = data[:c_sign]

        if product_sign == c_sign
          result_mant = product + c_aligned
          result_sign = product_sign
        elsif product >= c_aligned
          result_mant = product - c_aligned
          result_sign = product_sign
        else
          result_mant = c_aligned - product
          result_sign = c_sign
        end

        if result_mant == 0
          return {special: FloatBits.new(sign: 0, exponent: Array.new(fmt.exponent_bits, 0),
            mantissa: Array.new(fmt.mantissa_bits, 0), fmt: fmt)}
        end

        {result_sign: result_sign, result_mant: result_mant,
         result_exp: data[:result_exp], product_leading: data[:product_leading]}
      end

      def stage_normalize(data)
        return data if data.key?(:special)

        fmt = @fmt
        result_mant = data[:result_mant]
        result_exp = data[:result_exp]
        product_leading = data[:product_leading]
        target_pos = product_leading > fmt.mantissa_bits ? product_leading : fmt.mantissa_bits
        result_leading = result_mant.bit_length - 1

        if result_leading > target_pos
          shift = result_leading - target_pos
          result_exp += shift
        elsif result_leading < target_pos
          shift_needed = target_pos - result_leading
          result_exp -= shift_needed
        end

        {result_sign: data[:result_sign], result_mant: result_mant, result_exp: result_exp}
      end

      def stage_round_pack(data)
        return data[:special] if data.key?(:special)

        fmt = @fmt
        result_sign = data[:result_sign]
        result_exp = data[:result_exp]
        result_mant = data[:result_mant]
        result_leading = result_mant.bit_length - 1
        round_pos = result_leading - fmt.mantissa_bits

        if round_pos > 0
          guard = (result_mant >> (round_pos - 1)) & 1
          if round_pos >= 2
            round_bit = (result_mant >> (round_pos - 2)) & 1
            sticky = (result_mant & ((1 << (round_pos - 2)) - 1)) != 0 ? 1 : 0
          else
            round_bit = 0
            sticky = 0
          end
          result_mant >>= round_pos
          if guard == 1
            if round_bit == 1 || sticky == 1
              result_mant += 1
            elsif (result_mant & 1) == 1
              result_mant += 1
            end
          end
          if result_mant >= (1 << (fmt.mantissa_bits + 1))
            result_mant >>= 1
            result_exp += 1
          end
        elsif round_pos < 0
          result_mant <<= (-round_pos)
        end

        max_exp = (1 << fmt.exponent_bits) - 1
        if result_exp >= max_exp
          return FloatBits.new(sign: result_sign, exponent: Array.new(fmt.exponent_bits, 1),
            mantissa: Array.new(fmt.mantissa_bits, 0), fmt: fmt)
        end

        if result_exp <= 0
          if result_exp < -(fmt.mantissa_bits)
            return FloatBits.new(sign: result_sign, exponent: Array.new(fmt.exponent_bits, 0),
              mantissa: Array.new(fmt.mantissa_bits, 0), fmt: fmt)
          end
          shift = 1 - result_exp
          result_mant >>= shift
          result_exp = 0
        end

        result_mant &= (1 << fmt.mantissa_bits) - 1 if result_exp > 0

        FloatBits.new(
          sign: result_sign,
          exponent: FpArithmetic.int_to_bits_msb(result_exp, fmt.exponent_bits),
          mantissa: FpArithmetic.int_to_bits_msb(result_mant, fmt.mantissa_bits),
          fmt: fmt
        )
      end
    end

    # -----------------------------------------------------------------------
    # FPUnit -- a complete floating-point unit with all three pipelines.
    # -----------------------------------------------------------------------
    #
    # This is what sits inside every GPU core (CUDA core / shader processor).
    # A single FP unit contains:
    #   - Pipelined FP Adder (5 stages)
    #   - Pipelined FP Multiplier (4 stages)
    #   - Pipelined FMA Unit (6 stages)
    #
    # All three share the same clock signal.
    class FPUnit
      attr_reader :adder, :multiplier, :fma

      # @param clock [CodingAdventures::Clock::ClockGenerator] The shared clock.
      # @param fmt [FloatFormat] The floating-point format (default FP32).
      def initialize(clock, fmt = FP32)
        @clock = clock
        @fmt = fmt
        @adder = PipelinedFPAdder.new(clock, fmt)
        @multiplier = PipelinedFPMultiplier.new(clock, fmt)
        @fma = PipelinedFMA.new(clock, fmt)
      end

      # Run the clock for n complete cycles.
      #
      # @param n [Integer] Number of complete clock cycles.
      def tick(n = 1)
        n.times { @clock.full_cycle }
      end
    end
  end
end
