defmodule BuildTool.SourceInputRegistry do
  @moduledoc """
  The build tool's immutable package-local projection of Source Inputs v1.

  The neutral JSON is embedded into a generated `.ex` source by a deterministic
  sync command. The source collector therefore hashes its own projection as a
  normal Elixir source input; a running build tool never searches the repository
  for a test fixture. Native tests compare the complete bytes, structure, and
  domain-separated digest to the checked neutral authority.

  Selection is deliberately process-free. It decides whether inert relative
  candidate paths are inputs; native file opening and retained-handle safety
  are separate responsibilities.
  """

  alias BuildTool.{GlobMatch, TrackedArtifactUnicode17}

  @windows_reserved_basenames MapSet.new(
                                ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "CLOCK$"] ++
                                  Enum.map(1..9, &"COM#{&1}") ++
                                  Enum.map(1..9, &"LPT#{&1}") ++
                                  Enum.map(["¹", "²", "³"], &"COM#{&1}") ++
                                  Enum.map(["¹", "²", "³"], &"LPT#{&1}")
                              )

  @registry_bytes BuildTool.SourceInputRegistryData.bytes()
  @registry Jason.decode!(@registry_bytes)
  @digest_domain "coding-adventures/build-tool-language-source-input-registry/v1"

  def registry, do: @registry
  def snapshot_bytes, do: @registry_bytes

  def digest do
    canonical = @registry |> canonical_json() |> IO.iodata_to_binary()

    :crypto.hash(:sha256, [
      @digest_domain,
      <<0>>,
      <<byte_size(canonical)::unsigned-big-integer-size(64)>>,
      canonical
    ])
    |> Base.encode16(case: :lower)
  end

  @doc """
  Select package-local candidates using the exact seven registry roles.

  This accepts the neutral fixture's inert candidate shape and returns its
  sorted `files` shape. The same `selected_path?/5` lookup is used by native
  package hashing, so the conformance tests exercise production decisions.
  """
  def collect_candidates(options) when is_map(options) do
    if options["registry_sha256"] != digest() do
      raise ArgumentError, "source-input registry digest mismatch"
    end

    language = options["language"]
    entry = language_entry!(language)
    package_root = validate_package_root!(options["package_root"], language, entry)
    {mode, compiled} = compile_mode!(options["mode"], options["declared_srcs"])
    candidates = options["candidates"]

    unless is_list(candidates) and length(candidates) <= 100_000 do
      raise ArgumentError, "source candidate limit exceeded"
    end

    kinds = validate_candidates!(candidates)

    link_roots =
      kinds
      |> Enum.reject(fn {_path, kind} -> kind == "file" end)
      |> MapSet.new(fn {path, _kind} -> path end)

    candidates
    |> Enum.filter(fn candidate ->
      path = candidate["path"]

      candidate["kind"] == "file" and
        not MapSet.member?(link_roots, path) and
        not Enum.any?(ancestors(path), &MapSet.member?(link_roots, &1)) and
        selected_path?(entry, package_root, mode, compiled, path)
    end)
    |> Enum.map(fn candidate ->
      %{
        "path" => candidate["path"],
        "digest" =>
          candidate["content_hex"]
          |> Base.decode16!(case: :mixed)
          |> then(&:crypto.hash(:sha256, &1))
          |> Base.encode16(case: :lower)
      }
    end)
    |> Enum.sort_by(& &1["path"])
  end

  def language_entry!(language) when is_binary(language) do
    case Enum.find(@registry["languages"], &(&1["language"] == language)) do
      nil -> raise ArgumentError, "source-collection language is not registered: #{language}"
      entry -> entry
    end
  end

  def language_entry!(_), do: raise(ArgumentError, "source-collection language is not registered")

  def validate_package_root!(nil, _language, _entry), do: nil

  def validate_package_root!(root, language, entry) when is_binary(root) do
    parts = String.split(root, "/")

    valid =
      case parts do
        ["code", group, lane | package_parts]
        when group in ["packages", "programs"] and package_parts != [] ->
          lane == language

        ["code", "sites", _site] when language == "typescript" ->
          Enum.any?(entry["package_exact_inputs"], &(&1["package_root"] == root))

        _ ->
          false
      end

    if not valid or unsafe_relative_path?(root) do
      raise ArgumentError, "source package root does not match its language"
    end

    root
  end

  def validate_package_root!(_, _, _),
    do: raise(ArgumentError, "source package root does not match its language")

  def compile_mode!("extension", _declared_srcs), do: {:extension, []}

  def compile_mode!("declared_sources", declared_srcs) when is_list(declared_srcs),
    do: {:declared_sources, GlobMatch.compile_patterns!(declared_srcs)}

  def compile_mode!(_, _), do: raise(ArgumentError, "unsupported source-collection mode")

  def selected_path?(entry, package_root, mode, compiled, path) when is_binary(path) do
    components = String.split(path, "/")

    if unsafe_relative_path?(path) or
         Enum.any?(
           Enum.drop(components, -1),
           &(&1 in @registry["universal_inputs"]["generated_directory_components"])
         ) do
      false
    else
      basename = List.last(components)
      root_file? = length(components) == 1
      universal = @registry["universal_inputs"]

      fixed? =
        basename in universal["build_filenames"] or
          (root_file? and basename in universal["root_exact_basenames"]) or
          (root_file? and basename in entry["root_exact_basenames"]) or
          (root_file? and has_suffix?(basename, entry["root_variable_suffixes"])) or
          path in entry["root_exact_relative_paths"] or
          Enum.any?(entry["package_exact_inputs"], fn rule ->
            rule["package_root"] == package_root and path in rule["paths"]
          end)

      case mode do
        :extension ->
          fixed? or basename in entry["recursive_exact_basenames"] or
            has_suffix?(basename, entry["recursive_suffixes"]) or
            Enum.any?(entry["scoped_inputs"], &scoped_match?(&1, path, basename))

        :declared_sources ->
          fixed? or GlobMatch.match_any_compiled_path?(compiled, path)
      end
    end
  end

  defp scoped_match?(rule, path, basename) do
    in_scope =
      case rule["scope"] do
        "root" -> not String.contains?(path, "/")
        "subtree" -> String.starts_with?(path, rule["path_prefix"] <> "/")
      end

    in_scope and (basename in rule["exact_basenames"] or has_suffix?(basename, rule["suffixes"]))
  end

  defp has_suffix?(basename, suffixes),
    do: Enum.any?(suffixes, &String.ends_with?(basename, &1))

  defp validate_candidates!(candidates) do
    {kinds, _identities} =
      Enum.reduce(candidates, {%{}, MapSet.new()}, fn candidate, {paths, identities} ->
        unless is_map(candidate) and candidate["kind"] in ["file", "symlink", "reparse_point"] do
          raise ArgumentError, "invalid source candidate kind"
        end

        path = candidate["path"]

        if unsafe_relative_path?(path) do
          raise ArgumentError, "invalid source candidate path"
        end

        identity = TrackedArtifactUnicode17.casefold(path)

        if MapSet.member?(identities, identity) do
          raise ArgumentError, "source candidate identity collision"
        end

        {Map.put(paths, path, candidate["kind"]), MapSet.put(identities, identity)}
      end)

    Enum.each(kinds, fn {path, _kind} ->
      if Enum.any?(ancestors(path), &(Map.get(kinds, &1) == "file")) do
        raise ArgumentError, "source candidate file-prefix conflict"
      end
    end)

    kinds
  end

  defp ancestors(path) do
    path
    |> String.split("/")
    |> Enum.drop(-1)
    |> Enum.scan(fn component, prefix -> prefix <> "/" <> component end)
  end

  defp unsafe_relative_path?(path) when is_binary(path) do
    parts = String.split(path, "/")

    path == "" or not String.valid?(path) or byte_size(path) > 2048 or
      TrackedArtifactUnicode17.nfc(path) != path or
      Enum.any?(parts, fn part ->
        part in ["", ".", ".."] or String.ends_with?(part, [" ", "."]) or
          String.contains?(part, ["\\", ":", "<", ">", "\"", "|", "?", "*"]) or
          Regex.match?(~r/[\p{Cc}\p{Cf}]/u, part) or
          part
          |> String.split(".", parts: 2)
          |> hd()
          |> TrackedArtifactUnicode17.full_uppercase()
          |> then(&MapSet.member?(@windows_reserved_basenames, &1))
      end)
  end

  defp unsafe_relative_path?(_), do: true

  defp canonical_json(value) when is_map(value) do
    fields =
      value
      |> Enum.sort_by(fn {key, _} -> key end)
      |> Enum.map(fn {key, item} -> [Jason.encode!(key), ":", canonical_json(item)] end)

    ["{", Enum.intersperse(fields, ","), "}"]
  end

  defp canonical_json(value) when is_list(value),
    do: ["[", value |> Enum.map(&canonical_json/1) |> Enum.intersperse(","), "]"]

  defp canonical_json(value), do: Jason.encode!(value)
end
