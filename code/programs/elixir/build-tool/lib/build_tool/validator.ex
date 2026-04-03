defmodule BuildTool.Validator do
  @moduledoc false

  @ci_managed_toolchain_languages MapSet.new([
                                    "python",
                                    "ruby",
                                    "typescript",
                                    "rust",
                                    "elixir",
                                    "lua",
                                    "perl"
                                  ])

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
      [validate_ci_full_build_toolchains(repo_root, packages)] ++
        validate_lua_isolated_build_files(packages)
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

      pkg.path
      |> lua_build_files()
      |> Enum.flat_map(fn build_path ->
        lines = read_build_lines(build_path)

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

          state_machine_index = first_line_containing(lines, ["../state_machine", "..\\state_machine"])
          directed_graph_index = first_line_containing(lines, ["../directed_graph", "..\\directed_graph"])

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

          if guarded_local_lua_install?(lines) and not self_install_disables_deps?(lines, self_rock) do
            [
              "#{String.replace(build_path, "\\", "/")}: Lua BUILD uses guarded sibling rock installs " <>
                "but the final self-install does not pass --deps-mode=none or --no-manifest"
              | errors
            ]
          else
            errors
          end
          |> Enum.reverse()
        end
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

  defp self_install_disables_deps?(lines, self_rock) do
    Enum.any?(lines, fn line ->
      String.contains?(line, "luarocks make") and
        String.contains?(line, self_rock) and
        (String.contains?(line, "--deps-mode=none") or
           String.contains?(line, "--deps-mode none") or
           String.contains?(line, "--no-manifest"))
    end)
  end
end
