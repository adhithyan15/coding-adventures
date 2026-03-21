defmodule CodingAdventures.CliBuilder.SpecLoaderTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.{SpecLoader, SpecError}

  # ---------------------------------------------------------------------------
  # Helpers: minimal valid spec JSON strings
  # ---------------------------------------------------------------------------

  defp echo_spec do
    Jason.encode!(%{
      "cli_builder_spec_version" => "1.0",
      "name" => "echo",
      "description" => "Display a line of text",
      "version" => "8.32",
      "flags" => [
        %{
          "id" => "no-newline",
          "short" => "n",
          "description" => "Do not output trailing newline",
          "type" => "boolean"
        }
      ],
      "arguments" => [
        %{
          "id" => "string",
          "name" => "STRING",
          "description" => "Text to print",
          "type" => "string",
          "required" => false,
          "variadic" => true,
          "variadic_min" => 0
        }
      ]
    })
  end

  defp minimal_spec do
    Jason.encode!(%{
      "cli_builder_spec_version" => "1.0",
      "name" => "prog",
      "description" => "A minimal program"
    })
  end

  # ---------------------------------------------------------------------------
  # Happy-path tests
  # ---------------------------------------------------------------------------

  describe "load_from_string!/1" do
    test "loads a valid minimal spec" do
      spec = SpecLoader.load_from_string!(minimal_spec())
      assert spec["name"] == "prog"
      assert spec["description"] == "A minimal program"
      assert spec["parsing_mode"] == "gnu"
      assert spec["cli_builder_spec_version"] == "1.0"
    end

    test "fills in optional fields with defaults" do
      spec = SpecLoader.load_from_string!(minimal_spec())
      assert spec["global_flags"] == []
      assert spec["flags"] == []
      assert spec["arguments"] == []
      assert spec["commands"] == []
      assert spec["mutually_exclusive_groups"] == []
      assert spec["builtin_flags"]["help"] == true
      assert spec["builtin_flags"]["version"] == true
    end

    test "loads echo spec with flags and arguments" do
      spec = SpecLoader.load_from_string!(echo_spec())
      assert spec["name"] == "echo"
      assert length(spec["flags"]) == 1
      assert hd(spec["flags"])["id"] == "no-newline"
      assert hd(spec["flags"])["type"] == "boolean"
      assert length(spec["arguments"]) == 1
      arg = hd(spec["arguments"])
      assert arg["id"] == "string"
      assert arg["variadic"] == true
      assert arg["variadic_min"] == 0
    end

    test "normalises flag optional fields" do
      spec = SpecLoader.load_from_string!(echo_spec())
      flag = hd(spec["flags"])
      assert flag["required"] == false
      assert flag["default"] == nil
      assert flag["conflicts_with"] == []
      assert flag["requires"] == []
      assert flag["required_unless"] == []
      assert flag["repeatable"] == false
    end

    test "normalises argument optional fields" do
      spec = SpecLoader.load_from_string!(echo_spec())
      arg = hd(spec["arguments"])
      assert arg["required"] == false
      assert arg["default"] == nil
      assert arg["enum_values"] == []
      assert arg["required_unless_flag"] == []
    end

    test "accepts all valid parsing modes" do
      for mode <- ~w[gnu posix subcommand_first traditional] do
        json =
          Jason.encode!(%{
            "cli_builder_spec_version" => "1.0",
            "name" => "prog",
            "description" => "test",
            "parsing_mode" => mode
          })

        spec = SpecLoader.load_from_string!(json)
        assert spec["parsing_mode"] == mode
      end
    end

    test "loads spec with commands" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "git",
          "description" => "Version control",
          "commands" => [
            %{
              "id" => "cmd-commit",
              "name" => "commit",
              "description" => "Record changes",
              "flags" => [
                %{
                  "id" => "message",
                  "short" => "m",
                  "long" => "message",
                  "description" => "Commit message",
                  "type" => "string"
                }
              ],
              "arguments" => [],
              "commands" => []
            }
          ]
        })

      spec = SpecLoader.load_from_string!(json)
      assert length(spec["commands"]) == 1
      cmd = hd(spec["commands"])
      assert cmd["name"] == "commit"
      assert length(cmd["flags"]) == 1
    end

    test "loads spec with global_flags" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "global_flags" => [
            %{"id" => "verbose", "short" => "v", "description" => "Verbose", "type" => "boolean"}
          ]
        })

      spec = SpecLoader.load_from_string!(json)
      assert length(spec["global_flags"]) == 1
      assert hd(spec["global_flags"])["id"] == "verbose"
    end

    test "loads spec with mutually exclusive groups" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "flags" => [
            %{"id" => "flag-a", "short" => "a", "description" => "A", "type" => "boolean"},
            %{"id" => "flag-b", "short" => "b", "description" => "B", "type" => "boolean"}
          ],
          "mutually_exclusive_groups" => [
            %{"id" => "grp", "flag_ids" => ["flag-a", "flag-b"]}
          ]
        })

      spec = SpecLoader.load_from_string!(json)
      assert length(spec["mutually_exclusive_groups"]) == 1
      grp = hd(spec["mutually_exclusive_groups"])
      assert grp["flag_ids"] == ["flag-a", "flag-b"]
      assert grp["required"] == false
    end
  end

  # ---------------------------------------------------------------------------
  # Error cases
  # ---------------------------------------------------------------------------

  describe "spec validation errors" do
    test "raises on unsupported spec version" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "99.0",
          "name" => "prog",
          "description" => "test"
        })

      assert_raise SpecError, ~r/Unsupported cli_builder_spec_version/, fn ->
        SpecLoader.load_from_string!(json)
      end
    end

    test "raises when name is missing" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "1.0",
          "description" => "test"
        })

      assert_raise SpecError, ~r/Missing required field "name"/, fn ->
        SpecLoader.load_from_string!(json)
      end
    end

    test "raises when description is missing" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog"
        })

      assert_raise SpecError, ~r/Missing required field "description"/, fn ->
        SpecLoader.load_from_string!(json)
      end
    end

    test "raises on invalid parsing_mode" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "parsing_mode" => "cosmic"
        })

      assert_raise SpecError, ~r/Invalid parsing_mode/, fn ->
        SpecLoader.load_from_string!(json)
      end
    end

    test "raises on invalid JSON" do
      assert_raise SpecError, ~r/Invalid JSON/, fn ->
        SpecLoader.load_from_string!("not json {{{")
      end
    end

    test "raises when flag has no short, long, or single_dash_long" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "flags" => [
            %{"id" => "orphan", "description" => "No handle", "type" => "boolean"}
          ]
        })

      assert_raise SpecError, ~r/must have at least one of/, fn ->
        SpecLoader.load_from_string!(json)
      end
    end

    test "raises when enum flag has no enum_values" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "flags" => [
            %{
              "id" => "fmt",
              "long" => "format",
              "description" => "Output format",
              "type" => "enum"
            }
          ]
        })

      assert_raise SpecError, ~r/enum_values/, fn ->
        SpecLoader.load_from_string!(json)
      end
    end

    test "raises when more than one variadic argument in same scope" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "arguments" => [
            %{
              "id" => "a",
              "name" => "A",
              "description" => "First variadic",
              "type" => "string",
              "variadic" => true
            },
            %{
              "id" => "b",
              "name" => "B",
              "description" => "Second variadic",
              "type" => "string",
              "variadic" => true
            }
          ]
        })

      assert_raise SpecError, ~r/variadic/, fn ->
        SpecLoader.load_from_string!(json)
      end
    end

    test "raises on circular requires dependency" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "flags" => [
            %{
              "id" => "verbose",
              "short" => "v",
              "description" => "Verbose",
              "type" => "boolean",
              "requires" => ["quiet"]
            },
            %{
              "id" => "quiet",
              "short" => "q",
              "description" => "Quiet",
              "type" => "boolean",
              "requires" => ["verbose"]
            }
          ]
        })

      assert_raise SpecError, ~r/[Cc]ircular|cycle/i, fn ->
        SpecLoader.load_from_string!(json)
      end
    end

    test "raises on unknown conflicts_with reference" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "flags" => [
            %{
              "id" => "flag-a",
              "short" => "a",
              "description" => "A",
              "type" => "boolean",
              "conflicts_with" => ["nonexistent"]
            }
          ]
        })

      assert_raise SpecError, ~r/conflicts_with.*unknown/, fn ->
        SpecLoader.load_from_string!(json)
      end
    end

    test "raises on unknown requires reference" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "flags" => [
            %{
              "id" => "flag-a",
              "short" => "a",
              "description" => "A",
              "type" => "boolean",
              "requires" => ["ghost"]
            }
          ]
        })

      assert_raise SpecError, ~r/requires.*unknown/, fn ->
        SpecLoader.load_from_string!(json)
      end
    end

    test "raises when exclusive group references unknown flag" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "flags" => [
            %{"id" => "flag-a", "short" => "a", "description" => "A", "type" => "boolean"}
          ],
          "mutually_exclusive_groups" => [
            %{"id" => "grp", "flag_ids" => ["flag-a", "ghost"]}
          ]
        })

      assert_raise SpecError, ~r/unknown flag/i, fn ->
        SpecLoader.load_from_string!(json)
      end
    end

    test "raises when exclusive group has fewer than 2 flag_ids" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "flags" => [
            %{"id" => "flag-a", "short" => "a", "description" => "A", "type" => "boolean"}
          ],
          "mutually_exclusive_groups" => [
            %{"id" => "grp", "flag_ids" => ["flag-a"]}
          ]
        })

      assert_raise SpecError, ~r/at least 2/, fn ->
        SpecLoader.load_from_string!(json)
      end
    end

    test "raises on circular requires within a subcommand" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "commands" => [
            %{
              "id" => "cmd-sub",
              "name" => "sub",
              "description" => "Subcommand",
              "flags" => [
                %{
                  "id" => "a",
                  "short" => "a",
                  "description" => "A",
                  "type" => "boolean",
                  "requires" => ["b"]
                },
                %{
                  "id" => "b",
                  "short" => "b",
                  "description" => "B",
                  "type" => "boolean",
                  "requires" => ["a"]
                }
              ]
            }
          ]
        })

      assert_raise SpecError, ~r/[Cc]ircular|cycle/i, fn ->
        SpecLoader.load_from_string!(json)
      end
    end
  end

  # ---------------------------------------------------------------------------
  # File-loading error
  # ---------------------------------------------------------------------------

  describe "load!/1" do
    test "raises when file does not exist" do
      assert_raise SpecError, ~r/Cannot read spec file/, fn ->
        SpecLoader.load!("/nonexistent/path/to/spec.json")
      end
    end
  end
end
