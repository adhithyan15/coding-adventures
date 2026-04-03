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

  defp languages_needing_ci_toolchains(packages) do
    packages
    |> Enum.map(& &1.language)
    |> Enum.filter(&MapSet.member?(@ci_managed_toolchain_languages, &1))
    |> Enum.uniq()
    |> Enum.sort()
  end
end
