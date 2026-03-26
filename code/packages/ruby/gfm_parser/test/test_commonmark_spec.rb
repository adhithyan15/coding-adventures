# frozen_string_literal: true

# GFM 0.31.2 Specification Compliance Tests
#
# Runs all 652 examples from the GFM 0.31.2 specification, comparing
# our parser + HTML renderer output against the expected HTML.
#
# The spec JSON is from https://spec.commonmark.org/0.31.2/spec.json
# and is stored locally at test/spec.json — no network access needed.
#
# === Why 652 tests? ===
#
# The GFM 0.31.2 spec has exactly 652 numbered examples covering:
#   - Tabs and whitespace handling
#   - ATX and setext headings
#   - Thematic breaks
#   - Fenced and indented code blocks
#   - HTML blocks (7 types)
#   - Blockquotes
#   - Lists (ordered, unordered, tight, loose)
#   - Inline: code spans, emphasis, links, images, autolinks, HTML, backslash escapes
#   - Precedence rules
#   - Edge cases
#
# Passing all 652 examples means the parser is 100% GFM 0.31.2 compliant.

require "test_helper"
require "json"
require "coding_adventures_document_ast_to_html"

module CodingAdventures
  module CommonmarkParser
    class TestCommonmarkSpec < Minitest::Test
      SPEC_PATH = File.join(__dir__, "spec.json")
      SPEC_EXAMPLES = JSON.parse(File.read(SPEC_PATH, encoding: "utf-8")).freeze

      # Verify we loaded the expected number of spec examples.
      # If this assertion fails, the spec.json file may be corrupted or
      # from a different version.
      def test_spec_has_652_examples
        assert_equal 652, SPEC_EXAMPLES.length,
          "Expected 652 GFM 0.31.2 examples, got #{SPEC_EXAMPLES.length}"
      end

      # Dynamically generate one test method per spec example.
      #
      # Each test method name encodes the example number and section name so
      # that failures are easy to find in the spec:
      #   test_example_001_tabs
      #   test_example_042_atx_headings
      #
      # This approach (define_method) gives Minitest per-example test names
      # while keeping the test file readable.
      SPEC_EXAMPLES.each do |example|
        num = example["example"]
        section = example["section"].gsub(/[^a-zA-Z0-9]/, "_").gsub(/__+/, "_").downcase
        method_name = :"test_example_#{format("%03d", num)}_#{section}"

        define_method(method_name) do
          markdown = example["markdown"]
          expected = example["html"]
          document = CodingAdventures::CommonmarkParser.parse(markdown)
          actual = CodingAdventures::DocumentAstToHtml.to_html(document)

          assert_equal expected, actual,
            "GFM spec example #{num} (#{example["section"]}) failed.\n" \
            "Input:    #{markdown.inspect}\n" \
            "Expected: #{expected.inspect}\n" \
            "Actual:   #{actual.inspect}"
        end
      end
    end
  end
end
