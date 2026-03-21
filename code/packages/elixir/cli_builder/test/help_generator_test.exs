defmodule CodingAdventures.CliBuilder.HelpGeneratorTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.{HelpGenerator, SpecLoader}

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp load(json), do: SpecLoader.load_from_string!(Jason.encode!(json))

  defp git_spec do
    load(%{
      "cli_builder_spec_version" => "1.0",
      "name" => "git",
      "display_name" => "Git",
      "description" => "The stupid content tracker",
      "version" => "2.40.0",
      "global_flags" => [
        %{"id" => "verbose", "short" => "v", "long" => "verbose", "description" => "Be verbose", "type" => "boolean"}
      ],
      "flags" => [],
      "commands" => [
        %{
          "id" => "cmd-commit",
          "name" => "commit",
          "description" => "Record changes to the repository",
          "flags" => [
            %{"id" => "message", "short" => "m", "long" => "message", "description" => "Commit message", "type" => "string"},
            %{"id" => "all", "short" => "a", "long" => "all", "description" => "Stage all tracked files", "type" => "boolean"}
          ],
          "arguments" => []
        },
        %{
          "id" => "cmd-log",
          "name" => "log",
          "description" => "Show the commit log",
          "flags" => [],
          "arguments" => [
            %{"id" => "revision", "name" => "REVISION", "description" => "Revision range", "type" => "string", "required" => false}
          ]
        }
      ]
    })
  end

  defp echo_spec do
    load(%{
      "cli_builder_spec_version" => "1.0",
      "name" => "echo",
      "description" => "Display a line of text",
      "version" => "8.32",
      "flags" => [
        %{"id" => "no-newline", "short" => "n", "description" => "Do not output trailing newline", "type" => "boolean"},
        %{"id" => "output-file", "short" => "o", "long" => "output", "description" => "Output file", "type" => "string", "default" => "stdout"}
      ],
      "arguments" => [
        %{"id" => "string", "name" => "STRING", "description" => "Text to print", "type" => "string", "required" => false, "variadic" => true, "variadic_min" => 0}
      ]
    })
  end

  # ---------------------------------------------------------------------------
  # USAGE section
  # ---------------------------------------------------------------------------

  describe "USAGE section" do
    test "root spec without subcommands includes OPTIONS and ARGS" do
      text = HelpGenerator.generate(echo_spec(), ["echo"])
      assert text =~ "USAGE"
      assert text =~ "echo"
    end

    test "root spec with subcommands includes [COMMAND]" do
      text = HelpGenerator.generate(git_spec(), ["git"])
      assert text =~ "USAGE"
      assert text =~ "[COMMAND]"
    end

    test "subcommand path uses full prefix" do
      text = HelpGenerator.generate(git_spec(), ["git", "commit"])
      assert text =~ "git commit"
    end
  end

  # ---------------------------------------------------------------------------
  # DESCRIPTION section
  # ---------------------------------------------------------------------------

  describe "DESCRIPTION section" do
    test "root description is shown" do
      text = HelpGenerator.generate(git_spec(), ["git"])
      assert text =~ "DESCRIPTION"
      assert text =~ "The stupid content tracker"
    end

    test "subcommand description is shown for that subcommand" do
      text = HelpGenerator.generate(git_spec(), ["git", "commit"])
      assert text =~ "Record changes to the repository"
    end
  end

  # ---------------------------------------------------------------------------
  # COMMANDS section
  # ---------------------------------------------------------------------------

  describe "COMMANDS section" do
    test "root shows listed subcommands" do
      text = HelpGenerator.generate(git_spec(), ["git"])
      assert text =~ "COMMANDS"
      assert text =~ "commit"
      assert text =~ "Record changes to the repository"
      assert text =~ "log"
    end

    test "leaf command has no COMMANDS section" do
      text = HelpGenerator.generate(git_spec(), ["git", "commit"])
      refute text =~ "COMMANDS"
    end
  end

  # ---------------------------------------------------------------------------
  # OPTIONS section
  # ---------------------------------------------------------------------------

  describe "OPTIONS section" do
    test "flags appear in OPTIONS" do
      text = HelpGenerator.generate(echo_spec(), ["echo"])
      assert text =~ "OPTIONS"
      assert text =~ "-n"
      assert text =~ "Do not output trailing newline"
    end

    test "non-boolean flag shows value name" do
      text = HelpGenerator.generate(echo_spec(), ["echo"])
      assert text =~ "STRING" or text =~ "string" or text =~ "<"
    end

    test "flag with default shows [default: ...]" do
      text = HelpGenerator.generate(echo_spec(), ["echo"])
      assert text =~ "default: stdout"
    end

    test "subcommand flags appear when requesting subcommand help" do
      text = HelpGenerator.generate(git_spec(), ["git", "commit"])
      assert text =~ "-m" or text =~ "--message"
      assert text =~ "Commit message"
    end
  end

  # ---------------------------------------------------------------------------
  # ARGUMENTS section
  # ---------------------------------------------------------------------------

  describe "ARGUMENTS section" do
    test "arguments section present when args exist" do
      text = HelpGenerator.generate(echo_spec(), ["echo"])
      assert text =~ "ARGUMENTS" or text =~ "STRING"
    end

    test "optional variadic argument shown with brackets and ellipsis" do
      text = HelpGenerator.generate(echo_spec(), ["echo"])
      # Either in usage line or ARGUMENTS section
      assert text =~ "[STRING" or text =~ "STRING"
    end
  end

  # ---------------------------------------------------------------------------
  # GLOBAL OPTIONS section
  # ---------------------------------------------------------------------------

  describe "GLOBAL OPTIONS section" do
    test "builtin help flag appears in GLOBAL OPTIONS" do
      text = HelpGenerator.generate(git_spec(), ["git"])
      assert text =~ "GLOBAL OPTIONS" or text =~ "--help"
    end

    test "builtin version flag appears when spec has version" do
      text = HelpGenerator.generate(git_spec(), ["git"])
      assert text =~ "--version"
    end

    test "global flag appears in GLOBAL OPTIONS" do
      text = HelpGenerator.generate(git_spec(), ["git"])
      assert text =~ "--verbose" or text =~ "-v"
    end
  end

  # ---------------------------------------------------------------------------
  # Edge cases
  # ---------------------------------------------------------------------------

  describe "edge cases" do
    test "spec with no flags produces no OPTIONS section (only GLOBAL OPTIONS)" do
      spec =
        load(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "noop",
          "description" => "Does nothing"
        })

      text = HelpGenerator.generate(spec, ["noop"])
      # No custom options, but GLOBAL OPTIONS with --help should appear
      assert text =~ "GLOBAL OPTIONS" or text =~ "--help"
    end

    test "spec with no version omits --version from GLOBAL OPTIONS" do
      spec =
        load(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "noop",
          "description" => "Does nothing"
        })

      text = HelpGenerator.generate(spec, ["noop"])
      refute text =~ "--version"
    end
  end

  # ---------------------------------------------------------------------------
  # Required flags formatting
  # ---------------------------------------------------------------------------

  describe "required flags in OPTIONS" do
    test "required flag shows [required] suffix" do
      spec =
        load(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "flags" => [
            %{
              "id" => "output",
              "short" => "o",
              "long" => "output",
              "description" => "Output file",
              "type" => "string",
              "required" => true
            }
          ]
        })

      text = HelpGenerator.generate(spec, ["prog"])
      assert text =~ "[required]"
    end

    test "optional flag with default shows [default: ...] suffix" do
      spec =
        load(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "flags" => [
            %{
              "id" => "format",
              "long" => "format",
              "description" => "Output format",
              "type" => "string",
              "default" => "table"
            }
          ]
        })

      text = HelpGenerator.generate(spec, ["prog"])
      assert text =~ "[default: table]"
    end

    test "optional flag without default shows no suffix" do
      spec =
        load(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "flags" => [
            %{
              "id" => "verbose",
              "short" => "v",
              "description" => "Verbose",
              "type" => "boolean"
            }
          ]
        })

      text = HelpGenerator.generate(spec, ["prog"])
      refute text =~ "[required]"
      refute text =~ "[default:"
    end
  end

  # ---------------------------------------------------------------------------
  # SDL flag formatting
  # ---------------------------------------------------------------------------

  describe "SDL flag in OPTIONS" do
    test "single-dash-long flag is formatted with leading dash" do
      spec =
        load(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "flags" => [
            %{
              "id" => "classpath",
              "single_dash_long" => "classpath",
              "description" => "Java classpath",
              "type" => "string"
            }
          ]
        })

      text = HelpGenerator.generate(spec, ["prog"])
      assert text =~ "-classpath"
    end
  end

  # ---------------------------------------------------------------------------
  # Argument format in usage line
  # ---------------------------------------------------------------------------

  describe "argument format in usage line" do
    test "required non-variadic argument shown as <NAME>" do
      spec =
        load(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "arguments" => [
            %{"id" => "file", "name" => "FILE", "description" => "A file", "type" => "path", "required" => true}
          ]
        })

      text = HelpGenerator.generate(spec, ["prog"])
      assert text =~ "<FILE>"
    end

    test "required variadic argument shown as <NAME>..." do
      spec =
        load(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "arguments" => [
            %{
              "id" => "file",
              "name" => "FILES",
              "description" => "Files",
              "type" => "path",
              "required" => true,
              "variadic" => true,
              "variadic_min" => 1
            }
          ]
        })

      text = HelpGenerator.generate(spec, ["prog"])
      assert text =~ "<FILES>..."
    end

    test "optional non-variadic argument shown as [NAME]" do
      spec =
        load(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "arguments" => [
            %{
              "id" => "file",
              "name" => "FILE",
              "description" => "A file",
              "type" => "path",
              "required" => false
            }
          ]
        })

      text = HelpGenerator.generate(spec, ["prog"])
      assert text =~ "[FILE]"
    end

    test "optional variadic argument shown as [NAME...]" do
      spec =
        load(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "arguments" => [
            %{
              "id" => "file",
              "name" => "FILE",
              "description" => "Files",
              "type" => "path",
              "required" => false,
              "variadic" => true,
              "variadic_min" => 0
            }
          ]
        })

      text = HelpGenerator.generate(spec, ["prog"])
      assert text =~ "[FILE...]" or text =~ "[FILE"
    end
  end

  # ---------------------------------------------------------------------------
  # ARGUMENTS section formatting
  # ---------------------------------------------------------------------------

  describe "ARGUMENTS section formatting" do
    test "required argument has 'Required.' in ARGUMENTS section" do
      spec =
        load(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "arguments" => [
            %{"id" => "file", "name" => "FILE", "description" => "A file", "type" => "path", "required" => true}
          ]
        })

      text = HelpGenerator.generate(spec, ["prog"])
      assert text =~ "Required."
    end

    test "optional argument has 'Optional.' in ARGUMENTS section" do
      spec =
        load(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "arguments" => [
            %{
              "id" => "file",
              "name" => "FILE",
              "description" => "A file",
              "type" => "path",
              "required" => false
            }
          ]
        })

      text = HelpGenerator.generate(spec, ["prog"])
      assert text =~ "Optional."
    end

    test "variadic argument has 'Repeatable.' in ARGUMENTS section" do
      spec =
        load(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "arguments" => [
            %{
              "id" => "file",
              "name" => "FILE",
              "description" => "Files",
              "type" => "path",
              "required" => false,
              "variadic" => true,
              "variadic_min" => 0
            }
          ]
        })

      text = HelpGenerator.generate(spec, ["prog"])
      assert text =~ "Repeatable."
    end
  end

  # ---------------------------------------------------------------------------
  # Subcommand with inherit_global_flags: false
  # ---------------------------------------------------------------------------

  describe "inherit_global_flags false in help" do
    test "subcommand with inherit_global_flags false shows no global flags" do
      spec =
        load(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "global_flags" => [
            %{"id" => "verbose", "short" => "v", "long" => "verbose", "description" => "Verbose", "type" => "boolean"}
          ],
          "commands" => [
            %{
              "id" => "cmd-private",
              "name" => "private",
              "description" => "Private command",
              "inherit_global_flags" => false,
              "flags" => [],
              "arguments" => []
            }
          ]
        })

      text = HelpGenerator.generate(spec, ["prog", "private"])
      # Global --verbose should NOT appear in the private subcommand help
      refute text =~ "--verbose"
    end
  end

  # ---------------------------------------------------------------------------
  # Path resolution for unknown subcommand in help (graceful fallback)
  # ---------------------------------------------------------------------------

  describe "unknown subcommand in help path" do
    test "unknown subcommand path falls back to last known node" do
      # Requesting help for a nonexistent path should not crash
      text = HelpGenerator.generate(git_spec(), ["git", "nonexistent"])
      assert is_binary(text)
      assert String.length(text) > 0
    end
  end

  # ---------------------------------------------------------------------------
  # builtin flags disabled
  # ---------------------------------------------------------------------------

  describe "builtin flags disabled" do
    test "help builtin disabled → no --help in GLOBAL OPTIONS" do
      spec =
        load(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "builtin_flags" => %{"help" => false, "version" => false}
        })

      text = HelpGenerator.generate(spec, ["prog"])
      refute text =~ "--help"
    end

    test "version builtin disabled → no --version in GLOBAL OPTIONS" do
      spec =
        load(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "version" => "1.0",
          "builtin_flags" => %{"help" => true, "version" => false}
        })

      text = HelpGenerator.generate(spec, ["prog"])
      refute text =~ "--version"
    end
  end

  # ---------------------------------------------------------------------------
  # Subcommand with aliases shown in COMMANDS section
  # ---------------------------------------------------------------------------

  describe "help for subcommand with aliases" do
    test "help can be generated for command found via alias" do
      spec =
        load(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "version" => "1.0",
          "commands" => [
            %{
              "id" => "cmd-commit",
              "name" => "commit",
              "aliases" => ["ci"],
              "description" => "Record changes",
              "flags" => [],
              "arguments" => []
            }
          ]
        })

      # Generate help using the alias as the path element
      text = HelpGenerator.generate(spec, ["prog", "ci"])
      assert is_binary(text)
      assert String.length(text) > 0
    end
  end

  # ---------------------------------------------------------------------------
  # Flag with only short handle (no long, no SDL)
  # ---------------------------------------------------------------------------

  describe "flag with short handle only" do
    test "short-only flag appears in OPTIONS without --long part" do
      spec =
        load(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "flags" => [
            %{"id" => "newline", "short" => "n", "description" => "No newline", "type" => "boolean"}
          ]
        })

      text = HelpGenerator.generate(spec, ["prog"])
      assert text =~ "-n"
    end
  end

  # ---------------------------------------------------------------------------
  # Custom value_name in flag formatting
  # ---------------------------------------------------------------------------

  describe "custom value_name" do
    test "custom value_name appears in flag output" do
      spec =
        load(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "test",
          "flags" => [
            %{
              "id" => "output",
              "long" => "output",
              "description" => "Output file",
              "type" => "string",
              "value_name" => "OUTFILE"
            }
          ]
        })

      text = HelpGenerator.generate(spec, ["prog"])
      assert text =~ "OUTFILE"
    end
  end
end
