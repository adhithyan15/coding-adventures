-- types.lua -- CLI Builder Result Types
-- =====================================

local Types = {}

--- ParseResult represents a successful CLI invocation.
--
-- @param program string The name of the program.
-- @param command_path table Array of resolved subcommand names.
-- @param flags table Map of flag ID to value.
-- @param arguments table Map of argument ID to value(s).
-- @param explicit_flags table List of flag IDs provided by the user.
-- @return table The result object.
Types.ParseResult = function(program, command_path, flags, arguments, explicit_flags)
    return {
        type = "parse_result",
        program = program,
        command_path = command_path,
        flags = flags,
        arguments = arguments,
        explicit_flags = explicit_flags
    }
end

--- HelpResult represents a request for --help or -h.
--
-- @param text string The formatted help text.
-- @param command_path table Array of command names the help is for.
-- @return table The result object.
Types.HelpResult = function(text, command_path)
    return {
        type = "help_result",
        text = text,
        command_path = command_path
    }
end

--- VersionResult represents a request for --version.
--
-- @param version string The program version.
-- @return table The result object.
Types.VersionResult = function(version)
    return {
        type = "version_result",
        version = version
    }
end

return Types
