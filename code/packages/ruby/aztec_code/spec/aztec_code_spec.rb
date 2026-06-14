# frozen_string_literal: true

require "simplecov"
SimpleCov.start

require "coding_adventures/aztec_code"

RSpec.describe CodingAdventures::AztecCode do
  # ---------------------------------------------------------------------------
  # Version
  # ---------------------------------------------------------------------------

  describe "VERSION" do
    it "is 0.1.0" do
      expect(described_class::VERSION).to eq("0.1.0")
    end
  end

  # ---------------------------------------------------------------------------
  # Error hierarchy
  # ---------------------------------------------------------------------------

  describe "AztecError" do
    it "is a StandardError subclass" do
      e = described_class::AztecError.new("oops")
      expect(e).to be_a(StandardError)
      expect(e.message).to eq("oops")
    end
  end

  describe "InputTooLong" do
    it "is an AztecError subclass" do
      e = described_class::InputTooLong.new("big")
      expect(e).to be_a(described_class::AztecError)
    end

    it "is raised on oversized input" do
      expect { described_class.encode("x" * 3000) }.to raise_error(described_class::InputTooLong)
    end
  end

  # ---------------------------------------------------------------------------
  # Symbol size selection
  # ---------------------------------------------------------------------------

  describe ".encode" do
    it "returns a ModuleGrid for a single byte" do
      grid = described_class.encode("A")
      expect(grid).to respond_to(:rows)
      expect(grid).to respond_to(:cols)
      expect(grid).to respond_to(:modules)
    end

    it "encodes to a compact-1 symbol (15×15) for 'A'" do
      grid = described_class.encode("A")
      expect(grid.rows).to eq(15)
      expect(grid.cols).to eq(15)
    end

    it "always produces square symbols" do
      ["X", "Hello", "Hello, World!", "IATA boarding pass data here"].each do |data|
        grid = described_class.encode(data)
        expect(grid.rows).to eq(grid.cols), "not square for #{data.inspect}"
      end
    end

    it "grows the symbol for larger inputs" do
      small = described_class.encode("A")
      large = described_class.encode("A" * 200)
      expect(large.rows).to be > small.rows
    end

    it "accepts a byte array" do
      grid = described_class.encode([65, 66, 67]) # "ABC"
      expect(grid.rows).to be >= 15
    end

    it "raises ArgumentError for invalid input type" do
      expect { described_class.encode(123) }.to raise_error(ArgumentError)
    end

    it "is deterministic" do
      g1 = described_class.encode("Hello!")
      g2 = described_class.encode("Hello!")
      expect(g1.modules).to eq(g2.modules)
    end

    it "produces different grids for different inputs" do
      g1 = described_class.encode("ABC")
      g2 = described_class.encode("XYZ")
      expect(g1.modules).not_to eq(g2.modules)
    end
  end

  # ---------------------------------------------------------------------------
  # Bullseye structure
  # ---------------------------------------------------------------------------

  describe "bullseye structure" do
    let(:grid) { described_class.encode("A") }

    it "has the correct number of rows and columns" do
      expect(grid.rows).to eq(15)
      expect(grid.cols).to eq(15)
    end

    it "has a dark center module (compact-1 bullseye center)" do
      cx = grid.rows / 2
      cy = grid.cols / 2
      expect(grid.modules[cx][cy]).to be true
    end
  end

  # ---------------------------------------------------------------------------
  # ECC percentage
  # ---------------------------------------------------------------------------

  describe "min_ecc_percent option" do
    it "accepts a higher ECC percentage" do
      grid_low = described_class.encode("A", min_ecc_percent: 23)
      grid_high = described_class.encode("A", min_ecc_percent: 80)
      # Higher ECC may force a larger symbol; either way, it should encode.
      expect(grid_high.rows).to be >= grid_low.rows
    end
  end

  # ---------------------------------------------------------------------------
  # encode_and_layout
  # ---------------------------------------------------------------------------

  describe ".encode_and_layout" do
    it "returns a PaintScene-like object" do
      scene = described_class.encode_and_layout("Hello")
      expect(scene).to respond_to(:width)
      expect(scene).to respond_to(:height)
    end

    it "produces consistent output with encode + layout" do
      grid = described_class.encode("Test")
      scene_a = described_class.layout(grid)
      scene_b = described_class.encode_and_layout("Test")
      expect(scene_a.width).to eq(scene_b.width)
      expect(scene_a.height).to eq(scene_b.height)
    end
  end

  # ---------------------------------------------------------------------------
  # Edge cases
  # ---------------------------------------------------------------------------

  describe "edge cases" do
    it "encodes an empty string" do
      grid = described_class.encode("")
      expect(grid.rows).to be >= 15
    end

    it "encodes a single space" do
      grid = described_class.encode(" ")
      expect(grid.rows).to be >= 15
    end

    it "encodes a long string near the limit" do
      # 4x32-layer full symbol holds ~1914 bytes at 23% ECC
      data = "A" * 500
      grid = described_class.encode(data)
      expect(grid.rows).to be > 27
    end
  end
end
