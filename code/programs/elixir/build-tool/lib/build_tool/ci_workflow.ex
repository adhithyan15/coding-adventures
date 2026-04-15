defmodule BuildTool.CIWorkflow do
  @moduledoc false

  @ci_workflow_path ".github/workflows/ci.yml"

  @toolchain_markers %{
    "python" => [
      "needs_python",
      "setup-python",
      "python-version",
      "setup-uv",
      "python --version",
      "uv --version",
      "pytest",
      "set up python",
      "install uv"
    ],
    "ruby" => [
      "needs_ruby",
      "setup-ruby",
      "ruby-version",
      "bundler",
      "gem install bundler",
      "ruby --version",
      "bundle --version",
      "set up ruby",
      "install bundler"
    ],
    "go" => [
      "needs_go",
      "setup-go",
      "go-version",
      "go version",
      "set up go"
    ],
    "typescript" => [
      "needs_typescript",
      "setup-node",
      "node-version",
      "npm install -g jest",
      "node --version",
      "npm --version",
      "set up node"
    ],
    "rust" => [
      "needs_rust",
      "rust-toolchain",
      "cargo",
      "rustc",
      "tarpaulin",
      "wasm32-unknown-unknown",
      "set up rust",
      "install cargo-tarpaulin"
    ],
    "elixir" => [
      "needs_elixir",
      "setup-beam",
      "elixir-version",
      "otp-version",
      "elixir --version",
      "mix --version",
      "set up elixir"
    ],
    "lua" => [
      "needs_lua",
      "gh-actions-lua",
      "gh-actions-luarocks",
      "luarocks",
      "lua -v",
      "msvc",
      "set up lua",
      "set up luarocks"
    ],
    "perl" => [
      "needs_perl",
      "cpanm",
      "perl --version",
      "install cpanm"
    ],
    "haskell" => [
      "needs_haskell",
      "haskell-actions/setup",
      "ghc-version",
      "cabal-version",
      "ghc --version",
      "cabal --version",
      "set up haskell"
    ],
    "java" => [
      "needs_java",
      "setup-java",
      "java-version",
      "java --version",
      "temurin",
      "set up jdk",
      "set up gradle",
      "setup-gradle",
      "disable long-lived gradle services",
      "gradle_opts",
      "org.gradle.daemon",
      "org.gradle.vfs.watch"
    ],
    "kotlin" => [
      "needs_kotlin",
      "setup-java",
      "java-version",
      "temurin",
      "set up jdk",
      "set up gradle",
      "setup-gradle",
      "disable long-lived gradle services",
      "gradle_opts",
      "org.gradle.daemon",
      "org.gradle.vfs.watch"
    ],
    "dotnet" => [
      "needs_dotnet",
      "setup-dotnet",
      "dotnet-version",
      "dotnet --version",
      "set up .net"
    ]
  }

  @unsafe_markers [
    "./build-tool",
    "build-tool.exe",
    "-detect-languages",
    "-emit-plan",
    "-force",
    "-plan-file",
    "-validate-build-files",
    "actions/checkout",
    "build-plan",
    "cancel-in-progress:",
    "concurrency:",
    "diff-base",
    "download-artifact",
    "event_name",
    "fetch-depth",
    "git fetch origin main",
    "git_ref",
    "is_main",
    "matrix:",
    "permissions:",
    "pr_base_ref",
    "pull_request:",
    "push:",
    "runs-on:",
    "strategy:",
    "upload-artifact"
  ]

  def ci_workflow_path, do: @ci_workflow_path

  def analyze_changes(root, diff_base) do
    analyze_patch(file_diff(root, diff_base, @ci_workflow_path))
  end

  def analyze_patch(patch) do
    do_analyze_patch(String.split(patch, "\n"), MapSet.new(), [])
  end

  def sorted_toolchains(toolchains) do
    toolchains
    |> Enum.sort()
  end

  defp do_analyze_patch([], toolchains, hunk) do
    case classify_hunk(Enum.reverse(hunk)) do
      {hunk_toolchains, false} ->
        %{toolchains: MapSet.union(toolchains, hunk_toolchains), requires_full_rebuild: false}

      {_hunk_toolchains, true} ->
        %{toolchains: MapSet.new(), requires_full_rebuild: true}
    end
  end

  defp do_analyze_patch([line | rest], toolchains, hunk) do
    cond do
      String.starts_with?(line, "@@") ->
        case classify_hunk(Enum.reverse(hunk)) do
          {hunk_toolchains, false} ->
            do_analyze_patch(rest, MapSet.union(toolchains, hunk_toolchains), [])

          {_hunk_toolchains, true} ->
            %{toolchains: MapSet.new(), requires_full_rebuild: true}
        end

      String.starts_with?(line, "diff --git ") or
        String.starts_with?(line, "index ") or
        String.starts_with?(line, "--- ") or
          String.starts_with?(line, "+++ ") ->
        do_analyze_patch(rest, toolchains, hunk)

      true ->
        do_analyze_patch(rest, toolchains, [line | hunk])
    end
  end

  defp classify_hunk(lines) do
    {hunk_toolchains, changed_toolchains, changed_lines} =
      Enum.reduce(lines, {MapSet.new(), MapSet.new(), []}, fn line,
                                                              {hunk_toolchains,
                                                               changed_toolchains, changed_lines} ->
        cond do
          line == "" or not diff_line?(line) ->
            {hunk_toolchains, changed_toolchains, changed_lines}

          true ->
            content = line |> String.slice(1, String.length(line) - 1) |> String.trim()
            hunk_toolchains = MapSet.union(hunk_toolchains, detect_toolchains(content))

            cond do
              not changed_line?(line) or content == "" or String.starts_with?(content, "#") ->
                {hunk_toolchains, changed_toolchains, changed_lines}

              true ->
                changed_toolchains = MapSet.union(changed_toolchains, detect_toolchains(content))
                {hunk_toolchains, changed_toolchains, [content | changed_lines]}
            end
        end
      end)

    changed_lines = Enum.reverse(changed_lines)

    cond do
      changed_lines == [] ->
        {MapSet.new(), false}

      MapSet.size(changed_toolchains) == 0 and MapSet.size(hunk_toolchains) != 1 ->
        {MapSet.new(), true}

      true ->
        resolved_toolchains =
          if MapSet.size(changed_toolchains) == 0 do
            hunk_toolchains
          else
            changed_toolchains
          end

        unsafe_change =
          Enum.any?(changed_lines, fn content ->
            touches_shared_ci_behavior?(content) or
              (MapSet.size(detect_toolchains(content)) == 0 and
                 not toolchain_scoped_structural_line?(content))
          end)

        if unsafe_change do
          {MapSet.new(), true}
        else
          {resolved_toolchains, false}
        end
    end
  end

  defp detect_toolchains(content) do
    normalized = String.downcase(content)

    Enum.reduce(@toolchain_markers, MapSet.new(), fn {toolchain, markers}, found ->
      if Enum.any?(markers, &String.contains?(normalized, &1)) do
        MapSet.put(found, toolchain)
      else
        found
      end
    end)
  end

  defp touches_shared_ci_behavior?(content) do
    normalized = String.downcase(content)
    Enum.any?(@unsafe_markers, &String.contains?(normalized, &1))
  end

  defp toolchain_scoped_structural_line?(content) do
    String.starts_with?(
      content,
      [
        "if:",
        "run:",
        "shell:",
        "with:",
        "env:",
        "{",
        "}",
        "else",
        "fi",
        "then",
        "printf ",
        "echo ",
        "curl ",
        "powershell ",
        "call ",
        "cd "
      ]
    )
  end

  defp diff_line?(line) do
    String.starts_with?(line, " ") or changed_line?(line)
  end

  defp changed_line?(line) do
    String.starts_with?(line, "+") or String.starts_with?(line, "-")
  end

  defp file_diff(root, diff_base, relative_path) do
    [
      ["diff", "--unified=0", diff_base <> "...HEAD", "--", relative_path],
      ["diff", "--unified=0", diff_base, "HEAD", "--", relative_path]
    ]
    |> Enum.find_value("", fn args ->
      case System.cmd("git", args, cd: root, stderr_to_stdout: true) do
        {output, 0} -> output
        _ -> nil
      end
    end)
  end
end
