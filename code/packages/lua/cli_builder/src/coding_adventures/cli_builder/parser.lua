-- parser.lua -- CLI Argument Parser (Routing + Scanning)
-- ========================================================

local Errors = require("coding_adventures.cli_builder.errors")
local Types = require("coding_adventures.cli_builder.types")
local SpecLoader = require("coding_adventures.cli_builder.spec_loader")
local TokenClassifier = require("coding_adventures.cli_builder.token_classifier")
local sm = require("coding_adventures.state_machine")

local Parser = {}
Parser.__index = Parser

-- Parse modes
local MODE_SCANNING     = "SCANNING"
local MODE_FLAG_VALUE   = "FLAG_VALUE"
local MODE_END_OF_FLAGS = "END_OF_FLAGS"

function Parser.new(spec_file, argv)
    local spec, err = SpecLoader.load(spec_file)
    if not spec then return nil, err end
    return setmetatable({ spec = spec, argv = argv }, Parser)
end

function Parser.new_from_string(spec_str, argv)
    local spec, err = SpecLoader.parse_string(spec_str)
    if not spec then return nil, err end
    return setmetatable({ spec = spec, argv = argv }, Parser)
end

local function stringField(tbl, field)
    if not tbl then return "" end
    local v = tbl[field]
    if type(v) == "string" then return v end
    return ""
end

local function sliceOfMaps(v)
    if type(v) ~= "table" then return {} end
    return v
end

local function sliceOfStrings(v)
    if type(v) ~= "table" then return {} end
    return v
end

local function findCommand(token, commands)
    for _, cmd in ipairs(commands) do
        if stringField(cmd, "name") == token then
            return true, cmd
        end
        for _, alias in ipairs(sliceOfStrings(cmd.aliases)) do
            if alias == token then
                return true, cmd
            end
        end
    end
    return false, nil
end

function Parser:parse()
    local program = self.argv[0] or "lua"
    local tokens = {}
    for i = 1, #self.argv do
        table.insert(tokens, self.argv[i])
    end

    local parsingMode = stringField(self.spec, "parsing_mode")
    if parsingMode == "" then parsingMode = "gnu" end

    -- Phase 1: Routing
    local commandPath, resolvedNode, remainingTokens, routingErrs = self:phaseRouting(program, tokens, parsingMode)

    -- Phase 2: Scanning
    local activeFlags = self:buildActiveFlags(commandPath)
    -- Builtin flags (simplified for now)
    table.insert(activeFlags, { id = "help", short = "h", long = "help", type = "boolean" })
    table.insert(activeFlags, { id = "version", long = "version", type = "boolean" })

    local tc = TokenClassifier.new(activeFlags)
    local results = self:phaseScanning(remainingTokens, commandPath, tc, parsingMode, activeFlags)

    if results.helpRequested then
        -- Return HelpResult (stub for now)
        return Types.HelpResult("Help text placeholder", commandPath)
    end
    if results.versionRequested then
        return Types.VersionResult(stringField(self.spec, "version") or "(unknown)")
    end

    -- Phase 3: Validation (Simplified for now: return results)
    if #routingErrs > 0 or #results.errs > 0 then
        local allErrs = {}
        for _, e in ipairs(routingErrs) do table.insert(allErrs, e) end
        for _, e in ipairs(results.errs) do table.insert(allErrs, e) end
        return nil, allErrs
    end

    return Types.ParseResult(program, commandPath, results.flags, results.arguments, results.explicitFlags)
end

function Parser:phaseRouting(program, tokens, parsingMode)
    local commandPath = { program }
    local resolvedNode = {
        flags = self.spec.flags,
        arguments = self.spec.arguments,
        commands = self.spec.commands,
        mutually_exclusive_groups = self.spec.mutually_exclusive_groups
    }
    local consumedIdx = {}
    local errs = {}

    local i = 1
    while i <= #tokens do
        local token = tokens[i]
        if token == "--" then break end

        if token:sub(1, 1) == "-" then
            -- Skip flags in Phase 1 (simplified)
            i = i + 1
        else
            local currentCommands = sliceOfMaps(resolvedNode.commands)
            local matched, matchedCmd = findCommand(token, currentCommands)
            if matched then
                consumedIdx[i] = true
                table.insert(commandPath, stringField(matchedCmd, "name"))
                resolvedNode = {
                    flags = matchedCmd.flags,
                    arguments = matchedCmd.arguments,
                    commands = matchedCmd.commands,
                    mutually_exclusive_groups = matchedCmd.mutually_exclusive_groups,
                    inherit_global_flags = matchedCmd.inherit_global_flags
                }
                i = i + 1
            else
                break
            end
        end
    end

    local remaining = {}
    for j = 1, #tokens do
        if not consumedIdx[j] then
            table.insert(remaining, tokens[j])
        end
    end

    return commandPath, resolvedNode, remaining, errs
end

function Parser:buildActiveFlags(commandPath)
    local flags = {}
    -- Start from top and go down (simplified)
    local current = self.spec
    for i = 1, #flags do table.insert(flags, flags[i]) end -- Placeholder
    
    -- Actually, for now we only support root-level flags if commandPath is 1,
    -- or flags from the last command.
    -- TODO: Implement global flag inheritance.
    local active = {}
    local function addFlags(f)
        if not f then return end
        for _, flag in ipairs(f) do table.insert(active, flag) end
    end
    
    addFlags(self.spec.flags)
    -- If we have subcommands, we should add their flags too.
    -- This is a simplified version for cowsay.json which has no subcommands anyway.
    return active
