# frozen_string_literal: true

require_relative "test_helper"

class TestTfv < HookedTest
  T = Tfv

  def valid(text) = assert(T.valid?(text), text.inspect)

  def invalid(text, message = nil)
    refute T.valid?(text), text.inspect
    return unless message

    assert_match(message, assert_raises(T::InvalidTfvError) { T.validate(text) }.message)
  end

  def doc(*lines) = "tfv 1\n#{lines.join("\n")}\n"

  def test_the_committed_tfv_is_valid
    path = File.join(File.dirname(Fx.fixture_path), "cases.tfv")
    valid(File.binread(path))
  end

  def test_minimal_documents
    valid("tfv 1")
    valid("tfv 1\n")
    valid(doc("# comment", "", "r R0"))
    valid(doc("r R2 Kop S706274 Kdata X00ff"))
    valid(doc("a/b.c_d-9 R2 Kx I-5 Ky N"))
  end

  def test_values
    valid(doc("r R1 Kv T"))
    valid(doc("r R1 Kv F"))
    valid(doc("r R1 Kv I0"))
    valid(doc("r R1 Kv I-12"))
    valid(doc("r R1 Kv Dffffffffffffffff"))
    valid(doc("r R1 Kv D3ff0000000000000"))
    valid(doc("r R1 Kv X"))
    valid(doc("r R1 Kv S"))
    valid(doc("r R1 Kv S68c3a9"))
    valid(doc("r R1 Kv L2 I1 I2"))
    valid(doc("r R1 Kv L0"))
    valid(doc("r R1 Kv M2 I1 T I2 F"))
    valid(doc("r R1 Kv M1 L1 I1 L0"))
  end

  def test_rejections
    invalid("bad header\n", /first line/)
    invalid("", /first line/)
    invalid("tfv 2\n", /first line/)
    invalid(" tfv 1\n", /first line/)
    invalid("tfv 1\nr R2 Kop S706274 Kdata XFF\n", /bytes need/)
    invalid("tfv 1\nr R1 Kop S706274 Kdata XFF\n", /trailing/)
    invalid(doc("r R1 Kv X0"), /even-length/)
    invalid(doc("r R1 Kv S0"), /even-length/)
    invalid(doc("r R1 Kv Sff"), /UTF-8/)
    invalid(doc("r R1 Kv I-0"), /non-canonical/)
    invalid(doc("r R1 Kv I007"), /non-canonical/)
    invalid(doc("r R1 Kv I"), /empty integer/)
    invalid(doc("r R1 Kv I-"), /empty integer/)
    invalid(doc("r R1 Kv I1x"), /non-digit/)
    invalid(doc("r R1 Kv I#{"9" * 1001}"), /digits exceeds/)
    valid(doc("r R1 Kv I#{"9" * 1000}"))
    invalid(doc("r R1 Kv Dff"), /16 lowercase/)
    invalid(doc("r R1 Kv DFFFFFFFFFFFFFFFF"), /16 lowercase/)
    invalid(doc("r R1 Kv Nx"), /unexpected body/)
    invalid(doc("r R1 Kv Q"), /unknown tag/)
    invalid(doc("r R1 Kv L2 I1"), /ends early/)
    invalid(doc("r R1 Kv L1 I1 I2"), /trailing/)
    invalid(doc("r R2 Kv I1"), /ends early/)
    invalid(doc("r R1 Kv I1 Kw I2"), /trailing/)
    invalid(doc("r R1 Kv M1 I1"), /ends early/)
    invalid(doc("r R1 Kv L01 I1"), /leading zero/)
    invalid(doc("r R1 Kv L I1"), /empty count/)
    invalid(doc("r R1 Kv Lx"), /non-digit/)
    invalid(doc("r R1 Kv L9999999999"), /too large/)
    invalid(doc("r R1 Kv L999999999"), /ends early/) # no allocation from the count
    invalid(doc("r R0 extra"), /trailing/)
    invalid(doc("r R1 Kv  I1"), /doubled space/)
    invalid(doc("r R1 Kv I1 "), /doubled space/)
    invalid(doc("r R1 Kv I1\r"), /carriage return/)
    invalid(doc("r"), /needs an id/)
    invalid(doc("r X1"), /expected R<n>/)
    invalid(doc("r R"), /empty count/)
    invalid(doc("r! R0"), /bad record id/)
    invalid(doc("r R1 Vv I1"), /bad field key/)
    invalid(doc("r R1 KV I1"), /bad field key/)
    invalid(doc("r R1 K I1"), /bad field key/)
    invalid(doc("r R2 Kv I1 Kv I2"), /duplicate field/)
    invalid(doc("r R0", "r R0"), /duplicate record id/)
    invalid(doc("r R1 Kv M2 I1 T I1 F"), /duplicate map key/)
    invalid(doc("r R1 Kv M2 L1 I1 T L1 I1 F"), /duplicate map key/)
    invalid(doc("r R1 Kv M2 Dffffffffffffffff T D7ff8000000000001 F"), /duplicate map key/) # every NaN equals every NaN
    valid(doc("r R1 Kv M2 D0000000000000000 T D8000000000000000 F")) # +0 and -0 differ
  end

  def test_map_key_equality_ignores_entry_order
    invalid(doc("r R1 Kv M2 M2 I1 T I2 F T M2 I2 F I1 T F"), /duplicate map key/)
  end

  def test_nesting_depth_cap
    nest = ->(d) { doc("r R1 Kv #{"L1 " * d}I1") }
    valid(nest.call(64))
    invalid(nest.call(65), /nesting deeper/)
  end

  def test_document_level_rules
    invalid("tfv 1\n\xff\n".b, /UTF-8/)
    invalid("tfv 1\n" + ("# #{"x" * 100}\n" * 20_000), /exceeds/)
    invalid(5, /must be a String/)
    assert_match(/line 2:/, assert_raises(T::InvalidTfvError) { T.validate(doc("r R1 Kv Q")) }.message)
    assert_equal "\"#{"a" * 64}...\"", T.q("a" * 70)
    assert_equal '"a\\"b\\\\c"', T.q('a"b\\c')
  end

  def test_the_two_corpus_reject_texts_are_rejected
    Fx.cases.select { |c| c["op"] == "corpus_reject" }.each do |c|
      refute T.valid?(c["text"]), c["id"]
    end
  end
end
