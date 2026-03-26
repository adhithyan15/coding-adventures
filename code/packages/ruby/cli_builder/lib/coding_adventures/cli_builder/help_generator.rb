# frozen_string_literal: true

# ---------------------------------------------------------------------------
# help_generator.rb — Auto-generate help text from a CLI spec
# ---------------------------------------------------------------------------
#
# CLI Builder auto-generates help text from the spec so that it is always
# in sync with the actual structure. Developers never write help text by
# hand — they write descriptions in the spec, and the generator assembles
# them into a formatted, consistent help message.
#
# === Format (spec §9) ===
#
# The generated help follows this structure:
#
#   USAGE
#     <name> [OPTIONS] [COMMAND] [ARGS...]
#
#   DESCRIPTION
#     <description>
#
#   COMMANDS
#     subcommand    Description of the subcommand.
#
#   OPTIONS
#     -s, --long-name <VALUE>    Description. [default: val]
#     -b, --boolean              Boolean flag.
#
#   GLOBAL OPTIONS
#     -h, --help     Show this help message and exit.
#     --version      Show version and exit.
#
#   ARGUMENTS
#     <ARG>      Description. Required.
#     [ARG...]   Description. Optional, repeatable.
#
# Sections are only included if they have content. For example, a command
# with no subcommands omits the COMMANDS section entirely.
#
# === Formatting rules ===
#
# - Required positional args: <NAME>
# - Optional positional args: [NAME]
# - Variadic required:        <NAME>...
# - Variadic optional:        [NAME...]
# - Non-boolean flags: "-s, --long <VALUE>" where VALUE is value_name or the
#   type name uppercased
# - Boolean flags: "-s, --long"
# - single_dash_long flags: "-classpath <VALUE>"
# - Defaults: "[default: X]" appended when set and required is false
# ---------------------------------------------------------------------------

