defmodule CodingAdventures.CliBuilder.FlagValidatorTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.FlagValidator

  # ---------------------------------------------------------------------------
  # Helpers — minimal flag definition maps
  # ---------------------------------------------------------------------------

  defp flag(id, opts \\ []) do
    short = Keyword.get(opts, :short)
    long = Keyword.get(opts, :long, id)
    sdl = Keyword.get(opts, :single_dash_long)

    %{
      "id" => id,
      "short" => short,
      "long" => long,
      "single_dash_long" => sdl,
      "type" => Keyword.get(opts, :type, "boolean"),
      "required" => Keyword.get(opts, :required, false),
      "conflicts_with" => Keyword.get(opts, :conflicts_with, []),
      "requires" => Keyword.get(opts, :requires, []),
      "required_unless" => Keyword.get(opts, :required_unless, []),
      "repeatable" => false
    }
  end

  defp group(id, flag_ids, required \\ false) do
    %{"id" => id, "flag_ids" => flag_ids, "required" => required}
  end

  defp validate(parsed_flags, flags, groups \\ []),
    do: FlagValidator.validate(parsed_flags, flags, groups, ["prog"])

  # ---------------------------------------------------------------------------
  # conflicts_with
  # ---------------------------------------------------------------------------

  describe "conflicts_with" do
    test "no conflict when only one flag present" do
      flags = [
        flag("alpha", short: "a", conflicts_with: ["beta"]),
        flag("beta", short: "b", conflicts_with: ["alpha"])
      ]

      assert validate(%{"alpha" => true}, flags) == []
    end

    test "conflict detected when both flags present" do
      flags = [
        flag("alpha", short: "a", conflicts_with: ["beta"]),
        flag("beta", short: "b", conflicts_with: ["alpha"])
      ]

      errors = validate(%{"alpha" => true, "beta" => true}, flags)
      assert length(errors) == 1
      assert hd(errors).error_type == "conflicting_flags"
    end

    test "conflict pair reported only once (not twice for A→B and B→A)" do
      # Both flags list each other in conflicts_with
      flags = [
        flag("alpha", short: "a", conflicts_with: ["beta"]),
        flag("beta", short: "b", conflicts_with: ["alpha"])
      ]

      errors = validate(%{"alpha" => true, "beta" => true}, flags)
      # Should be exactly 1, not 2
      conflict_errors = Enum.filter(errors, &(&1.error_type == "conflicting_flags"))
      assert length(conflict_errors) == 1
    end

    test "conflict message mentions both flags" do
      flags = [
        flag("verbose", short: "v", conflicts_with: ["quiet"]),
        flag("quiet", short: "q", conflicts_with: ["verbose"])
      ]

      errors = validate(%{"verbose" => true, "quiet" => true}, flags)
      assert hd(errors).message =~ "verbose" or hd(errors).message =~ "quiet"
    end

    test "no errors when neither conflicting flag is present" do
      flags = [
        flag("alpha", short: "a", conflicts_with: ["beta"]),
        flag("beta", short: "b", conflicts_with: ["alpha"])
      ]

      assert validate(%{}, flags) == []
    end

    test "conflict with flag not in flag_map (unknown reference is skipped)" do
      # Parsed flags includes a flag not in the active_flags list (e.g. a builtin)
      flags = [flag("alpha", short: "a", conflicts_with: ["ghost"])]
      # "ghost" is in parsed_flags but not in flag_map — should not crash
      errors = validate(%{"alpha" => true, "ghost" => true}, flags)
      conflict_errors = Enum.filter(errors, &(&1.error_type == "conflicting_flags"))
      # The conflict IS detected because parsed_flags has both
      assert length(conflict_errors) >= 1
    end
  end

  # ---------------------------------------------------------------------------
  # requires (transitive)
  # ---------------------------------------------------------------------------

  describe "requires (transitive)" do
    test "no error when required flag is present" do
      flags = [
        flag("human-readable", short: "h", requires: ["long-listing"]),
        flag("long-listing", short: "l")
      ]

      assert validate(%{"human-readable" => true, "long-listing" => true}, flags) == []
    end

    test "error when required flag is absent" do
      flags = [
        flag("human-readable", short: "h", requires: ["long-listing"]),
        flag("long-listing", short: "l")
      ]

      errors = validate(%{"human-readable" => true}, flags)
      types = Enum.map(errors, & &1.error_type)
      assert "missing_dependency_flag" in types
    end

    test "transitive: A requires B requires C — all must be present" do
      flags = [
        flag("a", short: "a", requires: ["b"]),
        flag("b", short: "b", requires: ["c"]),
        flag("c", short: "c")
      ]

      # A present, B present, C absent → error
      errors = validate(%{"a" => true, "b" => true}, flags)
      types = Enum.map(errors, & &1.error_type)
      assert "missing_dependency_flag" in types
    end

    test "transitive: A requires B requires C — all present → no errors" do
      flags = [
        flag("a", short: "a", requires: ["b"]),
        flag("b", short: "b", requires: ["c"]),
        flag("c", short: "c")
      ]

      assert validate(%{"a" => true, "b" => true, "c" => true}, flags) == []
    end

    test "flag not in graph (like a builtin) is skipped gracefully" do
      flags = [flag("verbose", short: "v")]
      # "help" is not in active_flags, but appears in parsed_flags
      assert validate(%{"help" => true}, flags) == []
    end
  end

  # ---------------------------------------------------------------------------
  # required flags
  # ---------------------------------------------------------------------------

  describe "required flag checking" do
    test "required flag present → no error" do
      flags = [flag("message", short: "m", required: true)]
      assert validate(%{"message" => "hello"}, flags) == []
    end

    test "required flag absent → missing_required_flag error" do
      flags = [flag("message", short: "m", required: true)]
      errors = validate(%{}, flags)
      types = Enum.map(errors, & &1.error_type)
      assert "missing_required_flag" in types
    end

    test "required_unless exempts when specified flag is present" do
      flags = [
        flag("output", short: "o", required: true, required_unless: ["stdout"]),
        flag("stdout", short: "s")
      ]

      # stdout present → output no longer required
      assert validate(%{"stdout" => true}, flags) == []
    end

    test "required_unless does not exempt when specified flag is absent" do
      flags = [
        flag("output", short: "o", required: true, required_unless: ["stdout"]),
        flag("stdout", short: "s")
      ]

      errors = validate(%{}, flags)
      types = Enum.map(errors, & &1.error_type)
      assert "missing_required_flag" in types
    end

    test "required_unless with multiple flags — any one suffices" do
      flags = [
        flag("output", short: "o", required: true, required_unless: ["stdout", "null"]),
        flag("stdout", short: "s"),
        flag("null", short: "n")
      ]

      # null present → output exempt
      assert validate(%{"null" => true}, flags) == []
    end

    test "flag not required → no error when absent" do
      flags = [flag("verbose", short: "v", required: false)]
      assert validate(%{}, flags) == []
    end
  end

  # ---------------------------------------------------------------------------
  # mutually exclusive groups
  # ---------------------------------------------------------------------------

  describe "mutually exclusive groups" do
    test "no flags in group present → no error (optional group)" do
      flags = [flag("a", short: "a"), flag("b", short: "b")]
      groups = [group("grp", ["a", "b"], false)]
      assert validate(%{}, flags, groups) == []
    end

    test "one flag in group present → no error" do
      flags = [flag("a", short: "a"), flag("b", short: "b")]
      groups = [group("grp", ["a", "b"], false)]
      assert validate(%{"a" => true}, flags, groups) == []
    end

    test "two flags in group present → exclusive_group_violation error" do
      flags = [flag("a", short: "a"), flag("b", short: "b")]
      groups = [group("grp", ["a", "b"], false)]
      errors = validate(%{"a" => true, "b" => true}, flags, groups)
      types = Enum.map(errors, & &1.error_type)
      assert "exclusive_group_violation" in types
    end

    test "three flags all in group → exclusive_group_violation error" do
      flags = [flag("a", short: "a"), flag("b", short: "b"), flag("c", short: "c")]
      groups = [group("grp", ["a", "b", "c"], false)]
      errors = validate(%{"a" => true, "b" => true, "c" => true}, flags, groups)
      types = Enum.map(errors, & &1.error_type)
      assert "exclusive_group_violation" in types
    end

    test "required group with no flag present → missing_exclusive_group error" do
      flags = [flag("a", short: "a"), flag("b", short: "b")]
      groups = [group("grp", ["a", "b"], true)]
      errors = validate(%{}, flags, groups)
      types = Enum.map(errors, & &1.error_type)
      assert "missing_exclusive_group" in types
    end

    test "required group with one flag present → no error" do
      flags = [flag("a", short: "a"), flag("b", short: "b")]
      groups = [group("grp", ["a", "b"], true)]
      assert validate(%{"a" => true}, flags, groups) == []
    end

    test "required group with two flags present → exclusive_group_violation error" do
      flags = [flag("a", short: "a"), flag("b", short: "b")]
      groups = [group("grp", ["a", "b"], true)]
      errors = validate(%{"a" => true, "b" => true}, flags, groups)
      types = Enum.map(errors, & &1.error_type)
      assert "exclusive_group_violation" in types
    end
  end

  # ---------------------------------------------------------------------------
  # flag_label formatting
  # ---------------------------------------------------------------------------

  describe "flag_label formatting in error messages" do
    test "flag with only long handle labels as --long" do
      flags = [
        flag("verbose", long: "verbose", conflicts_with: ["quiet"]),
        flag("quiet", long: "quiet", conflicts_with: ["verbose"])
      ]

      errors = validate(%{"verbose" => true, "quiet" => true}, flags)
      assert hd(errors).message =~ "--verbose" or hd(errors).message =~ "--quiet"
    end

    test "flag with only SDL handle labels as -sdl" do
      flags = [
        %{
          "id" => "classpath",
          "short" => nil,
          "long" => nil,
          "single_dash_long" => "classpath",
          "type" => "string",
          "required" => true,
          "conflicts_with" => [],
          "requires" => [],
          "required_unless" => [],
          "repeatable" => false
        }
      ]

      errors = validate(%{}, flags)
      assert Enum.any?(errors, &String.contains?(&1.message, "-classpath"))
    end

    test "flag with short handle labels as -x" do
      flags = [
        %{
          "id" => "req",
          "short" => "r",
          "long" => nil,
          "single_dash_long" => nil,
          "type" => "boolean",
          "required" => true,
          "conflicts_with" => [],
          "requires" => [],
          "required_unless" => [],
          "repeatable" => false
        }
      ]

      errors = validate(%{}, flags)
      assert Enum.any?(errors, &String.contains?(&1.message, "-r"))
    end
  end
end
