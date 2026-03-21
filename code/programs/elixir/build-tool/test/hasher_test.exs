defmodule BuildTool.HasherTest do
  use ExUnit.Case, async: true

  alias BuildTool.Hasher
  alias BuildTool.DirectedGraph

  # ---------------------------------------------------------------------------
  # Setup
  # ---------------------------------------------------------------------------

  setup do
    tmp_dir = Path.join(System.tmp_dir!(), "build_tool_hasher_test_#{:rand.uniform(100_000)}")
    File.rm_rf!(tmp_dir)
    File.mkdir_p!(tmp_dir)

    on_exit(fn -> File.rm_rf!(tmp_dir) end)

    {:ok, tmp_dir: tmp_dir}
  end

  # ---------------------------------------------------------------------------
  # hash_file/1
  # ---------------------------------------------------------------------------

  describe "hash_file/1" do
    test "returns SHA256 hex digest of file contents", %{tmp_dir: tmp_dir} do
      path = Path.join(tmp_dir, "test.txt")
      File.write!(path, "hello world")

      {:ok, hash} = Hasher.hash_file(path)
      assert String.length(hash) == 64
      # Known SHA256 of "hello world"
      assert hash == "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    end

    test "returns error for nonexistent file" do
      assert {:error, _} = Hasher.hash_file("/nonexistent/file.txt")
    end

    test "different contents produce different hashes", %{tmp_dir: tmp_dir} do
      path1 = Path.join(tmp_dir, "a.txt")
      path2 = Path.join(tmp_dir, "b.txt")
      File.write!(path1, "content A")
      File.write!(path2, "content B")

      {:ok, hash1} = Hasher.hash_file(path1)
      {:ok, hash2} = Hasher.hash_file(path2)
      assert hash1 != hash2
    end

    test "identical contents produce identical hashes", %{tmp_dir: tmp_dir} do
      path1 = Path.join(tmp_dir, "a.txt")
      path2 = Path.join(tmp_dir, "b.txt")
      File.write!(path1, "same content")
      File.write!(path2, "same content")

      {:ok, hash1} = Hasher.hash_file(path1)
      {:ok, hash2} = Hasher.hash_file(path2)
      assert hash1 == hash2
    end
  end

  # ---------------------------------------------------------------------------
  # hash_string/1
  # ---------------------------------------------------------------------------

  describe "hash_string/1" do
    test "returns SHA256 hex digest of a string" do
      hash = Hasher.hash_string("hello")
      assert String.length(hash) == 64
      assert hash == "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    end

    test "empty string has a known hash" do
      hash = Hasher.hash_string("")
      assert hash == "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    end
  end

  # ---------------------------------------------------------------------------
  # hash_package/1
  # ---------------------------------------------------------------------------

  describe "hash_package/1" do
    test "hashes Python source files", %{tmp_dir: tmp_dir} do
      pkg_dir = Path.join(tmp_dir, "my-pkg")
      File.mkdir_p!(pkg_dir)
      File.write!(Path.join(pkg_dir, "main.py"), "print('hello')")
      File.write!(Path.join(pkg_dir, "BUILD"), "pytest")
      File.write!(Path.join(pkg_dir, "README.md"), "# Readme")

      package = %{path: pkg_dir, language: "python"}
      hash = Hasher.hash_package(package)
      assert String.length(hash) == 64
    end

    test "hash changes when source file changes", %{tmp_dir: tmp_dir} do
      pkg_dir = Path.join(tmp_dir, "pkg")
      File.mkdir_p!(pkg_dir)
      File.write!(Path.join(pkg_dir, "main.py"), "version 1")
      File.write!(Path.join(pkg_dir, "BUILD"), "pytest")

      package = %{path: pkg_dir, language: "python"}
      hash1 = Hasher.hash_package(package)

      File.write!(Path.join(pkg_dir, "main.py"), "version 2")
      hash2 = Hasher.hash_package(package)

      assert hash1 != hash2
    end

    test "hash is deterministic for same content", %{tmp_dir: tmp_dir} do
      pkg_dir = Path.join(tmp_dir, "pkg")
      File.mkdir_p!(pkg_dir)
      File.write!(Path.join(pkg_dir, "main.py"), "hello")
      File.write!(Path.join(pkg_dir, "BUILD"), "pytest")

      package = %{path: pkg_dir, language: "python"}
      hash1 = Hasher.hash_package(package)
      hash2 = Hasher.hash_package(package)

      assert hash1 == hash2
    end

    test "hash changes when BUILD file changes", %{tmp_dir: tmp_dir} do
      pkg_dir = Path.join(tmp_dir, "pkg")
      File.mkdir_p!(pkg_dir)
      File.write!(Path.join(pkg_dir, "BUILD"), "echo v1")

      package = %{path: pkg_dir, language: "python"}
      hash1 = Hasher.hash_package(package)

      File.write!(Path.join(pkg_dir, "BUILD"), "echo v2")
      hash2 = Hasher.hash_package(package)

      assert hash1 != hash2
    end

    test "empty package gets a hash" do
      pkg_dir = Path.join(System.tmp_dir!(), "empty_pkg_#{:rand.uniform(100_000)}")
      File.mkdir_p!(pkg_dir)
      on_exit(fn -> File.rm_rf!(pkg_dir) end)

      package = %{path: pkg_dir, language: "python"}
      hash = Hasher.hash_package(package)
      assert String.length(hash) == 64
    end
  end

  # ---------------------------------------------------------------------------
  # hash_deps/3
  # ---------------------------------------------------------------------------

  describe "hash_deps/3" do
    test "returns empty hash for package with no dependencies" do
      g = DirectedGraph.new() |> DirectedGraph.add_node("A")
      hashes = %{"A" => "abc123"}

      hash = Hasher.hash_deps("A", g, hashes)
      # Should be hash of empty string (no deps)
      assert hash == Hasher.hash_string("")
    end

    test "includes transitive dependency hashes" do
      g =
        DirectedGraph.new()
        |> DirectedGraph.add_edge("A", "B")
        |> DirectedGraph.add_edge("B", "C")

      hashes = %{"A" => "hash_a", "B" => "hash_b", "C" => "hash_c"}

      # C depends on B which depends on A. So C's dep hash should include A and B.
      hash = Hasher.hash_deps("C", g, hashes)
      assert String.length(hash) == 64
      assert hash != Hasher.hash_string("")
    end

    test "dep hash changes when a dependency hash changes" do
      g =
        DirectedGraph.new()
        |> DirectedGraph.add_edge("A", "B")

      hashes1 = %{"A" => "hash_v1", "B" => "hash_b"}
      hashes2 = %{"A" => "hash_v2", "B" => "hash_b"}

      hash1 = Hasher.hash_deps("B", g, hashes1)
      hash2 = Hasher.hash_deps("B", g, hashes2)

      assert hash1 != hash2
    end

    test "returns empty hash for unknown package" do
      g = DirectedGraph.new()
      hash = Hasher.hash_deps("UNKNOWN", g, %{})
      assert hash == Hasher.hash_string("")
    end
  end
end
