defmodule BuildTool.ToolchainDetectionTest do
  use ExUnit.Case, async: true

  alias BuildTool.ToolchainDetection

  @fixture_root Path.expand(
                  "../../../../specs/fixtures/build-tool-v1/cases",
                  __DIR__
                )

  test "independently consumes every neutral toolchain-detection fixture" do
    fixtures =
      @fixture_root
      |> Path.join("toolchain-detection-*.json")
      |> Path.wildcard()
      |> Enum.sort()

    assert length(fixtures) == 11

    Enum.each(fixtures, fn fixture_path ->
      fixture = fixture_path |> File.read!() |> Jason.decode!()
      options = fixture["input"]["options"]
      expected = fixture["expected"]

      packages =
        Enum.map(options["packages"], fn package ->
          %{
            name: package["name"],
            language: package["language"],
            build_files: package["build_files"]
          }
        end)

      actual =
        ToolchainDetection.evaluate_snapshot(
          options["platform"],
          options["force_full"],
          packages,
          options["scheduled_packages"],
          options["forced_toolchains"]
        )

      assert actual.outcome == expected["outcome"], fixture["id"]
      assert actual.toolchains == Map.get(expected["result"], "toolchains", %{}), fixture["id"]
      assert actual.diagnostics == expected_diagnostics(expected["diagnostics"]), fixture["id"]
    end)
  end

  test "rejects per-file and aggregate snapshot limit overruns" do
    oversized = String.duplicate("x", 65_537)

    assert_raise ArgumentError, ~r/per-file resource ceiling/, fn ->
      ToolchainDetection.evaluate_snapshot(
        "linux",
        false,
        [%{name: "rust/app", language: "rust", build_files: %{"BUILD" => oversized}}],
        nil,
        []
      )
    end

    build_files =
      Map.new(0..16, fn index ->
        {"BUILD_#{index}", String.duplicate("x", 65_536)}
      end)

    assert_raise ArgumentError, ~r/aggregate resource ceiling/, fn ->
      ToolchainDetection.evaluate_snapshot(
        "linux",
        false,
        [%{name: "rust/app", language: "rust", build_files: build_files}],
        nil,
        []
      )
    end
  end

  test "keeps declaration grammar byte-exact across CRLF and lone CR" do
    assert ToolchainDetection.parse_extra_toolchains(
             "  # needs-toolchain: python  \r\n\t# needs-toolchain:\tjava\t\r\n"
           ) == ["python", "java"]

    assert ToolchainDetection.parse_extra_toolchains("# needs-toolchain: python\r") == []
    assert ToolchainDetection.parse_extra_toolchains("# needs-toolchain: lua\r  ") == []
  end

  test "production snapshots union selected declarations with forced CI toolchains" do
    packages = [
      %{
        name: "rust/selected",
        language: "rust",
        build_content: "# needs-toolchain: python\r\n"
      },
      %{
        name: "go/unscheduled",
        language: "go",
        build_content: "# needs-toolchain: java\n"
      }
    ]

    actual =
      ToolchainDetection.evaluate_packages(
        packages,
        MapSet.new(["rust/selected"]),
        false,
        MapSet.new(["kotlin"])
      )

    assert actual.outcome == "ok"
    assert actual.toolchains["rust"]
    assert actual.toolchains["python"]
    assert actual.toolchains["kotlin"]
    refute actual.toolchains["go"]
    refute actual.toolchains["java"]
  end

  defp expected_diagnostics(diagnostics) do
    Enum.map(diagnostics, fn diagnostic ->
      %{code: diagnostic["code"], severity: diagnostic["severity"]}
      |> maybe_put_package(diagnostic)
    end)
  end

  defp maybe_put_package(result, %{"package" => package}), do: Map.put(result, :package, package)
  defp maybe_put_package(result, _diagnostic), do: result
end
