defmodule CodingAdventures.TestBootstrap.CorpusJson.Canon do
  @moduledoc """
  A stable, TOTAL text form of a witness (used in messages and the CI digest).

  It never raises and never calls `inspect`, `to_string` or any protocol: user
  code can define loops or crashes there, and this runs while a verdict is being
  built.  It walks the term structurally instead.

      nil / true / false      nil true false
      integer                 decimal (above 100,000 bits: "<bigint N bits>")
      atom                    :name   (escaped)
      float                   Float.to_string
      binary, valid UTF-8     "quoted" with \\" \\\\ \\n \\r \\t and \\uXXXX escapes
      binary, other           0x<lowercase hex>
      list / tuple            [a,b]  {a,b}
      map / struct            %{k=>v,...}  %Mod{k=>v,...}   sorted by the bytes of k
      MapSet                  #MapSet[a,b]                   sorted by the bytes of a
      pid, ref, port, fun     #PID #Reference #Port #Function

  Sorting maps and sets by the bytes of their rendered key makes the output (and
  so the CI digest) independent of the VM's iteration order.  Nesting deeper than
  16 renders "...", more than 10,000 nodes or 1 MiB of output truncates (cut on
  a character boundary, then "..."), and a failure while rendering turns the
  WHOLE witness into "<unrenderable>".
  """
  import Bitwise

  @max_depth 16
  @max_nodes 10_000
  @max_output 1 <<< 20
  @max_bigint_bits 100_000
  @unrenderable "<unrenderable>"

  # counter cells
  @nodes 1
  @size 2

  def unrenderable, do: @unrenderable
  def max_output, do: @max_output

  @doc "Renders any term; never raises."
  @spec render(term()) :: binary()
  def render(value) do
    ctx = :atomics.new(2, signed: true)

    try do
      IO.iodata_to_binary(emit(value, 0, ctx))
    catch
      :throw, {:canon_trunc, partial} -> truncate(partial)
      _kind, _reason -> @unrenderable
    end
  end

  defp truncate(partial) do
    bin = IO.iodata_to_binary(partial)
    bin = binary_part(bin, 0, min(byte_size(bin), @max_output))
    trim_partial(bin) <> "..."
  end

  # Drops up to 3 trailing bytes of a character cut in half.
  defp trim_partial(bin), do: trim_partial(bin, 3)
  defp trim_partial(bin, 0), do: bin

  defp trim_partial(bin, n) do
    if String.valid?(bin) or byte_size(bin) == 0 do
      bin
    else
      trim_partial(binary_part(bin, 0, byte_size(bin) - 1), n - 1)
    end
  end

  # -- escaping ----------------------------------------------------------------------

  @doc """
  Escapes control characters (and, with `quote: true`, backslash and the double
  quote) so a witness can never forge a log line.  Binaries that are not valid
  UTF-8 become `0x<hex>`.
  """
  def escape(text, opts \\ []) do
    quote? = Keyword.get(opts, :quote, false)

    if String.valid?(text) do
      for <<c::utf8 <- text>>, into: "", do: escape_char(c, quote?)
    else
      "0x" <> Base.encode16(text, case: :lower)
    end
  end

  defp escape_char(?\n, _), do: "\\n"
  defp escape_char(?\r, _), do: "\\r"
  defp escape_char(?\t, _), do: "\\t"
  defp escape_char(?\\, true), do: "\\\\"
  defp escape_char(?", true), do: "\\\""
  defp escape_char(c, _) when c < 0x20, do: u(c)
  defp escape_char(0x7F, _), do: u(0x7F)
  defp escape_char(c, _) when c in 0x80..0x9F, do: u(c)
  defp escape_char(c, _) when c in [0x2028, 0x2029, 0x061C, 0x200E, 0x200F], do: u(c)
  defp escape_char(c, _) when c in 0x202A..0x202E, do: u(c)
  defp escape_char(c, _) when c in 0x2066..0x2069, do: u(c)
  defp escape_char(c, _), do: <<c::utf8>>

  defp u(c), do: "\\u" <> String.pad_leading(Integer.to_string(c, 16), 4, "0") |> String.downcase()

  # -- emission ------------------------------------------------------------------------

  # Every piece of output is counted; the piece that crosses the cap is thrown
  # as the (over-long) partial result, and containers prepend what they have.
  defp leaf(ctx, text) do
    size = :atomics.add_get(ctx, @size, byte_size(text))
    if size > @max_output, do: throw({:canon_trunc, text})
    text
  end

  defp count_node(ctx) do
    n = :atomics.add_get(ctx, @nodes, 1)
    if n > @max_nodes, do: throw({:canon_trunc, []})
    :ok
  end

  defp emit(value, depth, ctx) do
    count_node(ctx)
    emit_value(value, depth, ctx)
  end

  defp emit_value(nil, _, ctx), do: leaf(ctx, "nil")
  defp emit_value(true, _, ctx), do: leaf(ctx, "true")
  defp emit_value(false, _, ctx), do: leaf(ctx, "false")
  defp emit_value(v, _, ctx) when is_integer(v), do: leaf(ctx, integer_text(v))
  defp emit_value(v, _, ctx) when is_atom(v), do: leaf(ctx, ":" <> escape(Atom.to_string(v)))
  defp emit_value(v, _, ctx) when is_float(v), do: leaf(ctx, Float.to_string(v))
  defp emit_value(v, _, ctx) when is_binary(v), do: leaf(ctx, string_text(v))

  defp emit_value(v, _, ctx) when is_bitstring(v),
    do: leaf(ctx, "<<bits #{bit_size(v)}>>")

  defp emit_value(v, _, ctx) when is_pid(v), do: leaf(ctx, "#PID")
  defp emit_value(v, _, ctx) when is_reference(v), do: leaf(ctx, "#Reference")
  defp emit_value(v, _, ctx) when is_port(v), do: leaf(ctx, "#Port")
  defp emit_value(v, _, ctx) when is_function(v), do: leaf(ctx, "#Function")

  defp emit_value(v, depth, ctx) when is_list(v) do
    if depth >= @max_depth do
      leaf(ctx, "...")
    else
      {items, tail} = take_list(v, [], 0)

      seq(items, "[", "]", ctx, fn item -> emit(item, depth + 1, ctx) end, fn ->
        case tail do
          [] -> []
          t -> [leaf(ctx, "|"), emit(t, depth + 1, ctx)]
        end
      end)
    end
  end

  defp emit_value(v, depth, ctx) when is_tuple(v) do
    if depth >= @max_depth do
      leaf(ctx, "...")
    else
      count = min(tuple_size(v), @max_nodes + 1)
      items = for i <- 0..(count - 1)//1, do: elem(v, i)
      seq(items, "{", "}", ctx, fn item -> emit(item, depth + 1, ctx) end, fn -> [] end)
    end
  end

  defp emit_value(%{__struct__: MapSet, map: m}, depth, ctx) when is_map(m) do
    if depth >= @max_depth do
      leaf(ctx, "...")
    else
      sorted(Map.keys(m), "#MapSet[", "]", ctx, fn k -> [render_to_bin(k, depth + 1, ctx)] end)
    end
  end

  defp emit_value(%{__struct__: mod} = v, depth, ctx) when is_atom(mod) and is_map(v) do
    if depth >= @max_depth do
      leaf(ctx, "...")
    else
      name = String.replace_prefix(Atom.to_string(mod), "Elixir.", "")
      fields = v |> Map.delete(:__struct__) |> Map.to_list()
      map_entries(fields, "%" <> escape(name) <> "{", "}", depth, ctx)
    end
  end

  defp emit_value(v, depth, ctx) when is_map(v) do
    if depth >= @max_depth do
      leaf(ctx, "...")
    else
      map_entries(Map.to_list(v), "%{", "}", depth, ctx)
    end
  end

  defp emit_value(_other, _depth, ctx), do: leaf(ctx, "#Term")

  # At most @max_nodes + 1 items (the node counter truncates the rest); an
  # improper tail is returned separately.
  defp take_list([h | t], acc, n) when n <= @max_nodes, do: take_list(t, [h | acc], n + 1)
  defp take_list(rest, acc, _n) when is_list(rest), do: {Enum.reverse(acc), rest_tail(rest)}
  defp take_list(rest, acc, _n), do: {Enum.reverse(acc), rest}

  defp rest_tail([]), do: []
  # more proper elements remain: the node counter has already refused them, so
  # emitting the (non-empty) remainder as a tail truncates.
  defp rest_tail(other), do: other

  # A comma-joined sequence.  A truncation inside an item carries the partial
  # output of everything before it up to the top.
  defp seq(items, open, close, ctx, render, tail) do
    start = [leaf(ctx, open)]

    {acc, _first} =
      Enum.reduce(items, {start, true}, fn item, {acc, first} ->
        acc = if first, do: acc, else: [acc, leaf(ctx, ",")]

        piece =
          try do
            render.(item)
          catch
            :throw, {:canon_trunc, p} -> throw({:canon_trunc, [acc, p]})
          end

        {[acc, piece], false}
      end)

    tail_piece =
      try do
        tail.()
      catch
        :throw, {:canon_trunc, p} -> throw({:canon_trunc, [acc, p]})
      end

    [acc, tail_piece, leaf(ctx, close)]
  end

  defp map_entries(pairs, open, close, depth, ctx) do
    sorted(pairs, open, close, ctx, fn {k, v} ->
      [render_to_bin(k, depth + 1, ctx), "=>", render_to_bin(v, depth + 1, ctx)]
    end)
  end

  # Renders every entry to a binary, sorts by bytes, then joins.
  defp sorted(items, open, close, ctx, entry) do
    start = leaf(ctx, open)

    texts =
      Enum.reduce(items, [], fn item, done ->
        text =
          try do
            IO.iodata_to_binary(entry.(item))
          catch
            :throw, {:canon_trunc, p} ->
              throw({:canon_trunc, [start, Enum.intersperse(Enum.sort(done), ","), ",", p]})
          end

        [text | done]
      end)

    joined = Enum.intersperse(Enum.sort(texts), ",")
    # separators (n - 1 commas) and the closer are counted too
    if texts != [], do: leaf(ctx, String.duplicate(",", length(texts) - 1))
    [start, joined, leaf(ctx, close)]
  end

  defp render_to_bin(value, depth, ctx), do: IO.iodata_to_binary(emit(value, depth, ctx))

  defp integer_text(value) do
    bits = bit_len(abs(value))

    if bits > @max_bigint_bits,
      do: "<bigint #{bits} bits>",
      else: Integer.to_string(value)
  end

  defp bit_len(0), do: 0
  defp bit_len(x), do: CodingAdventures.TestBootstrap.Prng.bit_length(x)

  defp string_text(value) do
    if String.valid?(value),
      do: "\"" <> escape(value, quote: true) <> "\"",
      else: "0x" <> Base.encode16(value, case: :lower)
  end
end
