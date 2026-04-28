# frozen_string_literal: true

require "simplecov"
SimpleCov.start

require "coding_adventures/pdf417"

RSpec.describe CodingAdventures::PDF417 do
  # ---------------------------------------------------------------------------
  # VERSION
  # ---------------------------------------------------------------------------

  describe "VERSION" do
    it "is 0.1.0" do
      expect(described_class::VERSION).to eq("0.1.0")
    end
  end

  # ---------------------------------------------------------------------------
  # Error hierarchy
  # ---------------------------------------------------------------------------

  describe "PDF417Error" do
    it "is a StandardError subclass" do
      e = described_class::PDF417Error.new("boom")
      expect(e).to be_a(StandardError)
      expect(e.message).to eq("boom")
    end
  end

  describe "InputTooLongError" do
    it "is a PDF417Error subclass" do
      e = described_class::InputTooLongError.new("too big")
      expect(e).to be_a(described_class::PDF417Error)
    end
  end

  describe "InvalidDimensionsError" do
    it "is a PDF417Error subclass" do
      e = described_class::InvalidDimensionsError.new("bad dims")
      expect(e).to be_a(described_class::PDF417Error)
    end
  end

  describe "InvalidECCLevelError" do
    it "is a PDF417Error subclass" do
      e = described_class::InvalidECCLevelError.new("bad ecc")
      expect(e).to be_a(described_class::PDF417Error)
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
  # Public constants
  # ---------------------------------------------------------------------------

  describe "public constants" do
    it "exposes GF929_PRIME = 929" do
      expect(described_class::GF929_PRIME).to eq(929)
    end

    it "exposes GF929_ALPHA = 3" do
      expect(described_class::GF929_ALPHA).to eq(3)
    end

    it "exposes GF929_ORDER = 928" do
      expect(described_class::GF929_ORDER).to eq(928)
    end

    it "exposes LATCH_BYTE = 924" do
      expect(described_class::LATCH_BYTE).to eq(924)
    end

    it "exposes PADDING_CW = 900" do
      expect(described_class::PADDING_CW).to eq(900)
    end

    it "exposes MIN_ROWS = 3" do
      expect(described_class::MIN_ROWS).to eq(3)
    end

    it "exposes MAX_ROWS = 90" do
      expect(described_class::MAX_ROWS).to eq(90)
    end

    it "exposes MIN_COLS = 1" do
      expect(described_class::MIN_COLS).to eq(1)
    end

    it "exposes MAX_COLS = 30" do
      expect(described_class::MAX_COLS).to eq(30)
    end

    it "START_PATTERN sums to 17 modules" do
      expect(described_class::START_PATTERN.sum).to eq(17)
    end

    it "STOP_PATTERN sums to 18 modules" do
      expect(described_class::STOP_PATTERN.sum).to eq(18)
    end
  end

  # ---------------------------------------------------------------------------
  # GF(929) arithmetic
  # ---------------------------------------------------------------------------

  describe ".gf_mul" do
    it "returns 0 when either operand is 0" do
      expect(described_class.gf_mul(0, 5)).to eq(0)
      expect(described_class.gf_mul(7, 0)).to eq(0)
      expect(described_class.gf_mul(0, 0)).to eq(0)
    end

    it "returns 1 for any element multiplied by its inverse" do
      # 3 * 310 = 1 in GF(929)  (3 * 310 = 930 ≡ 1 mod 929)
      expect(described_class.gf_mul(3, 310)).to eq(1)
    end

    it "is commutative" do
      expect(described_class.gf_mul(7, 13)).to eq(described_class.gf_mul(13, 7))
    end

    it "stays within GF(929) range" do
      (1..20).each do |a|
        (1..20).each do |b|
          result = described_class.gf_mul(a, b)
          expect(result).to be >= 0
          expect(result).to be < 929
        end
      end
    end

    it "GF_EXP/GF_LOG tables are consistent" do
      # For every non-zero v, α^{log(v)} should equal v
      (1..928).each do |v|
        log_v = described_class::GF_LOG[v]
        expect(described_class::GF_EXP[log_v]).to eq(v)
      end
    end
  end

  describe ".gf_add" do
    it "is just integer addition mod 929" do
      expect(described_class.gf_add(500, 500)).to eq(71)  # 1000 mod 929 = 71
      expect(described_class.gf_add(0, 0)).to eq(0)
      expect(described_class.gf_add(928, 1)).to eq(0)
    end
  end

  # ---------------------------------------------------------------------------
  # Reed-Solomon generator polynomial
  # ---------------------------------------------------------------------------

  describe ".build_generator" do
    it "produces a polynomial of degree 2^(ecc_level+1)" do
      [0, 1, 2, 3].each do |level|
        k = 1 << (level + 1)
        g = described_class.build_generator(level)
        expect(g.length).to eq(k + 1),
          "ecc_level #{level}: expected #{k + 1} coefficients, got #{g.length}"
      end
    end

    it "is monic — leading coefficient is 1" do
      [0, 1, 2, 3, 4].each do |level|
        g = described_class.build_generator(level)
        expect(g.first).to eq(1)
      end
    end

    it "all coefficients are in GF(929) range" do
      g = described_class.build_generator(3)
      g.each do |coeff|
        expect(coeff).to be >= 0
        expect(coeff).to be < 929
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Reed-Solomon encoder
  # ---------------------------------------------------------------------------

  describe ".rs_encode" do
    it "produces exactly 2^(ecc_level+1) ECC codewords" do
      data = [10, 20, 30, 40]
      [0, 1, 2, 3].each do |level|
        ecc = described_class.rs_encode(data, level)
        expect(ecc.length).to eq(1 << (level + 1))
      end
    end

    it "all ECC codewords are in 0..928" do
      data = [100, 200, 300, 400, 500]
      ecc = described_class.rs_encode(data, 2)
      ecc.each do |cw|
        expect(cw).to be >= 0
        expect(cw).to be <= 928
      end
    end

    it "is deterministic for the same input" do
      data = [1, 2, 3, 4, 5]
      ecc1 = described_class.rs_encode(data, 2)
      ecc2 = described_class.rs_encode(data, 2)
      expect(ecc1).to eq(ecc2)
    end

    it "changes when data changes" do
      ecc1 = described_class.rs_encode([1, 2, 3], 2)
      ecc2 = described_class.rs_encode([4, 5, 6], 2)
      expect(ecc1).not_to eq(ecc2)
    end
  end

  # ---------------------------------------------------------------------------
  # Byte compaction
  # ---------------------------------------------------------------------------

  describe ".byte_compact" do
    it "starts with LATCH_BYTE (924)" do
      cws = described_class.byte_compact([65, 66, 67])
      expect(cws.first).to eq(924)
    end

    it "encodes 6 bytes into exactly 5 codewords (plus latch)" do
      # 6 bytes → 5 base-900 codewords → total 6 codewords (latch + 5)
      cws = described_class.byte_compact([0, 1, 2, 3, 4, 5])
      expect(cws.length).to eq(6)
    end

    it "encodes a tail of 1..5 bytes as themselves (plus latch)" do
      [1, 2, 3, 4, 5].each do |tail_len|
        bytes = Array.new(tail_len, 65)
        cws = described_class.byte_compact(bytes)
        # 1 latch + tail_len codewords
        expect(cws.length).to eq(1 + tail_len)
        expect(cws[1..]).to eq(bytes)
      end
    end

    it "handles empty input — just the latch" do
      cws = described_class.byte_compact([])
      expect(cws).to eq([924])
    end

    it "all codewords are in 0..928" do
      bytes = (0..255).to_a
      cws = described_class.byte_compact(bytes)
      cws.each { |cw| expect(cw).to be_between(0, 928) }
    end

    it "round-trips 12 bytes into exactly 10 codewords (2 groups × 5) + latch" do
      bytes = Array.new(12, 42)
      cws = described_class.byte_compact(bytes)
      expect(cws.length).to eq(11)  # 1 latch + 10 data codewords
    end
  end

  # ---------------------------------------------------------------------------
  # Auto ECC level selection
  # ---------------------------------------------------------------------------

  describe ".auto_ecc_level" do
    it "returns level 2 for ≤ 40 data codewords" do
      expect(described_class.auto_ecc_level(1)).to eq(2)
      expect(described_class.auto_ecc_level(40)).to eq(2)
    end

    it "returns level 3 for 41..160 data codewords" do
      expect(described_class.auto_ecc_level(41)).to eq(3)
      expect(described_class.auto_ecc_level(160)).to eq(3)
    end

    it "returns level 4 for 161..320" do
      expect(described_class.auto_ecc_level(161)).to eq(4)
      expect(described_class.auto_ecc_level(320)).to eq(4)
    end

    it "returns level 5 for 321..863" do
      expect(described_class.auto_ecc_level(321)).to eq(5)
      expect(described_class.auto_ecc_level(863)).to eq(5)
    end

    it "returns level 6 for > 863" do
      expect(described_class.auto_ecc_level(864)).to eq(6)
      expect(described_class.auto_ecc_level(9999)).to eq(6)
    end
  end

  # ---------------------------------------------------------------------------
  # Dimension selection
  # ---------------------------------------------------------------------------

  describe ".choose_dimensions" do
    it "returns cols in [1..30] and rows in [3..90]" do
      [1, 5, 50, 200, 500, 1000, 2000].each do |total|
        cols, rows = described_class.choose_dimensions(total)
        expect(cols).to be_between(1, 30), "cols=#{cols} out of range for total=#{total}"
        expect(rows).to be_between(3, 90), "rows=#{rows} out of range for total=#{total}"
      end
    end

    it "always provides enough capacity" do
      [1, 10, 50, 200].each do |total|
        cols, rows = described_class.choose_dimensions(total)
        expect(cols * rows).to be >= total
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Row indicator computation
  # ---------------------------------------------------------------------------

  describe ".compute_lri and .compute_rri" do
    let(:rows) { 9 }
    let(:cols) { 5 }
    let(:ecc)  { 2 }

    it "returns non-negative integers for all valid rows" do
      rows.times do |r|
        lri = described_class.compute_lri(r, rows, cols, ecc)
        rri = described_class.compute_rri(r, rows, cols, ecc)
        expect(lri).to be >= 0
        expect(rri).to be >= 0
        expect(lri).to be <= 928
        expect(rri).to be <= 928
      end
    end

    it "LRI and RRI differ for clusters 0 and 1 when R_info ≠ C_info ≠ L_info" do
      # For most non-degenerate symbols the three indicator values differ.
      # Cluster 0 row: LRI=R_info, RRI=C_info
      lri0 = described_class.compute_lri(0, 9, 5, 2)
      rri0 = described_class.compute_rri(0, 9, 5, 2)
      r_info = (9 - 1) / 3   # = 2
      c_info = 5 - 1         # = 4
      expect(lri0).to eq(r_info)
      expect(rri0).to eq(c_info)
    end
  end

  # ---------------------------------------------------------------------------
  # Pattern expansion
  # ---------------------------------------------------------------------------

  describe ".expand_pattern" do
    it "always produces exactly 17 modules" do
      described_class::CLUSTER_TABLES[0].each do |packed|
        out = []
        described_class.expand_pattern(packed, out)
        expect(out.length).to eq(17)
      end
    end

    it "first module of every codeword is dark (bar starts the pattern)" do
      described_class::CLUSTER_TABLES[0].first(20).each do |packed|
        out = []
        described_class.expand_pattern(packed, out)
        expect(out.first).to eq(true)
      end
    end

    it "produces only boolean values" do
      out = []
      described_class.expand_pattern(described_class::CLUSTER_TABLES[0][0], out)
      out.each { |m| expect(m).to eq(true).or(eq(false)) }
    end
  end

  describe ".expand_widths" do
    it "expands START_PATTERN to 17 modules" do
      out = []
      described_class.expand_widths(described_class::START_PATTERN, out)
      expect(out.length).to eq(17)
    end

    it "expands STOP_PATTERN to 18 modules" do
      out = []
      described_class.expand_widths(described_class::STOP_PATTERN, out)
      expect(out.length).to eq(18)
    end
  end

  # ---------------------------------------------------------------------------
  # encode — the main API
  # ---------------------------------------------------------------------------

  describe ".encode" do
    # -- Basic return shape ----------------------------------------------------

    it "returns a ModuleGrid for a single byte input" do
      grid = described_class.encode("A")
      expect(grid).to be_a(described_class::ModuleGrid)
      expect(grid).to respond_to(:rows)
      expect(grid).to respond_to(:cols)
      expect(grid).to respond_to(:modules)
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

    it "modules is a 2D array of booleans" do
      grid = described_class.encode("Hello")
      expect(grid.modules).to be_a(Array)
      expect(grid.modules.first).to be_a(Array)
      flat = grid.modules.flatten
      flat.each { |m| expect(m).to eq(true).or(eq(false)) }
    end

    it "grid.modules has grid.rows rows" do
      grid = described_class.encode("Hello, PDF417!")
      expect(grid.modules.length).to eq(grid.rows)
    end

    it "every row of modules has exactly grid.cols columns" do
      grid = described_class.encode("Hello, PDF417!")
      grid.modules.each do |row|
        expect(row.length).to eq(grid.cols)
      end
    end

    # -- Dimension constraints -------------------------------------------------

    it "module cols = 69 + 17 * data_cols (width formula verification)" do
      # The module width is always 69 + 17*data_cols where data_cols is
      # the number of data columns chosen by choose_dimensions.  We can't
      # easily inspect data_cols from outside, but we can verify the total
      # is always 69 + 17*k for some integer k in 1..30.
      grid = described_class.encode("HELLO WORLD")
      remainder = (grid.cols - 69) % 17
      expect(remainder).to eq(0)
      data_cols = (grid.cols - 69) / 17
      expect(data_cols).to be_between(1, 30)
    end

    it "logical row count is in [3, 90]" do
      # row_height defaults to 3, so module rows = logical_rows * 3.
      # logical_rows = module_rows / row_height.
      grid = described_class.encode("HELLO WORLD")
      logical_rows = grid.rows / 3
      expect(logical_rows).to be_between(3, 90)
    end

    # -- Encoding empty string -------------------------------------------------

    it "encodes an empty string without raising" do
      expect { described_class.encode("") }.not_to raise_error
    end

    it "empty string produces a valid ModuleGrid" do
      grid = described_class.encode("")
      expect(grid.rows).to be > 0
      expect(grid.cols).to be > 0
    end

    # -- Determinism -----------------------------------------------------------

    it "is deterministic — same input always produces the same grid" do
      g1 = described_class.encode("Hello, PDF417!")
      g2 = described_class.encode("Hello, PDF417!")
      expect(g1.modules).to eq(g2.modules)
    end

    it "produces different grids for different inputs" do
      g1 = described_class.encode("AAAA")
      g2 = described_class.encode("BBBB")
      expect(g1.modules).not_to eq(g2.modules)
    end

    # -- Larger inputs grow the symbol ----------------------------------------

    it "larger input produces a larger symbol (or equal)" do
      small = described_class.encode("A")
      large = described_class.encode("A" * 100)
      total_small = small.rows * small.cols
      total_large = large.rows * large.cols
      expect(total_large).to be >= total_small
    end

    it "very large input produces a much larger symbol" do
      small = described_class.encode("Hi")
      large = described_class.encode("A" * 800)
      expect(large.rows).to be > small.rows
    end

    # -- Byte array input ------------------------------------------------------

    it "accepts an array of bytes (integers 0..255)" do
      grid = described_class.encode([72, 101, 108, 108, 111])  # "Hello"
      expect(grid).to be_a(described_class::ModuleGrid)
      expect(grid.rows).to be > 0
    end

    it "string and equivalent byte array produce the same grid" do
      g1 = described_class.encode("Hello")
      g2 = described_class.encode([72, 101, 108, 108, 111])
      expect(g1.modules).to eq(g2.modules)
    end

    # -- ECC level option ------------------------------------------------------

    it "accepts explicit ecc_level 0..8" do
      (0..8).each do |level|
        expect { described_class.encode("test", ecc_level: level) }.not_to raise_error
      end
    end

    it "raises InvalidECCLevelError for ecc_level < 0" do
      expect { described_class.encode("test", ecc_level: -1) }
        .to raise_error(described_class::InvalidECCLevelError)
    end

    it "raises InvalidECCLevelError for ecc_level > 8" do
      expect { described_class.encode("test", ecc_level: 9) }
        .to raise_error(described_class::InvalidECCLevelError)
    end

    it "raises InvalidECCLevelError for a non-integer ecc_level" do
      expect { described_class.encode("test", ecc_level: 2.5) }
        .to raise_error(described_class::InvalidECCLevelError)
    end

    it "higher ECC level produces a symbol at least as large" do
      g0 = described_class.encode("Hello World", ecc_level: 0)
      g4 = described_class.encode("Hello World", ecc_level: 4)
      total0 = g0.rows * g0.cols
      total4 = g4.rows * g4.cols
      expect(total4).to be >= total0
    end

    # -- columns option --------------------------------------------------------

    it "accepts explicit columns option in 1..30" do
      [1, 5, 10, 20, 30].each do |c|
        expect { described_class.encode("test data", columns: c) }.not_to raise_error
      end
    end

    it "raises InvalidDimensionsError for columns < 1" do
      expect { described_class.encode("test", columns: 0) }
        .to raise_error(described_class::InvalidDimensionsError)
    end

    it "raises InvalidDimensionsError for columns > 30" do
      expect { described_class.encode("test", columns: 31) }
        .to raise_error(described_class::InvalidDimensionsError)
    end

    it "with columns: 1 the symbol has exactly 1 data column (module width = 86)" do
      # 69 + 17*1 = 86
      grid = described_class.encode("HELLO", columns: 1)
      expect(grid.cols).to eq(86)
    end

    it "with columns: 5 the module width is 69 + 85 = 154" do
      grid = described_class.encode("HELLO WORLD", columns: 5)
      expect(grid.cols).to eq(154)
    end

    # -- row_height option -----------------------------------------------------

    it "doubles the module height when row_height: 6" do
      g3 = described_class.encode("Hello", row_height: 3)
      g6 = described_class.encode("Hello", row_height: 6)
      expect(g6.rows).to eq(g3.rows * 2)
    end

    it "row_height: 1 produces the minimum height symbol" do
      grid = described_class.encode("Hello")
      g1 = described_class.encode("Hello", row_height: 1)
      expect(g1.rows).to eq(grid.rows / 3)
    end

    # -- Input type validation -------------------------------------------------

    it "raises ArgumentError for non-String non-Array input" do
      expect { described_class.encode(12345) }.to raise_error(ArgumentError)
      expect { described_class.encode(nil)   }.to raise_error(ArgumentError)
    end

    it "raises PDF417Error for an array containing out-of-range values" do
      expect { described_class.encode([0, 256]) }
        .to raise_error(described_class::PDF417Error)
    end

    it "raises PDF417Error for an array containing negative values" do
      expect { described_class.encode([-1, 0, 1]) }
        .to raise_error(described_class::PDF417Error)
    end

    # -- Real-world payloads ---------------------------------------------------

    it "encodes a typical driver's licence snippet without error" do
      payload = "ANSI 6360300102DL00390187ZC03290024DLDCADACDBDDFDGE" \
                "DFGDHDIDAJDKDLDM"
      expect { described_class.encode(payload) }.not_to raise_error
    end

    it "encodes binary data (all 256 byte values)" do
      payload = (0..255).to_a
      grid = described_class.encode(payload)
      expect(grid.rows).to be > 0
      expect(grid.cols).to be > 0
    end

    it "encodes a URL" do
      grid = described_class.encode("https://example.com/boarding-pass?id=1234567890")
      expect(grid.rows).to be > 0
    end

    # -- Row structure sanity (start/stop columns are always dark) ------------

    it "first module of every row is dark (start-pattern bar)" do
      grid = described_class.encode("Hello, PDF417!")
      grid.modules.each_with_index do |row, r|
        expect(row.first).to eq(true), "row #{r}: first module should be dark"
      end
    end

    it "last module of every row is dark (stop-pattern bar)" do
      grid = described_class.encode("Hello, PDF417!")
      grid.modules.each_with_index do |row, r|
        expect(row.last).to eq(true), "row #{r}: last module should be dark"
      end
    end

    # -- Multiple distinct inputs ----------------------------------------------

    it "encodes numbers-only payload" do
      grid = described_class.encode("1234567890")
      expect(grid.rows).to be > 0
    end

    it "encodes unicode-encoded UTF-8 bytes" do
      # "café" as UTF-8 bytes
      payload = "café".b
      expect { described_class.encode(payload) }.not_to raise_error
    end
  end
end
