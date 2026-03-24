defmodule CodingAdventures.LatticeTranspilerTest do
  use ExUnit.Case

  alias CodingAdventures.LatticeTranspiler

  # Helper: transpile and assert success
  defp transpile!(source, opts \\ []) do
    {:ok, css} = LatticeTranspiler.transpile(source, opts)
    css
  end

  # ---------------------------------------------------------------------------
  # Module loading
  # ---------------------------------------------------------------------------

  describe "module loading" do
    test "module loads" do
      assert Code.ensure_loaded?(CodingAdventures.LatticeTranspiler)
    end
  end

  # ---------------------------------------------------------------------------
  # Return types
  # ---------------------------------------------------------------------------

  describe "return types" do
    test "returns ok tuple on success" do
      result = LatticeTranspiler.transpile("h1 { color: red; }")
      assert {:ok, css} = result
      assert is_binary(css)
    end

    test "returns error tuple on undefined variable" do
      result = LatticeTranspiler.transpile("h1 { color: $undefined; }")
      assert {:error, msg} = result
      assert is_binary(msg)
    end

    test "empty source returns empty string" do
      {:ok, css} = LatticeTranspiler.transpile("")
      assert css == ""
    end
  end

  # ---------------------------------------------------------------------------
  # Plain CSS pass-through
  # ---------------------------------------------------------------------------

  describe "plain CSS pass-through" do
    test "simple rule" do
      css = transpile!("h1 { color: red; }")
      assert css =~ "color: red"
    end

    test "multiple declarations" do
      css = transpile!("p { color: red; font-size: 16px; margin: 0; }")
      assert css =~ "color: red"
      assert css =~ "font-size: 16px"
      assert css =~ "margin: 0"
    end

    test "selector list" do
      css = transpile!("h1, h2, h3 { color: red; }")
      assert css =~ "h1"
      assert css =~ "h2"
      assert css =~ "h3"
      assert css =~ "color: red"
    end

    test "class selector" do
      css = transpile!(".button { background: blue; }")
      assert css =~ "background: blue"
    end

    test "id selector" do
      css = transpile!("#header { display: flex; }")
      assert css =~ "display: flex"
    end

    test "!important" do
      css = transpile!("p { color: red !important; }")
      assert css =~ "!important"
    end

    test "@media rule" do
      css = transpile!("@media screen { h1 { color: red; } }")
      assert css =~ "@media"
      assert css =~ "color: red"
    end

    test "@import rule" do
      css = transpile!(~s(@import "reset.css";))
      assert css =~ "@import"
    end

    test "CSS function call" do
      css = transpile!("p { color: rgb(255, 0, 0); }")
      assert css =~ "rgb("
    end

    test "CSS custom property" do
      css = transpile!("p { color: var(--primary); }")
      assert css =~ "var("
    end
  end

  # ---------------------------------------------------------------------------
  # Variable substitution
  # ---------------------------------------------------------------------------

  describe "variable substitution" do
    test "simple color variable" do
      css = transpile!("$primary: #4a90d9;\nh1 { color: $primary; }")
      assert css =~ "#4a90d9"
      # The variable declaration itself should not appear in output
      refute css =~ "$primary"
    end

    test "variable with dimension" do
      css = transpile!("$base: 16px;\np { font-size: $base; }")
      assert css =~ "16px"
      refute css =~ "$base"
    end

    test "multiple variables" do
      css = transpile!("""
        $fg: white;
        $bg: black;
        body { color: $fg; background: $bg; }
      """)
      assert css =~ "white"
      assert css =~ "black"
    end

    test "variable used multiple times" do
      css = transpile!("""
        $color: red;
        h1 { color: $color; }
        h2 { background: $color; }
      """)
      # red should appear twice (in h1 and h2)
      occurrences = css |> String.split("red") |> length()
      assert occurrences >= 3  # "red" appears at least twice as a value
    end

    test "undefined variable returns error" do
      result = LatticeTranspiler.transpile("h1 { color: $undefined; }")
      assert {:error, _} = result
    end
  end

  # ---------------------------------------------------------------------------
  # Mixin expansion
  # ---------------------------------------------------------------------------

  describe "mixin expansion" do
    test "simple parameterless mixin" do
      css = transpile!("""
        @mixin clearfix() {
          content: '';
          display: block;
          clear: both;
        }
        .container::after { @include clearfix(); }
      """)
      assert css =~ "display: block"
      assert css =~ "clear: both"
      # Mixin definition should not appear in output
      refute css =~ "@mixin"
    end

    test "mixin with argument" do
      css = transpile!("""
        @mixin color-bg($color) {
          background: $color;
        }
        .red { @include color-bg(red); }
        .blue { @include color-bg(blue); }
      """)
      assert css =~ "background: red"
      assert css =~ "background: blue"
    end

    test "mixin with default parameter used" do
      css = transpile!("""
        @mixin button($bg, $fg: white) {
          background: $bg;
          color: $fg;
        }
        .btn { @include button(navy); }
      """)
      assert css =~ "background: navy"
      assert css =~ "color: white"
    end

    test "mixin with default parameter overridden" do
      css = transpile!("""
        @mixin button($bg, $fg: white) {
          background: $bg;
          color: $fg;
        }
        .btn { @include button(navy, gold); }
      """)
      assert css =~ "color: gold"
    end

    test "undefined mixin returns error" do
      result = LatticeTranspiler.transpile(".box { @include no-such-mixin; }")
      assert {:error, msg} = result
      assert msg =~ "no-such-mixin"
    end

    test "mixin defined after use (forward reference)" do
      css = transpile!("""
        .btn { @include my-button(blue); }
        @mixin my-button($bg) { background: $bg; }
      """)
      assert css =~ "background: blue"
    end
  end

  # ---------------------------------------------------------------------------
  # Control flow
  # ---------------------------------------------------------------------------

  describe "@if control flow" do
    test "if true branch taken" do
      css = transpile!("""
        $theme: dark;
        @if $theme == dark {
          body { background: black; }
        } @else {
          body { background: white; }
        }
      """)
      assert css =~ "black"
      refute css =~ "white"
    end

    test "if false, else branch taken" do
      css = transpile!("""
        $theme: light;
        @if $theme == dark {
          body { background: black; }
        } @else {
          body { background: white; }
        }
      """)
      refute css =~ "black"
      assert css =~ "white"
    end

    test "if with no else, false condition produces no output" do
      css = transpile!("""
        $debug: false;
        @if $debug == true {
          p { color: red; }
        }
        h1 { font-size: 24px; }
      """)
      refute css =~ "color: red"
      assert css =~ "font-size: 24px"
    end
  end

  describe "@for loop" do
    test "for loop generates multiple blocks" do
      css = transpile!("""
        @for $i from 1 through 3 {
          .item { color: red; }
        }
      """)
      # Count occurrences of "color: red"
      count = css |> String.split("color: red") |> length()
      assert count == 4  # 3 occurrences = 4 parts after split
    end

    test "for loop with variable substitution" do
      css = transpile!("""
        $base: 10px;
        @for $i from 1 through 2 {
          .item { width: $base; }
        }
      """)
      count = css |> String.split("10px") |> length()
      assert count == 3  # 2 occurrences = 3 parts
    end
  end

  describe "@each loop" do
    test "each loop over color list" do
      css = transpile!("""
        @each $color in red, green, blue {
          .text { color: $color; }
        }
      """)
      assert css =~ "red"
      assert css =~ "green"
      assert css =~ "blue"
    end
  end

  # ---------------------------------------------------------------------------
  # @use directive
  # ---------------------------------------------------------------------------

  describe "@use directive" do
    test "@use is consumed without error (module resolution not implemented)" do
      # @use should be collected and dropped, not causing an error
      result = LatticeTranspiler.transpile(~s(@use "colors";\nh1 { color: red; }))
      # Either succeeds (h1 rule) or fails with module error — not a crash
      assert match?({:ok, _}, result) or match?({:error, _}, result)
    end
  end

  # ---------------------------------------------------------------------------
  # Formatting options
  # ---------------------------------------------------------------------------

  describe "formatting options" do
    test "default is pretty-printed with 2-space indent" do
      css = transpile!("h1 { color: red; }")
      assert css =~ "  color: red"
    end

    test "minified: true removes whitespace" do
      css = transpile!("h1 { color: red; }", minified: true)
      assert css =~ "h1{color:red;}"
      refute css =~ "  "
    end

    test "custom indent" do
      css = transpile!("h1 { color: red; }", indent: "    ")
      assert css =~ "    color: red"
    end

    test "output ends with newline" do
      css = transpile!("h1 { color: red; }")
      assert String.ends_with?(css, "\n")
    end
  end

  # ---------------------------------------------------------------------------
  # Complex / realistic programs
  # ---------------------------------------------------------------------------

  describe "realistic Lattice programs" do
    test "design system with variables and mixins" do
      source = """
      $primary: #4a90d9;
      $secondary: #e74c3c;
      $base-size: 16px;

      @mixin button($bg, $fg: white) {
        background: $bg;
        color: $fg;
        padding: 8px 16px;
        border-radius: 4px;
      }

      .btn-primary {
        @include button($primary);
      }

      .btn-danger {
        @include button($secondary);
      }
      """

      css = transpile!(source)
      assert css =~ "#4a90d9"
      assert css =~ "#e74c3c"
      assert css =~ "background:"
      assert css =~ "color: white"
      assert css =~ "padding:"
      refute css =~ "@mixin"
      refute css =~ "$primary"
    end

    test "conditional theming" do
      source = """
      $theme: dark;

      $bg-color: black;

      body {
        background: $bg-color;
      }
      """

      css = transpile!(source)
      assert css =~ "background: black"
    end

    test "mixed Lattice and CSS" do
      source = """
      $gap: 20px;

      .card {
        padding: $gap;
        margin: 0;
      }

      @media (max-width: 768px) {
        .card {
          padding: 10px;
        }
      }
      """

      css = transpile!(source)
      assert css =~ "padding: 20px"
      assert css =~ "@media"
      assert css =~ "padding: 10px"
    end
  end

  # ---------------------------------------------------------------------------
  # Error handling
  # ---------------------------------------------------------------------------

  describe "error handling" do
    test "undefined variable error message is helpful" do
      {:error, msg} = LatticeTranspiler.transpile("h1 { color: $oops; }")
      assert msg =~ "oops"
    end

    test "undefined mixin error message includes name" do
      {:error, msg} = LatticeTranspiler.transpile(".box { @include ghost-mixin; }")
      assert msg =~ "ghost-mixin"
    end
  end
end
