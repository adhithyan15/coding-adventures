defmodule CodingAdventures.CliBuilder.PositionalResolverTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.PositionalResolver

  # ---------------------------------------------------------------------------
  # Helpers — build lightweight argument definition maps matching SpecLoader output
  # ---------------------------------------------------------------------------

  defp arg(id, name, type, opts \\ []) do
    required = Keyword.get(opts, :required, true)
    variadic = Keyword.get(opts, :variadic, false)
    variadic_min_default = if required, do: 1, else: 0

    %{
      "id" => id,
      "name" => name,
      "type" => type,
      "required" => required,
      "variadic" => variadic,
      "variadic_min" => Keyword.get(opts, :variadic_min, variadic_min_default),
      "variadic_max" => Keyword.get(opts, :variadic_max),
      "default" => Keyword.get(opts, :default),
      "enum_values" => Keyword.get(opts, :enum_values, []),
      "required_unless_flag" => Keyword.get(opts, :required_unless_flag, [])
    }
  end

  defp resolve(tokens, defs, flags \\ %{}),
    do: PositionalResolver.resolve(tokens, defs, flags, ["prog"])

  # ---------------------------------------------------------------------------
  # coerce/2 — type coercion public API
  # ---------------------------------------------------------------------------

  describe "coerce/2 — float type" do
    test "parses a decimal float string" do
      assert PositionalResolver.coerce("3.14", "float") == {:ok, 3.14}
    end

    test "parses a negative float" do
      assert PositionalResolver.coerce("-0.5", "float") == {:ok, -0.5}
    end

    test "coerces an integer string to a float" do
      # Integer.parse fallback: 42 -> 42 * 1.0
      assert PositionalResolver.coerce("42", "float") == {:ok, 42.0}
    end

    test "rejects a non-numeric string" do
      assert {:error, msg} = PositionalResolver.coerce("abc", "float")
      assert msg =~ "Invalid float"
    end

    test "rejects partially numeric string with trailing chars" do
      # Float.parse("3.14x") returns {3.14, "x"}, not a full match → integer fallback also fails
      assert {:error, _} = PositionalResolver.coerce("3.14x", "float")
    end
  end

  describe "coerce/2 — path type" do
    test "any non-empty string is a valid path" do
      assert PositionalResolver.coerce("/some/path/to/file", "path") == {:ok, "/some/path/to/file"}
    end

    test "relative paths accepted" do
      assert PositionalResolver.coerce("relative/path.txt", "path") == {:ok, "relative/path.txt"}
    end

    test "empty string rejected" do
      assert {:error, msg} = PositionalResolver.coerce("", "path")
      assert msg =~ "non-empty"
    end
  end

  describe "coerce/2 — boolean type" do
    test "true string" do
      assert PositionalResolver.coerce("true", "boolean") == {:ok, true}
    end

    test "false string" do
      assert PositionalResolver.coerce("false", "boolean") == {:ok, false}
    end

    test "other string is an error" do
      assert {:error, msg} = PositionalResolver.coerce("yes", "boolean")
      assert msg =~ "Invalid boolean"
    end
  end

  describe "coerce/2 — string type" do
    test "non-empty string passes" do
      assert PositionalResolver.coerce("hello", "string") == {:ok, "hello"}
    end

    test "empty string is rejected" do
      assert {:error, msg} = PositionalResolver.coerce("", "string")
      assert msg =~ "non-empty"
    end
  end

  describe "coerce/2 — integer type" do
    test "valid integer" do
      assert PositionalResolver.coerce("123", "integer") == {:ok, 123}
    end

    test "negative integer" do
      assert PositionalResolver.coerce("-7", "integer") == {:ok, -7}
    end

    test "float string rejected" do
      assert {:error, msg} = PositionalResolver.coerce("1.5", "integer")
      assert msg =~ "Invalid integer"
    end

    test "alpha string rejected" do
      assert {:error, _} = PositionalResolver.coerce("abc", "integer")
    end
  end

  describe "coerce/2 — enum type" do
    test "any string is returned as-is (validation elsewhere)" do
      assert PositionalResolver.coerce("csv", "enum") == {:ok, "csv"}
      assert PositionalResolver.coerce("anything_goes", "enum") == {:ok, "anything_goes"}
    end
  end

  describe "coerce/2 — file type" do
    test "a non-existent path returns error" do
      assert {:error, msg} = PositionalResolver.coerce("/nonexistent/path/to/file.txt", "file")
      assert msg =~ "not found" or msg =~ "not readable"
    end

    test "an existing directory path returns 'not a regular file' error" do
      # Use the system temp dir, which exists on all platforms
      tmpdir = System.tmp_dir!()
      assert {:error, msg} = PositionalResolver.coerce(tmpdir, "file")
      assert msg =~ "Not a regular file" or msg =~ "not found"
    end
  end

  describe "coerce/2 — directory type" do
    test "a non-existent path returns error" do
      assert {:error, msg} = PositionalResolver.coerce("/nonexistent/dir/path", "directory")
      assert msg =~ "not found"
    end

    test "existing directory is accepted" do
      tmpdir = System.tmp_dir!()
      assert {:ok, ^tmpdir} = PositionalResolver.coerce(tmpdir, "directory")
    end

    test "a regular file path returns 'not a directory' error" do
      # Create a temp file to test against
      tmpdir = System.tmp_dir!()
      path = Path.join(tmpdir, "cli_builder_test_file_#{:rand.uniform(999_999)}.txt")
      File.write!(path, "data")

      try do
        assert {:error, msg} = PositionalResolver.coerce(path, "directory")
        assert msg =~ "Not a directory"
      after
        File.rm(path)
      end
    end
  end

  # ---------------------------------------------------------------------------
  # No-variadic resolution
  # ---------------------------------------------------------------------------

  describe "resolve/4 — no variadic arguments" do
    test "single required arg, token present" do
      defs = [arg("file", "FILE", "string")]
      assert {:ok, %{"file" => "hello.txt"}} = resolve(["hello.txt"], defs)
    end

    test "single optional arg, no token, uses default" do
      defs = [arg("file", "FILE", "string", required: false, default: "stdin")]
      assert {:ok, %{"file" => "stdin"}} = resolve([], defs)
    end

    test "single optional arg, no token, nil default" do
      defs = [arg("file", "FILE", "string", required: false)]
      assert {:ok, %{"file" => nil}} = resolve([], defs)
    end

    test "missing required arg → missing_required_argument error" do
      defs = [arg("file", "FILE", "string")]
      assert {:error, errors} = resolve([], defs)
      assert Enum.any?(errors, &(&1.error_type == "missing_required_argument"))
    end

    test "too many tokens → too_many_arguments error" do
      defs = [arg("file", "FILE", "string")]
      assert {:error, errors} = resolve(["a", "b"], defs)
      assert Enum.any?(errors, &(&1.error_type == "too_many_arguments"))
    end

    test "invalid type value → invalid_value error" do
      defs = [arg("count", "COUNT", "integer")]
      assert {:error, errors} = resolve(["notanumber"], defs)
      assert Enum.any?(errors, &(&1.error_type == "invalid_value"))
    end

    test "float argument coercion" do
      defs = [arg("ratio", "RATIO", "float")]
      assert {:ok, %{"ratio" => ratio}} = resolve(["2.71"], defs)
      assert_in_delta ratio, 2.71, 0.001
    end

    test "integer argument coercion" do
      defs = [arg("count", "COUNT", "integer")]
      assert {:ok, %{"count" => 42}} = resolve(["42"], defs)
    end

    test "path argument accepted without existence check" do
      defs = [arg("path", "PATH", "path")]
      assert {:ok, %{"path" => "/nonexistent/path.txt"}} = resolve(["/nonexistent/path.txt"], defs)
    end

    test "required_unless_flag exempts missing required arg" do
      # If the flag "verbose" is present in parsed_flags, "file" is not required
      defs = [arg("file", "FILE", "string", required_unless_flag: ["verbose"])]
      assert {:ok, _} = resolve([], defs, %{"verbose" => true})
    end

    test "required_unless_flag does not exempt when flag value is false" do
      defs = [arg("file", "FILE", "string", required_unless_flag: ["verbose"])]
      assert {:error, errors} = resolve([], defs, %{"verbose" => false})
      assert Enum.any?(errors, &(&1.error_type == "missing_required_argument"))
    end

    test "required_unless_flag does not exempt when flag absent" do
      defs = [arg("file", "FILE", "string", required_unless_flag: ["verbose"])]
      assert {:error, errors} = resolve([], defs, %{})
      assert Enum.any?(errors, &(&1.error_type == "missing_required_argument"))
    end

    test "multiple required args — both present" do
      defs = [arg("src", "SRC", "string"), arg("dst", "DST", "string")]
      assert {:ok, %{"src" => "a.txt", "dst" => "b.txt"}} = resolve(["a.txt", "b.txt"], defs)
    end

    test "multiple required args — second missing → error" do
      defs = [arg("src", "SRC", "string"), arg("dst", "DST", "string")]
      assert {:error, errors} = resolve(["a.txt"], defs)
      assert Enum.any?(errors, &(&1.error_type == "missing_required_argument"))
    end
  end

  # ---------------------------------------------------------------------------
  # Variadic resolution
  # ---------------------------------------------------------------------------

  describe "resolve/4 — variadic argument" do
    test "all tokens go to variadic when it is the only arg" do
      defs = [arg("files", "FILES", "path", required: false, variadic: true, variadic_min: 0)]
      assert {:ok, %{"files" => ["a", "b", "c"]}} = resolve(["a", "b", "c"], defs)
    end

    test "empty variadic accepted when variadic_min is 0" do
      defs = [arg("files", "FILES", "path", required: false, variadic: true, variadic_min: 0)]
      assert {:ok, %{"files" => []}} = resolve([], defs)
    end

    test "variadic_min > 0 with too few tokens → too_few_arguments error" do
      defs = [arg("files", "FILES", "path", required: true, variadic: true, variadic_min: 2)]
      assert {:error, errors} = resolve(["only_one"], defs)
      assert Enum.any?(errors, &(&1.error_type == "too_few_arguments"))
    end

    test "variadic_max exceeded → too_many_arguments error" do
      defs = [
        arg("files", "FILES", "path",
          required: false,
          variadic: true,
          variadic_min: 0,
          variadic_max: 2
        )
      ]

      assert {:error, errors} = resolve(["a", "b", "c"], defs)
      assert Enum.any?(errors, &(&1.error_type == "too_many_arguments"))
    end

    test "variadic_max not exceeded → ok" do
      defs = [
        arg("files", "FILES", "path",
          required: false,
          variadic: true,
          variadic_min: 0,
          variadic_max: 3
        )
      ]

      assert {:ok, %{"files" => ["a", "b", "c"]}} = resolve(["a", "b", "c"], defs)
    end

    test "leading + variadic + trailing pattern (cp-style)" do
      # source (variadic), dest (required trailing)
      defs = [
        arg("source", "SOURCE", "path", required: true, variadic: true, variadic_min: 1),
        arg("dest", "DEST", "path")
      ]

      assert {:ok, result} = resolve(["a.txt", "b.txt", "c.txt", "/dest/"], defs)
      assert result["source"] == ["a.txt", "b.txt", "c.txt"]
      assert result["dest"] == "/dest/"
    end

    test "leading required arg + variadic" do
      # head (required), tail (variadic optional)
      defs = [
        arg("head", "HEAD", "string"),
        arg("rest", "REST", "string", required: false, variadic: true, variadic_min: 0)
      ]

      assert {:ok, result} = resolve(["first", "second", "third"], defs)
      assert result["head"] == "first"
      assert result["rest"] == ["second", "third"]
    end

    test "trailing arg not provided when not enough tokens → missing_required_argument" do
      defs = [
        arg("source", "SOURCE", "path", required: true, variadic: true, variadic_min: 1),
        arg("dest", "DEST", "path")
      ]

      # Only one token — variadic (source) gets 0 tokens because dest claims it
      assert {:error, errors} = resolve(["only_src"], defs)
      assert Enum.any?(errors, &(&1.error_type == "too_few_arguments"))
    end

    test "variadic type coercion error is collected" do
      defs = [
        arg("nums", "NUM", "integer", required: false, variadic: true, variadic_min: 0)
      ]

      assert {:error, errors} = resolve(["1", "bad", "3"], defs)
      assert Enum.any?(errors, &(&1.error_type == "invalid_value"))
    end

    test "required_unless_flag in variadic trailing arg" do
      defs = [
        arg("source", "SOURCE", "path", required: true, variadic: true, variadic_min: 0),
        arg("dest", "DEST", "path", required_unless_flag: ["dry-run"])
      ]

      # dry-run flag present, dest should be exempt
      assert {:ok, _} = resolve([], defs, %{"dry-run" => true})
    end
  end
end
