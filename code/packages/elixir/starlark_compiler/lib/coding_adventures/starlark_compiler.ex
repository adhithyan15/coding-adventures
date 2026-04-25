defmodule CodingAdventures.StarlarkCompiler do
  @moduledoc """
  Starlark compiler opcode metadata.

  This module mirrors the Rust `starlark-compiler` opcode contract so Elixir
  integrations can share bytecode values and operator mappings with the VM.
  """

  @type op ::
          :load_const
          | :pop
          | :dup
          | :load_none
          | :load_true
          | :load_false
          | :store_name
          | :load_name
          | :store_local
          | :load_local
          | :store_closure
          | :load_closure
          | :add
          | :sub
          | :mul
          | :div
          | :floor_div
          | :mod
          | :power
          | :negate
          | :bit_and
          | :bit_or
          | :bit_xor
          | :bit_not
          | :l_shift
          | :r_shift
          | :cmp_eq
          | :cmp_ne
          | :cmp_lt
          | :cmp_gt
          | :cmp_le
          | :cmp_ge
          | :cmp_in
          | :cmp_not_in
          | :not
          | :jump
          | :jump_if_false
          | :jump_if_true
          | :jump_if_false_or_pop
          | :jump_if_true_or_pop
          | :make_function
          | :call_function
          | :call_function_kw
          | :return
          | :build_list
          | :build_dict
          | :build_tuple
          | :list_append
          | :dict_set
          | :load_subscript
          | :store_subscript
          | :load_attr
          | :store_attr
          | :load_slice
          | :get_iter
          | :for_iter
          | :unpack_sequence
          | :load_module
          | :import_from
          | :print
          | :halt

  @type category ::
          :stack
          | :variable
          | :arithmetic
          | :comparison
          | :control_flow
          | :function
          | :collection
          | :subscript_attribute
          | :iteration
          | :module
          | :io
          | :vm_control

  @opcodes %{
    load_const: 0x01,
    pop: 0x02,
    dup: 0x03,
    load_none: 0x04,
    load_true: 0x05,
    load_false: 0x06,
    store_name: 0x10,
    load_name: 0x11,
    store_local: 0x12,
    load_local: 0x13,
    store_closure: 0x14,
    load_closure: 0x15,
    add: 0x20,
    sub: 0x21,
    mul: 0x22,
    div: 0x23,
    floor_div: 0x24,
    mod: 0x25,
    power: 0x26,
    negate: 0x27,
    bit_and: 0x28,
    bit_or: 0x29,
    bit_xor: 0x2A,
    bit_not: 0x2B,
    l_shift: 0x2C,
    r_shift: 0x2D,
    cmp_eq: 0x30,
    cmp_ne: 0x31,
    cmp_lt: 0x32,
    cmp_gt: 0x33,
    cmp_le: 0x34,
    cmp_ge: 0x35,
    cmp_in: 0x36,
    cmp_not_in: 0x37,
    not: 0x38,
    jump: 0x40,
    jump_if_false: 0x41,
    jump_if_true: 0x42,
    jump_if_false_or_pop: 0x43,
    jump_if_true_or_pop: 0x44,
    make_function: 0x50,
    call_function: 0x51,
    call_function_kw: 0x52,
    return: 0x53,
    build_list: 0x60,
    build_dict: 0x61,
    build_tuple: 0x62,
    list_append: 0x63,
    dict_set: 0x64,
    load_subscript: 0x70,
    store_subscript: 0x71,
    load_attr: 0x72,
    store_attr: 0x73,
    load_slice: 0x74,
    get_iter: 0x80,
    for_iter: 0x81,
    unpack_sequence: 0x82,
    load_module: 0x90,
    import_from: 0x91,
    print: 0xA0,
    halt: 0xFF
  }

  @byte_to_op Map.new(@opcodes, fn {op, byte} -> {byte, op} end)

  @binary_ops %{
    "+" => :add,
    "-" => :sub,
    "*" => :mul,
    "/" => :div,
    "//" => :floor_div,
    "%" => :mod,
    "**" => :power,
    "&" => :bit_and,
    "|" => :bit_or,
    "^" => :bit_xor,
    "<<" => :l_shift,
    ">>" => :r_shift
  }

  @compare_ops %{
    "==" => :cmp_eq,
    "!=" => :cmp_ne,
    "<" => :cmp_lt,
    ">" => :cmp_gt,
    "<=" => :cmp_le,
    ">=" => :cmp_ge,
    "in" => :cmp_in,
    "not in" => :cmp_not_in
  }

  @augmented_assign_ops %{
    "+=" => :add,
    "-=" => :sub,
    "*=" => :mul,
    "/=" => :div,
    "//=" => :floor_div,
    "%=" => :mod,
    "&=" => :bit_and,
    "|=" => :bit_or,
    "^=" => :bit_xor,
    "<<=" => :l_shift,
    ">>=" => :r_shift,
    "**=" => :power
  }

  @unary_ops %{
    "-" => :negate,
    "~" => :bit_not
  }

  @spec all_ops() :: [op()]
  def all_ops do
    @opcodes
    |> Enum.sort_by(fn {_op, byte} -> byte end)
    |> Enum.map(fn {op, _byte} -> op end)
  end

  @spec op_byte(op()) :: non_neg_integer() | nil
  def op_byte(op), do: Map.get(@opcodes, op)

  @spec op_from_byte(non_neg_integer()) :: op() | nil
  def op_from_byte(byte), do: Map.get(@byte_to_op, byte)

  @spec op_category(op()) :: category() | nil
  def op_category(op) do
    case op_byte(op) do
      nil -> nil
      byte -> category_from_high_nibble(Bitwise.bsr(byte, 4))
    end
  end

  @spec binary_op_map() :: %{String.t() => op()}
  def binary_op_map, do: @binary_ops

  @spec compare_op_map() :: %{String.t() => op()}
  def compare_op_map, do: @compare_ops

  @spec augmented_assign_map() :: %{String.t() => op()}
  def augmented_assign_map, do: @augmented_assign_ops

  @spec unary_op_map() :: %{String.t() => op()}
  def unary_op_map, do: @unary_ops

  @spec binary_opcode(String.t()) :: op() | nil
  def binary_opcode(operator), do: Map.get(@binary_ops, operator)

  @spec compare_opcode(String.t()) :: op() | nil
  def compare_opcode(operator), do: Map.get(@compare_ops, operator)

  @spec augmented_assign_opcode(String.t()) :: op() | nil
  def augmented_assign_opcode(operator), do: Map.get(@augmented_assign_ops, operator)

  @spec unary_opcode(String.t()) :: op() | nil
  def unary_opcode(operator), do: Map.get(@unary_ops, operator)

  defp category_from_high_nibble(0x0), do: :stack
  defp category_from_high_nibble(0x1), do: :variable
  defp category_from_high_nibble(0x2), do: :arithmetic
  defp category_from_high_nibble(0x3), do: :comparison
  defp category_from_high_nibble(0x4), do: :control_flow
  defp category_from_high_nibble(0x5), do: :function
  defp category_from_high_nibble(0x6), do: :collection
  defp category_from_high_nibble(0x7), do: :subscript_attribute
  defp category_from_high_nibble(0x8), do: :iteration
  defp category_from_high_nibble(0x9), do: :module
  defp category_from_high_nibble(0xA), do: :io
  defp category_from_high_nibble(0xF), do: :vm_control
  defp category_from_high_nibble(_), do: nil
end
