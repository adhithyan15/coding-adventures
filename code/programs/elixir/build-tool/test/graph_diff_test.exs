defmodule BuildTool.GraphDiffTest do
  use ExUnit.Case, async: true

  alias BuildTool.GraphDiff
  alias BuildTool.GraphDiff.DiffInput
  alias BuildTool.GraphDiff.GraphInput
  alias BuildTool.GraphDiff.Package

  @fixture_root Path.expand("../../../../specs/fixtures/build-tool-v1", __DIR__)
  @case_root Path.join(@fixture_root, "cases")

  @graph_ids [
    "graph/canonical-edge-order",
    "graph/chain",
    "graph/cycle",
    "graph/diamond",
    "graph/empty",
    "graph/isolated",
    "graph/multiple-components",
    "graph/partial-cycle-no-output"
  ]

  @diff_ids [
    "diff-selection/exact-build-fronts",
    "diff-selection/forced-package",
    "diff-selection/known-unmatched-near-build",
    "diff-selection/match-work-at-limit",
    "diff-selection/match-work-over-limit",
    "diff-selection/package-prefix",
    "diff-selection/repository-boundary-reverse-index",
    "diff-selection/shared-input-multiconsumer",
    "diff-selection/strict-glob-character-classes",
    "diff-selection/transitive-package-change",
    "diff-selection/unknown-path-all",
    "diff-selection/unknown-path-error"
  ]

  test "dynamically consumes the exact eight graph fixtures" do
    documents = fixtures("graph")
    assert Enum.sort(Enum.map(documents, & &1["id"])) == Enum.sort(@graph_ids)

    Enum.each(documents, fn document ->
      options = document["input"]["options"]
      input = %GraphInput{packages: options["packages"], edges: options["edges"]}
      assert_fixture(document, GraphDiff.evaluate_graph(input))
    end)
  end

  test "dynamically consumes the exact twelve diff-selection fixtures" do
    documents = fixtures("diff-selection")
    assert Enum.sort(Enum.map(documents, & &1["id"])) == Enum.sort(@diff_ids)
    boundary = decode_fixture(Path.join(@fixture_root, "repository-source-input-boundary.json"))

    Enum.each(documents, fn document ->
      assert_fixture(
        document,
        document |> diff_input(boundary) |> GraphDiff.evaluate_diff_selection()
      )
    end)
  end

  test "shared repository input selects both registered consumers in stable order" do
    document = fixture_by_id("diff-selection/shared-input-multiconsumer")
    boundary = decode_fixture(Path.join(@fixture_root, "repository-source-input-boundary.json"))

    assert {:ok, result} =
             document
             |> diff_input(boundary)
             |> GraphDiff.evaluate_diff_selection()

    assert result["changed_packages"] == ["swift/conduit", "swift/sha256-native"]
    refute "swift/decoy" in result["changed_packages"]
    assert result["affected_packages"] == ["swift/app", "swift/conduit", "swift/sha256-native"]
    assert result["prerequisite_packages"] == ["swift/base"]
  end

  test "match work succeeds at exactly fifty million and fails without partial output above it" do
    at_limit = fixture_by_id("diff-selection/match-work-at-limit")
    over_limit = fixture_by_id("diff-selection/match-work-over-limit")

    assert {:ok, _result} = at_limit |> diff_input(nil) |> GraphDiff.evaluate_diff_selection()

    assert {:error, "DIFF_MATCH_LIMIT_EXCEEDED"} =
             over_limit |> diff_input(nil) |> GraphDiff.evaluate_diff_selection()
  end

  test "graph validation and cycles return stable all-or-error values" do
    assert {:error, "GRAPH_PACKAGE_DUPLICATE"} =
             GraphDiff.evaluate_graph(%GraphInput{
               packages: ["fixture/a", "fixture/a"],
               edges: []
             })

    assert {:error, "GRAPH_EDGE_UNKNOWN"} =
             GraphDiff.evaluate_graph(%GraphInput{
               packages: ["fixture/a"],
               edges: [["fixture/a", "fixture/b"]]
             })

    assert {:error, "GRAPH_EDGE_DUPLICATE"} =
             GraphDiff.evaluate_graph(%GraphInput{
               packages: ["fixture/a", "fixture/b"],
               edges: [["fixture/a", "fixture/b"], ["fixture/a", "fixture/b"]]
             })

    assert {:error, "GRAPH_CYCLE"} =
             GraphDiff.evaluate_graph(%GraphInput{
               packages: ["fixture/a", "fixture/b", "fixture/free"],
               edges: [["fixture/a", "fixture/b"], ["fixture/b", "fixture/a"]]
             })
  end

  test "diff validation rejects unsafe or ambiguous materialized inputs before selection" do
    base = %DiffInput{
      packages: [
        %Package{
          name: "fixture/a",
          rel_path: "code/a",
          source_mode: "strict_globs",
          source_globs: ["src/**/*.ex"]
        }
      ],
      edges: [],
      forced_packages: [],
      unknown_path_policy: "error",
      changed_paths: ["code/a/src/main.ex"]
    }

    assert {:error, "DIFF_PATH_INVALID"} =
             GraphDiff.evaluate_diff_selection(%{base | changed_paths: ["../secret"]})

    assert {:error, "DIFF_GLOB_INVALID"} =
             GraphDiff.evaluate_diff_selection(%{
               base
               | packages: [%{hd(base.packages) | source_globs: ["[z-a].ex"]}]
             })

    assert {:error, "DIFF_PATH_INVALID"} =
             GraphDiff.evaluate_diff_selection(%{
               base
               | packages: [
                   hd(base.packages),
                   %Package{
                     name: "fixture/b",
                     rel_path: "code/a/nested",
                     source_mode: "package_prefix"
                   }
                 ]
             })

    assert {:error, "DIFF_FORCED_PACKAGE_UNKNOWN"} =
             GraphDiff.evaluate_diff_selection(%{base | forced_packages: ["fixture/missing"]})

    assert {:error, "DIFF_BOUNDARY_DIGEST_MISMATCH"} =
             GraphDiff.evaluate_diff_selection(%{
               base
               | boundary_sha256: String.duplicate("0", 64)
             })
  end

  test "unknown-path errors never expose forced or local partial selections" do
    input = %DiffInput{
      packages: [
        %Package{
          name: "fixture/a",
          rel_path: "code/a",
          source_mode: "package_prefix"
        },
        %Package{
          name: "fixture/b",
          rel_path: "code/b",
          source_mode: "package_prefix"
        }
      ],
      edges: [["fixture/a", "fixture/b"]],
      forced_packages: ["fixture/a"],
      unknown_path_policy: "error",
      changed_paths: ["code/b/file.ex", "docs/unknown.md"]
    }

    assert {:error, "DIFF_UNKNOWN_PATH"} = GraphDiff.evaluate_diff_selection(input)
  end

  defp fixtures(prefix) do
    @case_root
    |> Path.join("#{prefix}-*.json")
    |> Path.wildcard()
    |> Enum.sort()
    |> Enum.map(&decode_fixture/1)
  end

  defp fixture_by_id(id) do
    "diff-selection"
    |> fixtures()
    |> Enum.find(&(&1["id"] == id))
  end

  defp decode_fixture(path) do
    path |> File.read!() |> Jason.decode!()
  end

  defp diff_input(document, boundary) do
    input = document["input"]
    options = input["options"]

    %DiffInput{
      packages:
        Enum.map(options["packages"], fn package ->
          %Package{
            name: package["name"],
            rel_path: package["rel_path"],
            source_mode: package["source_mode"],
            source_globs: Map.get(package, "source_globs", [])
          }
        end),
      edges: options["edges"],
      forced_packages: options["forced_packages"],
      unknown_path_policy: options["unknown_path_policy"],
      changed_paths: input["changed_paths"],
      boundary_sha256: options["boundary_sha256"],
      boundary: if(options["boundary_sha256"], do: boundary)
    }
  end

  defp assert_fixture(document, actual) do
    expected = document["expected"]

    case actual do
      {:ok, result} ->
        assert expected["outcome"] == "ok", document["id"]
        assert expected["diagnostics"] == [], document["id"]
        assert result == expected["result"], document["id"]

      {:error, code} ->
        assert expected["outcome"] == "error", document["id"]
        assert expected["result"] == %{}, document["id"]
        assert Enum.map(expected["diagnostics"], & &1["code"]) == [code], document["id"]
    end
  end
end
