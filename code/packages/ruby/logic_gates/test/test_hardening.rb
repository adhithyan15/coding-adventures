# frozen_string_literal: true

require_relative "test_helper"

# LGE-RB-HARDEN: escaped type names, capped n-ary inputs, exact block
# validation, state validation, frozen/shareable constants, LG02 section 9 rows.
class TestHardening < Minitest::Test
  LG = CodingAdventures::LogicGates
  COMB = LG::Combinational
  SEQ = LG::Sequential
  IBE = LG::InvalidBitError

  # A probe that counts every call made on it, and lies about its type.
  class Probe
    attr_reader :calls

    def initialize
      @calls = 0
    end

    %i[class is_a? kind_of? to_s inspect to_int to_i == coerce hash name respond_to?].each do |m|
      define_method(m) do |*|
        @calls += 1
        (m == :to_s || m == :inspect) ? "1\nINJECT" : 1
      end
    end
  end

  def bad_values
    probe = Probe.new
    [
      ["1abc", "1 ", "1\n", "0x1", 0.5, -Float::INFINITY, 2**32 + 1, 2**64 + 1,
        [0], [[1]], -> { 1 }, nil, true, false, Rational(1), Complex(1), :one, {}, Object.new, probe],
      probe
    ]
  end

  GATES2 = %i[and_gate or_gate xor_gate nand_gate nor_gate xnor_gate nand_and nand_or nand_xor].freeze
  GATES1 = %i[not_gate nand_not].freeze

  def assert_clean(err)
    assert_instance_of IBE, err
    assert_operator err.message.length, :<=, 256
    refute_match(/[\x00-\x1f\x7f]/, err.message)
  end

  def test_every_gate_rejects_every_bad_row_in_both_positions
    values, probe = bad_values
    values.each do |bad|
      GATES1.each { |g| assert_clean(assert_raises(IBE) { LG.public_send(g, bad) }) }
      GATES2.each do |g|
        [0, 1].each do |partner|
          assert_clean(assert_raises(IBE) { LG.public_send(g, bad, partner) })
          assert_clean(assert_raises(IBE) { LG.public_send(g, partner, bad) })
        end
      end
    end
    assert_equal 0, probe.calls, "no user method may be called on the probe"
  end

  def test_short_circuit_rows_still_raise
    values, = bad_values
    values.each do |bad|
      assert_raises(IBE) { LG.and_gate(0, bad) }
      assert_raises(IBE) { LG.or_gate(1, bad) }
      assert_raises(IBE) { LG.nand_gate(0, bad) }
      assert_raises(IBE) { LG.nor_gate(1, bad) }
      assert_raises(IBE) { LG.and_n(0, 0, bad) }
      assert_raises(IBE) { LG.or_n(1, 1, bad) }
    end
  end

  # --- class-name escaping ---------------------------------------------------

  def named(name)
    skip "Module#set_temporary_name needs Ruby 3.3+" unless Module.method_defined?(:set_temporary_name)
    Class.new.tap { |c| c.set_temporary_name(name) }.new
  end

  def test_huge_integer_described_by_bit_length
    assert_includes assert_raises(IBE) { LG.not_gate(2**100_000) }.message, "an Integer of 100001 bits"
  end

  # --- n-ary -----------------------------------------------------------------

  def test_nary_cap
    assert_equal 65_536, LG::MAX_INPUTS
    assert_equal 1, LG.and_n(*Array.new(LG::MAX_INPUTS, 1))
    assert_equal 0, LG.or_n(*Array.new(LG::MAX_INPUTS, 0))
    assert_raises(ArgumentError) { LG.and_n(*Array.new(LG::MAX_INPUTS + 1, 1)) }
    assert_raises(ArgumentError) { LG.or_n(*Array.new(LG::MAX_INPUTS + 1, 1)) }
    assert_raises(ArgumentError) { LG.and_n(1) }
    assert_raises(ArgumentError) { LG.or_n }
  end

  def test_nary_names_bad_element_and_rejects_containers
    assert_equal "inputs[2]", assert_raises(IBE) { LG.and_n(1, 1, 2) }.param
    assert_raises(IBE) { LG.or_n([1, 0], [0, 1]) }
    assert_raises(IBE) { LG.and_n((1..3).lazy, 1) }
    assert_raises(IBE) { LG.and_n(1, Probe.new) }
  end

  # --- block validators ------------------------------------------------------

  def test_blocks_use_exact_validator_and_own_parameter_names
    values, probe = bad_values
    values.each do |bad|
      assert_equal "d0", assert_raises(IBE) { COMB.mux2(bad, 0, 0) }.param
      assert_equal "sel", assert_raises(IBE) { COMB.mux2(0, 1, bad) }.param
      assert_equal "data", assert_raises(IBE) { COMB.tri_state(bad, 1) }.param
      assert_equal "enable", assert_raises(IBE) { COMB.tri_state(1, bad) }.param
      assert_equal "clock", assert_raises(IBE) { SEQ.d_flip_flop(data: 0, clock: bad) }.param
      assert_equal "data", assert_raises(IBE) { SEQ.d_latch(data: bad, enable: 1) }.param
      assert_equal "serial_in", assert_raises(IBE) { SEQ.shift_register(serial_in: bad, clock: 0) }.param
      assert_equal "data[1]", assert_raises(IBE) { SEQ.register(data: [0, bad], clock: 0) }.param
    end
    assert_equal 0, probe.calls
  end

  # --- caps ------------------------------------------------------------------

  def test_resource_caps
    assert_equal 16, COMB::MAX_SELECT_BITS
    assert_equal 65_536, SEQ::MAX_WIDTH
    assert_equal 64, SEQ::MAX_COUNTER_WIDTH
    assert_raises(ArgumentError) { COMB.decoder(Array.new(17, 0)) }
    assert_equal 8, COMB.decoder([0, 0, 0]).length
    assert_raises(ArgumentError) { COMB.demux(1, Array.new(17, 0), 1 << 17) }
    assert_raises(ArgumentError) { COMB.demux(1, [0], 1.0) }
    assert_raises(ArgumentError) { COMB.mux_n(Array.new(1 << 17, 0), []) }
    assert_raises(ArgumentError) { COMB.encoder(Array.new(1 << 17, 0)) }
    assert_raises(ArgumentError) { COMB.priority_encoder(Array.new(1 << 17, 0)) }
    assert_raises(ArgumentError) { COMB.mux_n(nil, []) }
    assert_raises(ArgumentError) { COMB.encoder(nil) }
    assert_raises(ArgumentError) { SEQ.shift_register(serial_in: 0, clock: 0, width: 65_537) }
    assert_raises(ArgumentError) { SEQ.shift_register(serial_in: 0, clock: 0, width: 2**80) }
    assert_raises(ArgumentError) { SEQ.counter(clock: 0, width: 65) }
    assert_raises(ArgumentError) { SEQ.register(data: Array.new(65_537, 0), clock: 0) }
    assert_equal 64, SEQ.counter(clock: 0, width: 64)[:bits].length
  end

  # --- state validation ------------------------------------------------------

  def test_bad_state_shapes_raise_argument_error_never_no_method_error
    [{}, [], "x", 5, {q: 0}, {q: 0, q_bar: 1, master_q: 0, master_q_bar: 1, extra: 0}].each do |bad|
      assert_raises(ArgumentError) { SEQ.d_flip_flop(data: 0, clock: 0, state: bad) }
    end
    [{}, [], "x", {bits: [0]}, {ff_states: []}, {ff_states: 1, bits: 1}].each do |bad|
      assert_raises(ArgumentError) { SEQ.register(data: [0, 1], clock: 0, state: bad) }
      assert_raises(ArgumentError) { SEQ.shift_register(serial_in: 0, clock: 0, width: 2, state: bad) }
      assert_raises(ArgumentError) { SEQ.counter(clock: 0, width: 2, state: bad) }
    end
  end

  def test_state_wrong_lengths_extra_keys_and_lying_bits
    good = SEQ.register(data: [0, 1], clock: 0)
    assert_raises(ArgumentError) { SEQ.register(data: [0], clock: 0, state: good) }
    assert_raises(ArgumentError) { SEQ.shift_register(serial_in: 0, clock: 0, width: 3, state: good) }
    assert_raises(ArgumentError) { SEQ.counter(clock: 0, width: 2, state: good.merge(extra: 1)) }
    assert_raises(ArgumentError) { SEQ.counter(clock: 0, width: 2, state: {bits: [0], ff_states: good[:ff_states]}) }
    assert_raises(ArgumentError) { SEQ.counter(clock: 0, width: 2, state: {bits: [0, 0], ff_states: [1, 2]}) }

    probe = Probe.new
    assert_raises(IBE) { SEQ.counter(clock: 0, width: 2, state: {bits: [0, probe], ff_states: good[:ff_states]}) }
    ff = good[:ff_states].map(&:dup)
    ff[0][:q] = probe
    assert_raises(IBE) { SEQ.register(data: [0, 1], clock: 0, state: {ff_states: ff}) }
    assert_raises(IBE) { SEQ.d_flip_flop(data: 0, clock: 0, state: {q: 0, q_bar: 1, master_q: 2, master_q_bar: 1}) }
    assert_equal 0, probe.calls
  end

  def test_valid_state_round_trips
    s = nil
    2.times do
      s = SEQ.counter(clock: 0, width: 4, state: s)
      s = SEQ.counter(clock: 1, width: 4, state: s)
    end
    assert_equal [0, 1, 0, 0], s[:bits]
  end

  # --- constants ---------------------------------------------------------------

  def test_public_constants_are_frozen_and_shareable
    %i[CMOS_INVERTER CMOS_NAND CMOS_NOR CMOS_AND CMOS_OR CMOS_XOR CMOS_XNOR].each do |c|
      assert LG.const_get(c).frozen?, "#{c} must be frozen"
      assert Ractor.shareable?(LG.const_get(c)), "#{c} must be shareable"
    end
  end

  # The Ractor check runs in a child process: creating a Ractor inside the
  # SimpleCov-instrumented test process makes coverage on Ruby 3.4 drop
  # executed lines. The child proves frozen/shareable constants let a gate run
  # in a non-main Ractor.
  def test_gate_runs_inside_a_ractor
    lib = File.expand_path("../lib", __dir__)
    script = <<~RUBY
      $VERBOSE = nil
      require "coding_adventures_logic_gates"
      abort("Ractor unavailable") unless defined?(Ractor)
      r = Ractor.new { CodingAdventures::LogicGates.and_gate(1, 1) * 10 + CodingAdventures::LogicGates.not_gate(1) }
      v = r.respond_to?(:value) ? r.value : r.take
      puts "RESULT=\#{v}"
    RUBY
    out = IO.popen([RbConfig.ruby, "-I", lib, "-e", script], err: [:child, :out], &:read)
    assert_predicate $?, :success?, out
    assert_includes out, "RESULT=10"
  end

  # --- exact-class container snapshots (subclass / duck spoofing) -------------

  class LiarArray < Array
    class << self
      attr_accessor :calls
    end
    self.calls = 0
    %i[length size each each_with_index map sum index dup [] is_a? to_ary first].each do |m|
      define_method(m) do |*a, &b|
        LiarArray.calls += 1
        (m == :each_with_index) ? [5, 7].each_with_index(&b) : super(*a, &b)
      end
    end
  end

  class LiarHash < Hash
    class << self
      attr_accessor :calls
    end
    self.calls = 0
    %i[size key? [] keys each to_hash fetch].each do |m|
      define_method(m) do |*a, &b|
        LiarHash.calls += 1
        super(*a, &b)
      end
    end
  end

  class DuckArray
    attr_reader :calls

    def initialize
      @calls = 0
    end

    %i[is_a? length size each_with_index map to_ary [] each].each do |m|
      define_method(m) { |*| (@calls += 1) && true }
    end
  end

  def test_array_subclass_and_duck_rejected_everywhere_without_dispatch
    LiarArray.calls = 0
    duck = DuckArray.new
    [LiarArray.new([0, 1]), LiarArray.new([0, 0]), duck].each do |bad|
      assert_raises(ArgumentError) { COMB.decoder(bad) }
      assert_raises(ArgumentError) { COMB.encoder(bad) }
      assert_raises(ArgumentError) { COMB.priority_encoder(bad) }
      assert_raises(ArgumentError) { COMB.mux_n(bad, [0]) }
      assert_raises(ArgumentError) { COMB.mux_n([0, 1], bad) }
      assert_raises(ArgumentError) { COMB.mux4(0, 0, 0, 0, bad) }
      assert_raises(ArgumentError) { COMB.demux(1, bad, 4) }
      assert_raises(ArgumentError) { SEQ.register(data: bad, clock: 0) }
      assert_raises(ArgumentError) { SEQ.register(data: [0], clock: 0, state: {ff_states: bad}) }
      assert_raises(ArgumentError) { SEQ.counter(clock: 0, width: 2, state: {bits: bad, ff_states: [1, 2]}) }
    end
    assert_equal 0, LiarArray.calls
    assert_equal 0, duck.calls
  end

  def test_hash_subclass_state_rejected_without_dispatch
    LiarHash.calls = 0
    good = SEQ.register(data: [0, 1], clock: 0)
    bad_state = LiarHash[good]
    assert_raises(ArgumentError) { SEQ.register(data: [0, 1], clock: 0, state: bad_state) }
    assert_raises(ArgumentError) { SEQ.shift_register(serial_in: 0, clock: 0, width: 2, state: bad_state) }
    assert_raises(ArgumentError) { SEQ.counter(clock: 0, width: 2, state: bad_state) }
    ff = LiarHash[q: 0, q_bar: 1, master_q: 0, master_q_bar: 1]
    assert_raises(ArgumentError) { SEQ.d_flip_flop(data: 0, clock: 0, state: ff) }
    assert_raises(ArgumentError) { SEQ.register(data: [0], clock: 0, state: {ff_states: [ff]}) }
    assert_equal 0, LiarHash.calls
  end

  def test_basic_object_container_is_rejected
    assert_raises(ArgumentError) { COMB.decoder(BasicObject.new) }
    assert_raises(ArgumentError) { SEQ.register(data: BasicObject.new, clock: 0) }
  end

  def test_exact_array_with_singleton_overrides_uses_snapshot
    arr = [1, 0]
    def arr.each_with_index
      yield 7, 0
    end
    assert_equal [0, 1, 0, 0], COMB.decoder(arr)
  end

  def test_demux_n_outputs_type_message
    err = assert_raises(ArgumentError) { COMB.demux(1, [0], 1.0) }
    assert_equal "n_outputs must be an Integer", err.message
  end

  # --- round 2: hash-free key validation, direction text, singleton methods ---

  class SpyKey
    attr_reader :calls

    def initialize
      @calls = 0
    end

    def reset!
      @calls = 0
    end

    %i[hash eql? == inspect to_s <=> to_sym].each do |m|
      define_method(m) { |*| (@calls += 1) && 1 }
    end
  end

  def test_non_symbol_keys_rejected_without_hashing_or_comparing
    good_ff = {q: 0, q_bar: 1, master_q: 0, master_q_bar: 1}
    spy = SpyKey.new
    with_spy = good_ff.merge(spy => 1)
    h = {}.compare_by_identity
    h[spy] = 1
    h.merge!(good_ff)
    reg = SEQ.register(data: [0, 1], clock: 0)
    top = {spy => 1}.merge(reg)
    nested = {ff_states: [good_ff.merge(spy => 1), good_ff]}
    spy.reset! # building the fixtures above hashed the key; only count the calls
    assert_raises(ArgumentError) { SEQ.d_flip_flop(data: 0, clock: 0, state: with_spy) }
    assert_raises(ArgumentError) { SEQ.d_flip_flop(data: 0, clock: 0, state: h) }
    assert_raises(ArgumentError) { SEQ.register(data: [0, 1], clock: 0, state: top) }
    assert_raises(ArgumentError) { SEQ.counter(clock: 0, width: 2, state: top) }
    assert_raises(ArgumentError) { SEQ.register(data: [0, 1], clock: 0, state: nested) }
    assert_equal 0, spy.calls
  end

  def test_unknown_symbol_key_rejected
    reg = SEQ.register(data: [0, 1], clock: 0)
    assert_raises(ArgumentError) { SEQ.register(data: [0, 1], clock: 0, state: reg.merge(bogus: 1)) }
  end

  def test_direction_object_is_not_inspected
    spy = SpyKey.new
    err = assert_raises(ArgumentError) { SEQ.shift_register(serial_in: 0, clock: 0, direction: spy) }
    assert_equal 0, spy.calls
    assert_equal "direction must be :left or :right, got a value of another type", err.message
    assert_clean_message(err.message)
  end

  def assert_clean_message(msg)
    refute_match(/[\x00-\x1f\x7f]/, msg)
  end

  def test_singleton_methods_on_exact_containers_are_never_called
    called = []
    arr = [1, 0]
    arr.define_singleton_method(:each) { |*| called << :each }
    arr.define_singleton_method(:to_ary) { called << :to_ary }
    arr.define_singleton_method(:length) { called << :length }
    assert_equal [0, 1, 0, 0], COMB.decoder(arr)

    h = {q: 0, q_bar: 1, master_q: 0, master_q_bar: 1}
    h.define_singleton_method(:each) { |*| called << :each }
    h.define_singleton_method(:to_hash) { called << :to_hash }
    h.define_singleton_method(:keys) { called << :keys }
    h.define_singleton_method(:[]) { |*| called << :[] }
    out = SEQ.d_flip_flop(data: 1, clock: 1, state: h)
    assert_equal 0, out[:q]
    assert_empty called
  end

  def test_default_proc_never_fires
    fired = false
    h = Hash.new { |_, _k| fired = true }
    h[:ff_states] = []
    assert_raises(ArgumentError) { SEQ.register(data: [0, 1], clock: 0, state: h) }
    refute fired
    ff = Hash.new { |_, _k| fired = true }
    ff[:q] = 0
    assert_raises(ArgumentError) { SEQ.d_flip_flop(data: 0, clock: 0, state: ff) }
    refute fired
  end

  # --- round 3: dispatch-free nil checks, bounded messages -------------------

  class NilSpy
    attr_reader :calls

    def initialize
      @calls = 0
    end

    %i[nil? equal? == ! respond_to? class].each do |m|
      define_method(m) { |*| (@calls += 1) && true }
    end
  end

  def test_nil_check_does_not_dispatch_and_non_nil_state_is_rejected
    spy = NilSpy.new
    assert_raises(ArgumentError) { SEQ.d_flip_flop(data: 0, clock: 0, state: spy) }
    assert_raises(ArgumentError) { SEQ.register(data: [0], clock: 0, state: spy) }
    assert_raises(ArgumentError) { SEQ.shift_register(serial_in: 0, clock: 0, state: spy) }
    assert_raises(ArgumentError) { SEQ.counter(clock: 0, state: spy) }
    assert_equal 0, spy.calls
    [BasicObject.new, false, 0].each do |bad|
      assert_raises(ArgumentError) { SEQ.d_flip_flop(data: 0, clock: 0, state: bad) }
      assert_raises(ArgumentError) { SEQ.counter(clock: 0, state: bad) }
    end
  end

  def test_huge_bignums_never_make_long_messages
    big = 10**200_000
    [
      -> { COMB.demux(0, [0], big) },
      -> { COMB.demux(0, [0], -big) },
      -> { SEQ.shift_register(serial_in: 0, clock: 0, width: big) },
      -> { SEQ.counter(clock: 0, width: big) },
      -> { SEQ.counter(clock: 0, width: -big) }
    ].each do |f|
      err = assert_raises(ArgumentError, &f)
      assert_operator err.message.length, :<=, 256
    end
  end

  def test_oversize_containers_rejected_before_copy
    assert_raises(ArgumentError) { COMB.mux4(0, 0, 0, 0, Array.new(100_000, 0)) }
    assert_raises(ArgumentError) { SEQ.register(data: [0], clock: 0, state: {ff_states: Array.new(70_000, 0)}) }
    assert_raises(ArgumentError) do
      SEQ.d_flip_flop(data: 0, clock: 0, state: (1..100).to_h { |i| [:"k#{i}", 0] })
    end
  end

  # --- round 4: fixed-text descriptions, unknown keywords, copy_hash branches --

  class StringishName < String
    def to_s = raise("must not be called")

    def to_str = raise("must not be called")

    def inspect = raise("must not be called")
  end

  def message_for(value)
    assert_raises(IBE) { LG.not_gate(value) }.message
  end

  def test_messages_never_contain_caller_controlled_text
    names = ["<Evil\n\e[31mX>", "caf\u00e9", "a\\b", "x" * 1000,
      "fake<\xff>".dup.force_encoding("UTF-8"),
      "abc".encode("UTF-16LE"), "abc".encode("ISO-2022-JP"), "+AGE-".dup.force_encoding("UTF-7")]
    names.each do |n|
      msg = message_for(named(n))
      assert_equal "a must be the Integer 0 or 1, got a value of another type", msg
    end
    klass = Class.new
    klass.set_temporary_name(StringishName.new("<boom>")) if Module.method_defined?(:set_temporary_name)
    assert_equal "a must be the Integer 0 or 1, got a value of another type", message_for(klass.new)
  end

  def test_fixed_descriptions_for_builtin_types
    {nil => "nil", true => "a boolean", false => "a boolean", 0.5 => "a Float", "x" => "a String",
     :s => "a Symbol", [1] => "an Array", {} => "a Hash", Rational(1) => "a Rational",
     Complex(1) => "a Complex", Object.new => "a value of another type"}.each do |v, text|
      assert_equal "a must be the Integer 0 or 1, got #{text}", message_for(v)
    end
  end

  def test_basic_object_description
    assert_equal "a must be the Integer 0 or 1, got a value of another type", message_for(BasicObject.new)
  end

  def test_unknown_keywords_get_fixed_message_and_are_not_touched
    # Ruby itself hashes a non-Symbol keyword key when building `rest`; what
    # matters is that OUR code never calls inspect / to_s on it.
    spy = Object.new
    counter = [0]
    %i[inspect to_s to_str].each { |m| spy.define_singleton_method(m) { |*| (counter[0] += 1).to_s } }
    calls = [
      -> { SEQ.sr_latch(set_input: 0, reset: 0, **{"x" * 1_000_000 => 1}) },
      -> { SEQ.d_latch(data: 0, enable: 0, **{spy => 1}) },
      -> { SEQ.d_flip_flop(data: 0, clock: 0, **{spy => 1}) },
      -> { SEQ.register(data: [0], clock: 0, **{spy => 1}) },
      -> { SEQ.shift_register(serial_in: 0, clock: 0, **{spy => 1}) },
      -> { SEQ.counter(clock: 0, **{spy => 1}) }
    ]
    calls.each do |c|
      err = assert_raises(ArgumentError, &c)
      assert_equal "unknown keyword", err.message
    end
    assert_equal 0, counter[0]
  end

  def test_in_range_size_state_hash_with_bad_keys
    ok = {q: 0, q_bar: 1, master_q: 0, master_q_bar: 1}
    assert_raises(ArgumentError) { SEQ.d_flip_flop(data: 0, clock: 0, state: {:q => 0, :q_bar => 1, :master_q => 0, "master_q_bar" => 1}) }
    assert_raises(ArgumentError) { SEQ.d_flip_flop(data: 0, clock: 0, state: {q: 0, q_bar: 1, master_q: 0, other: 1}) }
    assert_raises(ArgumentError) { SEQ.d_flip_flop(data: 0, clock: 0, state: ok.except(:q)) }
  end

  # --- round 5: the caller's class object is never the receiver ----------------

  def spying_class(raises:)
    hits = [0]
    k = Class.new
    k.define_singleton_method(:equal?) do |*|
      hits[0] += 1
      raise "boom" if raises
      true
    end
    [k, hits]
  end

  def test_class_with_spoofed_equal_is_never_called
    [false, true].each do |raises|
      k, hits = spying_class(raises: raises)
      [:not_gate, :nand_not].each do |g|
        assert_equal "a must be the Integer 0 or 1, got a value of another type",
          assert_raises(IBE) { LG.public_send(g, k.new) }.message
      end
      GATES2.each do |g|
        assert_raises(IBE) { LG.public_send(g, k.new, 1) }
        assert_raises(IBE) { LG.public_send(g, 1, k.new) }
      end
      assert_raises(IBE) { LG.and_n(1, k.new) }

      assert_raises(ArgumentError) { COMB.decoder(k.new) }
      assert_raises(ArgumentError) { COMB.mux_n(k.new, [0]) }
      assert_raises(ArgumentError) { COMB.mux_n([0, 1], k.new) }
      assert_raises(ArgumentError) { COMB.encoder(k.new) }
      assert_raises(ArgumentError) { COMB.priority_encoder(k.new) }
      assert_raises(ArgumentError) { SEQ.register(data: k.new, clock: 0) }
      assert_raises(ArgumentError) { SEQ.d_flip_flop(data: 0, clock: 0, state: k.new) }
      assert_raises(ArgumentError) { SEQ.register(data: [0], clock: 0, state: k.new) }
      assert_equal 0, hits[0]
    end
  end
end
