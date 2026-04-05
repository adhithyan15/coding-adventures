defmodule CodingAdventures.WasmExecution.Values do
  @moduledoc """
  Typed WASM values and constructor/assertion helpers.

  Every value in WebAssembly is typed -- it carries both a raw payload
  and a type tag. The four WASM 1.0 value types are:

    +------+--------------------------------------------------+
    | Type | Byte  | Elixir representation                    |
    +------+-------+------------------------------------------+
    | i32  | 0x7F  | integer (signed 32-bit, wrapping)        |
    | i64  | 0x7E  | integer (signed 64-bit, wrapping)        |
    | f32  | 0x7D  | float (single-precision via rounding)    |
    | f64  | 0x7C  | float (native Elixir float = f64)        |
    +------+-------+------------------------------------------+

  ## Wrapping Semantics

  WASM integers wrap on overflow (modular arithmetic). In Elixir, integers
  are arbitrary-precision, so we must explicitly mask to 32 or 64 bits
  after each operation. We use `Bitwise.band/2` and sign extension.

  ## WasmValue Map

  A WasmValue is represented as `%{type: type_byte, value: payload}`.
  This is compatible with GenericVM's typed stack.
  """

  import Bitwise
  alias CodingAdventures.WasmExecution.TrapError

  # Value type byte constants
  @i32 0x7F
  @i64 0x7E
  @f32 0x7D
  @f64 0x7C

  @type wasm_value :: %{type: non_neg_integer(), value: number()}

  # ===========================================================================
  # Constructor Functions
  # ===========================================================================

  @doc """
  Create an i32 (32-bit integer) WASM value.

  The value is masked to 32 bits and sign-extended to produce a value
  in the range [-2^31, 2^31 - 1]. This is WASM's wrapping behavior.

  ## Examples

      iex> CodingAdventures.WasmExecution.Values.i32(42)
      %{type: 0x7F, value: 42}

      iex> CodingAdventures.WasmExecution.Values.i32(0xFFFFFFFF)
      %{type: 0x7F, value: -1}
  """
  @spec i32(integer()) :: wasm_value()
  def i32(value) when is_integer(value) do
    %{type: @i32, value: wrap_i32(value)}
  end

  @doc """
  Create an i64 (64-bit integer) WASM value.

  The value is masked to 64 bits and sign-extended.
  """
  @spec i64(integer()) :: wasm_value()
  def i64(value) when is_integer(value) do
    %{type: @i64, value: wrap_i64(value)}
  end

  @doc """
  Create an f32 (32-bit float) WASM value.

  In Elixir, all floats are 64-bit doubles. We approximate f32 behavior
  by storing the value as-is (exact f32 rounding would require C NIF).
  For most purposes this is sufficient.
  """
  @spec f32(number()) :: wasm_value()
  def f32(value) when is_number(value) do
    %{type: @f32, value: value * 1.0}
  end

  @doc """
  Create an f64 (64-bit float) WASM value.

  Elixir floats are already 64-bit doubles, so no conversion needed.
  """
  @spec f64(number()) :: wasm_value()
  def f64(value) when is_number(value) do
    %{type: @f64, value: value * 1.0}
  end

  # ===========================================================================
  # Default Values
  # ===========================================================================

  @doc """
  Create a zero-initialized WasmValue for a given type.

  When a WASM function is called, all local variables are initialized to
  the default value for their type (the respective zero).
  """
  @spec default_value(atom() | non_neg_integer()) :: wasm_value()
  def default_value(:i32), do: i32(0)
  def default_value(:i64), do: i64(0)
  def default_value(:f32), do: f32(0.0)
  def default_value(:f64), do: f64(0.0)
  def default_value(@i32), do: i32(0)
  def default_value(@i64), do: i64(0)
  def default_value(@f32), do: f32(0.0)
  def default_value(@f64), do: f64(0.0)

  def default_value(type_code) do
    raise TrapError, "Unknown value type: 0x#{Integer.to_string(type_code, 16)}"
  end

  # ===========================================================================
  # Type Extraction Helpers
  # ===========================================================================

  @doc "Extract the integer value from an i32 WasmValue. Raises TrapError on type mismatch."
  @spec as_i32(wasm_value()) :: integer()
  def as_i32(%{type: @i32, value: v}), do: v

  def as_i32(%{type: t}) do
    raise TrapError, "Type mismatch: expected i32, got #{type_name(t)}"
  end

  @doc "Extract the integer value from an i64 WasmValue."
  @spec as_i64(wasm_value()) :: integer()
  def as_i64(%{type: @i64, value: v}), do: v

  def as_i64(%{type: t}) do
    raise TrapError, "Type mismatch: expected i64, got #{type_name(t)}"
  end

  @doc "Extract the float value from an f32 WasmValue."
  @spec as_f32(wasm_value()) :: float()
  def as_f32(%{type: @f32, value: v}), do: v

  def as_f32(%{type: t}) do
    raise TrapError, "Type mismatch: expected f32, got #{type_name(t)}"
  end

  @doc "Extract the float value from an f64 WasmValue."
  @spec as_f64(wasm_value()) :: float()
  def as_f64(%{type: @f64, value: v}), do: v

  def as_f64(%{type: t}) do
    raise TrapError, "Type mismatch: expected f64, got #{type_name(t)}"
  end

  # ===========================================================================
  # i32 Wrapping Arithmetic
  # ===========================================================================

  @doc """
  Wrap an integer to signed 32-bit range [-2^31, 2^31 - 1].

  Uses `band(val, 0xFFFFFFFF)` to mask to 32 bits, then sign-extends
  if the high bit (bit 31) is set.
  """
  @spec wrap_i32(integer()) :: integer()
  def wrap_i32(value) do
    masked = band(value, 0xFFFFFFFF)

    if band(masked, 0x80000000) != 0 do
      # Sign extend: the value is negative in two's complement
      masked - 0x100000000
    else
      masked
    end
  end

  @doc """
  Wrap an integer to signed 64-bit range [-2^63, 2^63 - 1].
  """
  @spec wrap_i64(integer()) :: integer()
  def wrap_i64(value) do
    masked = band(value, 0xFFFFFFFFFFFFFFFF)

    if band(masked, 0x8000000000000000) != 0 do
      masked - 0x10000000000000000
    else
      masked
    end
  end

  @doc "Convert to unsigned 32-bit interpretation."
  @spec to_unsigned_32(integer()) :: non_neg_integer()
  def to_unsigned_32(value) do
    band(value, 0xFFFFFFFF)
  end

  @doc "Convert to unsigned 64-bit interpretation."
  @spec to_unsigned_64(integer()) :: non_neg_integer()
  def to_unsigned_64(value) do
    band(value, 0xFFFFFFFFFFFFFFFF)
  end

  # ===========================================================================
  # Helpers
  # ===========================================================================

  defp type_name(@i32), do: "i32"
  defp type_name(@i64), do: "i64"
  defp type_name(@f32), do: "f32"
  defp type_name(@f64), do: "f64"
  defp type_name(t), do: "0x#{Integer.to_string(t, 16)}"
end
