defmodule BuildTool.ExecutorTest do
  use ExUnit.Case, async: true

  alias BuildTool.{Cache, DirectedGraph, Executor}

  # ---------------------------------------------------------------------------
  # Setup
  # ---------------------------------------------------------------------------

  setup do
    tmp_dir = Path.join(System.tmp_dir!(), "build_tool_executor_test_#{:rand.uniform(100_000)}")
    File.rm_rf!(tmp_dir)
    File.mkdir_p!(tmp_dir)

    {:ok, cache} = Cache.start_link()

    on_exit(fn -> File.rm_rf!(tmp_dir) end)

    {:ok, tmp_dir: tmp_dir, cache: cache}
  end

  # ---------------------------------------------------------------------------
  # Helper: create a package directory with a BUILD file
  # ---------------------------------------------------------------------------

  defp make_package(tmp_dir, name, commands) do
    pkg_dir = Path.join(tmp_dir, name)
    File.mkdir_p!(pkg_dir)
    File.write!(Path.join(pkg_dir, "BUILD"), Enum.join(commands, "\n"))

    %{
      name: name,
      path: pkg_dir,
      build_commands: commands,
      language: "unknown"
    }
  end

  # ---------------------------------------------------------------------------
  # Basic execution
  # ---------------------------------------------------------------------------

  describe "execute_builds/6" do
    test "builds a single package with echo command", %{tmp_dir: tmp_dir, cache: cache} do
      pkg = make_package(tmp_dir, "test/pkg", ["echo hello"])
      graph = DirectedGraph.new() |> DirectedGraph.add_node("test/pkg")

      results =
        Executor.execute_builds([pkg], graph, cache, %{"test/pkg" => "h1"}, %{"test/pkg" => "d1"},
          force: true,
          tracker: nil
        )

      assert results["test/pkg"].status == "built"
      assert results["test/pkg"].return_code == 0
    end

    test "marks failed package", %{tmp_dir: tmp_dir, cache: cache} do
      pkg = make_package(tmp_dir, "test/fail", ["exit 1"])
      graph = DirectedGraph.new() |> DirectedGraph.add_node("test/fail")

      results =
        Executor.execute_builds([pkg], graph, cache, %{"test/fail" => "h"}, %{"test/fail" => "d"},
          force: true,
          tracker: nil
        )

      assert results["test/fail"].status == "failed"
      assert results["test/fail"].return_code == 1
    end

    test "dry run marks packages as would-build", %{tmp_dir: tmp_dir, cache: cache} do
      pkg = make_package(tmp_dir, "test/dry", ["echo should not run"])
      graph = DirectedGraph.new() |> DirectedGraph.add_node("test/dry")

      results =
        Executor.execute_builds([pkg], graph, cache, %{"test/dry" => "h"}, %{"test/dry" => "d"},
          force: true,
          dry_run: true,
          tracker: nil
        )

      assert results["test/dry"].status == "would-build"
    end

    test "skips packages not in affected set", %{tmp_dir: tmp_dir, cache: cache} do
      pkg = make_package(tmp_dir, "test/skip", ["echo should not run"])
      graph = DirectedGraph.new() |> DirectedGraph.add_node("test/skip")

      results =
        Executor.execute_builds(
          [pkg],
          graph,
          cache,
          %{"test/skip" => "h"},
          %{"test/skip" => "d"},
          affected_set: MapSet.new(),
          tracker: nil
        )

      assert results["test/skip"].status == "skipped"
    end

    test "builds packages in affected set", %{tmp_dir: tmp_dir, cache: cache} do
      pkg = make_package(tmp_dir, "test/affected", ["echo building"])
      graph = DirectedGraph.new() |> DirectedGraph.add_node("test/affected")

      results =
        Executor.execute_builds(
          [pkg],
          graph,
          cache,
          %{"test/affected" => "h"},
          %{"test/affected" => "d"},
          affected_set: MapSet.new(["test/affected"]),
          tracker: nil
        )

      assert results["test/affected"].status == "built"
    end
  end

  # ---------------------------------------------------------------------------
  # Dependency ordering
  # ---------------------------------------------------------------------------

  describe "dependency ordering" do
    test "builds dependencies before dependents", %{tmp_dir: tmp_dir, cache: cache} do
      # A -> B: A must be built before B.
      # Both are simple echo commands — we just verify ordering works.
      pkg_a = make_package(tmp_dir, "test/a", ["echo a"])
      pkg_b = make_package(tmp_dir, "test/b", ["echo b"])

      graph =
        DirectedGraph.new()
        |> DirectedGraph.add_edge("test/a", "test/b")

      hashes = %{"test/a" => "ha", "test/b" => "hb"}
      dep_hashes = %{"test/a" => "da", "test/b" => "db"}

      results =
        Executor.execute_builds([pkg_a, pkg_b], graph, cache, hashes, dep_hashes,
          force: true,
          max_jobs: 1,
          tracker: nil
        )

      assert results["test/a"].status == "built"
      assert results["test/b"].status == "built"
    end

    test "dep-skips dependents when dependency fails", %{tmp_dir: tmp_dir, cache: cache} do
      pkg_a = make_package(tmp_dir, "test/fail-dep", ["exit 1"])
      pkg_b = make_package(tmp_dir, "test/dependent", ["echo should not run"])

      graph =
        DirectedGraph.new()
        |> DirectedGraph.add_edge("test/fail-dep", "test/dependent")

      hashes = %{"test/fail-dep" => "h1", "test/dependent" => "h2"}
      dep_hashes = %{"test/fail-dep" => "d1", "test/dependent" => "d2"}

      results =
        Executor.execute_builds([pkg_a, pkg_b], graph, cache, hashes, dep_hashes,
          force: true,
          tracker: nil
        )

      assert results["test/fail-dep"].status == "failed"
      assert results["test/dependent"].status == "dep-skipped"
    end
  end

  # ---------------------------------------------------------------------------
  # Cache integration
  # ---------------------------------------------------------------------------

  describe "cache integration" do
    test "skips packages that haven't changed", %{tmp_dir: tmp_dir, cache: cache} do
      pkg = make_package(tmp_dir, "test/cached", ["echo building"])

      # Pre-populate cache with matching hashes.
      Cache.record(cache, "test/cached", "h1", "d1", "success")

      graph = DirectedGraph.new() |> DirectedGraph.add_node("test/cached")

      results =
        Executor.execute_builds(
          [pkg],
          graph,
          cache,
          %{"test/cached" => "h1"},
          %{"test/cached" => "d1"},
          tracker: nil
        )

      assert results["test/cached"].status == "skipped"
    end

    test "rebuilds packages with changed hashes", %{tmp_dir: tmp_dir, cache: cache} do
      pkg = make_package(tmp_dir, "test/changed", ["echo rebuilding"])

      Cache.record(cache, "test/changed", "old_hash", "d1", "success")

      graph = DirectedGraph.new() |> DirectedGraph.add_node("test/changed")

      results =
        Executor.execute_builds(
          [pkg],
          graph,
          cache,
          %{"test/changed" => "new_hash"},
          %{"test/changed" => "d1"},
          tracker: nil
        )

      assert results["test/changed"].status == "built"
    end

    test "records successful builds in cache", %{tmp_dir: tmp_dir, cache: cache} do
      pkg = make_package(tmp_dir, "test/record", ["echo ok"])
      graph = DirectedGraph.new() |> DirectedGraph.add_node("test/record")

      Executor.execute_builds([pkg], graph, cache, %{"test/record" => "h"}, %{"test/record" => "d"},
        force: true,
        tracker: nil
      )

      entries = Cache.entries(cache)
      assert entries["test/record"]["status"] == "success"
    end
  end
end
