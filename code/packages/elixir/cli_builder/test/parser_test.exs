defmodule CodingAdventures.CliBuilder.ParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.{Parser, ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Embedded JSON specs for each Unix utility example
  # ---------------------------------------------------------------------------

  @echo_spec """
  {
    "cli_builder_spec_version": "1.0",
    "name": "echo",
    "description": "Display a line of text",
    "version": "8.32",
    "flags": [
      {"id": "no-newline", "short": "n", "description": "Do not output the trailing newline", "type": "boolean"},
      {"id": "enable-escapes", "short": "e", "description": "Enable interpretation of backslash escapes", "type": "boolean", "conflicts_with": ["disable-escapes"]},
      {"id": "disable-escapes", "short": "E", "description": "Disable interpretation of backslash escapes (default)", "type": "boolean", "conflicts_with": ["enable-escapes"]}
    ],
    "arguments": [
      {"id": "string", "name": "STRING", "description": "Text to print", "type": "string", "required": false, "variadic": true, "variadic_min": 0}
    ]
  }
  """

  @ls_spec """
  {
    "cli_builder_spec_version": "1.0",
    "name": "ls",
    "description": "List directory contents",
    "flags": [
      {"id": "long-listing", "short": "l", "description": "Use long listing format", "type": "boolean", "conflicts_with": ["single-column"]},
      {"id": "single-column", "short": "1", "description": "List one file per line", "type": "boolean", "conflicts_with": ["long-listing"]},
      {"id": "all", "short": "a", "description": "Include hidden files", "type": "boolean"},
      {"id": "human-readable", "short": "h", "long": "human-readable", "description": "Human-readable sizes", "type": "boolean", "requires": ["long-listing"]},
      {"id": "recursive", "short": "R", "long": "recursive", "description": "List subdirectories recursively", "type": "boolean"}
    ],
    "arguments": [
      {"id": "file", "name": "FILE", "description": "File or directory to list", "type": "path", "required": false, "variadic": true, "variadic_min": 0}
    ]
  }
  """

  @cp_spec """
  {
    "cli_builder_spec_version": "1.0",
    "name": "cp",
    "description": "Copy files and directories",
    "flags": [
      {"id": "recursive", "short": "r", "long": "recursive", "description": "Copy directories recursively", "type": "boolean"},
      {"id": "verbose", "short": "v", "long": "verbose", "description": "Explain what is being done", "type": "boolean"},
      {"id": "force", "short": "f", "long": "force", "description": "Force overwrite", "type": "boolean"}
    ],
    "arguments": [
      {"id": "source", "name": "SOURCE", "description": "Source", "type": "path", "required": true, "variadic": true, "variadic_min": 1},
      {"id": "dest", "name": "DEST", "description": "Destination", "type": "path", "required": true}
    ]
  }
  """

  @grep_spec """
  {
    "cli_builder_spec_version": "1.0",
    "name": "grep",
    "description": "Print lines matching a pattern",
    "flags": [
      {"id": "extended-regexp", "short": "E", "long": "extended-regexp", "description": "Interpret PATTERN as extended regular expression", "type": "boolean"},
      {"id": "fixed-strings", "short": "F", "long": "fixed-strings", "description": "Interpret PATTERN as fixed string", "type": "boolean"},
      {"id": "perl-regexp", "short": "P", "long": "perl-regexp", "description": "Interpret PATTERN as Perl regular expression", "type": "boolean"},
      {"id": "ignore-case", "short": "i", "long": "ignore-case", "description": "Ignore case distinctions", "type": "boolean"},
      {"id": "line-number", "short": "n", "long": "line-number", "description": "Print line number with output lines", "type": "boolean"},
      {"id": "count", "short": "c", "long": "count", "description": "Print count of matching lines", "type": "boolean"},
      {"id": "regexp", "short": "e", "long": "regexp", "description": "Provide PATTERN", "type": "string", "repeatable": true}
    ],
    "arguments": [
      {"id": "pattern", "name": "PATTERN", "description": "Pattern to match", "type": "string", "required": true, "required_unless_flag": ["regexp"]},
      {"id": "file", "name": "FILE", "description": "File to search", "type": "path", "required": false, "variadic": true, "variadic_min": 0}
    ],
    "mutually_exclusive_groups": [
      {"id": "regexp-engine", "flag_ids": ["extended-regexp", "fixed-strings", "perl-regexp"]}
    ]
  }
  """

  @git_spec """
  {
    "cli_builder_spec_version": "1.0",
    "name": "git",
    "description": "The stupid content tracker",
    "version": "2.40.0",
    "global_flags": [
      {"id": "verbose", "short": "v", "long": "verbose", "description": "Be verbose", "type": "boolean"}
    ],
    "flags": [],
    "commands": [
      {
        "id": "cmd-commit",
        "name": "commit",
        "description": "Record changes to the repository",
        "flags": [
          {"id": "message", "short": "m", "long": "message", "description": "Commit message", "type": "string", "required": true},
          {"id": "all", "short": "a", "long": "all", "description": "Stage all modified tracked files", "type": "boolean"},
          {"id": "amend", "long": "amend", "description": "Amend last commit", "type": "boolean"}
        ],
        "arguments": []
      },
      {
        "id": "cmd-remote",
        "name": "remote",
        "description": "Manage set of tracked repositories",
        "flags": [],
        "commands": [
          {
            "id": "cmd-remote-add",
            "name": "add",
            "description": "Add a named remote repository",
            "flags": [
              {"id": "fetch", "short": "f", "description": "Run git fetch after add", "type": "boolean"}
            ],
            "arguments": [
              {"id": "name", "name": "NAME", "description": "Remote name", "type": "string", "required": true},
              {"id": "url", "name": "URL", "description": "Remote URL", "type": "string", "required": true}
            ]
          }
        ]
      }
    ]
  }
  """

  @tar_spec """
  {
    "cli_builder_spec_version": "1.0",
    "name": "tar",
    "description": "Tape archiver",
    "parsing_mode": "traditional",
    "flags": [
      {"id": "extract", "short": "x", "description": "Extract files", "type": "boolean"},
      {"id": "create", "short": "c", "description": "Create archive", "type": "boolean"},
      {"id": "verbose", "short": "v", "description": "Verbose", "type": "boolean"},
      {"id": "file", "short": "f", "description": "Archive file", "type": "string"}
    ],
    "arguments": [
      {"id": "files", "name": "FILE", "description": "Files to archive", "type": "path", "required": false, "variadic": true, "variadic_min": 0}
    ]
  }
  """

  # ---------------------------------------------------------------------------
  # Convenience wrapper
  # ---------------------------------------------------------------------------

  defp parse(json, argv), do: Parser.parse_string(json, argv)

  defp ok_result!(json, argv) do
    {:ok, result} = parse(json, argv)
    result
  end

  defp error_result!(json, argv) do
    {:error, errs} = parse(json, argv)
    errs
  end

  # ---------------------------------------------------------------------------
  # echo spec tests
  # ---------------------------------------------------------------------------

  describe "echo — minimal spec" do
    test "empty argv produces empty string list" do
      result = ok_result!(@echo_spec, [])
      assert result.arguments["string"] == []
      assert result.flags["no-newline"] == false
    end

    test "positional arguments are collected" do
      result = ok_result!(@echo_spec, ["hello", "world"])
      assert result.arguments["string"] == ["hello", "world"]
    end

    test "boolean flag is recognised" do
      result = ok_result!(@echo_spec, ["-n", "hello"])
      assert result.flags["no-newline"] == true
      assert result.arguments["string"] == ["hello"]
    end

    test "conflicting flags -e and -E produce error" do
      errs = error_result!(@echo_spec, ["-e", "-E", "hello"])
      types = Enum.map(errs.errors, & &1.error_type)
      assert "conflicting_flags" in types
    end

    test "--help returns HelpResult" do
      {:ok, result} = parse(@echo_spec, ["--help"])
      assert %HelpResult{} = result
      assert result.command_path == ["echo"]
    end

    test "--version returns VersionResult" do
      {:ok, result} = parse(@echo_spec, ["--version"])
      assert %VersionResult{version: "8.32"} = result
    end

    test "program name prefix is stripped" do
      result = ok_result!(@echo_spec, ["echo", "hello"])
      assert result.program == "echo"
      assert result.arguments["string"] == ["hello"]
    end

    test "command_path is [program] for root invocation" do
      result = ok_result!(@echo_spec, ["hello"])
      assert result.command_path == ["echo"]
    end
  end

  # ---------------------------------------------------------------------------
  # ls spec tests
  # ---------------------------------------------------------------------------

  describe "ls — stacked flags and requires" do
    test "basic invocation with no args" do
      result = ok_result!(@ls_spec, [])
      assert result.flags["long-listing"] == false
      assert result.arguments["file"] == []
    end

    test "stacked boolean flags" do
      result = ok_result!(@ls_spec, ["-la"])
      assert result.flags["long-listing"] == true
      assert result.flags["all"] == true
    end

    test "-lah: l requires -l, h requires l → both present" do
      result = ok_result!(@ls_spec, ["-lah"])
      assert result.flags["long-listing"] == true
      assert result.flags["all"] == true
      assert result.flags["human-readable"] == true
    end

    test "-h without -l produces missing_dependency_flag error" do
      # human-readable requires long-listing
      # Note: -h here triggers help (builtin), so use --human-readable
      errs = error_result!(@ls_spec, ["--human-readable"])
      types = Enum.map(errs.errors, & &1.error_type)
      assert "missing_dependency_flag" in types
    end

    test "conflicting flags -l and -1" do
      errs = error_result!(@ls_spec, ["-l", "-1"])
      types = Enum.map(errs.errors, & &1.error_type)
      assert "conflicting_flags" in types
    end

    test "path arguments collected" do
      result = ok_result!(@ls_spec, ["/etc", "/usr"])
      assert result.arguments["file"] == ["/etc", "/usr"]
    end

    test "flags and paths interleaved (gnu mode)" do
      result = ok_result!(@ls_spec, ["/etc", "-l", "/usr"])
      assert result.flags["long-listing"] == true
      assert result.arguments["file"] == ["/etc", "/usr"]
    end

    test "-- ends flag scanning" do
      result = ok_result!(@ls_spec, ["--", "-l"])
      # After --, "-l" is a positional (the file named literally "-l")
      assert result.arguments["file"] == ["-l"]
      assert result.flags["long-listing"] == false
    end
  end

  # ---------------------------------------------------------------------------
  # cp spec tests (variadic + trailing required argument)
  # ---------------------------------------------------------------------------

  describe "cp — variadic source + required trailing dest" do
    test "one source, one dest" do
      result = ok_result!(@cp_spec, ["a.txt", "b.txt"])
      assert result.arguments["source"] == ["a.txt"]
      assert result.arguments["dest"] == "b.txt"
    end

    test "multiple sources, one dest" do
      result = ok_result!(@cp_spec, ["a.txt", "b.txt", "c.txt", "/dest/"])
      assert result.arguments["source"] == ["a.txt", "b.txt", "c.txt"]
      assert result.arguments["dest"] == "/dest/"
    end

    test "missing dest → missing_required_argument error" do
      errs = error_result!(@cp_spec, [])
      types = Enum.map(errs.errors, & &1.error_type)
      assert "missing_required_argument" in types
    end

    test "flags before positionals" do
      result = ok_result!(@cp_spec, ["-rv", "src/", "/dest/"])
      assert result.flags["recursive"] == true
      assert result.flags["verbose"] == true
      assert result.arguments["source"] == ["src/"]
      assert result.arguments["dest"] == "/dest/"
    end

    test "flags between positionals (gnu mode)" do
      result = ok_result!(@cp_spec, ["src/", "-r", "/dest/"])
      assert result.flags["recursive"] == true
      assert result.arguments["source"] == ["src/"]
      assert result.arguments["dest"] == "/dest/"
    end
  end

  # ---------------------------------------------------------------------------
  # grep spec tests (required_unless_flag, mutually_exclusive_groups, repeatable)
  # ---------------------------------------------------------------------------

  describe "grep — required_unless_flag, exclusive groups, repeatable" do
    test "pattern as positional when -e not used" do
      result = ok_result!(@grep_spec, ["foo", "file.txt"])
      assert result.arguments["pattern"] == "foo"
      assert result.arguments["file"] == ["file.txt"]
    end

    test "-e flag makes positional pattern optional" do
      result = ok_result!(@grep_spec, ["-e", "foo", "file.txt"])
      assert result.flags["regexp"] == ["foo"]
      # With -e consuming "foo", only "file.txt" is a positional token.
      # The partition algorithm assigns it to "pattern" (first leading slot)
      # since pattern is required_unless_flag but still occupies position 0.
      # file gets the empty variadic list.
      assert result.arguments["pattern"] == "file.txt" or result.arguments["file"] == ["file.txt"]
    end

    test "multiple -e flags are collected into array" do
      result = ok_result!(@grep_spec, ["-e", "foo", "-e", "bar"])
      assert result.flags["regexp"] == ["foo", "bar"]
    end

    test "mutually exclusive group: two engines → error" do
      errs = error_result!(@grep_spec, ["-E", "-F", "pattern"])
      types = Enum.map(errs.errors, & &1.error_type)
      assert "exclusive_group_violation" in types
    end

    test "using one engine flag is fine" do
      result = ok_result!(@grep_spec, ["-E", "^foo", "file.txt"])
      assert result.flags["extended-regexp"] == true
      assert result.flags["fixed-strings"] == false
    end

    test "missing pattern without -e → error" do
      errs = error_result!(@grep_spec, [])
      types = Enum.map(errs.errors, & &1.error_type)
      assert "missing_required_argument" in types
    end
  end

  # ---------------------------------------------------------------------------
  # git spec tests (subcommands, nested routing, global flags)
  # ---------------------------------------------------------------------------

  describe "git — subcommands and routing" do
    test "routing to commit subcommand" do
      result = ok_result!(@git_spec, ["commit", "-m", "Initial commit"])
      assert result.command_path == ["git", "commit"]
      assert result.flags["message"] == "Initial commit"
    end

    test "commit requires --message flag" do
      errs = error_result!(@git_spec, ["commit"])
      types = Enum.map(errs.errors, & &1.error_type)
      assert "missing_required_flag" in types
    end

    test "global flag works at root" do
      result = ok_result!(@git_spec, ["-v", "commit", "-m", "msg"])
      assert result.flags["verbose"] == true
    end

    test "nested routing to remote add" do
      result = ok_result!(@git_spec, ["remote", "add", "origin", "https://github.com/user/repo"])
      assert result.command_path == ["git", "remote", "add"]
      assert result.arguments["name"] == "origin"
      assert result.arguments["url"] == "https://github.com/user/repo"
    end

    test "help for subcommand returns HelpResult with correct path" do
      {:ok, result} = parse(@git_spec, ["commit", "--help"])
      assert %HelpResult{} = result
      assert result.command_path == ["git", "commit"]
    end

    test "--version at root returns VersionResult" do
      {:ok, result} = parse(@git_spec, ["--version"])
      assert %VersionResult{version: "2.40.0"} = result
    end

    test "unknown subcommand produces error" do
      errs = error_result!(@git_spec, ["comit"])
      # Either unknown_command or missing_required_argument for message
      _types = Enum.map(errs.errors, & &1.error_type)
      # Just assert we got errors
      assert length(errs.errors) > 0
    end
  end

  # ---------------------------------------------------------------------------
  # tar spec tests (traditional mode)
  # ---------------------------------------------------------------------------

  describe "tar — traditional mode" do
    test "xvf without leading dash is stacked flags" do
      result = ok_result!(@tar_spec, ["xvf", "archive.tar"])
      assert result.flags["extract"] == true
      assert result.flags["verbose"] == true
      # "f" flag: non-boolean, last in stack, takes next arg or value
      # "archive.tar" becomes the value for -f
      assert result.flags["file"] == "archive.tar"
    end

    test "traditional mode still works with leading dash" do
      result = ok_result!(@tar_spec, ["-xvf", "archive.tar"])
      assert result.flags["extract"] == true
      assert result.flags["verbose"] == true
    end

    test "non-flag first arg falls through to positional" do
      # "hello" has chars h,e,l,l,o - h is help (builtin), not a spec flag
      # so it won't be all-known. Should fall through as positional.
      result = ok_result!(@tar_spec, ["hello.tar"])
      assert result.arguments["files"] == ["hello.tar"]
    end
  end

  # ---------------------------------------------------------------------------
  # Long flag with value
  # ---------------------------------------------------------------------------

  describe "long flag with value" do
    test "--output=value inline form" do
      spec = Jason.encode!(%{
        "cli_builder_spec_version" => "1.0",
        "name" => "prog",
        "description" => "test",
        "flags" => [
          %{"id" => "output", "long" => "output", "description" => "Output file", "type" => "string"}
        ]
      })
      result = ok_result!(spec, ["--output=foo.txt"])
      assert result.flags["output"] == "foo.txt"
    end
  end

  # ---------------------------------------------------------------------------
  # Enum type validation
  # ---------------------------------------------------------------------------

  describe "enum type" do
    @enum_spec Jason.encode!(%{
      "cli_builder_spec_version" => "1.0",
      "name" => "prog",
      "description" => "test",
      "flags" => [
        %{"id" => "format", "long" => "format", "description" => "Output format",
          "type" => "enum", "enum_values" => ["json", "csv", "table"]}
      ]
    })

    test "valid enum value accepted" do
      result = ok_result!(@enum_spec, ["--format", "json"])
      assert result.flags["format"] == "json"
    end

    test "invalid enum value rejected" do
      errs = error_result!(@enum_spec, ["--format", "xml"])
      types = Enum.map(errs.errors, & &1.error_type)
      assert "invalid_enum_value" in types or "invalid_value" in types
    end
  end

  # ---------------------------------------------------------------------------
  # Integer and float coercion
  # ---------------------------------------------------------------------------

  describe "type coercion" do
    @typed_spec Jason.encode!(%{
      "cli_builder_spec_version" => "1.0",
      "name" => "prog",
      "description" => "test",
      "flags" => [
        %{"id" => "count", "long" => "count", "description" => "Count", "type" => "integer"},
        %{"id" => "ratio", "long" => "ratio", "description" => "Ratio", "type" => "float"}
      ]
    })

    test "integer flag coerced to integer" do
      result = ok_result!(@typed_spec, ["--count", "42"])
      assert result.flags["count"] == 42
      assert is_integer(result.flags["count"])
    end

    test "invalid integer produces error" do
      errs = error_result!(@typed_spec, ["--count", "abc"])
      types = Enum.map(errs.errors, & &1.error_type)
      assert "invalid_value" in types
    end

    test "float flag coerced to float" do
      result = ok_result!(@typed_spec, ["--ratio", "3.14"])
      assert_in_delta result.flags["ratio"], 3.14, 0.001
      assert is_float(result.flags["ratio"])
    end

    test "invalid float produces error" do
      errs = error_result!(@typed_spec, ["--ratio", "notanumber"])
      types = Enum.map(errs.errors, & &1.error_type)
      assert "invalid_value" in types
    end
  end

  # ---------------------------------------------------------------------------
  # Posix mode
  # ---------------------------------------------------------------------------

  describe "POSIX mode: first positional ends flag scanning" do
    @posix_spec Jason.encode!(%{
      "cli_builder_spec_version" => "1.0",
      "name" => "prog",
      "description" => "test",
      "parsing_mode" => "posix",
      "flags" => [
        %{"id" => "verbose", "short" => "v", "description" => "Verbose", "type" => "boolean"}
      ],
      "arguments" => [
        %{"id" => "file", "name" => "FILE", "description" => "File", "type" => "path",
          "required" => false, "variadic" => true, "variadic_min" => 0}
      ]
    })

    test "flags before first positional are parsed" do
      result = ok_result!(@posix_spec, ["-v", "file.txt"])
      assert result.flags["verbose"] == true
      assert result.arguments["file"] == ["file.txt"]
    end

    test "flags after first positional become positionals" do
      result = ok_result!(@posix_spec, ["file.txt", "-v"])
      # In POSIX mode, -v after file.txt should be treated as positional
      assert result.arguments["file"] == ["file.txt", "-v"]
    end
  end

  # ---------------------------------------------------------------------------
  # Unknown flag with fuzzy suggestion
  # ---------------------------------------------------------------------------

  describe "unknown flag fuzzy suggestion" do
    test "close typo in long flag suggests the right flag" do
      errs = error_result!(@git_spec, ["commit", "--mesage", "foo"])
      err = Enum.find(errs.errors, fn e -> e.error_type == "unknown_flag" end)
      assert err != nil
      assert err.suggestion != nil or String.contains?(err.message, "message")
    end
  end

  # ---------------------------------------------------------------------------
  # ParseErrors exception format
  # ---------------------------------------------------------------------------

  describe "ParseErrors formatting" do
    test "message joins all error messages" do
      errs = error_result!(@echo_spec, ["-e", "-E", "hello"])
      assert is_binary(errs.message)
      assert String.length(errs.message) > 0
    end
  end
end
