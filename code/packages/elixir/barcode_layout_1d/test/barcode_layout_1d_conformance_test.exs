defmodule CodingAdventures.BarcodeLayout1DConformanceTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BarcodeLayout1D, as: Layout
  alias CodingAdventures.BarcodeLayout1D.V1Error

  @fixture_root Path.expand("../../../../specs/fixtures/barcode-layout-1d-v1", __DIR__)
  @max_bytes 131_072
  @max_depth 8

  # Keep JSON decoding in the test boundary. The production geometry code has
  # no filesystem or parser authority. OTP's object callbacks also let us
  # reject duplicate keys before they can be collapsed into a map.
  defp decode_unique!(bytes) when byte_size(bytes) <= @max_bytes do
    decoders = %{
      object_start: fn _ -> %{} end,
      object_push: fn key, value, object ->
        if Map.has_key?(object, key), do: raise(ArgumentError, "duplicate fixture key")
        Map.put(object, key, value)
      end,
      object_finish: fn object, previous -> {object, previous} end
    }

    try do
      case :json.decode(bytes, :ok, decoders) do
        {value, :ok, <<>>} -> value
        _ -> raise(ArgumentError, "trailing fixture bytes")
      end
    rescue
      ErlangError -> raise(ArgumentError, "invalid fixture JSON")
    end
  end

  defp decode_unique!(_), do: raise(ArgumentError, "fixture too large")

  defp depth!(value, level \\ 0, limit \\ @max_depth) do
    if level > limit, do: raise(ArgumentError, "fixture too deep")

    case value do
      object when is_map(object) ->
        Enum.each(object, fn {key, child} ->
          depth!(key, level + 1, limit)
          depth!(child, level + 1, limit)
        end)

      list when is_list(list) ->
        Enum.each(list, &depth!(&1, level + 1, limit))

      text when is_binary(text) ->
        if not String.valid?(text), do: raise(ArgumentError, "invalid fixture scalar")

      number when is_float(number) ->
        if not (number == number and abs(number) < 1.0e308),
          do: raise(ArgumentError, "invalid fixture number")

      _ ->
        :ok
    end

    value
  end

  defp read_fixture!(name, depth_limit \\ @max_depth) do
    path = Path.join(@fixture_root, name)
    {:ok, file} = File.open(path, [:read, :binary])

    try do
      bytes = IO.binread(file, @max_bytes + 1)
      if bytes == :eof, do: raise(ArgumentError, "empty fixture")
      bytes |> decode_unique!() |> depth!(0, depth_limit)
    after
      File.close(file)
    end
  end

  defp load_cases! do
    schema = read_fixture!("schema.json", 24)
    if not is_map(schema), do: raise(ArgumentError, "invalid schema")

    document = read_fixture!("cases.json")
    cases = Map.fetch!(document, "cases")

    if document["schema_version"] != 1 or document["profile"] != "barcode-layout-1d-v1" or
         not is_list(cases) or length(cases) > 64 or length(cases) == 0,
       do: raise(ArgumentError, "invalid corpus")

    ids =
      Enum.map(cases, fn row ->
        if not is_map(row) or
             Map.keys(row) |> Enum.sort() != ["expected", "id", "input", "operation"],
           do: raise(ArgumentError, "invalid case")

        if not is_map(row["input"]) or not is_map(row["expected"]),
          do: raise(ArgumentError, "invalid case")

        row["id"]
      end)

    if length(Enum.uniq(ids)) != length(ids), do: raise(ArgumentError, "duplicate case")
    cases
  end

  defp pattern(input) do
    case input do
      %{"pattern" => value} ->
        value

      %{"repeat" => %{"token" => token, "count" => count} = repeated} ->
        suffix = Map.get(repeated, "suffix", "")

        unless is_integer(count) and count in 0..65_569 and is_binary(token) and
                 String.length(token) in 1..2 and String.length(suffix) <= 1,
               do: raise(ArgumentError, "invalid repeat")

        String.duplicate(token, count) <> suffix
    end
  end

  defp runs(input) do
    rows =
      case input do
        %{"runs" => value} ->
          value

        %{"repeatRuns" => %{"count" => count} = repeated} ->
          unless is_integer(count) and count in 0..40_980,
            do: raise(ArgumentError, "invalid repeatRuns")

          Enum.map(0..(count - 1)//1, fn index ->
            first = repeated["firstColor"]

            %{
              "color" =>
                if(rem(index, 2) == 0,
                  do: first,
                  else: if(first == "bar", do: "space", else: "bar")
                ),
              "modules" => repeated["modules"],
              "sourceLabel" => repeated["sourceLabel"],
              "sourceIndex" => repeated["sourceIndex"],
              "role" => repeated["role"]
            }
          end)
      end

    Enum.map(rows, fn row ->
      %{
        color: row["color"],
        modules: row["modules"],
        source_label: row["sourceLabel"],
        source_index: row["sourceIndex"],
        role: row["role"]
      }
    end)
  end

  defp symbols(input) do
    cond do
      Map.has_key?(input, "symbols") ->
        input["symbols"]

      Map.has_key?(input, "repeatSymbols") ->
        repeated = input["repeatSymbols"]
        count = repeated["count"]

        unless is_integer(count) and count in 0..40_980,
          do: raise(ArgumentError, "invalid repeatSymbols")

        Enum.map(0..(count - 1)//1, fn index ->
          %{
            "label" => repeated["label"],
            "modules" => repeated["modules"],
            "sourceIndex" => index,
            "role" => repeated["role"]
          }
        end)

      true ->
        nil
    end
  end

  defp execute(%{"operation" => "expand-binary", "input" => input}) do
    Layout.runs_from_binary_pattern_v1(pattern(input),
      source_label: input["sourceLabel"],
      source_index: input["sourceIndex"],
      role: input["role"]
    )
  end

  defp execute(%{"operation" => "expand-width", "input" => input}) do
    Layout.runs_from_width_pattern_v1(pattern(input),
      source_label: input["sourceLabel"],
      source_index: input["sourceIndex"],
      role: input["role"],
      narrow_marker: Map.get(input, "narrowMarker", "N"),
      wide_marker: Map.get(input, "wideMarker", "W"),
      narrow_modules: Map.get(input, "narrowModules", 1),
      wide_modules: Map.get(input, "wideModules", 3),
      starting_color: Map.get(input, "startingColor", "bar")
    )
  end

  defp execute(%{"operation" => "compute-layout", "input" => input}) do
    Layout.compute_barcode_1d_layout_v1(runs(input), input["quietZoneModules"], symbols(input))
  end

  defp execute(%{"operation" => "project-scene", "input" => input}) do
    render = Map.get(input, "renderConfig", %{})

    Layout.project_barcode_1d_scene_v1(runs(input), input["quietZoneModules"], %{
      render_config: %{
        module_width: Map.get(render, "moduleWidth", 4),
        bar_height: Map.get(render, "barHeight", 120),
        foreground: Map.get(render, "foreground", "#000000"),
        background: Map.get(render, "background", "#ffffff"),
        include_human_readable_text: Map.get(render, "includeHumanReadableText", false)
      },
      label: Map.get(input, "label", "1D barcode"),
      metadata: Map.get(input, "metadata", %{}),
      human_readable_text: Map.get(input, "humanReadableText"),
      symbols: symbols(input)
    })
  end

  defp run_projection(run) do
    %{
      "color" => run.color,
      "modules" => run.modules,
      "sourceLabel" => run.source_label,
      "sourceIndex" => run.source_index,
      "role" => run.role
    }
  end

  defp scene_projection(scene) do
    %{
      "width" => scene.width,
      "height" => scene.height,
      "background" => scene.background,
      "rectangles" =>
        Enum.map(scene.instructions, fn rect ->
          %{
            "x" => rect.x,
            "y" => rect.y,
            "width" => rect.width,
            "height" => rect.height,
            "fill" => rect.fill,
            "metadata" => rect.metadata
          }
        end),
      "metadata" => scene.metadata
    }
  end

  defp canonical_runs(runs) do
    objects =
      Enum.map(runs, fn run ->
        "{\"color\":" <>
          IO.iodata_to_binary(:json.encode(run["color"])) <>
          ",\"modules\":" <>
          Integer.to_string(run["modules"]) <>
          ",\"role\":" <>
          IO.iodata_to_binary(:json.encode(run["role"])) <>
          ",\"sourceIndex\":" <>
          Integer.to_string(run["sourceIndex"]) <>
          ",\"sourceLabel\":" <> IO.iodata_to_binary(:json.encode(run["sourceLabel"])) <> "}"
      end)

    "[" <> Enum.join(objects, ",") <> "]"
  end

  test "all 56 neutral cases execute through the native v1 facade" do
    cases = load_cases!()

    assert Enum.frequencies_by(cases, & &1["operation"]) ==
             %{
               "expand-binary" => 12,
               "expand-width" => 12,
               "compute-layout" => 19,
               "project-scene" => 13
             }

    Enum.each(cases, fn row ->
      expected = row["expected"]
      id = row["id"]

      cond do
        Map.has_key?(expected, "error") ->
          assert_raise V1Error, expected["error"], fn -> execute(row) end

        Map.has_key?(expected, "runs") ->
          assert Enum.map(execute(row), &run_projection/1) == expected["runs"], id

        Map.has_key?(expected, "runDigest") ->
          projected = Enum.map(execute(row), &run_projection/1)
          digest = expected["runDigest"]
          assert length(projected) == digest["runCount"], id
          assert Enum.sum(Enum.map(projected, & &1["modules"])) == digest["contentModules"], id
          assert hd(projected) == digest["firstRun"], id
          assert List.last(projected) == digest["lastRun"], id

          actual_sha =
            :crypto.hash(:sha256, canonical_runs(projected)) |> Base.encode16(case: :lower)

          assert actual_sha == digest["runsSha256"], id

        Map.has_key?(expected, "layout") ->
          assert execute(row) == expected["layout"], id

        true ->
          assert scene_projection(execute(row)) == expected["scene"], id
      end
    end)
  end

  test "both text request forms fail before native resolution" do
    resolver = fn -> send(self(), :resolver_called) end
    invalid_runs = [%{color: "unknown", modules: 0}]

    assert_raise V1Error, "human-readable-text-unsupported", fn ->
      Layout.project_barcode_1d_scene_v1(invalid_runs, 0, %{
        human_readable_text: "text",
        font_resolver: resolver
      })
    end

    refute_received :resolver_called

    assert_raise V1Error, "human-readable-text-unsupported", fn ->
      Layout.project_barcode_1d_scene_v1(invalid_runs, 0, %{
        render_config: %{include_human_readable_text: true},
        font_resolver: resolver
      })
    end

    refute_received :resolver_called
  end

  test "the bounded loader rejects duplicate keys, deep and malformed bytes" do
    assert_raise ArgumentError, fn -> decode_unique!(~s({"a":1,"a":2})) end
    assert_raise ArgumentError, fn -> decode_unique!(String.duplicate("x", @max_bytes + 1)) end
    assert_raise ArgumentError, fn -> decode_unique!(<<255>>) end
    assert_raise ArgumentError, fn -> depth!(Enum.reduce(1..9, 0, fn _, acc -> [acc] end)) end
  end
end
