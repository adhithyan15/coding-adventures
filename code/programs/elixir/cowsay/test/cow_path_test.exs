defmodule CodingAdventures.Cowsay.CowPathTest do
  # Tests for cow-file selection -- the path-traversal fix for issue #12169.
  #
  # Each test builds a throwaway tree like this:
  #
  #   <tmp>/
  #     secret.cow          <- must NEVER be readable via -f
  #     cows/
  #       default.cow       <- the fallback
  #       tux.cow           <- a legitimate cow
  #       nested/inner.cow  <- exists, but "nested/inner" is not a bare name
  #
  # and asserts that every hostile spelling of "secret" draws the default cow.
  use ExUnit.Case, async: true

  alias CodingAdventures.Cowsay.CowPath

  setup do
    base =
      Path.join(
        System.tmp_dir!(),
        "cowsay-traversal-#{System.unique_integer([:positive])}"
      )

    cows = Path.join(base, "cows")
    write_cow(Path.join(cows, "default.cow"), "DEFAULT")
    write_cow(Path.join(cows, "tux.cow"), "TUX")
    write_cow(Path.join([cows, "nested", "inner.cow"]), "NESTED")
    write_cow(Path.join(base, "secret.cow"), "SECRET")
    on_exit(fn -> File.rm_rf!(base) end)
    %{base: base, cows: cows}
  end

  defp write_cow(path, body) do
    File.mkdir_p!(Path.dirname(path))
    File.write!(path, "$the_cow = <<EOC;\n#{body}\nEOC\n")
  end

  defp drawn(cow_name, cows), do: File.read!(CowPath.resolve(cow_name, cows))

  test "bare names are safe" do
    for name <- ["default", "tux", "bud-frogs", "three_eyes", "v2", "dragon.and.cow"] do
      assert CowPath.safe_name?(name), "should accept #{inspect(name)}"
    end
  end

  test "hostile names are unsafe" do
    hostile = [
      "",
      "..",
      "../secret",
      "..\\secret",
      "a/b",
      "a\\b",
      "/etc/passwd",
      "C:secret",
      "C:\\Windows\\win",
      "tux" <> <<0>>,
      "..%2Fsecret",
      "%2e%2e/secret",
      nil,
      42
    ]

    for name <- hostile do
      refute CowPath.safe_name?(name), "should reject #{inspect(name)}"
    end
  end

  test "normal cow names still load", %{cows: cows} do
    assert drawn("tux", cows) =~ "TUX"
    assert drawn("default", cows) =~ "DEFAULT"
  end

  test "an unknown cow falls back to default", %{cows: cows} do
    assert drawn("does-not-exist", cows) =~ "DEFAULT"
  end

  test "relative traversal falls back to default", %{cows: cows} do
    # Sanity: the target really is reachable by naive joining.
    assert File.exists?(Path.join([cows, "..", "secret.cow"]))

    for hostile <- ["../secret", "..\\secret", "./../secret", "tux/../../secret"] do
      assert drawn(hostile, cows) =~ "DEFAULT", "for #{inspect(hostile)}"
    end
  end

  test "an absolute path falls back to default", %{base: base, cows: cows} do
    assert drawn(Path.join(base, "secret"), cows) =~ "DEFAULT"
  end

  test "nested names are refused even inside the cows dir", %{cows: cows} do
    for nested <- ["nested/inner", "nested\\inner"] do
      assert drawn(nested, cows) =~ "DEFAULT", "for #{inspect(nested)}"
    end
  end

  test "encoded and NUL names fall back to default", %{cows: cows} do
    for hostile <- [
          "..%2Fsecret",
          "%2e%2e%2fsecret",
          "%2E%2E%5Csecret",
          "tux" <> <<0>> <> "../secret"
        ] do
      assert drawn(hostile, cows) =~ "DEFAULT", "for #{inspect(hostile)}"
    end
  end

  # Layer 2 in action: "evil" is syntactically fine, but the file it names is a
  # symlink leading out of the cows directory.
  test "a symlink escaping the cows dir falls back to default", %{base: base, cows: cows} do
    case File.ln_s(Path.join(base, "secret.cow"), Path.join(cows, "evil.cow")) do
      :ok -> assert drawn("evil", cows) =~ "DEFAULT"
      # Windows without developer mode cannot create symlinks; nothing to test.
      {:error, _reason} -> :ok
    end
  end

  test "the repository's real cows load" do
    cows = Path.expand("../../../../specs/cows", __DIR__)
    refute drawn("tux", cows) == drawn("default", cows)
  end
end
