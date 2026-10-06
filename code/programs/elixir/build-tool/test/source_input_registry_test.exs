defmodule BuildTool.SourceInputRegistryTest do
  use ExUnit.Case, async: true

  alias BuildTool.Hasher

  @fixture_dir Path.expand("../../../../specs/fixtures/build-tool-v1", __DIR__)
  @checked_registry Path.join(@fixture_dir, "language-source-input-registry.json")
  @packaged_registry Path.expand("../priv/language-source-input-registry.json", __DIR__)
  @registry_digest "5201a045ea3e2086fd9be316f2692743ca329f1d84f1c0983a0da47e96b3f621"

  test "the production projection is the complete checked registry" do
    checked_bytes = File.read!(@checked_registry)
    assert File.read!(@packaged_registry) == checked_bytes
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
  end

  test "Hashing v1 frames repository paths and raw file bytes" do
    root = Path.join(System.tmp_dir!(), "elixir_source_hash_#{System.unique_integer([:positive])}")
    File.mkdir_p!(Path.join(root, "src"))
    on_exit(fn -> File.rm_rf!(root) end)
    File.write!(Path.join(root, "src/data.bin"), <<0, 255, 97, 98, 99>>)

    package = %{path: root, language: "python", package_root: "code/packages/python/demo"}
    # A non-source binary file is not selected in extension mode.
    assert Hasher.hash_package(package) == Hasher.hash_string("")

    File.write!(Path.join(root, "src/data.py"), "abc")
    package_path = "code/packages/python/demo/src/data.py"
    expected = :crypto.hash(:sha256, [
      <<byte_size(package_path)::unsigned-big-integer-size(64)>>,
      package_path,
      <<3::unsigned-big-integer-size(64)>>,
      "abc"
    ]) |> Base.encode16(case: :lower)

    assert Hasher.hash_package(package) == expected
    File.write!(Path.join(root, "src/data.py"), <<0, 255, 97, 98, 99>>)
    assert Hasher.hash_package(package) != expected
  end

  test "the exact generated-directory rule does not erase near-name sources" do
    root = Path.join(System.tmp_dir!(), "elixir_prune_#{System.unique_integer([:positive])}")
    File.mkdir_p!(Path.join(root, "_build"))
    File.mkdir_p!(Path.join(root, "_Build"))
    on_exit(fn -> File.rm_rf!(root) end)
    File.write!(Path.join(root, "_build/ignored.ex"), "old")
    File.write!(Path.join(root, "_Build/kept.ex"), "old")

    package = %{path: root, language: "elixir", package_root: "code/programs/elixir/demo"}
    before_digest = Hasher.hash_package(package)
    File.write!(Path.join(root, "_build/ignored.ex"), "changed")
    assert Hasher.hash_package(package) == before_digest
    File.write!(Path.join(root, "_Build/kept.ex"), "changed")
    refute Hasher.hash_package(package) == before_digest
  end
end
