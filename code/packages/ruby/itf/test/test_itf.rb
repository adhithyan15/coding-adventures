# frozen_string_literal: true

require "minitest/autorun"
require "digest"
require "json"
require "coding_adventures_itf"

class TestItf < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::Itf::VERSION
  end

  def test_normalize_rejects_odd_input
    assert_raises(ArgumentError) { CodingAdventures::Itf.normalize_itf("12345") }
  end

  def test_encode_interleaves_digit_pairs
    encoded = CodingAdventures::Itf.encode_itf("123456")
    assert_equal 3, encoded.length
    assert_equal "12", encoded.first[:pair]
  end

  def test_expand_runs_include_start_and_stop
    roles = CodingAdventures::Itf.expand_itf_runs("123456").map { |run| run[:role] }
    assert_includes roles, "start"
    assert_includes roles, "stop"
  end

  def test_paint_scene
    scene = CodingAdventures::Itf.draw_itf("123456")
    assert_equal "itf", scene.metadata[:symbology]
    assert_equal 3, scene.metadata[:pair_count]
    assert_operator scene.width, :>, 0
    assert_equal 120, scene.height
  end

  def test_barcode_symbologies_v1_corpus
    fixture_path = File.expand_path(
      "../../../../specs/fixtures/barcode-symbologies-v1/cases.json",
      __dir__,
    )
    cases = JSON.parse(File.read(fixture_path)).fetch("cases").select { |entry| entry["symbology"] == "itf" }
    assert_equal 10, cases.length

    cases.each do |test_case|
      data = fixture_input(test_case.fetch("input"))
      expected = test_case.fetch("expected")
      if expected.key?("error")
        error = assert_raises(ArgumentError) { CodingAdventures::Itf.normalize_itf(data) }
        assert_equal expected["error"], CodingAdventures::Itf.error_id(error)
        next
      end

      normalized = CodingAdventures::Itf.normalize_itf(data)
      modules = fixture_modules(data)
      runs = fixture_runs(modules)
      if expected.key?("normalized")
        assert_equal expected["normalized"], normalized
        assert_equal expected["modules"], modules
        assert_equal expected["run_lengths"], runs
      else
        assert_equal expected["normalized_sha256"], Digest::SHA256.hexdigest(normalized)
        assert_equal expected["module_count"], modules.length
        assert_equal expected["module_sha256"], Digest::SHA256.hexdigest(modules)
        assert_equal expected["run_count"], runs.length
        assert_equal expected["run_lengths_sha256"], Digest::SHA256.hexdigest(JSON.generate(runs))
      end
    end
  end

  private

  def fixture_input(input)
    return input["text"] if input.key?("text")

    input.fetch("repeat").fetch("text") * input.fetch("repeat").fetch("count")
  end

  def fixture_modules(data)
    "1010#{CodingAdventures::Itf.encode_itf(data).map { |pair| pair[:binary_pattern] }.join}11101"
  end

  def fixture_runs(bits)
    bits.chars.chunk_while { |left, right| left == right }.map(&:length)
  end
end
