# frozen_string_literal: true

# ---------------------------------------------------------------------------
# types.rb — Result types returned by the CLI Builder parser
# ---------------------------------------------------------------------------
#
# The CLI Builder parser can return one of three result types depending on
# what the user typed:
#
#   ParseResult   — normal case: the user typed a valid invocation
#   HelpResult    — the user passed --help or -h
#   VersionResult — the user passed --version
#
# These are distinct value objects (Structs) rather than a single type with
# a "kind" field. Pattern matching on the result type is idiomatic Ruby:
#
#   result = parser.parse
#   case result
#   when CodingAdventures::CliBuilder::ParseResult
#     # Handle normal invocation
#     puts result.flags["verbose"]
#   when CodingAdventures::CliBuilder::HelpResult
#     puts result.text
#     exit 0
#   when CodingAdventures::CliBuilder::VersionResult
#     puts result.version
#     exit 0
#   end
#
# === Why Structs? ===
#
# Ruby Structs give us immutable value objects with named fields, equality
# based on field values (not object identity), and a readable #inspect.
# They are the right tool for "plain data containers" that do not need
# behaviour — the result types here carry data only; the logic lives in
# the Parser class.
# ---------------------------------------------------------------------------

module CodingAdventures
  module CliBuilder
    # Result of a successful parse.
    #
    # Returned when the user typed a valid invocation that is not a help
    # or version request. All flags in scope are present in the flags hash
    # (absent optional booleans are false, other absent optionals are nil
    # or their configured default).
    #
    # Fields:
    #   program      — the program name as invoked (argv[0])
    #   command_path — full path from root to resolved command, e.g. ["git","remote","add"]
    #   flags        — hash from flag id to parsed/coerced value
    #   arguments    — hash from argument id to parsed/coerced value (variadic args → arrays)
    #
    # Example:
    #   ParseResult.new(
    #     program:        "git",
    #     command_path:   ["git", "remote", "add"],
    #     flags:          { "verbose" => false, "dry-run" => false },
    #     arguments:      { "name" => "origin", "url" => "https://example.com" },
    #     explicit_flags: ["verbose"]
    #   )
    #
    # === explicit_flags (v1.1) ===
    #
    # The explicit_flags field tracks which flags the user explicitly typed
    # on the command line, as opposed to flags whose values come from defaults.
    # This enables tools to distinguish "the user chose this value" from
    # "the parser filled in the default." Common use cases:
    #
    #   - Config file merging: CLI flags override config only if explicitly set
    #   - Warnings: "you didn't specify --format, defaulting to json"
    #   - Conditional behavior based on whether the user made an active choice
    #
    # The list contains flag IDs (strings). Each ID appears at most once,
    # even if the flag was repeated. Built-in flags (--help, --version) are
    # never included because they trigger special result types, not ParseResult.
    ParseResult = Struct.new(:program, :command_path, :flags, :arguments, :explicit_flags, keyword_init: true)

    # Result returned when the user requested help.
    #
    # Triggered by --help or -h at any point in the invocation. The text
    # field contains the fully rendered help message for the deepest
    # resolved command context. The caller should print this and exit 0.
    #
    # Fields:
    #   text         — the formatted help string (see spec §9 for format)
    #   command_path — the command context for which help was generated
    #
    # Example:
    #   HelpResult.new(
    #     text:         "USAGE\n  git remote [OPTIONS] COMMAND\n\n...",
    #     command_path: ["git", "remote"]
    #   )
    HelpResult = Struct.new(:text, :command_path, keyword_init: true)

    # Result returned when the user requested the version string.
    #
    # Triggered by --version. Contains the version string from the spec's
    # top-level "version" field. The caller should print this and exit 0.
    #
    # Fields:
    #   version — the version string (e.g. "2.39.0")
    #
    # Example:
    #   VersionResult.new(version: "2.39.0")
    VersionResult = Struct.new(:version, keyword_init: true)
  end
end
