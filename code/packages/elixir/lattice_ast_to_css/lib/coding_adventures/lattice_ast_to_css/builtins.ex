defmodule CodingAdventures.LatticeAstToCss.Builtins do
  @moduledoc """
  Built-in function registry for Lattice v2.

  Built-in functions are called at compile time during Lattice-to-CSS
  transformation. They are organized into five categories:

  1. **Map functions**: `map-get`, `map-keys`, `map-values`, `map-has-key`,
     `map-merge`, `map-remove`
  2. **Color functions**: `lighten`, `darken`, `saturate`, `desaturate`,
     `adjust-hue`, `complement`, `mix`, `rgba`, `red`, `green`, `blue`,
     `hue`, `saturation`, `lightness`
  3. **List functions**: `nth`, `length`, `join`, `append`, `index`
  4. **Type functions**: `type-of`, `unit`, `unitless`, `comparable`
  5. **Math functions**: `math.div`, `math.floor`, `math.ceil`, `math.round`,
     `math.abs`, `math.min`, `math.max`

  Each function takes a list of `lattice_value` arguments and returns a
  `lattice_value`. The scope is passed for potential variable lookups but
  most built-ins don't need it.

  ## Usage

      case Builtins.call("map-get", [map_val, key_val]) do
        {:ok, result} -> result
        {:error, msg} -> handle_error(msg)
      end
  """

  alias CodingAdventures.LatticeAstToCss.Values

  @doc """
  Look up and call a built-in function by name.

  Returns `{:ok, result}` if the function exists and succeeds,
  `{:error, message}` if it fails, or `:not_found` if the name
  is not a built-in.
  """
  @spec call(String.t(), [Values.lattice_value()]) :: {:ok, Values.lattice_value()} | {:error, String.t()} | :not_found
  def call(name, args) do
    case registry()[name] do
      nil -> :not_found
      func -> func.(args)
    end
  end

  @doc "Check if a function name is a built-in."
  @spec builtin?(String.t()) :: boolean()
  def builtin?(name), do: Map.has_key?(registry(), name)

  @doc "Return the set of all built-in function names."
  @spec names() :: MapSet.t(String.t())
  def names, do: MapSet.new(Map.keys(registry()))

  # ---------------------------------------------------------------------------
  # Registry
  # ---------------------------------------------------------------------------

  defp registry do
    %{
      # Map functions
      "map-get" => &builtin_map_get/1,
      "map-keys" => &builtin_map_keys/1,
      "map-values" => &builtin_map_values/1,
      "map-has-key" => &builtin_map_has_key/1,
      "map-merge" => &builtin_map_merge/1,
      "map-remove" => &builtin_map_remove/1,
      # Color functions
      "lighten" => &builtin_lighten/1,
      "darken" => &builtin_darken/1,
      "saturate" => &builtin_saturate/1,
      "desaturate" => &builtin_desaturate/1,
      "adjust-hue" => &builtin_adjust_hue/1,
      "complement" => &builtin_complement/1,
      "mix" => &builtin_mix/1,
      "rgba" => &builtin_rgba/1,
      "red" => &builtin_red/1,
      "green" => &builtin_green/1,
      "blue" => &builtin_blue/1,
      "hue" => &builtin_hue/1,
      "saturation" => &builtin_saturation_fn/1,
      "lightness" => &builtin_lightness/1,
      # List functions
      "nth" => &builtin_nth/1,
      "length" => &builtin_length/1,
      "join" => &builtin_join/1,
      "append" => &builtin_append/1,
      "index" => &builtin_index/1,
      # Type functions
      "type-of" => &builtin_type_of/1,
      "unit" => &builtin_unit/1,
      "unitless" => &builtin_unitless/1,
      "comparable" => &builtin_comparable/1,
      # Math functions
      "math.div" => &builtin_math_div/1,
      "math.floor" => &builtin_math_floor/1,
      "math.ceil" => &builtin_math_ceil/1,
      "math.round" => &builtin_math_round/1,
      "math.abs" => &builtin_math_abs/1,
      "math.min" => &builtin_math_min/1,
      "math.max" => &builtin_math_max/1
    }
  end

  # ---------------------------------------------------------------------------
  # Map Functions
  # ---------------------------------------------------------------------------

  defp builtin_map_get([{:map, _} = m, key | _]) do
    key_str = key |> Values.to_css() |> String.trim("\"")
    case Values.map_get(m, key_str) do
      {:ok, val} -> {:ok, val}
      :error -> {:ok, :null}
    end
  end

  defp builtin_map_get(_), do: {:error, "map-get requires 2 arguments"}

  defp builtin_map_keys([{:map, _} = m]) do
    {:ok, Values.map_keys(m)}
  end

  defp builtin_map_keys(_), do: {:error, "map-keys requires 1 argument"}

  defp builtin_map_values([{:map, _} = m]) do
    {:ok, Values.map_values(m)}
  end

  defp builtin_map_values(_), do: {:error, "map-values requires 1 argument"}

  defp builtin_map_has_key([{:map, _} = m, key | _]) do
    key_str = key |> Values.to_css() |> String.trim("\"")
    {:ok, {:bool, Values.map_has_key?(m, key_str)}}
  end

  defp builtin_map_has_key(_), do: {:error, "map-has-key requires 2 arguments"}

  defp builtin_map_merge([{:map, _} = m1, {:map, _} = m2 | _]) do
    {:ok, Values.map_merge(m1, m2)}
  end

  defp builtin_map_merge(_), do: {:error, "map-merge requires 2 map arguments"}

  defp builtin_map_remove([{:map, _} = m | rest_args]) do
    keys = Enum.map(rest_args, fn v -> v |> Values.to_css() |> String.trim("\"") end)
    {:ok, Values.map_remove(m, keys)}
  end

  defp builtin_map_remove(_), do: {:error, "map-remove requires at least 1 argument"}

  # ---------------------------------------------------------------------------
  # Color Functions
  # ---------------------------------------------------------------------------

  defp ensure_color({:color, hex}), do: {:ok, hex}
  defp ensure_color(other), do: {:error, "Expected a color, got #{Values.type_name_of(other)}"}

  defp ensure_amount(val) do
    case Values.get_numeric_value(val) do
      {:ok, n} when n >= 0 and n <= 100 -> {:ok, n}
      {:ok, _} -> {:error, "Amount must be between 0% and 100%"}
      :error -> {:error, "Expected a number"}
    end
  end

  defp ensure_numeric(val) do
    case Values.get_numeric_value(val) do
      {:ok, n} -> {:ok, n}
      :error -> {:error, "Expected a number, got #{Values.type_name_of(val)}"}
    end
  end

  defp builtin_lighten([color_val, amount_val | _]) do
    with {:ok, hex} <- ensure_color(color_val),
         {:ok, amount} <- ensure_amount(amount_val) do
      {h, s, l, a} = Values.color_to_hsl(hex)
      {:ok, Values.color_from_hsl(h, s, min(100.0, l + amount), a)}
    end
  end

  defp builtin_lighten(_), do: {:error, "lighten requires 2 arguments"}

  defp builtin_darken([color_val, amount_val | _]) do
    with {:ok, hex} <- ensure_color(color_val),
         {:ok, amount} <- ensure_amount(amount_val) do
      {h, s, l, a} = Values.color_to_hsl(hex)
      {:ok, Values.color_from_hsl(h, s, max(0.0, l - amount), a)}
    end
  end

  defp builtin_darken(_), do: {:error, "darken requires 2 arguments"}

  defp builtin_saturate([color_val, amount_val | _]) do
    with {:ok, hex} <- ensure_color(color_val),
         {:ok, amount} <- ensure_amount(amount_val) do
      {h, s, l, a} = Values.color_to_hsl(hex)
      {:ok, Values.color_from_hsl(h, min(100.0, s + amount), l, a)}
    end
  end

  defp builtin_saturate(_), do: {:error, "saturate requires 2 arguments"}

  defp builtin_desaturate([color_val, amount_val | _]) do
    with {:ok, hex} <- ensure_color(color_val),
         {:ok, amount} <- ensure_amount(amount_val) do
      {h, s, l, a} = Values.color_to_hsl(hex)
      {:ok, Values.color_from_hsl(h, max(0.0, s - amount), l, a)}
    end
  end

  defp builtin_desaturate(_), do: {:error, "desaturate requires 2 arguments"}

  defp builtin_adjust_hue([color_val, degrees_val | _]) do
    with {:ok, hex} <- ensure_color(color_val),
         {:ok, degrees} <- ensure_numeric(degrees_val) do
      {h, s, l, a} = Values.color_to_hsl(hex)
      {:ok, Values.color_from_hsl(:math.fmod(h + degrees, 360.0), s, l, a)}
    end
  end

  defp builtin_adjust_hue(_), do: {:error, "adjust-hue requires 2 arguments"}

  defp builtin_complement([color_val | _]) do
    with {:ok, hex} <- ensure_color(color_val) do
      {h, s, l, a} = Values.color_to_hsl(hex)
      {:ok, Values.color_from_hsl(:math.fmod(h + 180.0, 360.0), s, l, a)}
    end
  end

  defp builtin_complement(_), do: {:error, "complement requires 1 argument"}

  defp builtin_mix([c1_val, c2_val | rest]) do
    with {:ok, hex1} <- ensure_color(c1_val),
         {:ok, hex2} <- ensure_color(c2_val) do
      weight = case rest do
        [w | _] ->
          case Values.get_numeric_value(w) do
            {:ok, n} -> n / 100.0
            :error -> 0.5
          end
        _ -> 0.5
      end
      {r1, g1, b1, a1} = Values.color_to_rgb(hex1)
      {r2, g2, b2, a2} = Values.color_to_rgb(hex2)
      r = round(r1 * weight + r2 * (1 - weight))
      g = round(g1 * weight + g2 * (1 - weight))
      b = round(b1 * weight + b2 * (1 - weight))
      a = a1 * weight + a2 * (1 - weight)
      {:ok, Values.color_from_rgb(r, g, b, a)}
    end
  end

  defp builtin_mix(_), do: {:error, "mix requires at least 2 arguments"}

  defp builtin_rgba([{:color, hex}, alpha_val | _]) do
    with {:ok, alpha} <- ensure_numeric(alpha_val) do
      {r, g, b, _} = Values.color_to_rgb(hex)
      {:ok, Values.color_from_rgb(r, g, b, alpha)}
    end
  end

  defp builtin_rgba([r_val, g_val, b_val, a_val | _]) do
    with {:ok, r} <- ensure_numeric(r_val),
         {:ok, g} <- ensure_numeric(g_val),
         {:ok, b} <- ensure_numeric(b_val),
         {:ok, a} <- ensure_numeric(a_val) do
      {:ok, Values.color_from_rgb(round(r), round(g), round(b), a)}
    end
  end

  defp builtin_rgba(_), do: {:ok, :null}

  defp builtin_red([{:color, hex} | _]) do
    {r, _, _, _} = Values.color_to_rgb(hex)
    {:ok, {:number, r * 1.0}}
  end

  defp builtin_red(_), do: {:error, "red requires a color argument"}

  defp builtin_green([{:color, hex} | _]) do
    {_, g, _, _} = Values.color_to_rgb(hex)
    {:ok, {:number, g * 1.0}}
  end

  defp builtin_green(_), do: {:error, "green requires a color argument"}

  defp builtin_blue([{:color, hex} | _]) do
    {_, _, b, _} = Values.color_to_rgb(hex)
    {:ok, {:number, b * 1.0}}
  end

  defp builtin_blue(_), do: {:error, "blue requires a color argument"}

  defp builtin_hue([{:color, hex} | _]) do
    {h, _, _, _} = Values.color_to_hsl(hex)
    {:ok, {:dimension, round(h) * 1.0, "deg"}}
  end

  defp builtin_hue(_), do: {:error, "hue requires a color argument"}

  defp builtin_saturation_fn([{:color, hex} | _]) do
    {_, s, _, _} = Values.color_to_hsl(hex)
    {:ok, {:percentage, round(s) * 1.0}}
  end

  defp builtin_saturation_fn(_), do: {:error, "saturation requires a color argument"}

  defp builtin_lightness([{:color, hex} | _]) do
    {_, _, l, _} = Values.color_to_hsl(hex)
    {:ok, {:percentage, round(l) * 1.0}}
  end

  defp builtin_lightness(_), do: {:error, "lightness requires a color argument"}

  # ---------------------------------------------------------------------------
  # List Functions
  # ---------------------------------------------------------------------------

  defp builtin_nth([lst, n_val | _]) do
    case Values.get_numeric_value(n_val) do
      {:ok, n} ->
        idx = trunc(n)
        if idx < 1 do
          {:error, "List index must be 1 or greater"}
        else
          items = case lst do
            {:list, items} -> items
            other -> [other]
          end
          if idx > length(items) do
            {:error, "Index #{idx} out of bounds for list of length #{length(items)}"}
          else
            {:ok, Enum.at(items, idx - 1)}
          end
        end
      :error ->
        {:error, "nth requires a numeric index"}
    end
  end

  defp builtin_nth(_), do: {:error, "nth requires 2 arguments"}

  defp builtin_length([val | _]) do
    count = case val do
      {:list, items} -> length(items)
      {:map, items} -> length(items)
      _ -> 1
    end
    {:ok, {:number, count * 1.0}}
  end

  defp builtin_length(_), do: {:error, "length requires 1 argument"}

  defp builtin_join([l1, l2 | _]) do
    items1 = case l1 do
      {:list, items} -> items
      other -> [other]
    end
    items2 = case l2 do
      {:list, items} -> items
      other -> [other]
    end
    {:ok, {:list, items1 ++ items2}}
  end

  defp builtin_join(_), do: {:error, "join requires at least 2 arguments"}

  defp builtin_append([lst, val | _]) do
    items = case lst do
      {:list, items} -> items
      other -> [other]
    end
    {:ok, {:list, items ++ [val]}}
  end

  defp builtin_append(_), do: {:error, "append requires at least 2 arguments"}

  defp builtin_index([lst, target | _]) do
    items = case lst do
      {:list, items} -> items
      other -> [other]
    end
    target_str = Values.to_css(target)
    case Enum.find_index(items, fn item -> Values.to_css(item) == target_str end) do
      nil -> {:ok, :null}
      idx -> {:ok, {:number, (idx + 1) * 1.0}}
    end
  end

  defp builtin_index(_), do: {:error, "index requires 2 arguments"}

  # ---------------------------------------------------------------------------
  # Type Introspection Functions
  # ---------------------------------------------------------------------------

  defp builtin_type_of([val | _]) do
    {:ok, {:string, Values.type_name_of(val)}}
  end

  defp builtin_type_of(_), do: {:error, "type-of requires 1 argument"}

  defp builtin_unit([val | _]) do
    result = case val do
      {:dimension, _, u} -> u
      {:percentage, _} -> "%"
      {:number, _} -> ""
      _ -> ""
    end
    {:ok, {:string, result}}
  end

  defp builtin_unit(_), do: {:error, "unit requires 1 argument"}

  defp builtin_unitless([val | _]) do
    {:ok, {:bool, match?({:number, _}, val)}}
  end

  defp builtin_unitless(_), do: {:error, "unitless requires 1 argument"}

  defp builtin_comparable([a, b | _]) do
    result = cond do
      match?({:dimension, _, _}, a) and match?({:dimension, _, _}, b) ->
        elem(a, 2) == elem(b, 2)
      match?({:number, _}, a) or match?({:number, _}, b) ->
        is_numeric?(a) and is_numeric?(b)
      true ->
        Values.type_name_of(a) == Values.type_name_of(b) and is_numeric?(a)
    end
    {:ok, {:bool, result}}
  end

  defp builtin_comparable(_), do: {:error, "comparable requires 2 arguments"}

  defp is_numeric?({:number, _}), do: true
  defp is_numeric?({:dimension, _, _}), do: true
  defp is_numeric?({:percentage, _}), do: true
  defp is_numeric?(_), do: false

  # ---------------------------------------------------------------------------
  # Math Functions
  # ---------------------------------------------------------------------------

  defp builtin_math_div([a, b | _]) do
    with {:ok, b_val} <- ensure_numeric(b),
         {:ok, a_val} <- ensure_numeric(a) do
      if b_val == 0 do
        {:error, "Division by zero"}
      else
        result = a_val / b_val
        val = cond do
          match?({:dimension, _, _}, a) and match?({:number, _}, b) ->
            {:dimension, result, elem(a, 2)}
          match?({:dimension, _, _}, a) and match?({:dimension, _, _}, b) and elem(a, 2) == elem(b, 2) ->
            {:number, result}
          match?({:percentage, _}, a) and match?({:number, _}, b) ->
            {:percentage, result}
          true ->
            {:number, result}
        end
        {:ok, val}
      end
    end
  end

  defp builtin_math_div(_), do: {:error, "math.div requires 2 arguments"}

  defp builtin_math_floor([val | _]) do
    with {:ok, n} <- ensure_numeric(val) do
      result = Float.floor(n)
      {:ok, preserve_unit(val, result)}
    end
  end

  defp builtin_math_floor(_), do: {:error, "math.floor requires 1 argument"}

  defp builtin_math_ceil([val | _]) do
    with {:ok, n} <- ensure_numeric(val) do
      result = Float.ceil(n)
      {:ok, preserve_unit(val, result)}
    end
  end

  defp builtin_math_ceil(_), do: {:error, "math.ceil requires 1 argument"}

  defp builtin_math_round([val | _]) do
    with {:ok, n} <- ensure_numeric(val) do
      result = round(n) * 1.0
      {:ok, preserve_unit(val, result)}
    end
  end

  defp builtin_math_round(_), do: {:error, "math.round requires 1 argument"}

  defp builtin_math_abs([val | _]) do
    with {:ok, n} <- ensure_numeric(val) do
      result = abs(n)
      {:ok, preserve_unit(val, result)}
    end
  end

  defp builtin_math_abs(_), do: {:error, "math.abs requires 1 argument"}

  defp builtin_math_min(args) when is_list(args) and length(args) > 0 do
    case ensure_all_numeric(args) do
      {:ok, _} ->
        best = Enum.min_by(args, fn v ->
          {:ok, n} = Values.get_numeric_value(v)
          n
        end)
        {:ok, best}
      err -> err
    end
  end

  defp builtin_math_min(_), do: {:error, "math.min requires at least 1 argument"}

  defp builtin_math_max(args) when is_list(args) and length(args) > 0 do
    case ensure_all_numeric(args) do
      {:ok, _} ->
        best = Enum.max_by(args, fn v ->
          {:ok, n} = Values.get_numeric_value(v)
          n
        end)
        {:ok, best}
      err -> err
    end
  end

  defp builtin_math_max(_), do: {:error, "math.max requires at least 1 argument"}

  # Helpers

  defp preserve_unit({:dimension, _, u}, result), do: {:dimension, result, u}
  defp preserve_unit({:percentage, _}, result), do: {:percentage, result}
  defp preserve_unit(_, result), do: {:number, result}

  defp ensure_all_numeric(args) do
    if Enum.all?(args, fn v -> Values.get_numeric_value(v) != :error end) do
      {:ok, true}
    else
      {:error, "All arguments must be numbers"}
    end
  end
end
