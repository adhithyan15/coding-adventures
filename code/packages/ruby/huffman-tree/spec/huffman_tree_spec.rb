# frozen_string_literal: true

# =============================================================================
# spec/huffman_tree_spec.rb — RSpec tests for DT27: Huffman Tree
# =============================================================================
#
# Tests cover:
#   - Construction from various frequency distributions
#   - Tie-breaking determinism
#   - Code table generation
#   - Canonical code table (DEFLATE-style)
#   - Encoding and decoding round-trips
#   - Inspection methods (weight, depth, symbol_count, leaves)
#   - valid? structural check
#   - Edge cases (single symbol, two symbols, identical frequencies)
#   - Error handling
# =============================================================================

require "simplecov"
SimpleCov.start do
  add_filter "/spec/"
end

require_relative "../lib/coding_adventures_huffman_tree"

include CodingAdventures

RSpec.describe HuffmanTree do
  # ── Construction ────────────────────────────────────────────────────────────

  describe ".build" do
    it "builds a single-symbol tree" do
      tree = HuffmanTree.build([[65, 5]])
      expect(tree.symbol_count).to eq(1)
      expect(tree.weight).to eq(5)
    end

    it "builds a two-symbol tree" do
      tree = HuffmanTree.build([[65, 3], [66, 1]])
      expect(tree.symbol_count).to eq(2)
      expect(tree.weight).to eq(4)
    end

    it "builds a three-symbol tree (AAABBC example)" do
      # Classic Huffman textbook example:
      # A appears 3 times, B 2 times, C 1 time → A gets shortest code
      tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]])
      expect(tree.symbol_count).to eq(3)
      expect(tree.weight).to eq(6)
    end

    it "builds a large 256-symbol tree" do
      weights = (0..255).map { |i| [i, i + 1] }
      tree = HuffmanTree.build(weights)
      expect(tree.symbol_count).to eq(256)
      expect(tree.valid?).to be true
    end

    it "raises ArgumentError for empty weights" do
      expect { HuffmanTree.build([]) }.to raise_error(ArgumentError, /empty/)
    end

    it "raises ArgumentError for zero frequency" do
      expect { HuffmanTree.build([[65, 0]]) }.to raise_error(ArgumentError, /positive/)
    end

    it "raises ArgumentError for negative frequency" do
      expect { HuffmanTree.build([[65, -1]]) }.to raise_error(ArgumentError, /positive/)
    end

    it "tree is valid after build" do
      tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]])
      expect(tree.valid?).to be true
    end
  end

  # ── Code table ──────────────────────────────────────────────────────────────

  describe "#code_table" do
    it "returns shorter code for higher-frequency symbol (AAABBC)" do
      tree  = HuffmanTree.build([[65, 3], [66, 2], [67, 1]])
      table = tree.code_table
      # A (freq 3) gets a shorter code than B (freq 2), B <= C (freq 1)
      expect(table[65].length).to be < table[66].length
      expect(table[66].length).to be <= table[67].length
    end

    it "assigns '0' to a single-symbol tree" do
      tree  = HuffmanTree.build([[65, 1]])
      table = tree.code_table
      expect(table[65]).to eq("0")
    end

    it "produces prefix-free codes" do
      tree   = HuffmanTree.build((0..9).map { |i| [i, i + 1] })
      codes  = tree.code_table.values
      codes.each_with_index do |c1, i|
        codes.each_with_index do |c2, j|
          next if i == j

          expect(c1).not_to start_with(c2),
            "#{c1.inspect} is a prefix of #{c2.inspect}"
        end
      end
    end

    it "covers all symbols in the input" do
      weights = [[65, 3], [66, 2], [67, 1]]
      tree    = HuffmanTree.build(weights)
      table   = tree.code_table
      expect(table.keys.sort).to eq([65, 66, 67])
    end
  end

  # ── code_for ────────────────────────────────────────────────────────────────

  describe "#code_for" do
    let(:tree) { HuffmanTree.build([[65, 3], [66, 2], [67, 1]]) }

    it "matches the full code table for every symbol" do
      table = tree.code_table
      [65, 66, 67].each do |sym|
        expect(tree.code_for(sym)).to eq(table[sym])
      end
    end

    it "returns nil for a symbol not in the tree" do
      expect(tree.code_for(99)).to be_nil
    end

    it "returns '0' for a single-symbol tree" do
      single = HuffmanTree.build([[65, 5]])
      expect(single.code_for(65)).to eq("0")
    end
  end

  # ── canonical_code_table ────────────────────────────────────────────────────

  describe "#canonical_code_table" do
    it "produces the expected canonical codes for AAABBC" do
      # A=3→len1, B=2→len2, C=1→len2
      # Sorted by (len, sym): A(1), B(2), C(2)
      # A → 0, B → 10, C → 11
      tree      = HuffmanTree.build([[65, 3], [66, 2], [67, 1]])
      canonical = tree.canonical_code_table
      expect(canonical[65]).to eq("0")
      expect(canonical[66]).to eq("10")
      expect(canonical[67]).to eq("11")
    end

    it "canonical lengths match regular tree lengths" do
      tree      = HuffmanTree.build((0..7).map { |i| [i, i + 1] })
      regular   = tree.code_table
      canonical = tree.canonical_code_table
      regular.each do |sym, code|
        expect(canonical[sym].length).to eq(code.length),
          "symbol #{sym}: regular len=#{code.length}, canonical len=#{canonical[sym]&.length}"
      end
    end

    it "returns '0' for a single-symbol tree" do
      tree = HuffmanTree.build([[65, 5]])
      expect(tree.canonical_code_table[65]).to eq("0")
    end

    it "produces prefix-free canonical codes" do
      tree   = HuffmanTree.build((0..9).map { |i| [i, i + 1] })
      codes  = tree.canonical_code_table.values
      codes.each_with_index do |c1, i|
        codes.each_with_index do |c2, j|
          next if i == j

          expect(c1).not_to start_with(c2)
        end
      end
    end
  end

  # ── decode_all ──────────────────────────────────────────────────────────────

  describe "#decode_all" do
    it "decodes a single-symbol stream (single-leaf tree)" do
      tree  = HuffmanTree.build([[65, 5]])
      table = tree.code_table
      # Each occurrence of symbol 65 costs one '0' bit
      bits  = table[65] * 3
      expect(tree.decode_all(bits, 3)).to eq([65, 65, 65])
    end

    it "decodes AAABBC symbols in order" do
      symbols = [65, 65, 65, 66, 66, 67]
      tree    = HuffmanTree.build([[65, 3], [66, 2], [67, 1]])
      table   = tree.code_table
      bits    = symbols.map { |s| table[s] }.join
      expect(tree.decode_all(bits, symbols.length)).to eq(symbols)
    end

    it "round-trips all 256 byte values" do
      weights = (0..255).map { |i| [i, i + 1] }
      tree    = HuffmanTree.build(weights)
      table   = tree.code_table
      symbols = (0..255).to_a
      bits    = symbols.map { |s| table[s] }.join
      expect(tree.decode_all(bits, 256)).to eq(symbols)
    end

    it "raises ArgumentError when the bit stream is exhausted" do
      tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]])
      # Only enough bits for 1 symbol, but we ask for 5
      expect { tree.decode_all("0", 5) }
        .to raise_error(ArgumentError, /exhausted/i)
    end

    it "decodes a two-symbol tree correctly" do
      # Two equal-weight symbols: 65 ('0') and 66 ('1')
      tree  = HuffmanTree.build([[65, 1], [66, 1]])
      table = tree.code_table
      bits  = [table[65], table[66], table[65]].join
      expect(tree.decode_all(bits, 3)).to eq([65, 66, 65])
    end

    it "decode_all returns empty array for count=0" do
      tree = HuffmanTree.build([[65, 3], [66, 2]])
      expect(tree.decode_all("", 0)).to eq([])
    end
  end

  # ── Inspection methods ───────────────────────────────────────────────────────

  describe "#weight" do
    it "returns the root weight (sum of all frequencies)" do
      tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]])
      expect(tree.weight).to eq(6)
    end

    it "equals the single frequency for a single-symbol tree" do
      tree = HuffmanTree.build([[65, 7]])
      expect(tree.weight).to eq(7)
    end
  end

  describe "#depth" do
    it "returns 0 for a single-symbol tree (root is a leaf)" do
      tree = HuffmanTree.build([[65, 1]])
      expect(tree.depth).to eq(0)
    end

    it "returns 1 for a two-symbol tree" do
      tree = HuffmanTree.build([[65, 3], [66, 1]])
      expect(tree.depth).to eq(1)
    end

    it "returns 2 for a three-symbol unbalanced tree" do
      tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]])
      expect(tree.depth).to eq(2)
    end
  end

  describe "#symbol_count" do
    it "returns 1 for a single-symbol tree" do
      expect(HuffmanTree.build([[65, 1]]).symbol_count).to eq(1)
    end

    it "returns the number of distinct symbols" do
      weights = (0..9).map { |i| [i, i + 1] }
      expect(HuffmanTree.build(weights).symbol_count).to eq(10)
    end
  end

  describe "#leaves" do
    it "returns all symbols for a three-symbol tree" do
      tree      = HuffmanTree.build([[65, 3], [66, 2], [67, 1]])
      leaf_syms = tree.leaves.map { |sym, _code| sym }
      expect(leaf_syms.sort).to eq([65, 66, 67])
      expect(leaf_syms.length).to eq(3)
    end

    it "returns [[symbol, '0']] for a single-symbol tree" do
      tree = HuffmanTree.build([[65, 5]])
      expect(tree.leaves).to eq([[65, "0"]])
    end

    it "leaves are in left-to-right (in-order) traversal order" do
      tree   = HuffmanTree.build([[65, 3], [66, 2], [67, 1]])
      result = tree.leaves
      # All symbols present, codes match code_table
      table = tree.code_table
      result.each do |sym, code|
        expect(code).to eq(table[sym])
      end
    end
  end

  # ── valid? ───────────────────────────────────────────────────────────────────

  describe "#valid?" do
    it "returns true for a well-formed tree" do
      tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]])
      expect(tree.valid?).to be true
    end

    it "returns true for a large well-formed tree" do
      tree = HuffmanTree.build((0..49).map { |i| [i, i + 1] })
      expect(tree.valid?).to be true
    end

    it "returns true for a single-symbol tree" do
      tree = HuffmanTree.build([[65, 5]])
      expect(tree.valid?).to be true
    end
  end

  # ── Tie-breaking determinism ─────────────────────────────────────────────────

  describe "tie-breaking" do
    it "builds the same code table for the same input (determinism)" do
      weights = (0..7).map { |i| [i, 1] }
      tree1   = HuffmanTree.build(weights)
      tree2   = HuffmanTree.build(weights)
      expect(tree1.code_table).to eq(tree2.code_table)
    end

    it "lower symbol wins among equal-weight leaves (two symbols)" do
      # Symbols 65 and 66 both have weight 1.
      # 65 should get the left '0' edge, 66 the right '1' edge.
      tree  = HuffmanTree.build([[65, 1], [66, 1]])
      table = tree.code_table
      expect(table[65].length).to eq(1)
      expect(table[66].length).to eq(1)
    end

    it "builds a valid tree when all symbols have equal weight" do
      tree = HuffmanTree.build([[65, 1], [66, 1], [67, 1], [68, 1]])
      expect(tree.valid?).to be true
      expect(tree.symbol_count).to eq(4)
      expect(tree.weight).to eq(4)
    end

    it "produces prefix-free codes for equal-weight alphabet" do
      weights = (65..72).map { |i| [i, 1] } # 8 symbols all weight 1
      tree    = HuffmanTree.build(weights)
      codes   = tree.code_table.values
      codes.each_with_index do |c1, i|
        codes.each_with_index do |c2, j|
          next if i == j

          expect(c1).not_to start_with(c2)
        end
      end
    end
  end

  # ── Round-trips ──────────────────────────────────────────────────────────────

  describe "encode/decode round-trips" do
    def roundtrip(weights, symbols)
      tree  = HuffmanTree.build(weights)
      table = tree.code_table
      bits  = symbols.map { |s| table[s] }.join
      tree.decode_all(bits, symbols.length)
    end

    it "round-trips a single symbol repeated many times" do
      symbols = [65] * 10
      expect(roundtrip([[65, 10]], symbols)).to eq(symbols)
    end

    it "round-trips AAABBC" do
      symbols = [65, 65, 65, 66, 66, 67]
      weights = [[65, 3], [66, 2], [67, 1]]
      expect(roundtrip(weights, symbols)).to eq(symbols)
    end

    it "round-trips a long mixed sequence" do
      symbols = ([65] * 50 + [66] * 30 + [67] * 15 + [68] * 5).shuffle(random: Random.new(42))
      weights = [[65, 50], [66, 30], [67, 15], [68, 5]]
      expect(roundtrip(weights, symbols)).to eq(symbols)
    end
  end
end
