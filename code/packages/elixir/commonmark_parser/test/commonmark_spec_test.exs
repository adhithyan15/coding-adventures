defmodule CodingAdventures.CommonmarkSpecTest do
  @moduledoc """
  CommonMark 0.31.2 specification conformance tests.

  Loads all 652 examples from spec.json and verifies that the parser produces
  the expected HTML output for each one.

  Each test example has:
    - `markdown` — the input Markdown text
    - `html` — the expected HTML output
    - `example` — the example number (1–652)
    - `section` — the spec section name

  A test failure reports the example number, section, input, expected output,
  and actual output for easy debugging.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CommonmarkParser
  alias CodingAdventures.DocumentAstToHtml

  @spec_path Path.join([__DIR__, "fixtures", "spec.json"])

  @doc false
  def spec_examples do
    @spec_path
    |> File.read!()
    |> Jason.decode!()
  end

  # Generate a test case for each spec example
  for example <- Jason.decode!(File.read!(Path.join([__DIR__, "fixtures", "spec.json"]))) do
    example_num = example["example"]
    section = example["section"]
    markdown = example["markdown"]
    expected_html = example["html"]

    @tag example: example_num, section: section
    test "spec example #{example_num} - #{section}" do
      markdown = unquote(markdown)
      expected = unquote(expected_html)

      ast = CommonmarkParser.parse(markdown)
      actual = DocumentAstToHtml.render(ast)

      assert actual == expected,
        """
        CommonMark spec example #{unquote(example_num)} (#{unquote(section)}) FAILED

        Input markdown:
        #{inspect(markdown)}

        Expected HTML:
        #{inspect(expected)}

        Actual HTML:
        #{inspect(actual)}
        """
    end
  end
end
