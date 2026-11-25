# frozen_string_literal: true

require_relative "test_helper"

# The 820 differential vectors of the Go lane, computed by the independent Python
# oracle `scripts/fuzz_conformance.py` (generator:
# `go/test-bootstrap/fuzz/testdata/gen_reference_vectors.py`).  The Go table is a
# committed source file, so it is read from its committed path: there is no copy.
#
# fuzz-v1 alone cannot pin these (VER06i section 9 item 10): its splice has equal
# splits and its mutate_sequence case does not pin every constant.
class TestFuzzVectors < Minitest::Test
  include CodingAdventures::TestBootstrap

  ROW = /\A\t\{"([a-z_]+)", \[\]string\{(.*)\}, "([0-9a-f]*)"\},\z/
  OPS = %w[bit_flip byte_flip arithmetic boundary block_delete block_insert block_duplicate splice sequence].freeze

  def self.load_vectors
    path = File.join(Fx.repo_root, "code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go")
    lines = File.readlines(path, chomp: true)
    start = lines.index("var referenceVectors = []refVector{")
    raise "the vector table moved" unless start

    lines[(start + 1)..].take_while { |l| l != "}" }.each_with_index.map do |line, i|
      m = ROW.match(line) or raise "unparseable vector row #{start + i + 2}: #{line}"
      [m[1], m[2].scan(/"([^"]*)"/).flatten, m[3]]
    end
  end

  def test_all_820_vectors_match
    vectors = self.class.load_vectors
    assert_equal 820, vectors.size
    counts = Hash.new(0)
    vectors.each_with_index do |(op, a, out), i|
      counts[op] += 1
      m = Fuzz::Mutate
      got =
        case op
        when "bit_flip" then m.bit_flip(Fx.hex(a[0]), a[1].to_i, a[2].to_i)
        when "byte_flip" then m.byte_flip(Fx.hex(a[0]), a[1].to_i, a[2].to_i)
        when "arithmetic" then m.byte_arithmetic(Fx.hex(a[0]), a[1].to_i, a[2].to_i, a[3].to_i, a[4])
        when "boundary" then m.boundary_insert(Fx.hex(a[0]), a[1].to_i, a[2].to_i, a[3].to_i, a[4])
        when "block_delete" then m.block_delete(Fx.hex(a[0]), a[1].to_i, a[2].to_i)
        when "block_insert" then m.block_insert(Fx.hex(a[0]), a[1].to_i, Fx.hex(a[2]))
        when "block_duplicate" then m.block_duplicate(Fx.hex(a[0]), a[1].to_i, a[2].to_i, a[3].to_i)
        when "splice" then m.splice(Fx.hex(a[0]), Fx.hex(a[1]), a[2].to_i, a[3].to_i)
        when "sequence" then m.mutate_sequence(Fx.hex(a[0]), a[1].to_i, a[2].to_i, a[3].to_i)
        else flunk "unknown vector op #{op}"
        end
      assert_equal out, got.unpack1("H*"), "vector #{i} #{op} #{a}"
    end
    OPS.each { |op| assert_operator counts[op], :>=, 50, "only #{counts[op]} vectors for #{op}" }
  end
end
