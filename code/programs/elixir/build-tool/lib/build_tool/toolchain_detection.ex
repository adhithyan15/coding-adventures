defmodule BuildTool.ToolchainDetection do
  @moduledoc """
  Pure, bounded CI toolchain detection over caller-supplied BUILD snapshots.

  The evaluator never reads the filesystem or environment. Production callers
  pass the raw content of the BUILD front already selected by discovery, while
  conformance callers provide every platform variant in an inert snapshot.
  """

  @max_build_bytes 65_536
  @max_build_lines 4_096
  @max_aggregate_build_bytes 1_048_576
  @declaration_prefix "# needs-toolchain:"

  @canonical_toolchains [
    "cpp",
    "dart",
    "dotnet",
    "elixir",
    "go",
    "haskell",
    "java",
    "kotlin",
    "lua",
    "ocaml",
    "perl",
    "python",
    "ruby",
    "rust",
    "swift",
    "typescript"
  ]
  @canonical_toolchain_set MapSet.new(@canonical_toolchains)

  @doc "Returns the complete, stable toolchain registry."
  def canonical_toolchains, do: @canonical_toolchains

  @doc """
  Parses exact, canonical `# needs-toolchain: NAME` comment records.

  A carriage return is stripped only when it is the CR half of a CRLF line
  terminator. Unknown, malformed, duplicate, or oversized declarations remain
  inert.
  """
  def parse_extra_toolchains(raw_content) when is_binary(raw_content) do
    if byte_size(raw_content) > @max_build_bytes or
         logical_line_count(raw_content) > @max_build_lines do
      []
    else
      lines = String.split(raw_content, "\n", trim: false)
      final_index = length(lines) - 1

      lines
      |> Enum.with_index()
      |> Enum.reduce({[], MapSet.new()}, fn {raw_line, index}, {declarations, seen} ->
        line =
          raw_line
          |> trim_leading_ascii_space()
          |> strip_crlf_carriage_return(index < final_index)
          |> trim_trailing_ascii_space()

        with true <- String.starts_with?(line, @declaration_prefix),
             suffix <-
               binary_part(
                 line,
                 byte_size(@declaration_prefix),
                 byte_size(line) - byte_size(@declaration_prefix)
               ),
             true <- starts_with_ascii_space?(suffix),
             name <- trim_ascii_space(suffix),
             true <- MapSet.member?(@canonical_toolchain_set, name),
             false <- MapSet.member?(seen, name) do
          {declarations ++ [name], MapSet.put(seen, name)}
        else
          _ -> {declarations, seen}
        end
      end)
      |> elem(0)
    end
  end

  @doc "Evaluates one bounded, platform-neutral toolchain snapshot."
  def evaluate_snapshot(platform, force_full, packages, scheduled_packages, forced_toolchains) do
    validate_snapshot_limits!(packages)

    prepared_packages =
      Enum.map(packages, fn package ->
        Map.put(
          package,
          :extra_toolchains,
          extra_toolchains_for_snapshot(package.build_files, platform)
        )
      end)

    evaluate(prepared_packages, scheduled_packages, force_full, forced_toolchains)
  end

  @doc """
  Evaluates production package snapshots whose `:build_content` is already the
  selected platform BUILD front.
  """
  def evaluate_packages(packages, scheduled_packages, force_full, forced_toolchains) do
    prepared_packages =
      Enum.map(packages, fn package ->
        Map.put(
          package,
          :extra_toolchains,
          parse_extra_toolchains(Map.get(package, :build_content, ""))
        )
      end)

    evaluate(prepared_packages, scheduled_packages, force_full, forced_toolchains)
  end

  defp evaluate(packages, scheduled_packages, force_full, forced_toolchains) do
    selected_packages = select_packages(packages, scheduled_packages)
    initial_toolchains = Map.new(@canonical_toolchains, &{&1, force_full})

    with {:ok, toolchains} <-
           evaluate_selected(selected_packages, initial_toolchains, force_full),
         {:ok, toolchains} <- apply_forced_toolchains(toolchains, forced_toolchains) do
      %{outcome: "ok", toolchains: toolchains, diagnostics: []}
    else
      {:error, package} ->
        diagnostic = %{code: "TOOLCHAIN_UNSUPPORTED", severity: "error"}

        diagnostic =
          if package == nil, do: diagnostic, else: Map.put(diagnostic, :package, package)

        %{outcome: "error", toolchains: %{}, diagnostics: [diagnostic]}
    end
  end

  defp evaluate_selected(packages, toolchains, force_full) do
    Enum.reduce_while(packages, {:ok, toolchains}, fn package, {:ok, acc} ->
      case toolchain_for_language(package.language) do
        {:ok, toolchain} ->
          if force_full do
            {:cont, {:ok, acc}}
          else
            updated =
              Enum.reduce(package.extra_toolchains, Map.put(acc, toolchain, true), fn extra,
                                                                                      flags ->
                Map.put(flags, extra, true)
              end)

            {:cont, {:ok, updated}}
          end

        :error ->
          {:halt, {:error, package.name}}
      end
    end)
  end

  defp apply_forced_toolchains(toolchains, forced_toolchains) do
    Enum.reduce_while(forced_toolchains, {:ok, toolchains}, fn toolchain, {:ok, acc} ->
      if MapSet.member?(@canonical_toolchain_set, toolchain) do
        {:cont, {:ok, Map.put(acc, toolchain, true)}}
      else
        {:halt, {:error, nil}}
      end
    end)
  end

  defp select_packages(packages, nil), do: packages

  defp select_packages(packages, scheduled_packages) do
    scheduled = MapSet.new(scheduled_packages)
    Enum.filter(packages, &MapSet.member?(scheduled, &1.name))
  end

  defp toolchain_for_language(language) do
    case language do
      "wasm" -> {:ok, "rust"}
      lang when lang in ["c", "cpp"] -> {:ok, "cpp"}
      lang when lang in ["csharp", "fsharp", "dotnet"] -> {:ok, "dotnet"}
      lang -> if MapSet.member?(@canonical_toolchain_set, lang), do: {:ok, lang}, else: :error
    end
  end

  defp extra_toolchains_for_snapshot(build_files, platform) do
    platform
    |> build_file_candidates()
    |> Enum.find_value([], fn filename ->
      case Map.fetch(build_files, filename) do
        {:ok, content} -> parse_extra_toolchains(content)
        :error -> nil
      end
    end)
  end

  defp build_file_candidates("darwin"), do: ["BUILD_mac", "BUILD_mac_and_linux", "BUILD"]
  defp build_file_candidates("linux"), do: ["BUILD_linux", "BUILD_mac_and_linux", "BUILD"]

  defp build_file_candidates(platform) when platform in ["windows", "win32"],
    do: ["BUILD_windows", "BUILD"]

  defp build_file_candidates(platform) do
    raise ArgumentError, "unsupported target platform: #{inspect(platform)}"
  end

  defp validate_snapshot_limits!(packages) do
    aggregate_bytes =
      Enum.reduce(packages, 0, fn package, aggregate ->
        Enum.reduce(package.build_files, aggregate, fn {_filename, content}, subtotal ->
          bytes = byte_size(content)

          if bytes > @max_build_bytes or logical_line_count(content) > @max_build_lines do
            raise ArgumentError, "toolchain BUILD snapshot exceeds its per-file resource ceiling"
          end

          subtotal + bytes
        end)
      end)

    if aggregate_bytes > @max_aggregate_build_bytes do
      raise ArgumentError, "toolchain BUILD snapshot exceeds its aggregate resource ceiling"
    end

    :ok
  end

  defp logical_line_count(content) do
    content
    |> :binary.matches("\n")
    |> length()
    |> Kernel.+(1)
  end

  defp strip_crlf_carriage_return(line, true) do
    if String.ends_with?(line, "\r"), do: binary_part(line, 0, byte_size(line) - 1), else: line
  end

  defp strip_crlf_carriage_return(line, false), do: line

  defp starts_with_ascii_space?(<<first, _rest::binary>>) when first in [?\s, ?\t], do: true
  defp starts_with_ascii_space?(_suffix), do: false

  defp trim_ascii_space(value) do
    value
    |> trim_leading_ascii_space()
    |> trim_trailing_ascii_space()
  end

  defp trim_leading_ascii_space(<<first, rest::binary>>) when first in [?\s, ?\t],
    do: trim_leading_ascii_space(rest)

  defp trim_leading_ascii_space(value), do: value

  defp trim_trailing_ascii_space(value) do
    case value do
      <<rest::binary-size(byte_size(value) - 1), last>> when last in [?\s, ?\t] ->
        trim_trailing_ascii_space(rest)

      _ ->
        value
    end
  end
end