module CodingAdventures
  module CliBuilder
    # Generates a formatted help string for a specific command context.
    class HelpGenerator
      # Create a generator for the given spec and command path.
      #
      # @param spec [Hash] The full normalized spec hash.
      # @param command_path [Array<String>] The path to the resolved command, e.g. ["git", "remote"].
      def initialize(spec, command_path)
        @spec = spec
        @command_path = command_path
        @resolved = resolve_command(spec, command_path)
      end

      # Generate the help string.
      #
      # @return [String] The formatted help text.
      def generate
        sections = []

        sections << usage_section
        sections << description_section
        sections << commands_section
        sections << options_section
        sections << global_options_section
        sections << arguments_section

        sections.compact.join("\n\n") + "\n"
      end

      private

      # ---------------------------------------------------------------------------
      # Command resolution
      # ---------------------------------------------------------------------------
      #
      # Walk the spec's command tree along the command_path to find the deepest
      # command node. For the root path (just the program name), return the root spec.

      def resolve_command(spec, command_path)
        # command_path[0] is the program name, [1..] are subcommands
        current = spec
        command_path[1..].each do |cmd_name|
          commands = current["commands"] || []
          found = commands.find do |c|
            c["name"] == cmd_name || (c["aliases"] || []).include?(cmd_name)
          end
          current = found if found
        end
        current
      end

      # ---------------------------------------------------------------------------
      # USAGE section
      # ---------------------------------------------------------------------------

      def usage_section
        name = @command_path.join(" ")
        parts = [name]

        # Flags section indicator
        all_flags = (@resolved["flags"] || []) + (@spec["global_flags"] || [])
        parts << "[OPTIONS]" unless all_flags.empty?

        # Subcommands indicator
        cmds = @resolved["commands"] || []
        parts << "COMMAND" unless cmds.empty?

        # Arguments
        args = @resolved["arguments"] || []
        args.each do |a|
          parts << format_arg_usage(a)
        end

        "USAGE\n  #{parts.join(" ")}"
      end

      def format_arg_usage(arg)
        # Prefer display_name, fall back to name for backward compatibility.
        name = arg["display_name"] || arg["name"]
        required = arg["required"] != false
        variadic = arg["variadic"]

        if required
          variadic ? "<#{name}>..." : "<#{name}>"
        else
          variadic ? "[#{name}...]" : "[#{name}]"
        end
      end

      # ---------------------------------------------------------------------------
      # DESCRIPTION section
      # ---------------------------------------------------------------------------

      def description_section
        desc = @resolved["description"] || @spec["description"]
        return nil unless desc && !desc.empty?
        "DESCRIPTION\n  #{desc}"
      end

      # ---------------------------------------------------------------------------
      # COMMANDS section
      # ---------------------------------------------------------------------------

      def commands_section
        cmds = @resolved["commands"] || []
        return nil if cmds.empty?

        max_len = cmds.map { |c| c["name"].length }.max
        lines = cmds.map do |c|
          padded = c["name"].ljust(max_len + 2)
          "  #{padded}#{c["description"]}"
        end

        "COMMANDS\n#{lines.join("\n")}"
      end

      # ---------------------------------------------------------------------------
      # OPTIONS section (command-local flags)
      # ---------------------------------------------------------------------------

      def options_section
        flags = @resolved["flags"] || []
        return nil if flags.empty?

        lines = flags.map { |f| format_flag_line(f) }
        "OPTIONS\n#{lines.join("\n")}"
      end

      # ---------------------------------------------------------------------------
      # GLOBAL OPTIONS section
      # ---------------------------------------------------------------------------

      def global_options_section
        global_flags = @spec["global_flags"] || []

        # Add builtin flags
        builtins = []
        bf = @spec["builtin_flags"] || {}

        if bf["help"] != false
          builtins << {
            "id" => "__help__",
            "short" => "h",
            "long" => "help",
            "description" => "Show this help message and exit.",
            "type" => "boolean"
          }
        end

        if bf["version"] != false && @spec["version"]
          builtins << {
            "id" => "__version__",
            "long" => "version",
            "description" => "Show version and exit.",
            "type" => "boolean"
          }
        end

        all_global = global_flags + builtins
        return nil if all_global.empty?

        lines = all_global.map { |f| format_flag_line(f) }
        "GLOBAL OPTIONS\n#{lines.join("\n")}"
      end

      # ---------------------------------------------------------------------------
      # ARGUMENTS section
      # ---------------------------------------------------------------------------

      def arguments_section
        args = @resolved["arguments"] || []
        return nil if args.empty?

        lines = args.map do |a|
          display = format_arg_usage(a)
          required = a["required"] != false
          req_label = required ? "Required." : "Optional."
          "  #{display.ljust(16)}#{a["description"]} #{req_label}"
        end

        "ARGUMENTS\n#{lines.join("\n")}"
      end

      # ---------------------------------------------------------------------------
      # Flag formatting helpers
      # ---------------------------------------------------------------------------

      def format_flag_line(flag)
        name_part = format_flag_name(flag)
        desc = flag["description"] || ""
        default_part = format_default(flag)
        "  #{name_part.ljust(28)}#{desc}#{default_part}"
      end

      # ---------------------------------------------------------------------------
      # Flag name formatting (v1.1 updated)
      # ---------------------------------------------------------------------------
      #
      # v1.1 changes: When an enum flag has default_when_present, its value
      # is optional. We show this as "[=VALUE]" instead of " <VALUE>" to
      # communicate that the value can be omitted:
      #
      #   --color [=WHEN]       ← enum with default_when_present
      #   --output <FILE>       ← regular non-boolean flag
      #   --verbose             ← boolean (no value shown)
      #   --verbose             ← count (no value shown, like boolean)

      def format_flag_name(flag)
        parts = []
        parts << "-#{flag["short"]}" if flag["short"]
        parts << "--#{flag["long"]}" if flag["long"]
        parts << "-#{flag["single_dash_long"]}" if flag["single_dash_long"]

        name = parts.join(", ")

        unless %w[boolean count].include?(flag["type"])
          value_label = flag["value_name"] || flag["type"]&.upcase || "VALUE"
          # Optional value (v1.1): show as [=VALUE] to indicate the value can be omitted
          name += if flag["default_when_present"]
            " [=#{value_label}]"
          else
            " <#{value_label}>"
          end
        end

        name
      end

      def format_default(flag)
        return "" if flag["default"].nil?
        return "" if flag["required"]
        " [default: #{flag["default"]}]"
      end
    end
  end
end
