# frozen_string_literal: true

require_relative "test_helper"

# Pins what fuzz-v1 does not (VER06i section 9 item 10): every interest and delta
# constant, unequal splice splits with target and donor swapped, real sig_hash
# values (cross-checked with the Python oracle code/scripts/fuzz_conformance.py),
# power-schedule rows including exact half-way ties, and the mutator edge cases.
class TestFuzzPins < Minitest::Test
  include CodingAdventures::TestBootstrap

  M = Fuzz::Mutate

  def hx(text) = Fx.hex(text)

  def hex(bytes) = bytes.unpack1("H*")

  # -- constants ----------------------------------------------------------------

  def test_every_interest_value_and_delta_is_pinned
    assert_equal [0, 1, 127, 128, 255], M::INTEREST_8
    assert_equal [0, 1, 255, 256, 32_767, 32_768, 65_535], M::INTEREST_16
    assert_equal [0, 1, 65_535, 65_536, 0x7FFFFFFF, 0x80000000, 0xFFFFFFFF], M::INTEREST_32
    assert_equal [0, 1, 0xFFFFFFFF, 0x100000000, 0x7FFFFFFFFFFFFFFF, 0x8000000000000000, 0xFFFFFFFFFFFFFFFF], M::INTEREST_64
    assert_equal [-35, -8, -1, 1, 8, 35], M::DELTAS
    assert_equal({1 => M::INTEREST_8, 2 => M::INTEREST_16, 4 => M::INTEREST_32, 8 => M::INTEREST_64}, M::INTEREST_BY_WIDTH)
    assert(M::INTEREST_BY_WIDTH.values.all?(&:frozen?))
  end

  def test_every_interest_value_round_trips_through_boundary_insert_in_both_endiannesses
    M::INTEREST_BY_WIDTH.each do |width, values|
      values.each do |val|
        zero = "\x00".b * (width + 2)
        le = M.boundary_insert(zero, 1, width, val, "little")
        be = M.boundary_insert(zero, 1, width, val, "big")
        assert_equal val, le.byteslice(1, width).unpack1({1 => "C", 2 => "S<", 4 => "L<", 8 => "Q<"}[width])
        assert_equal val, be.byteslice(1, width).unpack1({1 => "C", 2 => "S>", 4 => "L>", 8 => "Q>"}[width])
        assert_equal "00", hex(le)[0, 2]
        assert_equal "00", hex(le)[-2..]
      end
    end
  end

  def test_arithmetic_wraps_modulo_the_width_in_every_width_and_endianness
    {1 => 0xFF, 2 => 0xFFFF, 4 => 0xFFFF_FFFF, 8 => 0xFFFF_FFFF_FFFF_FFFF}.each do |w, max|
      %w[little big].each do |e|
        ff = "\xFF".b * w
        assert_equal "00" * w, hex(M.byte_arithmetic(ff, 0, 1, w, e)), "w#{w} #{e} +1"
        assert_equal "ff" * w, hex(M.byte_arithmetic("\x00".b * w, 0, -1, w, e)), "w#{w} #{e} -1"
        assert_equal hex(ff), hex(M.byte_arithmetic(ff, 0, 1 << (8 * w), w, e)), "w#{w} full wrap"
        assert_operator max, :>, 0
      end
    end
    assert_equal "0001", hex(M.byte_arithmetic(hx("ff00"), 0, 1, 2, "little"))
    assert_equal "ff01", hex(M.byte_arithmetic(hx("ff00"), 0, 1, 2, "big"))
  end

  def test_unsupported_widths_and_short_buffers_return_the_input_unchanged
    [0, 3, 5, 16, -1, 2**70].each do |w|
      assert_equal "0102", hex(M.byte_arithmetic(hx("0102"), 0, 1, w))
      assert_equal "0102", hex(M.boundary_insert(hx("0102"), 0, w, 9))
    end
    assert_equal "01", hex(M.byte_arithmetic(hx("01"), 0, 1, 2))
    assert_equal "", hex(M.boundary_insert("".b, 0, 1, 9))
  end

  # -- splice -------------------------------------------------------------------

  def test_unequal_splits_with_target_and_donor_swapped
    a = hx("01020304")
    b = hx("0506070809")
    assert_equal "010809", hex(M.splice(a, b, 1, 3))
    assert_equal "050607" + "020304", hex(M.splice(b, a, 3, 1))
    assert_equal "0102030409", hex(M.splice(a, b, 4, 4))
    assert_equal "05060708090304", hex(M.splice(b, a, 5, 2))
    assert_equal "0102070809", hex(M.splice(a, b, 2, 2))
    refute_equal hex(M.splice(a, b, 1, 3)), hex(M.splice(b, a, 3, 1))
  end

  def test_splice_clamps_both_splits
    a = hx("0102")
    b = hx("0304")
    assert_equal "0102", hex(M.splice(a, b, 99, 99))
    assert_equal "0304", hex(M.splice(a, b, -5, -5))
    assert_equal "0102" + "0304", hex(M.splice(a, b, 2**80, -2**80))
    assert_equal "", hex(M.splice("".b, "".b, 1, 1))
  end

  # -- flips ----------------------------------------------------------------------

  def test_flips_wrap_and_use_parity_for_any_count
    assert_equal "0002", hex(M.bit_flip(hx("0000"), 9, 1))
    assert_equal "0002", hex(M.bit_flip(hx("0000"), -7, 1)) # floored: -7 mod 16 = 9
    assert_equal "feff", hex(M.bit_flip(hx("0000"), 1, 15)) # bits 1..15
    assert_equal "ffbf", hex(M.bit_flip(hx("0000"), 15, 15)) # bit 15 then wraps to bits 0..13
    # 17 flips from bit 0 of 2 bytes: every bit once (odd), bit 0 once more -> bit 0 unchanged
    assert_equal "feff", hex(M.bit_flip(hx("0000"), 0, 17))
    assert_equal "0000", hex(M.bit_flip(hx("0000"), 5, 32)) # even full cycles
    assert_equal "ffff", hex(M.bit_flip(hx("0000"), 5, 16))
    assert_equal "0080", hex(M.bit_flip(hx("0000"), 15, 1))
    assert_equal "ed3456", hex(M.byte_flip(hx("123456"), 0, 1))
    assert_equal "12cba9", hex(M.byte_flip(hx("123456"), 1, 2))
    assert_equal "ed34a9", hex(M.byte_flip(hx("123456"), -1, 2)) # wraps: byte 2 then byte 0
    assert_equal "edcba9", hex(M.byte_flip(hx("123456"), 0, 3))
    assert_equal "123456", hex(M.byte_flip(hx("123456"), 1, 6))
    assert_equal "", hex(M.bit_flip("".b, 3, 3))
    assert_equal "0102", hex(M.byte_flip(hx("0102"), 0, 0))
    assert_equal "0102", hex(M.byte_flip(hx("0102"), 0, -4))
  end

  def test_a_huge_flip_count_is_cheap
    t = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    out = M.bit_flip("\x00".b * 1000, 12_345, 10**1000 + 7)
    M.byte_flip("\x00".b * 1000, 12_345, 10**1000 + 7)
    assert_equal 1000, out.bytesize
    assert_operator Process.clock_gettime(Process::CLOCK_MONOTONIC) - t, :<, 2
  end

  def test_flips_are_involutions
    data = hx("0a1b2c3d4e")
    [0, 1, 9, 39, 40, 2**65].each do |off|
      [1, 4, 17, 2**64 + 5].each do |count|
        assert_equal hex(data), hex(M.bit_flip(M.bit_flip(data, off, count), off, count))
        assert_equal hex(data), hex(M.byte_flip(M.byte_flip(data, off, count), off, count))
      end
    end
  end

  # -- blocks ---------------------------------------------------------------------

  def test_block_operations_and_their_clamping
    assert_equal "010405", hex(M.block_delete(hx("0102030405"), 1, 2))
    assert_equal "0102", hex(M.block_delete(hx("0102030405"), 2, 99))
    assert_equal "0102030405", hex(M.block_delete(hx("0102030405"), 2, 0))
    assert_equal "", hex(M.block_delete("".b, 2, 2))
    assert_equal "0304", hex(M.block_insert("".b, 5, hx("0304")))
    assert_equal "0102030405", hex(M.block_insert(hx("010205"), 2, hx("0304")))
    assert_equal "010203", hex(M.block_insert(hx("0102"), 2, hx("03"))) # end is a valid point
    assert_equal "0102030102", hex(M.block_duplicate(hx("010203"), 0, 2, 3))
    assert_equal "010203", hex(M.block_duplicate(hx("010203"), 0, 0, 3))
    assert_equal "", hex(M.block_duplicate("".b, 0, 1, 0))
    assert_equal "0103" + "03", hex(M.block_duplicate(hx("0103"), 1, 9, 2)) # clamped copy
  end

  def test_no_mutator_modifies_freezes_or_returns_its_argument
    data = hx("0102030405").freeze
    outs = [M.bit_flip(data, 0), M.byte_flip(data, 0), M.byte_arithmetic(data, 0, 0, 3), M.boundary_insert(data, 0, 3, 1),
      M.block_delete(data, 0, 0), M.block_insert(data, 0, "".b), M.block_duplicate(data, 0, 0, 0),
      M.splice(data, data, 5, 0), M.mutate_sequence(data, 1, 0)]
    outs.each do |o|
      refute o.equal?(data)
      refute_predicate o, :frozen?
      assert_equal Encoding::BINARY, o.encoding
    end
    assert_equal "0102030405", hex(data)
    utf8 = "é".dup.force_encoding("UTF-8")
    assert_equal "c3a9", hex(M.byte_flip(utf8, 0, 0))
  end

  def test_type_errors
    assert_raises(TypeError) { M.bit_flip(1, 0) }
    assert_raises(TypeError) { M.bit_flip("".b, true) }
    assert_raises(TypeError) { M.bit_flip("".b, 1.0) }
    assert_raises(TypeError) { M.byte_arithmetic("".b, 0, nil) }
    assert_raises(TypeError) { M.block_insert("a", 0, 5) }
    assert_raises(TypeError) { M.splice("a", nil, 0, 0) }
    assert_raises(ArgumentError) { M.byte_arithmetic("ab", 0, 1, 1, "middle") }
    assert_raises(ArgumentError) { M.boundary_insert("ab", 0, 1, 1, 7) }
    assert_equal "02", hex(M.byte_arithmetic("\x01", 0, 1, 1, :big)) # symbols are accepted
  end

  # -- mutate_sequence ------------------------------------------------------------

  def test_sequence_edges
    assert_equal "0102", hex(M.mutate_sequence(hx("0102"), 1, 0))
    assert_equal "0102", hex(M.mutate_sequence(hx("0102"), 1, -5))
    # an empty input is refilled to 00 before every step:
    refute_empty M.mutate_sequence("".b, 7, 1)
    assert_equal hex(M.mutate_sequence(hx("00"), 9, 8)), hex(M.mutate_sequence(hx("00"), 9, 8))
    assert_raises(ArgumentError) { M.mutate_sequence("a", 1, Fuzz::Limits::MAX_ITERATIONS + 1) }
    assert_raises(ArgumentError) { M.mutate_sequence("a" * (Fuzz::Limits::MAX_INPUT_BYTES + 1), 1, 1) }
    assert_raises(TypeError) { M.mutate_sequence("a", "1", 1) }
  end

  def test_sequence_respects_max_len_and_keeps_the_stream_aligned
    # Inserts/duplicates over the bound are skipped AFTER their draws: the same
    # seed with a tight bound gives a prefix-compatible stream and never grows past it.
    1000.times do |seed|
      out = M.mutate_sequence(hx("0001020304050607"), seed, 40, 8)
      assert_operator out.bytesize, :<=, 8
    end
    wide = M.mutate_sequence(hx("0001020304050607"), 3, 40, 1 << 20)
    assert_operator wide.bytesize, :>, 8
  end

  def test_sequence_work_is_the_bytes_actually_copied
    out, work = M.mutate_sequence_counted(hx("00" * 10), 5, 3, 1 << 20, 1 << 30)
    assert_operator work, :>=, 10 + 3 # the initial copy plus a buffer per step
    assert_operator work, :<=, 10 + (3 * out.bytesize.clamp(10, 100))
    _, w0 = M.mutate_sequence_counted(hx("0102"), 5, 0, 100, 100)
    assert_equal 2, w0 # the initial copy is charged even with zero steps
  end

  def test_sequence_work_cap_raises_with_the_work_done
    err = assert_raises(M::WorkExhausted) { M.mutate_sequence_counted("a" * 100, 1, 10, 1 << 20, 250) }
    assert_equal 200, err.work # 100 initial + 100 first step; the second would pass 250
    err = assert_raises(M::WorkExhausted) { M.mutate_sequence_counted("a" * 100, 1, 10, 1 << 20, 99) }
    assert_equal 0, err.work # the initial copy alone is over the cap
    # a failed call is still bounded: it stops as soon as the cap would be passed
    err = assert_raises(M::WorkExhausted) { M.mutate_sequence_counted("a" * (1 << 20), 1, 100, 1 << 21, 1 << 22) }
    assert_operator err.work, :<=, 1 << 22
  end

  # -- bitmap, admission, energy ----------------------------------------------------

  def test_bitmap
    b = Fuzz::Bitmap.new
    assert_equal 0, b.count
    b.record(3)
    b.record(3 + 65_536) # reduced mod 65,536
    b.record(-1)         # floored: 65,535
    assert_equal [3, 65_535], b.covered_edges
    assert b.has?(65_539)
    refute b.has?(4)
    assert_equal [true, 2], b.admit([3, 4, 5, 5])
    assert_equal [false, 0], b.admit([3, 4])
    assert_equal 8192, b.raw.bytesize
    other = Fuzz::Bitmap.new
    other.record(9)
    b.merge(other)
    assert b.has?(9)
    b.reset
    assert_equal 0, b.count
    assert_equal "\x00".b * 8192, b.raw
    assert_raises(TypeError) { b.record("1") }
    assert_raises(TypeError) { b.record(1.5) }
  end

  def test_corpus_admit_counts_unique_novel_edges_over_unreduced_ids
    assert_equal [true, 1], Fuzz.corpus_admit([1, 2, 3], [2, 4])
    assert_equal [false, 0], Fuzz.corpus_admit([1, 2, 3], [1, 2])
    assert_equal [true, 1], Fuzz.corpus_admit([], [7, 7, 7])
    assert_equal [true, 1], Fuzz.corpus_admit([1], [1 + 65_536]) # different ids, same bit
    assert_equal [false, 0], Fuzz.corpus_admit([1], [])
  end

  # [base, len, new, total, expected] from the Python oracle formula.
  ENERGY = [
    [10, 4, 3, 6, 8], [10, 1, 0, 0, 10], [10, 4, 0, 0, 5], [1, 4, 0, 0, 1], [10, 16, 0, 0, 3],
    [10, 100, 0, 0, 1], [3, 4, 0, 0, 2], [5, 4, 0, 0, 3], [10, 9, 0, 0, 3], [10, 2, 1, 2, 11],
    [7, 1, 1, 3, 9], [0, 4, 0, 0, 1], [10, 4, 1, 1, 10], [25, 100, 0, 0, 3], [10, 25, 0, 0, 2],
    [15, 1, 0, 0, 15], [10, 1, 5, 5, 20]
  ].freeze

  def test_energy_rows_including_half_way_ties
    ENERGY.each do |base, len, new, total, want|
      assert_equal want, Fuzz.calculate_energy(base, len, new, total), [base, len, new, total].inspect
    end
    # exact .5 ties round UP (banker's rounding would give 2, 2, 2):
    assert_equal 2, Fuzz.calculate_energy(3, 4, 0, 0)  # 1.5
    assert_equal 3, Fuzz.calculate_energy(5, 4, 0, 0)  # 2.5
    assert_equal 8, Fuzz.calculate_energy(10, 4, 3, 6) # 7.5
    assert_equal 3, Fuzz.calculate_energy(25, 100, 0, 0) # 2.5
    assert_equal 1, Fuzz.calculate_energy(1, 4, 0, 0)  # 0.5 rounds to 1
    assert_equal 1, Fuzz.calculate_energy(0, 1, 0, 0)  # base clamped to 1
    assert_equal 10, Fuzz.calculate_energy(10, 0, 0, 0) # len 0 counts as 1
  end

  # -- signatures -------------------------------------------------------------------

  SIGS = [
    ["panic", %w[eval step], "4eca7ae8"], ["segv", %w[alloc write], "a0aa3aa2"], ["timeout", [], "4e61ea76"],
    ["oom", [], "7146a410"], ["RuntimeError", %w[t.rb#Foo.bar t.rb#block_in_Foo.bar], "c12b6a11"],
    ["ZeroDivisionError", %w[target.rb#Object#parse], "19321229"], ["stale-budget-sentinel", [], "8a20b3bf"],
    ["ArgumentError", %w[a.rb#x], "c93b2322"]
  ].freeze

  def test_real_sig_hash_values_match_the_python_oracle
    SIGS.each { |kind, frames, want| assert_equal want, Fuzz::Signature.crash_signature(kind, frames), kind }
    assert_equal 16_654_208_175_385_433_931, Prng.fnv1a64("abc")
  end

  def test_the_signature_is_documented_as_unescaped
    assert Fuzz::Signature.duplicate_crash?("k", ["x:y"], "k", %w[x y])
    refute Fuzz::Signature.duplicate_crash?("k", %w[a], "k", %w[b])
    assert_raises(TypeError) { Fuzz::Signature.crash_signature(:k, []) }
    assert_raises(TypeError) { Fuzz::Signature.crash_signature("k", "x") }
    assert_raises(TypeError) { Fuzz::Signature.crash_signature("k", [1]) }
  end

  def test_frames_and_kinds_are_normalised_without_colons
    n = Fuzz::Signature
    assert_equal "a.rb#Foo#bar", n.normalise_frame("/x/y/a.rb", "Foo#bar")
    assert_equal "a.rb#block_in_Foo#bar", n.normalise_frame("/x/y/a.rb", "block in Foo#bar")
    assert_equal "a_b.rb#x__y", n.normalise_frame("/p/a:b.rb", "x::y")
    refute_includes n.normalise_frame("/p/a:b.rb", "x::y"), ":"
    assert_equal "file.rb#x", n.normalise_frame("C:\\path\\file.rb", "x")
    assert_equal "a.rb#x", n.normalise_frame("/p/a.rb///", "x")
    assert_equal "a.rb#x", n.normalise_frame("C:\\path\\a.rb\\\\", "x")
    assert_equal "C_#root", n.normalise_frame("C:\\", "root")
    assert_equal "/#root", n.normalise_frame("/", "root")
    assert_equal "#empty", n.normalise_frame("", "empty")
    assert_equal "Foo.Bar", n.normalise_kind("Foo::Bar")
    assert_equal "<anonymous>", n.normalise_kind(nil)
    assert_equal "a_b", n.normalise_kind("a b")
    assert_equal 64, n.normalise_kind("A" * 500).size
    assert_equal %w[a b a], n.collapse(%w[a a b b a])
    assert_equal 64, n.collapse((1..500).map(&:to_s)).size
  end

  # -- TAP ---------------------------------------------------------------------------

  def test_tap_format_and_closed_ids
    ids = %w[fuzz-crash fuzz-timeout fuzz-oom fuzz-leak fuzz-panic]
    assert_equal ids, Fuzz::FailureIds::ALL
    ids.each do |id|
      assert_equal "not ok 7 - ct_compare [#{id}:3a8f19b2]", Fuzz::Signature.format_tap_failure(7, "ct_compare", id, "3a8f19b2")
      assert Fuzz::FailureIds.fuzz_failure_id?(id)
    end
    [nil, 1, "fuzz-bogus", "FUZZ-CRASH", "fuzz-crash\n", :"fuzz-crash"].each do |id|
      refute Fuzz::FailureIds.fuzz_failure_id?(id), id.inspect
      assert_raises(ArgumentError) { Fuzz::Signature.format_tap_failure(1, "t", id, "3a8f19b2") } unless id.nil? || id == 1 || id.is_a?(Symbol)
    end
    ["3A8F19B2", "3a8f19b", "3a8f19b22", "", "3a8f19b\n", "zzzzzzzz"].each do |sig|
      assert_raises(ArgumentError) { Fuzz::Signature.format_tap_failure(1, "t", "fuzz-crash", sig) }
    end
    assert_raises(ArgumentError) { Fuzz::Signature.format_tap_failure(0, "t", "fuzz-crash", "3a8f19b2") }
    assert_raises(ArgumentError) { Fuzz::Signature.format_tap_failure(2**31, "t", "fuzz-crash", "3a8f19b2") }
    assert_raises(TypeError) { Fuzz::Signature.format_tap_failure("1", "t", "fuzz-crash", "3a8f19b2") }
    assert_raises(TypeError) { Fuzz::Signature.format_tap_failure(1, :t, "fuzz-crash", "3a8f19b2") }
  end

  def test_tap_targets_cannot_forge_lines_and_are_truncated_before_encoding
    line = Fuzz::Signature.format_tap_failure(1, "a\nok 2 - forged\r\x00\e[31m é", "fuzz-crash", "3a8f19b2")
    refute_match(/[\r\n\x00\e]/, line)
    assert_equal 1, line.lines.size
    assert_match(/\Anot ok 1 - [A-Za-z0-9._\/-]+ \[fuzz-crash:3a8f19b2\]\z/, line)
    long = Fuzz::Signature.format_tap_failure(1, "x" * 10_000_000, "fuzz-crash", "3a8f19b2")
    assert_equal "not ok 1 - #{"x" * 128} [fuzz-crash:3a8f19b2]", long
    # truncation happens at 128 BYTES before encoding: a multi-byte tail is cut, not expanded
    mb = Fuzz::Signature.format_tap_failure(1, "é" * 200, "fuzz-crash", "3a8f19b2")
    assert_equal "not ok 1 - #{"_" * 128} [fuzz-crash:3a8f19b2]", mb
    assert_equal "not ok 1 -  [fuzz-crash:3a8f19b2]", Fuzz::Signature.format_tap_failure(1, "", "fuzz-crash", "3a8f19b2")
  end

  def test_tap_failures_for_a_report
    crash = Fuzz::CrashRecord.new(signature: "4eca7ae8", kind: "panic", frames: %w[eval step], minimal_input: "x", hits: 2,
      minimized: true, input_dropped: false)
    report = Fuzz::Report.new(iterations: 5, admitted_seeds: 1, total_edges: 0, crashes: [crash], timeouts: 1, ooms: 2,
      crashes_dropped: 0, admissions_dropped: 0, coverage_tier: "black-box-uninstrumented", seed: 7, corpus: ["a"])
    assert_equal ["not ok 3 - t [fuzz-crash:4eca7ae8]", "not ok 4 - t [fuzz-timeout:4e61ea76]", "not ok 5 - t [fuzz-oom:7146a410]"],
      Fuzz::Signature.tap_failures(report, "t", 3)
    clean = Fuzz::Report.new(iterations: 5, admitted_seeds: 1, total_edges: 0, crashes: [], timeouts: 0, ooms: 0,
      crashes_dropped: 0, admissions_dropped: 0, coverage_tier: "x", seed: 7, corpus: [])
    assert_equal [], Fuzz::Signature.tap_failures(clean, "t")
    assert_predicate clean, :clean?
    refute_predicate report, :clean?
    assert_equal 1, report.unique_crashes
  end

  # -- inert reports -------------------------------------------------------------------

  def test_reports_and_records_show_no_bytes_seed_or_corpus
    secret = "SECRETBYTES-123"
    crash = Fuzz::CrashRecord.new(signature: "4eca7ae8", kind: "panic", frames: %w[eval], minimal_input: secret, hits: 1,
      minimized: true, input_dropped: false)
    report = Fuzz::Report.new(iterations: 1, admitted_seeds: 1, total_edges: 0, crashes: [crash], timeouts: 0, ooms: 0,
      crashes_dropped: 0, admissions_dropped: 0, coverage_tier: "x", seed: 987_654_321, corpus: [secret])
    opts = Fuzz::Options.new(seed: 987_654_321)
    outcome = Fuzz::Outcome.crash("K", ["f"])
    [crash, report, opts, outcome, Fuzz::Handle.new(1, 1, Fuzz::Handle::CampaignToken.new, 1, nil)].each do |obj|
      [obj.inspect, obj.to_s, String(obj)].each do |text|
        refute_includes text, secret
        refute_includes text, "987654321"
        refute_includes text, "3535" # hex of the secret prefix is not printed either
      end
    end
    assert_predicate report.crashes, :frozen?
    assert_predicate report.corpus, :frozen?
    assert(report.corpus.all?(&:frozen?))
    assert_predicate crash.frames, :frozen?
    assert_predicate crash.minimal_input, :frozen?
    assert_predicate report, :frozen?
    assert_equal secret, crash.minimal_input
    assert_equal 987_654_321, report.seed
  end

  def test_internal_objects_never_dump_bytes_or_rng_state
    secret = "SECRETBYTES"
    campaign = Fuzz::Campaign.new(->(_d, _c) { Fuzz::Outcome.pass }, Fuzz::Options.new(seed: 424_242), 1 << 20)
    bitmap = Fuzz::Bitmap.new
    bitmap.record(77)
    objects = [campaign, bitmap, Fuzz::SeedEntry.new(secret, 3, 1), Fuzz::CrashEntry.new("sig", "K", ["f"], secret, 1, true, false),
      Prng::SplitMix64.new(424_242)]
    objects.each do |o|
      [o.inspect, o.to_s].each do |text|
        refute_includes text, secret
        refute_includes text, "424242"
        refute_includes text, Prng::SplitMix64.new(424_242).state.to_s
        refute_includes text, "77\n"
      end
    end
  end

  def test_options_validation
    assert_equal 1000, Fuzz::Options.new.max_iterations
    {max_iterations: -1, max_input_len: 2**21, base_energy: -1, minimize_budget: 100_001, step_budget: 100_000_001, mem_budget: (1 << 30) + 1}.each do |k, v|
      assert_raises(ArgumentError, k.to_s) { Fuzz::Options.new(k => v) }
    end
    [:max_iterations, :seed, :max_input_len, :base_energy, :minimize_budget, :step_budget, :mem_budget].each do |k|
      assert_raises(TypeError, k.to_s) { Fuzz::Options.new(k => 1.0) }
      assert_raises(TypeError, k.to_s) { Fuzz::Options.new(k => true) }
    end
    assert Fuzz::Options.new(seed: -5).seed == -5
  end

  def test_outcome
    assert_predicate Fuzz::Outcome.pass, :pass?
    refute_predicate Fuzz::Outcome.timeout, :crash?
    assert_equal :oom, Fuzz::Outcome.oom.kind
    c = Fuzz::Outcome.crash("K", %w[a b])
    assert_equal "K", c.crash_kind
    assert_equal %w[a b], c.frames
    assert_predicate c, :frozen?
    assert_predicate c.frames, :frozen?
    assert_match(/crash K/, c.inspect)
  end
end
