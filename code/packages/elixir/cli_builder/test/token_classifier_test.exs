defmodule CodingAdventures.CliBuilder.TokenClassifierTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.TokenClassifier

  # ---------------------------------------------------------------------------
  # Test fixtures: a rich set of flag definitions
  # ---------------------------------------------------------------------------

  # A set of flags covering all token types:
  # - boolean short: -v, -l, -a, -h
  # - non-boolean short: -o (string), -n (integer), -f (file)
  # - boolean long: --verbose, --long-listing, --all, --help
  # - non-boolean long: --output (string), --count (integer)
  # - SDL: -classpath (non-boolean), -version (boolean)
  defp flags do
    [
      %{"id" => "verbose", "short" => "v", "long" => "verbose", "single_dash_long" => nil, "type" => "boolean"},
      %{"id" => "long", "short" => "l", "long" => "long-listing", "single_dash_long" => nil, "type" => "boolean"},
      %{"id" => "all", "short" => "a", "long" => "all", "single_dash_long" => nil, "type" => "boolean"},
      %{"id" => "help", "short" => "h", "long" => "help", "single_dash_long" => nil, "type" => "boolean"},
      %{"id" => "output", "short" => "o", "long" => "output", "single_dash_long" => nil, "type" => "string"},
      %{"id" => "count", "short" => "n", "long" => "count", "single_dash_long" => nil, "type" => "integer"},
      %{"id" => "file", "short" => "f", "long" => "file", "single_dash_long" => nil, "type" => "string"},
      %{"id" => "classpath", "short" => nil, "long" => nil, "single_dash_long" => "classpath", "type" => "string"},
      %{"id" => "sdk-version", "short" => nil, "long" => nil, "single_dash_long" => "version", "type" => "boolean"}
    ]
  end

  # ---------------------------------------------------------------------------
  # :end_of_flags
  # ---------------------------------------------------------------------------

  describe "end-of-flags sentinel" do
    test "exactly -- is end_of_flags" do
      assert TokenClassifier.classify("--", flags()) == :end_of_flags
    end

    test "--- is a long flag (name '-')" do
      assert TokenClassifier.classify("---", flags()) == {:long_flag, "-"}
    end
  end

  # ---------------------------------------------------------------------------
  # Long flags
  # ---------------------------------------------------------------------------

  describe "long flags (--name)" do
    test "boolean long flag" do
      assert TokenClassifier.classify("--verbose", flags()) == {:long_flag, "verbose"}
    end

    test "hyphenated long flag name" do
      assert TokenClassifier.classify("--long-listing", flags()) == {:long_flag, "long-listing"}
    end

    test "long flag with value (= separator)" do
      assert TokenClassifier.classify("--output=foo.txt", flags()) ==
               {:long_flag_with_value, "output", "foo.txt"}
    end

    test "long flag with value containing = sign" do
      assert TokenClassifier.classify("--output=foo=bar", flags()) ==
               {:long_flag_with_value, "output", "foo=bar"}
    end

    test "unknown long flag still returns {:long_flag, ...}" do
      # Unknown flags are handled by the parser, not the classifier.
      assert TokenClassifier.classify("--zzzunknown", flags()) == {:long_flag, "zzzunknown"}
    end
  end

  # ---------------------------------------------------------------------------
  # Single-dash-long flags (SDL)
  # ---------------------------------------------------------------------------

  describe "single-dash-long flags" do
    test "SDL flag match" do
      assert TokenClassifier.classify("-classpath", flags()) == {:single_dash_long, "classpath"}
    end

    test "SDL boolean flag" do
      assert TokenClassifier.classify("-version", flags()) == {:single_dash_long, "version"}
    end

    test "SDL flag does not match partial prefix" do
      # "-class" is not a known SDL; it would fall through to short flag rules
      result = TokenClassifier.classify("-class", flags())
      # -c is not a known short flag, so this becomes unknown
      assert result == {:unknown_flag, "-class"}
    end
  end

  # ---------------------------------------------------------------------------
  # Short flags
  # ---------------------------------------------------------------------------

  describe "short flags" do
    test "single boolean short flag" do
      assert TokenClassifier.classify("-v", flags()) == {:short_flag, "v"}
    end

    test "single non-boolean short flag (value follows)" do
      assert TokenClassifier.classify("-o", flags()) == {:short_flag, "o"}
    end

    test "non-boolean short flag with inline value" do
      assert TokenClassifier.classify("-ofoo.txt", flags()) ==
               {:short_flag_with_value, "o", "foo.txt"}
    end

    test "non-boolean short flag with numeric inline value" do
      assert TokenClassifier.classify("-n42", flags()) ==
               {:short_flag_with_value, "n", "42"}
    end
  end

  # ---------------------------------------------------------------------------
  # Stacked flags
  # ---------------------------------------------------------------------------

  describe "stacked flags" do
    test "two boolean flags stacked" do
      result = TokenClassifier.classify("-vl", flags())
      assert result == {:stacked_flags, ["v", "l"]}
    end

    test "three boolean flags stacked" do
      result = TokenClassifier.classify("-vla", flags())
      assert result == {:stacked_flags, ["v", "l", "a"]}
    end

    test "boolean + non-boolean last in stack (last takes next token as value)" do
      # -vf: v=boolean, f=non-boolean(string). f is last so it's valid.
      result = TokenClassifier.classify("-vf", flags())
      assert result == {:stacked_flags, ["v", "f"]}
    end

    test "boolean + non-boolean in middle → unknown" do
      # -von: v=boolean, o=non-boolean, n=integer. o is not last.
      # Should be classified as stacked only if o can be last; here n follows.
      result = TokenClassifier.classify("-von", flags())
      # non-boolean in the middle produces unknown
      assert match?({:unknown_flag, _}, result) or result == {:stacked_flags, ["v", "o", "n"]}
      # The important thing is it doesn't crash
    end
  end

  # ---------------------------------------------------------------------------
  # Positional tokens
  # ---------------------------------------------------------------------------

  describe "positional tokens" do
    test "plain word is positional" do
      assert TokenClassifier.classify("hello", flags()) == {:positional, "hello"}
    end

    test "empty string is positional" do
      assert TokenClassifier.classify("", flags()) == {:positional, ""}
    end

    test "single dash is positional (stdin convention)" do
      assert TokenClassifier.classify("-", flags()) == {:positional, "-"}
    end

    test "path with slashes is positional" do
      assert TokenClassifier.classify("/etc/hosts", flags()) == {:positional, "/etc/hosts"}
    end

    test "number is positional" do
      assert TokenClassifier.classify("42", flags()) == {:positional, "42"}
    end
  end

  # ---------------------------------------------------------------------------
  # Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "short flag with unknown character" do
      assert TokenClassifier.classify("-z", flags()) == {:unknown_flag, "-z"}
    end

    test "stacked starting with unknown character" do
      assert TokenClassifier.classify("-zv", flags()) == {:unknown_flag, "-zv"}
    end
  end

  # ---------------------------------------------------------------------------
  # Empty active flags
  # ---------------------------------------------------------------------------

  describe "with no active flags" do
    test "-- is still end_of_flags" do
      assert TokenClassifier.classify("--", []) == :end_of_flags
    end

    test "long flag is still long_flag" do
      assert TokenClassifier.classify("--verbose", []) == {:long_flag, "verbose"}
    end

    test "short token becomes unknown flag" do
      assert TokenClassifier.classify("-v", []) == {:unknown_flag, "-v"}
    end

    test "positional is still positional" do
      assert TokenClassifier.classify("hello", []) == {:positional, "hello"}
    end
  end

  # ---------------------------------------------------------------------------
  # Additional edge cases for stacked flags
  # ---------------------------------------------------------------------------

  describe "stacked flags edge cases" do
    test "non-boolean flag as last in stack is accepted (stacked_flags)" do
      # -vf: v=boolean, f=string (last) → stacked
      result = TokenClassifier.classify("-vf", flags())
      assert result == {:stacked_flags, ["v", "f"]}
    end

    test "non-boolean flag as non-last in stack → unknown_flag" do
      # -von: v=boolean, o=string (not last), n=integer → unknown because o in middle
      result = TokenClassifier.classify("-von", flags())
      assert match?({:unknown_flag, _}, result)
    end

    test "single unknown char stacked produces unknown_flag" do
      result = TokenClassifier.classify("-z", flags())
      assert result == {:unknown_flag, "-z"}
    end

    test "stack starting with unknown char is unknown_flag" do
      # 'z' is not known → immediate unknown
      result = TokenClassifier.classify("-zv", flags())
      assert result == {:unknown_flag, "-zv"}
    end

    test "stack with unknown char in the middle is unknown_flag" do
      # v=boolean, then z=unknown → unknown
      result = TokenClassifier.classify("-vzl", flags())
      assert match?({:unknown_flag, _}, result)
    end

    test "four boolean flags stacked" do
      result = TokenClassifier.classify("-vlah", flags())
      assert result == {:stacked_flags, ["v", "l", "a", "h"]}
    end
  end

  # ---------------------------------------------------------------------------
  # SDL priority over short flags
  # ---------------------------------------------------------------------------

  describe "SDL takes priority over short-flag rule" do
    test "SDL match returns single_dash_long even if first char is also a short flag" do
      # Build flags where 'v' is a short flag but 'verbose' is also an SDL
      sdl_and_short_flags = [
        %{"id" => "verbose-short", "short" => "v", "long" => nil, "single_dash_long" => nil, "type" => "boolean"},
        %{"id" => "verbose-sdl", "short" => nil, "long" => nil, "single_dash_long" => "verbose", "type" => "boolean"}
      ]

      # "-verbose" should match SDL first (Rule 1) over short (Rule 2)
      result = TokenClassifier.classify("-verbose", sdl_and_short_flags)
      assert result == {:single_dash_long, "verbose"}
    end
  end

  # ---------------------------------------------------------------------------
  # Long flag edge cases
  # ---------------------------------------------------------------------------

  describe "long flag edge cases" do
    test "long flag with empty value (--flag=)" do
      result = TokenClassifier.classify("--output=", flags())
      assert result == {:long_flag_with_value, "output", ""}
    end

    test "long flag with multiple = signs only splits on first" do
      # "--key=a=b=c" → name="key", value="a=b=c"
      result = TokenClassifier.classify("--key=a=b=c", flags())
      assert result == {:long_flag_with_value, "key", "a=b=c"}
    end
  end

  # ---------------------------------------------------------------------------
  # Short flag with value edge cases
  # ---------------------------------------------------------------------------

  describe "short flag with value edge cases" do
    test "inline value after non-boolean flag is captured" do
      result = TokenClassifier.classify("-o/some/path", flags())
      assert result == {:short_flag_with_value, "o", "/some/path"}
    end

    test "inline value with spaces (no spaces in token) is the full remainder" do
      result = TokenClassifier.classify("-ofilename with spaces.txt", flags())
      assert result == {:short_flag_with_value, "o", "filename with spaces.txt"}
    end
  end
end
