defmodule BuildTool.SourceInputRegistryTest do
  use ExUnit.Case, async: true

  alias BuildTool.Hasher

  @fixture_dir Path.expand("../../../../specs/fixtures/build-tool-v1", __DIR__)
  @checked_registry Path.join(@fixture_dir, "language-source-input-registry.json")
  @registry_digest "5201a045ea3e2086fd9be316f2692743ca329f1d84f1c0983a0da47e96b3f621"

  test "the production projection is the complete checked registry" do
    checked_bytes = File.read!(@checked_registry)
    assert BuildTool.SourceInputRegistryData.bytes() == checked_bytes
    assert Hasher.source_input_registry_snapshot() == checked_bytes
    assert Hasher.source_input_registry() == Jason.decode!(checked_bytes)
    assert Hasher.source_input_registry_digest() == @registry_digest
  end

  test "the production selector consumes every package-local neutral case" do
    cases =
      @fixture_dir
      |> Path.join("cases/source-collection-*.json")
      |> Path.wildcard()
      |> Enum.map(&(&1 |> File.read!() |> Jason.decode!()))
      |> Enum.filter(fn fixture ->
        fixture["input"]["operation"] == "source_collection" and
          Map.has_key?(fixture["input"]["options"], "registry_sha256")
      end)

    assert length(cases) == 7

    for fixture <- cases do
      options = fixture["input"]["options"]
      assert options["registry_sha256"] == Hasher.source_input_registry_digest()

      assert Hasher.collect_candidates(options) == fixture["expected"]["result"]["files"],
             fixture["id"]
    end
  end

  test "unknown languages and mismatched package roots fail before candidates" do
    options = %{
      "language" => "unknown",
      "package_root" => "code/packages/unknown/demo",
      "mode" => "extension",
      "declared_srcs" => [],
      "registry_sha256" => @registry_digest,
      "candidates" => :must_not_be_enumerated
    }

    assert_raise ArgumentError, ~r/not registered/, fn ->
      Hasher.collect_candidates(options)
    end

    assert_raise ArgumentError, ~r/package root/, fn ->
      Hasher.collect_candidates(%{options | "language" => "python"})
    end

    for root <- [
          "code/packages/python/CON",
          "code/packages/python/name ",
          "code/packages/python/a:b"
        ] do
      assert_raise ArgumentError, ~r/package root/, fn ->
        Hasher.collect_candidates(%{options | "language" => "python", "package_root" => root})
      end
    end
  end

  test "candidate identities and impossible file prefixes fail before selection" do
    base = %{
      "language" => "python",
      "package_root" => "code/packages/python/demo",
      "mode" => "extension",
      "declared_srcs" => [],
      "registry_sha256" => @registry_digest
    }

    file = %{"path" => "src/a.py", "kind" => "file", "content_hex" => "61"}

    for candidates <- [
          [file, file],
          [file, %{file | "path" => "src/A.py"}],
          [file, %{file | "path" => "src/a.py/child.py"}],
          [%{file | "path" => "src/CON.py"}],
          [%{file | "path" => "src/a.py "}]
        ] do
      assert_raise ArgumentError, fn ->
        Hasher.collect_candidates(Map.put(base, "candidates", candidates))
      end
    end
  end

  test "Hashing v1 frames repository paths and raw file bytes" do
    root =
      Path.join(System.tmp_dir!(), "elixir_source_hash_#{System.unique_integer([:positive])}")

    File.mkdir_p!(Path.join(root, "src"))
    on_exit(fn -> File.rm_rf!(root) end)
    File.write!(Path.join(root, "src/data.bin"), <<0, 255, 97, 98, 99>>)

    package = %{path: root, language: "python", package_root: "code/packages/python/demo"}
    # A non-source binary file is not selected in extension mode.
    assert Hasher.hash_package(package) == Hasher.hash_string("")

    File.write!(Path.join(root, "src/data.py"), "abc")
    package_path = "code/packages/python/demo/src/data.py"

    expected =
      :crypto.hash(:sha256, [
        <<byte_size(package_path)::unsigned-big-integer-size(64)>>,
        package_path,
        <<3::unsigned-big-integer-size(64)>>,
        "abc"
      ])
      |> Base.encode16(case: :lower)

    assert Hasher.hash_package(package) == expected
    binary_contents = <<0, 255, 97, 98, 99>>
    File.write!(Path.join(root, "src/data.py"), binary_contents)

    binary_expected =
      :crypto.hash(:sha256, [
        <<byte_size(package_path)::unsigned-big-integer-size(64)>>,
        package_path,
        <<byte_size(binary_contents)::unsigned-big-integer-size(64)>>,
        binary_contents
      ])
      |> Base.encode16(case: :lower)

    assert Hasher.hash_package(package) == binary_expected
    refute binary_expected == expected

    File.rename!(Path.join(root, "src/data.py"), Path.join(root, "src/renamed.py"))
    refute Hasher.hash_package(package) == binary_expected
  end

  test "the generated registry projection is itself selected as Elixir source" do
    options = %{
      "language" => "elixir",
      "package_root" => "code/programs/elixir/build-tool",
      "mode" => "extension",
      "declared_srcs" => [],
      "registry_sha256" => @registry_digest,
      "candidates" => [
        %{
          "path" => "lib/build_tool/source_input_registry_data.ex",
          "kind" => "file",
          "content_hex" => Base.encode16("projection", case: :lower)
        }
      ]
    }

    assert [%{"path" => "lib/build_tool/source_input_registry_data.ex"}] =
             Hasher.collect_candidates(options)
  end

  test "the exact generated-directory rule does not erase near-name sources" do
    root = Path.join(System.tmp_dir!(), "elixir_prune_#{System.unique_integer([:positive])}")
    File.mkdir_p!(Path.join(root, "_build"))
    # Windows cannot hold `_build` and `_Build` as sibling names; use a nested
    # near-name component just as the neutral fixture does.
    File.mkdir_p!(Path.join(root, "near/_Build"))
    on_exit(fn -> File.rm_rf!(root) end)
    File.write!(Path.join(root, "_build/ignored.ex"), "old")
    File.write!(Path.join(root, "near/_Build/kept.ex"), "old")

    package = %{path: root, language: "elixir", package_root: "code/programs/elixir/demo"}
    before_digest = Hasher.hash_package(package)
    File.write!(Path.join(root, "_build/ignored.ex"), "changed")
    assert Hasher.hash_package(package) == before_digest
    File.write!(Path.join(root, "near/_Build/kept.ex"), "changed")
    refute Hasher.hash_package(package) == before_digest
  end

  test "an explicitly declared empty source list selects only fixed inputs" do
    root = Path.join(System.tmp_dir!(), "elixir_declared_#{System.unique_integer([:positive])}")
    File.mkdir_p!(root)
    on_exit(fn -> File.rm_rf!(root) end)
    File.write!(Path.join(root, "src.ex"), "source")

    package = %{path: root, language: "elixir", package_root: "code/programs/elixir/demo"}
    refute Hasher.hash_package(package) == Hasher.hash_string("")

    assert Hasher.hash_package(Map.merge(package, %{is_starlark: true, declared_srcs: []})) ==
             Hasher.hash_string("")
  end

  test "a regular file with a generated-directory basename remains selectable" do
    root = Path.join(System.tmp_dir!(), "elixir_named_file_#{System.unique_integer([:positive])}")
    File.mkdir_p!(root)
    on_exit(fn -> File.rm_rf!(root) end)
    File.write!(Path.join(root, "build"), "source")

    package = %{
      path: root,
      language: "elixir",
      package_root: "code/programs/elixir/demo",
      is_starlark: true,
      declared_srcs: ["build"]
    }

    refute Hasher.hash_package(package) == Hasher.hash_string("")
  end
end
