defmodule CodingAdventures.CliBuilder.ValidatorTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Validator

  # ---------------------------------------------------------------------------
  # Helpers: spec JSON builders
  # ---------------------------------------------------------------------------

  # A minimal valid spec — the smallest document that passes all checks.
  defp minimal_valid_spec do
    Jason.encode!(%{
      "cli_builder_spec_version" => "1.0",
      "name" => "prog",
      "description" => "A minimal program"
    })
  end

  # A richer valid spec with a flag and an argument.
  defp full_valid_spec do
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
          "id" => "text",
          "display_name" => "TEXT",
          "description" => "Text to print",
          "type" => "string",
          "required" => false,
          "variadic" => true,
          "variadic_min" => 0
        }
      ]
    })
  end

  # ---------------------------------------------------------------------------
  # validate_spec_string/1 — happy path
  # ---------------------------------------------------------------------------

  describe "validate_spec_string/1 with valid specs" do
    test "minimal valid spec returns valid: true" do
      result = Validator.validate_spec_string(minimal_valid_spec())

      assert result.valid == true
      assert result.errors == []
    end

    test "full valid spec with flags and arguments returns valid: true" do
      result = Validator.validate_spec_string(full_valid_spec())

      assert result.valid == true
      assert result.errors == []
    end
  end

  # ---------------------------------------------------------------------------
  # validate_spec_string/1 — invalid JSON
  # ---------------------------------------------------------------------------

  describe "validate_spec_string/1 with invalid JSON" do
    test "garbage string returns valid: false with JSON parse error" do
      result = Validator.validate_spec_string("this is not json")

      assert result.valid == false
      assert length(result.errors) == 1
      assert hd(result.errors) =~ "Invalid JSON"
    end

    test "empty string returns valid: false" do
      result = Validator.validate_spec_string("")

      assert result.valid == false
      assert length(result.errors) == 1
      assert hd(result.errors) =~ "Invalid JSON"
    end

    test "truncated JSON returns valid: false" do
      result = Validator.validate_spec_string(~s({"name": "hello"))

      assert result.valid == false
      assert length(result.errors) == 1
    end
  end

  # ---------------------------------------------------------------------------
  # validate_spec_string/1 — missing version
  # ---------------------------------------------------------------------------

  describe "validate_spec_string/1 with missing version" do
    test "spec without cli_builder_spec_version is invalid" do
      json =
        Jason.encode!(%{
          "name" => "prog",
          "description" => "A program"
        })

      result = Validator.validate_spec_string(json)

      assert result.valid == false
      assert hd(result.errors) =~ "Unsupported cli_builder_spec_version"
    end
  end

  # ---------------------------------------------------------------------------
  # validate_spec_string/1 — unsupported version
  # ---------------------------------------------------------------------------

  describe "validate_spec_string/1 with unsupported version" do
    test "spec with version 99.0 is invalid" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "99.0",
          "name" => "prog",
          "description" => "A program"
        })

      result = Validator.validate_spec_string(json)

      assert result.valid == false
      assert hd(result.errors) =~ "Unsupported cli_builder_spec_version"
      assert hd(result.errors) =~ "99.0"
    end
  end

  # ---------------------------------------------------------------------------
  # validate_spec_string/1 — missing required fields
  # ---------------------------------------------------------------------------

  describe "validate_spec_string/1 with missing required fields" do
    test "missing name field" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "1.0",
          "description" => "A program"
        })

      result = Validator.validate_spec_string(json)

      assert result.valid == false
      assert hd(result.errors) =~ "Missing required field"
      assert hd(result.errors) =~ "name"
    end

    test "missing description field" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog"
        })

      result = Validator.validate_spec_string(json)

      assert result.valid == false
      assert hd(result.errors) =~ "Missing required field"
      assert hd(result.errors) =~ "description"
    end
  end

  # ---------------------------------------------------------------------------
  # validate_spec_string/1 — flag with no short/long
  # ---------------------------------------------------------------------------

  describe "validate_spec_string/1 with flag missing short/long" do
    test "flag with neither short, long, nor single_dash_long is invalid" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "A program",
          "flags" => [
            %{
              "id" => "orphan-flag",
              "description" => "A flag with no handle",
              "type" => "boolean"
            }
          ]
        })

      result = Validator.validate_spec_string(json)

      assert result.valid == false
      assert hd(result.errors) =~ "must have at least one of"
      assert hd(result.errors) =~ "orphan-flag"
    end
  end

  # ---------------------------------------------------------------------------
  # validate_spec/1 — file-based validation
  # ---------------------------------------------------------------------------

  describe "validate_spec/1 with a nonexistent file" do
    test "returns valid: false with file-read error" do
      result = Validator.validate_spec("/tmp/does_not_exist_cli_builder_spec.json")

      assert result.valid == false
      assert length(result.errors) == 1
      assert hd(result.errors) =~ "Cannot read spec file"
    end
  end

  describe "validate_spec/1 with a valid file" do
    # Write a temp file, validate it, then clean up.
    @tag :tmp_dir
    test "valid spec file returns valid: true", %{tmp_dir: tmp_dir} do
      path = Path.join(tmp_dir, "valid.json")
      File.write!(path, minimal_valid_spec())

      result = Validator.validate_spec(path)

      assert result.valid == true
      assert result.errors == []
    end
  end

  describe "validate_spec/1 with an invalid file" do
    @tag :tmp_dir
    test "invalid spec file returns valid: false with error details", %{tmp_dir: tmp_dir} do
      path = Path.join(tmp_dir, "bad.json")
      # Valid JSON but missing required fields
      File.write!(path, ~s({"cli_builder_spec_version": "1.0"}))

      result = Validator.validate_spec(path)

      assert result.valid == false
      assert hd(result.errors) =~ "Missing required field"
    end
  end

  # ---------------------------------------------------------------------------
  # validate_spec_string/1 — additional structural errors
  # ---------------------------------------------------------------------------

  describe "validate_spec_string/1 with invalid parsing_mode" do
    test "unrecognised parsing mode is rejected" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "A program",
          "parsing_mode" => "yolo"
        })

      result = Validator.validate_spec_string(json)

      assert result.valid == false
      assert hd(result.errors) =~ "Invalid parsing_mode"
    end
  end

  describe "validate_spec_string/1 with flag missing type" do
    test "flag without type field is rejected" do
      json =
        Jason.encode!(%{
          "cli_builder_spec_version" => "1.0",
          "name" => "prog",
          "description" => "A program",
          "flags" => [
            %{
              "id" => "verbose",
              "long" => "verbose",
              "description" => "Be verbose"
              # type is missing
            }
          ]
        })

      result = Validator.validate_spec_string(json)

      assert result.valid == false
      assert hd(result.errors) =~ "Missing required field"
      assert hd(result.errors) =~ "type"
    end
  end
end
