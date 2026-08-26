defmodule BuildTool.Validator do
  @moduledoc false

  alias BuildTool.TrackedArtifactUnicode17

  @tracked_artifact_component_identity "node_modules"
  @tracked_artifact_redacted_path "repository"
  @tracked_artifact_unicode_version TrackedArtifactUnicode17.unicode_version()
  @orphan_scan_root "code"
  @orphan_ledger_path "code/BUILD-EXEMPTIONS"
  @orphan_build_names [
    "BUILD",
    "BUILD_windows",
    "BUILD_mac",
    "BUILD_linux",
    "BUILD_mac_and_linux"
  ]
  @orphan_build_rank @orphan_build_names |> Enum.with_index() |> Map.new()
  @orphan_skip_components MapSet.new([
                            ".git",
                            "target",
                            "node_modules",
                            "vendor",
                            ".venv",
                            "_build",
                            "deps",
                            ".build",
                            "dist-newstyle",
                            ".cargo"
                          ])
  @python_blank_codepoints MapSet.new(
                             Enum.to_list(0x0009..0x000D) ++
                               Enum.to_list(0x001C..0x0020) ++
                               [
                                 0x0085,
                                 0x00A0,
                                 0x1680,
                                 0x2028,
                                 0x2029,
                                 0x202F,
                                 0x205F,
                                 0x3000
                               ] ++ Enum.to_list(0x2000..0x200A)
                           )
  @unsafe_tracked_artifact_scalars MapSet.new(~c[<>:"|?*])
  @windows_reserved_basenames MapSet.new(
                                ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "CLOCK$"] ++
                                  Enum.map(1..9, &"COM#{&1}") ++
                                  Enum.map(1..9, &"LPT#{&1}") ++
                                  Enum.map(["¹", "²", "³"], &"COM#{&1}") ++
                                  Enum.map(["¹", "²", "³"], &"LPT#{&1}")
                              )

  @ci_managed_toolchain_languages MapSet.new([
                                    "python",
                                    "ruby",
                                    "typescript",
                                    "rust",
                                    "elixir",
                                    "lua",
                                    "perl",
                                    "java",
                                    "kotlin",
                                    "haskell"
                                  ])

  def tracked_artifact_unicode_version, do: @tracked_artifact_unicode_version

  # Validate caller-supplied records rather than discovering paths here. Native
  # Git or filesystem enumeration stays outside this process-free policy oracle.
  def validate_tracked_artifact_snapshot(
        entries,
        unicode_version \\ @tracked_artifact_unicode_version
      ) do
    if unicode_version != @tracked_artifact_unicode_version do
      raise ArgumentError,
            "tracked artifact Unicode version must be #{@tracked_artifact_unicode_version}"
    end

    entries
    |> Enum.flat_map(&tracked_artifact_diagnostic/1)
    |> Enum.sort_by(fn diagnostic ->
      {
        String.to_charlist(diagnostic["code"]),
        String.to_charlist(diagnostic["path"]),
        [],
        diagnostic["details"] |> canonical_details() |> String.to_charlist()
      }
    end)
  end

  defp tracked_artifact_diagnostic(entry) do
    details = %{
      "ordinal" => Map.fetch!(entry, "ordinal"),
      "entry_kind" => Map.fetch!(entry, "entry_kind")
    }

    case normalize_tracked_artifact_path(Map.fetch!(entry, "path")) do
      {:error, problem} ->
        [
          %{
            "code" => "TRACKED_ARTIFACT_PATH_INVALID",
            "severity" => "error",
            "path" => @tracked_artifact_redacted_path,
            "details" => Map.put(details, "problem", problem)
          }
        ]

      {:ok, normalized_path} ->
        forbidden =
          normalized_path
          |> String.split("/", trim: false)
          |> Enum.any?(fn component ->
            TrackedArtifactUnicode17.nfkc_casefold(component) ==
              @tracked_artifact_component_identity
          end)

        if forbidden do
          [
            %{
              "code" => "TRACKED_ARTIFACT_FORBIDDEN",
              "severity" => "error",
              "path" => normalized_path,
              "details" => details
            }
          ]
        else
          []
        end
    end
  end

  # Separator replacement is deliberately lexical. Path helpers would erase
  # empty or dot segments before the portable policy can reject them.
  defp normalize_tracked_artifact_path(path) do
    normalized = String.replace(path, "\\", "/")
    segments = String.split(normalized, "/", trim: false)

    cond do
      normalized == "" ->
        {:error, "EMPTY"}

      length(String.to_charlist(normalized)) > 512 ->
        {:error, "TOO_LONG"}

      TrackedArtifactUnicode17.nfc(normalized) != normalized ->
        {:error, "NON_NFC"}

      String.starts_with?(normalized, "/") ->
        {:error, "ABSOLUTE"}

      Regex.match?(~r/^[A-Za-z]:/, normalized) ->
        {:error, "DRIVE_QUALIFIED"}

      Enum.any?(segments, &(&1 == "")) ->
        {:error, "EMPTY_SEGMENT"}

      unsafe_tracked_artifact_path?(normalized) ->
        {:error, "UNSAFE_CHARACTER"}

      true ->
        case Enum.find_value(segments, &tracked_artifact_segment_problem/1) do
          nil -> {:ok, normalized}
          problem -> {:error, problem}
        end
    end
  end

  defp unsafe_tracked_artifact_path?(path) do
    path
    |> String.to_charlist()
    |> Enum.any?(fn scalar ->
      scalar < 0x20 or MapSet.member?(@unsafe_tracked_artifact_scalars, scalar)
    end)
  end

  defp tracked_artifact_segment_problem(segment) when segment in [".", ".."],
    do: "DOT_SEGMENT"

  defp tracked_artifact_segment_problem(segment) do
    cond do
      String.ends_with?(segment, [" ", "."]) ->
        "TRAILING_DOT_OR_SPACE"

      segment
      |> String.split(".", parts: 2)
      |> hd()
      |> TrackedArtifactUnicode17.full_uppercase()
      |> then(&MapSet.member?(@windows_reserved_basenames, &1)) ->
        "RESERVED_BASENAME"

      true ->
        nil
    end
  end

  # Validate a closed Cargo/BUILD/ledger snapshot without touching the host.
  #
  # The values are inert records supplied by a separately reviewed discovery
  # layer. Keeping the policy here pure makes filesystem, Git, process,
  # environment, and network authority impossible to acquire accidentally.
  def validate_orphan_crate_snapshot(snapshot) do
    manifests =
      snapshot
      |> Map.fetch!("manifests")
      |> Enum.reject(&orphan_artifact_path?(Map.fetch!(&1, "path")))

    directories = snapshot |> Map.fetch!("directories") |> MapSet.new()
    manifest_by_path = Map.new(manifests, &{Map.fetch!(&1, "path"), &1})
    build_files = Map.fetch!(snapshot, "build_files")

    coverage =
      Map.new(manifests, fn manifest ->
        path = Map.fetch!(manifest, "path")
        {path, covering_orphan_build(build_files, path, "runnable")}
      end)

    empty_builds =
      Map.new(manifests, fn manifest ->
        path = Map.fetch!(manifest, "path")
        {path, covering_orphan_build(build_files, path, "empty")}
      end)

    {diagnostics, _seen, valid_exemptions} =
      snapshot
      |> Map.fetch!("exemptions")
      |> Enum.reduce({[], MapSet.new(), []}, fn exemption, {diagnostics, seen, valid} ->
        path = Map.fetch!(exemption, "path")
        {path_problem, identity} = orphan_path_problem_and_identity(path)
        duplicate = not is_nil(identity) and MapSet.member?(seen, identity)

        seen =
          if is_nil(identity) or duplicate do
            seen
          else
            MapSet.put(seen, identity)
          end

        problem =
          cond do
            Map.fetch!(exemption, "kind") not in ["EXCLUDED", "PENDING"] ->
              "UNKNOWN_KIND"

            python_blank?(Map.fetch!(exemption, "reason")) ->
              "REASON_MISSING"

            duplicate ->
              "DUPLICATE_PATH"

            true ->
              path_problem
          end

        if is_nil(problem) do
          {diagnostics, seen, [exemption | valid]}
        else
          diagnostic = %{
            "code" => "ORPHAN_EXEMPTION_INVALID",
            "severity" => "error",
            "path" => @orphan_ledger_path,
            "details" => %{
              "line" => Map.fetch!(exemption, "line"),
              "problem" => problem
            }
          }

          {[diagnostic | diagnostics], seen, valid}
        end
      end)

    {diagnostics, active_exemptions, pending_exemption_count} =
      Enum.reduce(
        valid_exemptions,
        {diagnostics, %{}, 0},
        fn exemption, {diagnostics, active, pending_count} ->
          path = Map.fetch!(exemption, "path")

          stale_problem =
            cond do
              not MapSet.member?(directories, path) -> "MISSING_DIRECTORY"
              not Map.has_key?(manifest_by_path, path) -> "NO_MANIFEST"
              not is_nil(Map.fetch!(coverage, path)) -> "COVERED"
              true -> nil
            end

          if is_nil(stale_problem) do
            pending_count =
              if Map.fetch!(exemption, "kind") == "PENDING",
                do: pending_count + 1,
                else: pending_count

            {diagnostics, Map.put(active, path, exemption), pending_count}
          else
            diagnostic = %{
              "code" => "ORPHAN_EXEMPTION_STALE",
              "severity" => "error",
              "path" => @orphan_ledger_path,
              "details" => %{
                "entry_path" => path,
                "kind" => Map.fetch!(exemption, "kind"),
                "line" => Map.fetch!(exemption, "line"),
                "problem" => stale_problem
              }
            }

            {[diagnostic | diagnostics], active, pending_count}
          end
        end
      )

    diagnostics =
      Enum.reduce(manifests, diagnostics, fn manifest, diagnostics ->
        path = Map.fetch!(manifest, "path")

        if not is_nil(Map.fetch!(coverage, path)) or Map.has_key?(active_exemptions, path) do
          diagnostics
        else
          diagnostic = orphan_manifest_diagnostic(manifest, Map.fetch!(empty_builds, path))
          [diagnostic | diagnostics]
        end
      end)
      |> Enum.sort_by(&orphan_diagnostic_sort_key/1)

    diagnostic_codes =
      diagnostics
      |> Enum.map(&Map.fetch!(&1, "code"))
      |> Enum.uniq()
      |> Enum.sort_by(&String.to_charlist/1)

    %{
      "valid" => diagnostics == [],
      "diagnostic_codes" => diagnostic_codes,
      "pending_exemption_count" => pending_exemption_count,
      "diagnostics" => diagnostics
    }
  end

  defp orphan_manifest_diagnostic(manifest, nil) do
    %{
      "code" => "ORPHAN_CRATE_UNLISTED",
      "severity" => "error",
      "path" => Map.fetch!(manifest, "path"),
      "details" => %{"manifest_kind" => Map.fetch!(manifest, "kind")}
    }
  end

  defp orphan_manifest_diagnostic(manifest, empty_build) do
    %{
      "code" => "ORPHAN_CRATE_EMPTY_BUILD",
      "severity" => "error",
      "path" => Map.fetch!(manifest, "path"),
      "details" => %{
        "build_path" => Map.fetch!(empty_build, "path"),
        "manifest_kind" => Map.fetch!(manifest, "kind")
      }
    }
  end

  defp covering_orphan_build(build_files, manifest_path, state) do
    build_files
    |> Enum.filter(fn build_file ->
      path = Map.fetch!(build_file, "path")
      {parent, name} = portable_parent_and_basename(path)

      Map.fetch!(build_file, "state") == state and
        under_orphan_scan_root?(parent) and
        (manifest_path == parent or String.starts_with?(manifest_path, parent <> "/")) and
        Map.has_key?(@orphan_build_rank, name)
    end)
    |> Enum.min_by(
      fn build_file ->
        path = Map.fetch!(build_file, "path")
        {parent, name} = portable_parent_and_basename(path)

        {
          -length(String.split(parent, "/", trim: false)),
          Map.fetch!(@orphan_build_rank, name),
          String.to_charlist(path)
        }
      end,
      fn -> nil end
    )
  end

  defp portable_parent_and_basename(path) do
    parts = String.split(path, "/", trim: false)
    {parts |> Enum.drop(-1) |> Enum.join("/"), List.last(parts)}
  end

  defp orphan_path_problem_and_identity(path) do
    if portable_orphan_path?(path) do
      identity = path |> TrackedArtifactUnicode17.nfc() |> TrackedArtifactUnicode17.casefold()

      problem =
        cond do
          not under_orphan_scan_root?(path) -> "PATH_OUTSIDE_SCAN"
          orphan_artifact_path?(path) -> "PATH_ARTIFACT"
          true -> nil
        end

      {problem, identity}
    else
      {"PATH_UNSAFE", nil}
    end
  end

  defp portable_orphan_path?(path) when is_binary(path) do
    if String.valid?(path) do
      segments = String.split(path, "/", trim: false)

      path != "" and
        length(String.to_charlist(path)) <= 512 and
        TrackedArtifactUnicode17.nfc(path) == path and
        not String.starts_with?(path, "/") and
        not String.contains?(path, "\\") and
        not String.contains?(path, "//") and
        not Regex.match?(~r/^[A-Za-z]:/, path) and
        not unsafe_orphan_path?(path) and
        Enum.all?(segments, &portable_orphan_segment?/1)
    else
      false
    end
  end

  defp portable_orphan_path?(_path), do: false

  defp unsafe_orphan_path?(path) do
    path
    |> String.to_charlist()
    |> Enum.any?(fn scalar ->
      scalar < 0x20 or MapSet.member?(@unsafe_tracked_artifact_scalars, scalar)
    end)
  end

  defp portable_orphan_segment?(segment) do
    basename = segment |> String.split(".", parts: 2) |> hd()

    segment != "" and
      segment not in [".", ".."] and
      not String.ends_with?(segment, [" ", "."]) and
      not MapSet.member?(
        @windows_reserved_basenames,
        TrackedArtifactUnicode17.full_uppercase(basename)
      )
  end

  defp under_orphan_scan_root?(path),
    do: path == @orphan_scan_root or String.starts_with?(path, @orphan_scan_root <> "/")

  defp orphan_artifact_path?(path) do
    path
    |> String.split("/", trim: false)
    |> Enum.any?(&MapSet.member?(@orphan_skip_components, &1))
  end

  defp python_blank?(value) when is_binary(value) do
    String.valid?(value) and
      value
      |> String.to_charlist()
      |> Enum.all?(&MapSet.member?(@python_blank_codepoints, &1))
  end

  defp python_blank?(_value), do: false

  defp orphan_diagnostic_sort_key(diagnostic) do
    {
      diagnostic |> Map.fetch!("code") |> String.to_charlist(),
      diagnostic |> Map.fetch!("path") |> String.to_charlist(),
      [],
      diagnostic |> Map.fetch!("details") |> canonical_details() |> String.to_charlist()
    }
  end

  defp canonical_details(details) do
    body =
      details
      |> Map.keys()
      |> Enum.sort()
      |> Enum.map(fn key ->
        "#{python_ascii_json_string(key)}: #{python_ascii_json_value(Map.fetch!(details, key))}"
      end)
      |> Enum.join(", ")

    "{#{body}}"
  end

  defp python_ascii_json_value(value) when is_binary(value),
    do: python_ascii_json_string(value)

  defp python_ascii_json_value(value) when is_integer(value), do: Integer.to_string(value)
  defp python_ascii_json_value(value), do: Jason.encode!(value)

  defp python_ascii_json_string(value) do
    escaped =
      value
      |> String.to_charlist()
      |> Enum.map_join(&python_ascii_json_scalar/1)

    "\"#{escaped}\""
  end

  defp python_ascii_json_scalar(?\"), do: "\\\""
  defp python_ascii_json_scalar(?\\), do: "\\\\"
  defp python_ascii_json_scalar(0x08), do: "\\b"
  defp python_ascii_json_scalar(0x0C), do: "\\f"
  defp python_ascii_json_scalar(?\n), do: "\\n"
  defp python_ascii_json_scalar(?\r), do: "\\r"
  defp python_ascii_json_scalar(?\t), do: "\\t"

  defp python_ascii_json_scalar(scalar) when scalar in 0x20..0x7E,
    do: <<scalar::utf8>>

  defp python_ascii_json_scalar(scalar) when scalar <= 0xFFFF,
    do: "\\u" <> python_json_hex(scalar)

  defp python_ascii_json_scalar(scalar) do
    supplementary = scalar - 0x10000
    high = 0xD800 + Bitwise.bsr(supplementary, 10)
    low = 0xDC00 + Bitwise.band(supplementary, 0x3FF)
    "\\u#{python_json_hex(high)}\\u#{python_json_hex(low)}"
  end

  defp python_json_hex(value) do
    value
    |> Integer.to_string(16)
    |> String.downcase()
    |> String.pad_leading(4, "0")
  end

  def validate_ci_full_build_toolchains(repo_root, packages) do
    ci_path = Path.join([repo_root, ".github", "workflows", "ci.yml"])

    case File.read(ci_path) do
      {:ok, workflow} ->
        if String.contains?(workflow, "Full build on main merge") do
          compact_workflow = String.replace(workflow, ~r/\s+/, "")

          missing_output_binding =
            packages
            |> languages_needing_ci_toolchains()
            |> Enum.filter(fn lang ->
              not String.contains?(
                compact_workflow,
                "needs_#{lang}:${{steps.toolchains.outputs.needs_#{lang}}}"
              )
            end)

          missing_main_force =
            packages
            |> languages_needing_ci_toolchains()
            |> Enum.filter(fn lang ->
              not String.contains?(compact_workflow, "needs_#{lang}=true")
            end)

          if missing_output_binding == [] and missing_main_force == [] do
            nil
          else
            parts = []

            parts =
              if missing_output_binding == [] do
                parts
              else
                parts ++
                  [
                    "detect outputs for forced main full builds are not normalized through " <>
                      "steps.toolchains for: #{Enum.join(missing_output_binding, ", ")}"
                  ]
              end

            parts =
              if missing_main_force == [] do
                parts
              else
                parts ++
                  [
                    "forced main full-build path does not explicitly enable toolchains for: " <>
                      Enum.join(missing_main_force, ", ")
                  ]
              end

            "#{String.replace(ci_path, "\\", "/")}: #{Enum.join(parts, "; ")}"
          end
        end

      {:error, _reason} ->
        nil
    end
  end

  def validate_build_contracts(repo_root, packages) do
    errors =
      ([validate_ci_full_build_toolchains(repo_root, packages)] ++
         validate_lua_isolated_build_files(packages) ++
         validate_perl_build_files(packages))
      |> Enum.reject(&is_nil/1)

    case errors do
      [] -> nil
      values -> Enum.join(values, "\n  - ")
    end
  end

  defp languages_needing_ci_toolchains(packages) do
    packages
    |> Enum.map(& &1.language)
    |> Enum.filter(&MapSet.member?(@ci_managed_toolchain_languages, &1))
    |> Enum.uniq()
    |> Enum.sort()
  end

  defp validate_lua_isolated_build_files(packages) do
    packages
    |> Enum.filter(&(&1.language == "lua"))
    |> Enum.flat_map(fn pkg ->
      self_rock = "coding-adventures-" <> String.replace(Path.basename(pkg.path), "_", "-")

      build_lines =
        pkg.path
        |> lua_build_files()
        |> Map.new(fn build_path -> {Path.basename(build_path), read_build_lines(build_path)} end)

      build_lines
      |> Enum.map(fn {name, lines} -> {Path.join(pkg.path, name), lines} end)
      |> Enum.flat_map(fn build_path ->
        {build_path, lines} = build_path

        if lines == [] do
          []
        else
          errors = []

          errors =
            case first_foreign_lua_remove(lines, self_rock) do
              nil ->
                errors

              foreign_remove ->
                [
                  "#{String.replace(build_path, "\\", "/")}: Lua BUILD removes unrelated rock " <>
                    "#{foreign_remove}; isolated package builds should only remove the package they are rebuilding"
                  | errors
                ]
            end

          state_machine_index =
            first_line_containing(lines, ["../state_machine", "..\\state_machine"])

          directed_graph_index =
            first_line_containing(lines, ["../directed_graph", "..\\directed_graph"])

          errors =
            if not is_nil(state_machine_index) and not is_nil(directed_graph_index) and
                 state_machine_index < directed_graph_index do
              [
                "#{String.replace(build_path, "\\", "/")}: Lua BUILD installs state_machine " <>
                  "before directed_graph; isolated LuaRocks builds require directed_graph first"
                | errors
              ]
            else
              errors
            end

          if (guarded_local_lua_install?(lines) or
                (Path.basename(build_path) == "BUILD_windows" and
                   local_lua_sibling_install?(lines))) and
               not self_install_disables_deps?(lines, self_rock) do
            [
              "#{String.replace(build_path, "\\", "/")}: Lua BUILD bootstraps sibling rocks " <>
                "but the final self-install does not pass --deps-mode=none or --no-manifest"
              | errors
            ]
          else
            errors
          end
          |> Enum.reverse()
        end
      end)
      |> Kernel.++(
        case missing_lua_sibling_installs(
               Map.get(build_lines, "BUILD", []),
               Map.get(build_lines, "BUILD_windows", [])
             ) do
          [] ->
            []

          missing ->
            [
              "#{String.replace(Path.join(pkg.path, "BUILD_windows"), "\\", "/")}: Lua BUILD_windows is missing sibling installs present in BUILD: #{Enum.join(missing, ", ")}"
            ]
        end
      )
    end)
  end

  defp validate_perl_build_files(packages) do
    packages
    |> Enum.filter(&(&1.language == "perl"))
    |> Enum.flat_map(fn pkg ->
      pkg.path
      |> lua_build_files()
      |> Enum.filter(fn build_path ->
        build_path
        |> read_build_lines()
        |> Enum.any?(fn line ->
          String.contains?(line, "cpanm") and
            String.contains?(line, "Test2::V0") and
            not String.contains?(line, "--notest")
        end)
      end)
      |> Enum.map(fn build_path ->
        "#{String.replace(build_path, "\\", "/")}: Perl BUILD bootstraps Test2::V0 without --notest; isolated Windows installs can fail while installing the test framework itself"
      end)
    end)
  end

  defp lua_build_files(pkg_path) do
    case File.ls(pkg_path) do
      {:ok, entries} ->
        entries
        |> Enum.filter(&String.starts_with?(&1, "BUILD"))
        |> Enum.sort()
        |> Enum.map(&Path.join(pkg_path, &1))

      {:error, _reason} ->
        []
    end
  end

  defp read_build_lines(build_path) do
    case File.read(build_path) do
      {:ok, contents} ->
        contents
        |> String.split("\n")
        |> Enum.map(&String.trim/1)
        |> Enum.filter(&(not (&1 == "" or String.starts_with?(&1, "#"))))

      {:error, _reason} ->
        []
    end
  end

  defp first_foreign_lua_remove(lines, self_rock) do
    Enum.find_value(lines, fn line ->
      case Regex.run(~r/\bluarocks remove --force ([^ \t]+)/, line) do
        [_, target] when target != self_rock -> target
        _ -> nil
      end
    end)
  end

  defp first_line_containing(lines, needles) do
    lines
    |> Enum.with_index()
    |> Enum.find_value(fn {line, index} ->
      if Enum.any?(needles, &String.contains?(line, &1)), do: index, else: nil
    end)
  end

  defp guarded_local_lua_install?(lines) do
    Enum.any?(lines, fn line ->
      String.contains?(line, "luarocks show ") and
        (String.contains?(line, "../") or String.contains?(line, "..\\"))
    end)
  end

  defp local_lua_sibling_install?(lines) do
    lua_sibling_install_dirs(lines) != []
  end

  defp self_install_disables_deps?(lines, self_rock) do
    Enum.any?(lines, fn line ->
      String.contains?(line, "luarocks make") and
        String.contains?(line, self_rock) and
        (String.contains?(line, "--deps-mode=none") or
           String.contains?(line, "--deps-mode none") or
           String.contains?(line, "--no-manifest"))
    end)
  end

  defp missing_lua_sibling_installs(unix_lines, windows_lines) do
    windows_deps = MapSet.new(lua_sibling_install_dirs(windows_lines))

    unix_lines
    |> lua_sibling_install_dirs()
    |> Enum.reject(&MapSet.member?(windows_deps, &1))
  end

  defp lua_sibling_install_dirs(lines) do
    lines
    |> Enum.filter(&String.contains?(&1, "luarocks make"))
    |> Enum.flat_map(fn line ->
      case Regex.run(~r|\bcd\s+([.][.][\\/][^ \t\r\n&()]+)|, line) do
        [_, dep] -> [String.replace(dep, "\\", "/")]
        _ -> []
      end
    end)
    |> Enum.uniq()
    |> Enum.sort()
  end
end
