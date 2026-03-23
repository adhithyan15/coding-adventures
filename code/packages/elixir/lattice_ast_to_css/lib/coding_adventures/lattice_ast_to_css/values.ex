defmodule CodingAdventures.LatticeAstToCss.Values do
  @moduledoc """
  Lattice value types — the intermediate representation for compile-time evaluation.

  ## Why Separate Value Types?

  CSS tokens are just text. When we need to evaluate `$n * 8px`, we can't just
  multiply strings — we need typed values that carry semantic information:

  - Is `16px` a number? What unit? (DIMENSION)
  - Is `red` a color keyword or an identifier? (IDENT)
  - Is `"hello"` a string? (STRING)

  The value types here mirror the CSS/Lattice token types but carry structured
  data that allows arithmetic, comparison, and conversion back to CSS text.

  ## The 9 Value Types

  | Type              | Example         | CSS Token   |
  |-------------------|-----------------|-------------|
  | `{:number, n}`    | `42`, `3.14`    | NUMBER      |
  | `{:dimension, n, u}` | `16px`, `2em` | DIMENSION  |
  | `{:percentage, n}` | `50%`          | PERCENTAGE  |
  | `{:string, s}`    | `"hello"`       | STRING      |
  | `{:ident, s}`     | `red`, `bold`   | IDENT       |
  | `{:color, s}`     | `#4a90d9`       | HASH        |
  | `{:bool, b}`      | `true`, `false` | IDENT (literal) |
  | `:null`           | `null`          | IDENT (literal) |
  | `{:list, items}`  | `red, blue`     | (multiple values) |

  ## Elixir Tagged Tuples

  Following Elixir conventions (and the pattern from `json_value`), we use
  tagged tuples instead of structs for value types. This enables natural
  pattern matching:

      case value do
        {:number, n}         -> "plain number \#{n}"
        {:dimension, n, u}   -> "\#{n}\#{u}"
        {:percentage, n}     -> "\#{n}%"
        {:string, s}         -> ~s("\#{s}")
        {:ident, s}          -> s
        {:color, s}          -> s
        {:bool, true}        -> "true"
        {:bool, false}       -> "false"
        :null                -> ""
        {:list, items}       -> Enum.map_join(items, ", ", &to_css/1)
      end

  ## Truthiness

  Lattice truthiness rules (matching Sass conventions):
  - `{:bool, false}` → falsy
  - `:null` → falsy
  - `{:number, 0}` → falsy
  - Everything else → truthy (including empty strings and empty lists)
  """

  # ---------------------------------------------------------------------------
  # Type alias
  # ---------------------------------------------------------------------------

  @type lattice_value ::
          {:number, float()}
          | {:dimension, float(), String.t()}
          | {:percentage, float()}
          | {:string, String.t()}
          | {:ident, String.t()}
          | {:color, String.t()}
          | {:bool, boolean()}
          | :null
          | {:list, [lattice_value()]}

  # ---------------------------------------------------------------------------
  # Truthiness
  # ---------------------------------------------------------------------------

  @doc """
  Determine whether a Lattice value is truthy.

  Truthiness rules (matching Sass conventions):

  - `{:bool, false}` → falsy
  - `:null` → falsy
  - `{:number, 0}` → falsy
  - Everything else → truthy (including empty strings and empty lists)

  ## Examples

      true  = Values.truthy?({:ident, "red"})
      false = Values.truthy?({:bool, false})
      false = Values.truthy?(:null)
      false = Values.truthy?({:number, 0})
      true  = Values.truthy?({:number, 1.0})
  """
  @spec truthy?(lattice_value()) :: boolean()
  def truthy?({:bool, false}), do: false
  def truthy?(:null), do: false
  # Use a guard instead of literal 0.0 to avoid Erlang/OTP 27+ signed-zero warning
  def truthy?({:number, n}) when n == 0, do: false
  def truthy?(_), do: true

  # ---------------------------------------------------------------------------
  # Token → Value conversion
  # ---------------------------------------------------------------------------

  @doc """
  Convert a parser `Token` to a `lattice_value` tagged tuple.

  Maps token types to value types:

  - `NUMBER` → `{:number, float}`
  - `DIMENSION` → `{:dimension, float, unit_string}`
  - `PERCENTAGE` → `{:percentage, float}`
  - `STRING` → `{:string, string}` (quotes already stripped by lexer)
  - `HASH` → `{:color, string}` (includes the `#`)
  - `IDENT` → `{:ident, string}` (or `{:bool, _}` / `:null` for literals)
  - Anything else → `{:ident, string}` (fallback)

  ## Examples

      # From a Token struct %{type: "NUMBER", value: "42"}:
      {:number, 42.0} = Values.token_to_value(token)

      # From %{type: "DIMENSION", value: "16px"}:
      {:dimension, 16.0, "px"} = Values.token_to_value(token)
  """
  @spec token_to_value(map()) :: lattice_value()
  def token_to_value(%{type: type, value: value}) do
    parse_token(type, value)
  end

  # Handle both string and atom type keys (defensive)
  def token_to_value(token) when is_map(token) do
    type = Map.get(token, :type) || Map.get(token, "type")
    value = Map.get(token, :value) || Map.get(token, "value") || ""
    parse_token(to_string(type), to_string(value))
  end

  defp parse_token("NUMBER", value) do
    {:number, parse_float(value)}
  end

  defp parse_token("DIMENSION", value) do
    # Split "16px" into number part and unit part.
    # Find where the numeric part ends and the unit begins.
    # Numeric chars: digits, decimal point, and a leading minus sign.
    {num_str, unit} = split_dimension(value)
    {:dimension, parse_float(num_str), unit}
  end

  defp parse_token("PERCENTAGE", value) do
    # "50%" → strip the % and parse
    num_str = String.trim_trailing(value, "%")
    {:percentage, parse_float(num_str)}
  end

  defp parse_token("STRING", value) do
    # The lexer already strips quotes, so value is the raw content
    {:string, value}
  end

  defp parse_token("HASH", value) do
    {:color, value}
  end

  defp parse_token("IDENT", "true"), do: {:bool, true}
  defp parse_token("IDENT", "false"), do: {:bool, false}
  defp parse_token("IDENT", "null"), do: :null
  defp parse_token("IDENT", value), do: {:ident, value}

  # VARIABLE is special — it should be resolved, but as a fallback:
  defp parse_token("VARIABLE", value), do: {:ident, value}

  # Fallback for any other token type
  defp parse_token(_, value), do: {:ident, to_string(value)}

  # Split a DIMENSION value like "16px" into {"16", "px"} or "-2.5rem" into {"-2.5", "rem"}
  defp split_dimension(value) do
    # Find where the numeric chars end (digits, dot, leading minus)
    {num_chars, rest} =
      value
      |> String.graphemes()
      |> split_num_chars([], false)

    {Enum.join(num_chars), Enum.join(rest)}
  end

  defp split_num_chars([], acc, _), do: {Enum.reverse(acc), []}

  defp split_num_chars(["-" | rest], [], false) do
    # Leading minus allowed only at the start
    split_num_chars(rest, ["-"], true)
  end

  defp split_num_chars([c | rest], acc, seen_any) do
    if c =~ ~r/[0-9.]/ do
      split_num_chars(rest, [c | acc], seen_any)
    else
      # This is the start of the unit
      {Enum.reverse(acc), [c | rest]}
    end
  end

  # ---------------------------------------------------------------------------
  # Value → CSS text
  # ---------------------------------------------------------------------------

  @doc """
  Convert a `lattice_value` to its CSS text representation.

  This is used when substituting evaluated values back into CSS output.
  Each value type produces its canonical CSS representation.

  ## Examples

      "16"       = Values.to_css({:number, 16.0})
      "16px"     = Values.to_css({:dimension, 16.0, "px"})
      "50%"      = Values.to_css({:percentage, 50.0})
      ~s("hello") = Values.to_css({:string, "hello"})
      "red"      = Values.to_css({:ident, "red"})
      "#4a90d9"  = Values.to_css({:color, "#4a90d9"})
      "true"     = Values.to_css({:bool, true})
      ""         = Values.to_css(:null)
  """
  @spec to_css(lattice_value()) :: String.t()
  def to_css({:number, n}) do
    format_number(n)
  end

  def to_css({:dimension, n, unit}) do
    format_number(n) <> unit
  end

  def to_css({:percentage, n}) do
    format_number(n) <> "%"
  end

  def to_css({:string, s}) do
    ~s("#{s}")
  end

  def to_css({:ident, s}), do: s
  def to_css({:color, s}), do: s
  def to_css({:bool, true}), do: "true"
  def to_css({:bool, false}), do: "false"
  def to_css(:null), do: ""

  def to_css({:list, items}) do
    Enum.map_join(items, ", ", &to_css/1)
  end

  # ---------------------------------------------------------------------------
  # Arithmetic
  # ---------------------------------------------------------------------------

  @doc """
  Add two Lattice values.

  - `{:number, a} + {:number, b}` → `{:number, a+b}`
  - `{:dimension, a, u} + {:dimension, b, u}` → `{:dimension, a+b, u}` (same unit)
  - `{:percentage, a} + {:percentage, b}` → `{:percentage, a+b}`
  - `{:string, a} + {:string, b}` → `{:string, a<>b}` (string concatenation)

  Returns `{:ok, result}` or `{:error, message}`.
  """
  @spec add(lattice_value(), lattice_value()) :: {:ok, lattice_value()} | {:error, String.t()}
  def add({:number, a}, {:number, b}), do: {:ok, {:number, a + b}}

  def add({:dimension, a, u}, {:dimension, b, u}) do
    {:ok, {:dimension, a + b, u}}
  end

  def add({:dimension, _, _} = left, {:dimension, _, _} = right) do
    {:error, "Cannot add '#{to_css(left)}' and '#{to_css(right)}'"}
  end

  def add({:percentage, a}, {:percentage, b}), do: {:ok, {:percentage, a + b}}

  def add({:string, a}, {:string, b}), do: {:ok, {:string, a <> b}}

  def add(left, right) do
    {:error, "Cannot add '#{to_css(left)}' and '#{to_css(right)}'"}
  end

  @doc """
  Subtract two Lattice values.

  Same rules as `add/2` but with subtraction.
  """
  @spec subtract(lattice_value(), lattice_value()) :: {:ok, lattice_value()} | {:error, String.t()}
  def subtract({:number, a}, {:number, b}), do: {:ok, {:number, a - b}}

  def subtract({:dimension, a, u}, {:dimension, b, u}) do
    {:ok, {:dimension, a - b, u}}
  end

  def subtract({:dimension, _, _} = left, {:dimension, _, _} = right) do
    {:error, "Cannot subtract '#{to_css(left)}' and '#{to_css(right)}'"}
  end

  def subtract({:percentage, a}, {:percentage, b}), do: {:ok, {:percentage, a - b}}

  def subtract(left, right) do
    {:error, "Cannot subtract '#{to_css(left)}' and '#{to_css(right)}'"}
  end

  @doc """
  Multiply two Lattice values.

  - `{:number, a} * {:number, b}` → `{:number, a*b}`
  - `{:number, n} * {:dimension, d, u}` → `{:dimension, n*d, u}` (scales the value)
  - `{:dimension, d, u} * {:number, n}` → `{:dimension, d*n, u}` (commutative)
  - `{:number, n} * {:percentage, p}` → `{:percentage, n*p}`
  - `{:percentage, p} * {:number, n}` → `{:percentage, p*n}`
  """
  @spec multiply(lattice_value(), lattice_value()) :: {:ok, lattice_value()} | {:error, String.t()}
  def multiply({:number, a}, {:number, b}), do: {:ok, {:number, a * b}}
  def multiply({:number, n}, {:dimension, d, u}), do: {:ok, {:dimension, n * d, u}}
  def multiply({:dimension, d, u}, {:number, n}), do: {:ok, {:dimension, d * n, u}}
  def multiply({:number, n}, {:percentage, p}), do: {:ok, {:percentage, n * p}}
  def multiply({:percentage, p}, {:number, n}), do: {:ok, {:percentage, p * n}}

  def multiply(left, right) do
    {:error, "Cannot multiply '#{to_css(left)}' and '#{to_css(right)}'"}
  end

  @doc """
  Negate a numeric value.

  - `negate({:number, n})` → `{:number, -n}`
  - `negate({:dimension, n, u})` → `{:dimension, -n, u}`
  - `negate({:percentage, n})` → `{:percentage, -n}`
  """
  @spec negate(lattice_value()) :: {:ok, lattice_value()} | {:error, String.t()}
  def negate({:number, n}), do: {:ok, {:number, -n}}
  def negate({:dimension, n, u}), do: {:ok, {:dimension, -n, u}}
  def negate({:percentage, n}), do: {:ok, {:percentage, -n}}
  def negate(v), do: {:error, "Cannot negate '#{to_css(v)}'"}

  # ---------------------------------------------------------------------------
  # Comparison
  # ---------------------------------------------------------------------------

  @doc """
  Compare two Lattice values using the given operator.

  Returns `{:bool, boolean}`.

  For numeric types (same type), performs numeric comparison.
  For other types, falls back to string equality.

  ## Operators

  - `"EQUALS_EQUALS"` — equality
  - `"NOT_EQUALS"` — inequality
  - `"GREATER"` — greater than
  - `"GREATER_EQUALS"` — greater than or equal
  - `"LESS_EQUALS"` — less than or equal
  """
  @spec compare(lattice_value(), lattice_value(), String.t()) :: {:bool, boolean()}
  def compare(left, right, op) do
    case {left, right, op} do
      # Same numeric type — numeric comparison
      {{:number, a}, {:number, b}, "EQUALS_EQUALS"} -> {:bool, a == b}
      {{:number, a}, {:number, b}, "NOT_EQUALS"} -> {:bool, a != b}
      {{:number, a}, {:number, b}, "GREATER"} -> {:bool, a > b}
      {{:number, a}, {:number, b}, "GREATER_EQUALS"} -> {:bool, a >= b}
      {{:number, a}, {:number, b}, "LESS_EQUALS"} -> {:bool, a <= b}

      # Dimensions with same unit — numeric comparison
      {{:dimension, a, u}, {:dimension, b, u}, "EQUALS_EQUALS"} -> {:bool, a == b}
      {{:dimension, a, u}, {:dimension, b, u}, "NOT_EQUALS"} -> {:bool, a != b}
      {{:dimension, a, u}, {:dimension, b, u}, "GREATER"} -> {:bool, a > b}
      {{:dimension, a, u}, {:dimension, b, u}, "GREATER_EQUALS"} -> {:bool, a >= b}
      {{:dimension, a, u}, {:dimension, b, u}, "LESS_EQUALS"} -> {:bool, a <= b}

      # Dimensions with DIFFERENT units — only equality works
      {{:dimension, _, _}, {:dimension, _, _}, "EQUALS_EQUALS"} -> {:bool, false}
      {{:dimension, _, _}, {:dimension, _, _}, "NOT_EQUALS"} -> {:bool, true}

      # Percentages
      {{:percentage, a}, {:percentage, b}, "EQUALS_EQUALS"} -> {:bool, a == b}
      {{:percentage, a}, {:percentage, b}, "NOT_EQUALS"} -> {:bool, a != b}
      {{:percentage, a}, {:percentage, b}, "GREATER"} -> {:bool, a > b}
      {{:percentage, a}, {:percentage, b}, "GREATER_EQUALS"} -> {:bool, a >= b}
      {{:percentage, a}, {:percentage, b}, "LESS_EQUALS"} -> {:bool, a <= b}

      # Fallback: string equality
      {_, _, "EQUALS_EQUALS"} -> {:bool, to_css(left) == to_css(right)}
      {_, _, "NOT_EQUALS"} -> {:bool, to_css(left) != to_css(right)}

      # Can't order non-numeric types
      _ -> {:bool, false}
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  # Format a float as CSS: integers without decimal point, floats with it.
  # 16.0 → "16", 3.14 → "3.14"
  defp format_number(n) when is_float(n) do
    if n == Float.floor(n) and n >= -1.0e15 and n <= 1.0e15 do
      Integer.to_string(trunc(n))
    else
      Float.to_string(n)
    end
  end

  defp format_number(n) when is_integer(n), do: Integer.to_string(n)

  defp parse_float(s) do
    case Float.parse(s) do
      {f, ""} -> f
      {f, _} -> f
      :error ->
        case Integer.parse(s) do
          {i, _} -> i * 1.0
          :error -> 0.0
        end
    end
  end
end
