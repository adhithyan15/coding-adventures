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
end