end

function Parser:phaseScanning(tokens, commandPath, tc, parsingMode, activeFlags)
    local flags = {}
    local arguments = {}
    local errs = {}
    local explicitFlags = {}
    local posTokens = {}
    local helpRequested = false
    local versionRequested = false

    local dfaScanning = sm.DFA.new({MODE_SCANNING}, {"token"}, {[{MODE_SCANNING, "token"}] = MODE_SCANNING}, MODE_SCANNING, {MODE_SCANNING})
    local dfaValue    = sm.DFA.new({MODE_FLAG_VALUE}, {"token"}, {[{MODE_FLAG_VALUE, "token"}] = MODE_FLAG_VALUE}, MODE_FLAG_VALUE, {MODE_FLAG_VALUE})
    local dfaEOF      = sm.DFA.new({MODE_END_OF_FLAGS}, {"token"}, {[{MODE_END_OF_FLAGS, "token"}] = MODE_END_OF_FLAGS}, MODE_END_OF_FLAGS, {MODE_END_OF_FLAGS})

    local msm = sm.ModalStateMachine.new({
        [MODE_SCANNING]     = dfaScanning,
        [MODE_FLAG_VALUE]   = dfaValue,
        [MODE_END_OF_FLAGS] = dfaEOF
    }, {
        [{MODE_SCANNING, "to_flag_value"}]    = MODE_FLAG_VALUE,
        [{MODE_SCANNING, "to_end_of_flags"}]  = MODE_END_OF_FLAGS,
        [{MODE_FLAG_VALUE, "to_scanning"}]     = MODE_SCANNING,
        [{MODE_END_OF_FLAGS, "stay_eof"}]      = MODE_END_OF_FLAGS
    }, MODE_SCANNING)

    local pendingFlag = nil

    for _, token in ipairs(tokens) do
        local mode = msm:current_mode()
        msm:process("token")

        if mode == MODE_END_OF_FLAGS then
            table.insert(posTokens, token)
        elseif mode == MODE_FLAG_VALUE then
            if pendingFlag then
                flags[pendingFlag.id] = token -- coercion later
                table.insert(explicitFlags, pendingFlag.id)
                pendingFlag = nil
            end
            msm:switch_mode("to_scanning")
        else
            local ev = tc:classify(token)
            if ev.kind == tc.KIND.END_OF_FLAGS then
                msm:switch_mode("to_end_of_flags")
            elseif ev.kind == tc.KIND.LONG_FLAG or ev.kind == tc.KIND.SHORT or ev.kind == tc.KIND.SDL then
                local flagDef = (ev.kind == tc.KIND.SDL) and tc.singleDashLongs[ev.name] or
                                (ev.kind == tc.KIND.LONG_FLAG) and tc.longFlags[ev.name] or
                                tc.shortFlags[ev.name]
                if flagDef then
                    if flagDef.id == "help" then helpRequested = true; break end
                    if flagDef.id == "version" then versionRequested = true; break end
                    
                    if flagDef.type == "boolean" then
                        flags[flagDef.id] = true
                        table.insert(explicitFlags, flagDef.id)
                    elseif flagDef.type == "count" then
                        flags[flagDef.id] = (flags[flagDef.id] or 0) + 1
                        table.insert(explicitFlags, flagDef.id)
                    else
                        pendingFlag = flagDef
                        msm:switch_mode("to_flag_value")
                    end
                else
                    table.insert(errs, Errors.ParseError(Errors.PARSE_ERRORS.UNKNOWN_FLAG, "Unknown flag: " .. ev.name))
                end
            elseif ev.kind == tc.KIND.LONG_FLAG_VAL or ev.kind == tc.KIND.SHORT_VAL then
                local flagDef = (ev.kind == tc.KIND.LONG_FLAG_VAL) and tc.longFlags[ev.name] or tc.shortFlags[ev.name]
                if flagDef then
                    flags[flagDef.id] = ev.value
                    table.insert(explicitFlags, flagDef.id)
                else
                    table.insert(errs, Errors.ParseError(Errors.PARSE_ERRORS.UNKNOWN, "Unknown flag: " .. ev.name))
                end
            elseif ev.kind == tc.KIND.STACKED then
                for _, ch in ipairs(ev.chars) do
                    local flagDef = tc.shortFlags[ch]
                    if flagDef then
                        if flagDef.type == "boolean" then
                            flags[flagDef.id] = true
                            table.insert(explicitFlags, flagDef.id)
                        elseif flagDef.type == "count" then
                            flags[flagDef.id] = (flags[flagDef.id] or 0) + 1
                            table.insert(explicitFlags, flagDef.id)
                        else
                            pendingFlag = flagDef
                            msm:switch_mode("to_flag_value")
                        end
                    end
                end
            elseif ev.kind == tc.KIND.UNKNOWN then
                table.insert(errs, Errors.ParseError(Errors.PARSE_ERRORS.UNKNOWN_FLAG, "Unknown flag: " .. ev.raw))
            else
                table.insert(posTokens, token)
                if parsingMode == "posix" then msm:switch_mode("to_end_of_flags") end
            end
        end
    end

    -- Assign positional tokens (simplified)
    -- TODO: Use PositionalResolver
    return {
        flags = flags,
        arguments = { message = table.concat(posTokens, " ") },
        errs = errs,
        helpRequested = helpRequested,
        versionRequested = versionRequested,
        explicitFlags = explicitFlags
    }
end

return Parser
