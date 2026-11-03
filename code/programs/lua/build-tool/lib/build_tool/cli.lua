-- cli.lua -- Fail-Closed Command-Line Parsing
-- ============================================
--
-- A command line is a trust boundary even for an educational build tool. If
-- an option is silently ignored, the caller can believe it selected one root
-- or language while the tool builds another. Missing values are subtler: the
-- next option can be consumed as a path and turn a malformed invocation into
-- a successful no-op.
--
-- Parsing therefore has an all-or-error result. Diagnostics name only the
-- fixed option whose value is missing; unknown input is never repeated because
-- argv can contain checkout paths or other host-specific data.

local Cli = {}

local function is_option_token(value)
    return value == "-h" or value:sub(1, 2) == "--"
end

local function required_value(argv, index, option)
    local value = argv[index + 1]
    if type(value) ~= "string" or value == "" or is_option_token(value) then
        return nil, "CLI_ARGUMENT_ERROR: " .. option .. " requires a value"
    end
    return value, nil
end

--- Parse build-tool command-line arguments without opening files or exiting.
--
-- @param argv table Positive-indexed argument strings, excluding the program.
-- @return table|nil Parsed options on success.
-- @return string|nil Stable, path-redacted diagnostic on failure.
function Cli.parse(argv)
    local options = {
        root = nil,
        dry_run = false,
        language = "all",
        force = false,
        validate_build_files = false,
        help = false,
    }

    local index = 1
    while index <= #argv do
        local argument = argv[index]
        if argument == "--root" or argument == "--language" then
            local value, diagnostic = required_value(argv, index, argument)
            if diagnostic then
                return nil, diagnostic
            end
            if argument == "--root" then
                options.root = value
            else
                options.language = value
            end
            index = index + 1
        elseif argument == "--dry-run" then
            options.dry_run = true
        elseif argument == "--force" then
            options.force = true
        elseif argument == "--validate-build-files" then
            options.validate_build_files = true
        elseif argument == "--help" or argument == "-h" then
            options.help = true
        elseif type(argument) == "string" and argument:sub(1, 1) == "-" then
            return nil, "CLI_ARGUMENT_ERROR: unknown option"
        else
            return nil, "CLI_ARGUMENT_ERROR: unexpected positional argument"
        end
        index = index + 1
    end

    return options, nil
end

return Cli
