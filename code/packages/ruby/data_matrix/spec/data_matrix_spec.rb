# frozen_string_literal: true

require "simplecov"
SimpleCov.start

require "coding_adventures/data_matrix"

RSpec.describe CodingAdventures::DataMatrix do
  # ---------------------------------------------------------------------------
  # VERSION
  # ---------------------------------------------------------------------------

  describe "VERSION" do
    it "is 0.1.0" do
      expect(described_class::VERSION).to eq("0.1.0")
    end
  end

  # ---------------------------------------------------------------------------
  # Public constants
  # ---------------------------------------------------------------------------

  describe "public constants" do
    it "exposes GF256_PRIME = 0x12D" do
      expect(described_class::GF256_PRIME).to eq(0x12D)
      expect(described_class::GF256_PRIME).to eq(301)
    end

    it "exposes MIN_SIZE = 10" do
      expect(described_class::MIN_SIZE).to eq(10)
    end

    it "exposes MAX_SIZE = 144" do
      expect(described_class::MAX_SIZE).to eq(144)
    end

    it "exposes MAX_DATA_CW = 1558" do
      expect(described_class::MAX_DATA_CW).to eq(1558)
    end

    it "GF256_PRIME differs from QR Code's polynomial 0x11D" do
      expect(described_class::GF256_PRIME).not_to eq(0x11D)
    end
  end

  # ---------------------------------------------------------------------------
  # Error hierarchy
  # ---------------------------------------------------------------------------

  describe "DataMatrixError" do
    it "is a StandardError subclass" do
      e = described_class::DataMatrixError.new("boom")
      expect(e).to be_a(StandardError)
      expect(e.message).to eq("boom")
    end
  end

  describe "InputTooLongError" do
    it "is a DataMatrixError subclass" do
      e = described_class::InputTooLongError.new("too big")
      expect(e).to be_a(described_class::DataMatrixError)
      expect(e).to be_a(StandardError)
    end
  end

  describe "InvalidSizeError" do
    it "is a DataMatrixError subclass" do
      e = described_class::InvalidSizeError.new("bad size")
      expect(e).to be_a(described_class::DataMatrixError)
      expect(e).to be_a(StandardError)
    end
  end

  # ---------------------------------------------------------------------------
  # ModuleGrid struct
  # ---------------------------------------------------------------------------

  describe "ModuleGrid" do
    it "responds to rows, cols, and modules" do
      grid = described_class::ModuleGrid.new(10, 20, [])
      expect(grid).to respond_to(:rows)
      expect(grid).to respond_to(:cols)
      expect(grid).to respond_to(:modules)
      expect(grid.rows).to eq(10)
      expect(grid.cols).to eq(20)
    end
  end

  # ---------------------------------------------------------------------------
  # Symbol size tables
  # ---------------------------------------------------------------------------

  describe "SQUARE_SIZES" do
    it "has 24 entries" do
      expect(described_class::SQUARE_SIZES.length).to eq(24)
    end

    it "smallest is 10×10" do
      e = described_class::SQUARE_SIZES.first
      expect(e.symbol_rows).to eq(10)
      expect(e.symbol_cols).to eq(10)
    end

    it "largest is 144×144" do
      e = described_class::SQUARE_SIZES.last
      expect(e.symbol_rows).to eq(144)
      expect(e.symbol_cols).to eq(144)
    end

    it "all entries are frozen" do
      described_class::SQUARE_SIZES.each do |e|
        expect(e).to be_frozen
      end
    end

    it "data_cw values are strictly increasing" do
      caps = described_class::SQUARE_SIZES.map(&:data_cw)
      expect(caps).to eq(caps.sort)
    end
  end

  describe "RECT_SIZES" do
    it "has 6 entries" do
      expect(described_class::RECT_SIZES.length).to eq(6)
    end

    it "smallest is 8×18" do
      e = described_class::RECT_SIZES.first
      expect(e.symbol_rows).to eq(8)
      expect(e.symbol_cols).to eq(18)
    end

    it "largest is 16×48" do
      e = described_class::RECT_SIZES.last
      expect(e.symbol_rows).to eq(16)
      expect(e.symbol_cols).to eq(48)
    end
  end

  # ---------------------------------------------------------------------------
  # GF(256)/0x12D arithmetic
  # ---------------------------------------------------------------------------

  describe ".gf_mul" do
    it "returns 0 when either operand is 0" do
      expect(described_class.gf_mul(0, 5)).to eq(0)
      expect(described_class.gf_mul(7, 0)).to eq(0)
      expect(described_class.gf_mul(0, 0)).to eq(0)
    end

    it "multiplying any element by 1 returns itself" do
      [1, 2, 7, 42, 127, 255].each do |v|
        expect(described_class.gf_mul(v, 1)).to eq(v)
      end
    end

    it "is commutative" do
      expect(described_class.gf_mul(7, 13)).to eq(described_class.gf_mul(13, 7))
      expect(described_class.gf_mul(42, 99)).to eq(described_class.gf_mul(99, 42))
    end

    it "stays within GF(256) range 0..255" do
      (1..20).each do |a|
        (1..20).each do |b|
          result = described_class.gf_mul(a, b)
          expect(result).to be_between(0, 255)
        end
      end
    end

    it "GF_EXP / GF_LOG tables are internally consistent" do
      # For every non-zero v, α^{log(v)} should equal v.
      (1..255).each do |v|
        log_v = described_class::GF_LOG[v]
        expect(described_class::GF_EXP[log_v]).to eq(v)
      end
    end

    it "GF_EXP[0] = 1 (α^0 = 1)" do
      expect(described_class::GF_EXP[0]).to eq(1)
    end

    it "GF_EXP[1] = 2 (α = 2 is the generator)" do
      expect(described_class::GF_EXP[1]).to eq(2)
    end

    it "GF_EXP[255] = 1 (multiplicative group has order 255)" do
      expect(described_class::GF_EXP[255]).to eq(1)
    end
  end

  # ---------------------------------------------------------------------------
  # RS generator polynomial
  # ---------------------------------------------------------------------------

  describe ".build_generator" do
    it "produces a polynomial of length n_ecc + 1" do
      [5, 7, 10, 12, 14].each do |n|
        g = described_class.build_generator(n)
        expect(g.length).to eq(n + 1), "n_ecc #{n}: expected #{n + 1} coefficients, got #{g.length}"
      end
    end

    it "is monic — leading coefficient is 1" do
      [5, 7, 10, 12].each do |n|
        g = described_class.build_generator(n)
        expect(g.first).to eq(1)
      end
    end

    it "all coefficients are in GF(256) range 0..255" do
      g = described_class.build_generator(10)
      g.each { |coeff| expect(coeff).to be_between(0, 255) }
    end

    it "is deterministic and cached" do
      g1 = described_class.build_generator(7)
      g2 = described_class.build_generator(7)
      expect(g1).to equal(g2)  # same object (from cache)
    end
  end

  # ---------------------------------------------------------------------------
  # RS block encoder
  # ---------------------------------------------------------------------------

  describe ".rs_encode_block" do
    it "produces exactly n_ecc bytes" do
      gen = described_class.build_generator(10)
      ecc = described_class.rs_encode_block([66, 129, 70], gen)
      expect(ecc.length).to eq(10)
    end

    it "all ECC bytes are in 0..255" do
      gen = described_class.build_generator(10)
      ecc = described_class.rs_encode_block([100, 200, 150], gen)
      ecc.each { |b| expect(b).to be_between(0, 255) }
    end

    it "is deterministic for the same input" do
      gen = described_class.build_generator(7)
      data = [66, 129, 70, 10, 20]
      ecc1 = described_class.rs_encode_block(data, gen)
      ecc2 = described_class.rs_encode_block(data, gen)
      expect(ecc1).to eq(ecc2)
    end

    it "changes when data changes" do
      gen = described_class.build_generator(7)
      ecc1 = described_class.rs_encode_block([1, 2, 3], gen)
      ecc2 = described_class.rs_encode_block([4, 5, 6], gen)
      expect(ecc1).not_to eq(ecc2)
    end
  end

  # ---------------------------------------------------------------------------
  # ASCII encoding
  # ---------------------------------------------------------------------------

  describe ".encode_ascii" do
    it "maps a single ASCII letter to char+1" do
      expect(described_class.encode_ascii("A".bytes)).to eq([66])  # 65+1
    end

    it "maps space to 33" do
      expect(described_class.encode_ascii(" ".bytes)).to eq([33])  # 32+1
    end

    it "compacts two digit chars into one codeword: 130 + d1*10 + d2" do
      expect(described_class.encode_ascii("12".bytes)).to eq([142])  # 130+12
      expect(described_class.encode_ascii("00".bytes)).to eq([130])  # 130+0
      expect(described_class.encode_ascii("99".bytes)).to eq([229])  # 130+99
    end

    it "compacts consecutive digit pairs separately" do
      # "1234" → two digit-pair codewords:
      #   "12" → 130 + 1*10 + 2 = 142
      #   "34" → 130 + 3*10 + 4 = 164
      expect(described_class.encode_ascii("1234".bytes)).to eq([142, 164])
    end

    it "does NOT compact a digit pair when second char is not a digit" do
      # "1A" → '1' alone (50), 'A' → (66)
      expect(described_class.encode_ascii("1A".bytes)).to eq([50, 66])
    end

    it "encodes extended ASCII via UPPER_SHIFT (235) then (byte - 127)" do
      # 0xFF = 255: UPPER_SHIFT=235, then 255-127=128
      result = described_class.encode_ascii([0xFF])
      expect(result).to eq([235, 128])
    end

    it "encodes empty input to empty array" do
      expect(described_class.encode_ascii([])).to eq([])
    end
  end

  # ---------------------------------------------------------------------------
  # Pad codewords
  # ---------------------------------------------------------------------------

  describe ".pad_codewords" do
    it "pads to exactly data_cw bytes" do
      padded = described_class.pad_codewords([66], 3)
      expect(padded.length).to eq(3)
    end

    it "first pad byte is always 129" do
      padded = described_class.pad_codewords([66], 3)
      expect(padded[1]).to eq(129)
    end

    it "subsequent pad bytes are scrambled" do
      padded = described_class.pad_codewords([66], 4)
      # k=3 for third byte: 129 + (149*3 mod 253) + 1
      expected = 129 + (149 * 3) % 253 + 1
      expected -= 254 if expected > 254
      expect(padded[2]).to eq(expected)
    end

    it "does not pad when already at capacity" do
      data = [66, 67, 68]
      padded = described_class.pad_codewords(data, 3)
      expect(padded).to eq(data)
    end

    it "first pad for encode('A') into 10×10 is 129" do
      # encode_ascii("A") → [66]; 10×10 data_cw = 3; first pad at index 1 = 129
      cws = described_class.encode_ascii("A".bytes)
      padded = described_class.pad_codewords(cws, 3)
      expect(padded[1]).to eq(129)
    end
  end

  # ---------------------------------------------------------------------------
  # Symbol selection
  # ---------------------------------------------------------------------------

  describe ".select_symbol" do
    it "returns 10×10 for 1 codeword with shape :square" do
      entry = described_class.select_symbol(1, :square)
      expect(entry.symbol_rows).to eq(10)
      expect(entry.symbol_cols).to eq(10)
    end

    it "returns a square symbol that can hold the codewords" do
      [1, 3, 8, 12, 18, 22, 36, 44].each do |count|
        entry = described_class.select_symbol(count, :square)
        expect(entry.data_cw).to be >= count
      end
    end

    it "returns the smallest fitting symbol" do
      # 3 codewords fits in 10×10 (data_cw=3), not 12×12 (data_cw=5)
      entry = described_class.select_symbol(3, :square)
      expect(entry.symbol_rows).to eq(10)
    end

    it "raises InputTooLongError when no symbol is large enough" do
      expect { described_class.select_symbol(9999, :square) }
        .to raise_error(described_class::InputTooLongError)
    end

    it "considers only rectangular symbols when shape: :rectangle" do
      entry = described_class.select_symbol(1, :rectangle)
      # Smallest rect is 8×18
      expect(entry.symbol_rows).to eq(8)
      expect(entry.symbol_cols).to eq(18)
    end

    it "considers both shapes when shape: :any" do
      # With :any, should pick same or smaller than :square for small inputs
      sq = described_class.select_symbol(1, :square)
      any = described_class.select_symbol(1, :any)
      expect(sq.data_cw * sq.symbol_rows * sq.symbol_cols)
        .to be >= any.data_cw * any.symbol_rows * any.symbol_cols
    end
  end

  # ---------------------------------------------------------------------------
  # apply_wrap
  # ---------------------------------------------------------------------------

  describe ".apply_wrap" do
    let(:nr) { 8 }
    let(:nc) { 8 }

    it "rule 1: row < 0 AND col == 0 → (1, 3)" do
      expect(described_class.apply_wrap(-1, 0, nr, nc)).to eq([1, 3])
    end

    it "rule 2: row < 0 AND col == n_cols → (0, col-2)" do
      expect(described_class.apply_wrap(-1, nc, nr, nc)).to eq([0, nc - 2])
    end

    it "rule 3: row < 0 (general) → (row+n_rows, col-4)" do
      expect(described_class.apply_wrap(-1, 3, nr, nc)).to eq([nr - 1, -1])
    end

    it "rule 4: col < 0 → (row-4, col+n_cols)" do
      expect(described_class.apply_wrap(5, -1, nr, nc)).to eq([1, nc - 1])
    end

    it "no wrap when in bounds" do
      expect(described_class.apply_wrap(3, 3, nr, nc)).to eq([3, 3])
    end
  end

  # ---------------------------------------------------------------------------
  # encode — the main API
  # ---------------------------------------------------------------------------

  describe ".encode" do
    # -- Basic return shape ----------------------------------------------------

    it "returns a ModuleGrid for single-byte input" do
      grid = described_class.encode("A")
      expect(grid).to be_a(described_class::ModuleGrid)
      expect(grid).to respond_to(:rows)
      expect(grid).to respond_to(:cols)
      expect(grid).to respond_to(:modules)
    end

    it "encode('A') produces a 10×10 symbol (smallest possible)" do
      grid = described_class.encode("A")
      expect(grid.rows).to eq(10)
      expect(grid.cols).to eq(10)
    end

    it "grid.modules has exactly grid.rows rows" do
      grid = described_class.encode("Hello, World!")
      expect(grid.modules.length).to eq(grid.rows)
    end

    it "every row of modules has exactly grid.cols columns" do
      grid = described_class.encode("Hello, World!")
      grid.modules.each do |row|
        expect(row.length).to eq(grid.cols)
      end
    end

    it "all modules are Booleans (true or false)" do
      grid = described_class.encode("Hello")
      flat = grid.modules.flatten
      flat.each { |m| expect(m).to eq(true).or(eq(false)) }
    end

    it "rows is a positive integer" do
      grid = described_class.encode("Hello")
      expect(grid.rows).to be_a(Integer)
      expect(grid.rows).to be > 0
    end

    it "cols is a positive integer" do
      grid = described_class.encode("Hello")
      expect(grid.cols).to be_a(Integer)
      expect(grid.cols).to be > 0
    end

    # -- L-finder pattern -------------------------------------------------------

    it "bottom row is entirely dark (L-finder horizontal leg)" do
      grid = described_class.encode("A")
      last_row = grid.modules[grid.rows - 1]
      expect(last_row.all? { |m| m == true }).to be true
    end

    it "left column is entirely dark (L-finder vertical leg)" do
      grid = described_class.encode("A")
      grid.modules.each_with_index do |row, r|
        expect(row[0]).to be(true), "row #{r}: left column module should be dark"
      end
    end

    it "top row alternates dark/light starting dark (timing clock) — inner columns" do
      grid = described_class.encode("A")
      # The top row alternates: even col = dark, odd col = light.
      # Exception: the top-right corner (col = C-1) is set dark by the right-column
      # timing rule (row 0 of the right col is always dark), overriding the top row.
      # We test all columns EXCEPT the last (C-1) to avoid that corner intersection.
      top_row = grid.modules[0]
      (0...(grid.cols - 1)).each do |c|
        if c.even?
          expect(top_row[c]).to be(true),  "top row col #{c} (even) should be dark"
        else
          expect(top_row[c]).to be(false), "top row col #{c} (odd) should be light"
        end
      end
    end

    it "right column alternates dark/light starting dark (timing clock) — inner rows" do
      grid = described_class.encode("A")
      # The right column alternates: even row = dark, odd row = light.
      # Exception: the bottom-right corner (row = R-1) is set dark by the L-finder
      # bottom row (all dark), overriding the right column timing.
      # We test all rows EXCEPT the last (R-1) to avoid that corner intersection.
      (0...(grid.rows - 1)).each do |r|
        if r.even?
          expect(grid.modules[r][grid.cols - 1]).to be(true),
            "right col row #{r} (even) should be dark"
        else
          expect(grid.modules[r][grid.cols - 1]).to be(false),
            "right col row #{r} (odd) should be light"
        end
      end
    end

    # -- Empty string -----------------------------------------------------------

    it "encodes an empty string without raising" do
      expect { described_class.encode("") }.not_to raise_error
    end

    it "empty string produces a valid 10×10 ModuleGrid" do
      grid = described_class.encode("")
      expect(grid.rows).to eq(10)
      expect(grid.cols).to eq(10)
    end

    # -- Determinism ------------------------------------------------------------

    it "is deterministic — same input always produces same modules" do
      g1 = described_class.encode("Hello, DataMatrix!")
      g2 = described_class.encode("Hello, DataMatrix!")
      expect(g1.modules).to eq(g2.modules)
    end

    it "produces different grids for different inputs" do
      g1 = described_class.encode("AAAA")
      g2 = described_class.encode("BBBB")
      expect(g1.modules).not_to eq(g2.modules)
    end

    # -- Larger input → bigger symbol -------------------------------------------

    it "larger input produces a larger or equal symbol" do
      small = described_class.encode("A")
      large = described_class.encode("A" * 50)
      total_small = small.rows * small.cols
      total_large = large.rows * large.cols
      expect(total_large).to be >= total_small
    end

    it "very large input produces a much larger symbol" do
      small = described_class.encode("Hi")
      large = described_class.encode("A" * 200)
      expect(large.rows).to be > small.rows
    end

    # -- size option ------------------------------------------------------------

    it "accepts explicit size: [18, 18]" do
      grid = described_class.encode("A", size: [18, 18])
      expect(grid.rows).to eq(18)
      expect(grid.cols).to eq(18)
    end

    it "raises InvalidSizeError for a non-existent size" do
      expect { described_class.encode("A", size: [11, 11]) }
        .to raise_error(described_class::InvalidSizeError)
    end

    it "raises InputTooLongError when input does not fit the forced size" do
      # 10×10 holds 3 codewords; "Hello World" needs more
      expect { described_class.encode("Hello World", size: [10, 10]) }
        .to raise_error(described_class::InputTooLongError)
    end

    # -- shape option -----------------------------------------------------------

    it "accepts shape: :square (default)" do
      grid = described_class.encode("Hi", shape: :square)
      expect(grid.rows).to eq(grid.cols)  # square symbol
    end

    it "accepts shape: :rectangle" do
      grid = described_class.encode("Hi", shape: :rectangle)
      # smallest rect is 8×18
      expect(grid.rows).to eq(8)
      expect(grid.cols).to eq(18)
    end

    it "accepts shape: :any" do
      expect { described_class.encode("Hi", shape: :any) }.not_to raise_error
    end

    # -- Error on overly long input ---------------------------------------------

    it "raises InputTooLongError for extremely long input" do
      # More than 1558 data codewords. A long ASCII string where each char is
      # one codeword: 1600 "A" chars → 1600 codewords > 1558 max.
      expect { described_class.encode("A" * 1600) }
        .to raise_error(described_class::InputTooLongError)
    end

    # -- Real-world payloads ----------------------------------------------------

    it "encodes a GS1 DataMatrix typical payload" do
      payload = "(01)09312345678903(17)141231(10)ABC123"
      expect { described_class.encode(payload) }.not_to raise_error
    end

    it "encodes numeric-only payload" do
      grid = described_class.encode("1234567890")
      expect(grid.rows).to be > 0
    end

    it "encodes a URL" do
      grid = described_class.encode("https://example.com/item?id=ABC123")
      expect(grid.rows).to be > 0
    end

    it "encodes binary-like UTF-8 payload" do
      payload = "café"  # contains non-ASCII bytes
      expect { described_class.encode(payload) }.not_to raise_error
    end

    # -- Symbol size progression ------------------------------------------------

    it "encode produces 10×10 for short inputs" do
      grid = described_class.encode("Hi")
      expect(grid.rows).to eq(10)
      expect(grid.cols).to eq(10)
    end

    it "encode produces 12×12 for inputs requiring 4–5 codewords" do
      # Need 4 data codewords: "ABCD" → [66,67,68,69] = 4 codewords; 10×10 holds 3
      grid = described_class.encode("ABCD")
      expect(grid.rows).to eq(12)
      expect(grid.cols).to eq(12)
    end

    it "symbol rows == symbol cols for square symbols" do
      grid = described_class.encode("Hello World")
      expect(grid.rows).to eq(grid.cols)
    end
  end

  # ---------------------------------------------------------------------------
  # encode_and_layout
  # ---------------------------------------------------------------------------

  describe ".encode_and_layout" do
    it "returns a hash with :grid and :scene keys" do
      result = described_class.encode_and_layout("A")
      expect(result).to have_key(:grid)
      expect(result).to have_key(:scene)
    end

    it ":grid is a ModuleGrid" do
      result = described_class.encode_and_layout("Hello")
      expect(result[:grid]).to be_a(described_class::ModuleGrid)
    end

    it ":scene is nil (v0.1.0 — no paint_instructions dependency)" do
      result = described_class.encode_and_layout("Hello")
      expect(result[:scene]).to be_nil
    end
  end

  # ---------------------------------------------------------------------------
  # grid_to_string
  # ---------------------------------------------------------------------------

  describe ".grid_to_string" do
    it "returns a string with grid.rows lines" do
      grid = described_class.encode("A")
      str = described_class.grid_to_string(grid)
      expect(str.split("\n").length).to eq(grid.rows)
    end

    it "each line has exactly grid.cols characters" do
      grid = described_class.encode("A")
      str = described_class.grid_to_string(grid)
      str.split("\n").each do |line|
        expect(line.length).to eq(grid.cols)
      end
    end

    it "contains only '0' and '1' characters" do
      grid = described_class.encode("A")
      str = described_class.grid_to_string(grid)
      expect(str.chars.uniq.sort).to match_array(["0", "1", "\n"].uniq.sort)
    end

    it "is deterministic" do
      grid = described_class.encode("Hello")
      s1 = described_class.grid_to_string(grid)
      s2 = described_class.grid_to_string(grid)
      expect(s1).to eq(s2)
    end
  end

  # ---------------------------------------------------------------------------
  # compute_interleaved (internal)
  # ---------------------------------------------------------------------------

  describe ".compute_interleaved" do
    it "returns data_cw + ecc_cw bytes" do
      entry = described_class::SQUARE_SIZES.first  # 10×10: 3 data, 5 ecc
      padded = described_class.pad_codewords([66], entry.data_cw)
      result = described_class.compute_interleaved(padded, entry)
      expect(result.length).to eq(entry.data_cw + entry.ecc_cw)
    end

    it "all bytes are in 0..255" do
      entry = described_class::SQUARE_SIZES.first
      padded = described_class.pad_codewords([66], entry.data_cw)
      result = described_class.compute_interleaved(padded, entry)
      result.each { |b| expect(b).to be_between(0, 255) }
    end
  end

  # ---------------------------------------------------------------------------
  # logical_to_physical (internal)
  # ---------------------------------------------------------------------------

  describe ".logical_to_physical" do
    it "maps (0, 0) to (1, 1) for a single-region symbol" do
      entry = described_class::SQUARE_SIZES.first  # 10×10, 1×1 regions
      expect(described_class.logical_to_physical(0, 0, entry)).to eq([1, 1])
    end

    it "adds 1-module outer border offset" do
      entry = described_class::SQUARE_SIZES.first
      pr, pc = described_class.logical_to_physical(0, 0, entry)
      expect(pr).to be >= 1
      expect(pc).to be >= 1
    end

    it "maps within bounds for all logical positions" do
      entry = described_class::SQUARE_SIZES[9]  # 32×32, 2×2 regions
      n_rows = entry.region_rows * entry.data_region_height
      n_cols = entry.region_cols * entry.data_region_width
      n_rows.times do |r|
        n_cols.times do |c|
          pr, pc = described_class.logical_to_physical(r, c, entry)
          expect(pr).to be_between(1, entry.symbol_rows - 2),
            "r=#{r},c=#{c} phys_row=#{pr} out of data range"
          expect(pc).to be_between(1, entry.symbol_cols - 2),
            "r=#{r},c=#{c} phys_col=#{pc} out of data range"
        end
      end
    end
  end
end
