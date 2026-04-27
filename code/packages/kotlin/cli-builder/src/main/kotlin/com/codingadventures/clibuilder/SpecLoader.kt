package com.codingadventures.clibuilder

import com.fasterxml.jackson.databind.JsonNode
import com.fasterxml.jackson.databind.ObjectMapper
import com.fasterxml.jackson.databind.node.ArrayNode
import java.io.IOException
import java.nio.file.Files
import java.nio.file.Path
import java.util.ArrayDeque

internal class SpecLoader {
    private val mapper = ObjectMapper()
    private val validTypes = setOf("boolean", "count", "string", "integer", "float", "path", "file", "directory", "enum")
    private val validParsingModes = setOf("posix", "gnu", "subcommand_first", "traditional")

    fun load(specFilePath: String): CliSpec = load(Path.of(specFilePath))

    fun load(specFilePath: Path): CliSpec = try {
        loadString(Files.readString(specFilePath))
    } catch (error: IOException) {
        throw SpecError("Cannot read spec file '$specFilePath': ${error.message}", error)
    }

    fun loadString(json: String): CliSpec = try {
        val root = mapper.readTree(json)
        if (root == null || !root.isObject) {
            throw SpecError("Spec must be a JSON object at the top level")
        }
        parseSpec(root)
    } catch (error: SpecError) {
        throw error
    } catch (error: IOException) {
        throw SpecError("Spec file is not valid JSON: ${error.message}", error)
    }

    private fun parseSpec(root: JsonNode): CliSpec {
        val version = text(root, "cli_builder_spec_version")
            ?: throw SpecError("Missing required field: 'cli_builder_spec_version'")
        if (version != "1.0") {
            throw SpecError("Unsupported spec version '$version'. Expected '1.0'.")
        }

        val name = text(root, "name")?.takeIf { it.isNotBlank() }
            ?: throw SpecError("Missing required field: 'name'")
        val description = text(root, "description")?.takeIf { it.isNotBlank() }
            ?: throw SpecError("Missing required field: 'description'")
        val displayName = text(root, "display_name") ?: name
        val parsingMode = text(root, "parsing_mode") ?: "gnu"
        if (parsingMode !in validParsingModes) {
            throw SpecError("Invalid parsing_mode '$parsingMode'. Must be one of: $validParsingModes")
        }

        val builtinFlags = parseBuiltinFlags(root["builtin_flags"])
        val globalFlags = parseFlags(array(root, "global_flags"), "global_flags", null)
        val scope = parseScope(root, "root", globalFlags.mapTo(linkedSetOf()) { it.id })

        return CliSpec(
            name = name,
            displayName = displayName,
            description = description,
            version = text(root, "version"),
            parsingMode = parsingMode,
            builtinFlags = builtinFlags,
            globalFlags = globalFlags,
            flags = scope.flags,
            arguments = scope.arguments,
            commands = scope.commands,
            mutuallyExclusiveGroups = scope.groups,
        )
    }

    private fun parseBuiltinFlags(node: JsonNode?): BuiltinFlags {
        if (node == null || node.isNull || node.isMissingNode) {
            return BuiltinFlags(help = true, version = true)
        }
        return BuiltinFlags(
            help = booleanValue(node, "help", true),
            version = booleanValue(node, "version", true),
        )
    }

    private fun parseScope(node: JsonNode, scopeName: String, globalFlagIds: Set<String>): ScopeParts {
        val flagNodes = array(node, "flags")
        val availableIds = LinkedHashSet(globalFlagIds).apply { addAll(collectIds(flagNodes, "id")) }
        val flags = parseFlags(flagNodes, scopeName, availableIds)
        val arguments = parseArguments(array(node, "arguments"), scopeName)
        val groups = parseGroups(array(node, "mutually_exclusive_groups"), scopeName, availableIds)
        validateRequiresCycles(flags, scopeName)
        val commands = parseCommands(array(node, "commands"), scopeName, globalFlagIds)
        return ScopeParts(flags, arguments, commands, groups)
    }

    private fun parseCommands(commandsNode: ArrayNode, parentScope: String, globalFlagIds: Set<String>): List<CommandDef> {
        val commands = mutableListOf<CommandDef>()
        val seenIds = mutableSetOf<String>()
        val seenNames = mutableSetOf<String>()

        for (commandNode in commandsNode) {
            val id = text(commandNode, "id")?.takeIf { it.isNotBlank() }
                ?: throw SpecError("Command in scope '$parentScope' is missing required field 'id'")
            if (!seenIds.add(id)) {
                throw SpecError("Duplicate command id '$id' in scope '$parentScope'")
            }

            val name = text(commandNode, "name")?.takeIf { it.isNotBlank() }
                ?: throw SpecError("Command '$id' in scope '$parentScope' is missing 'name'")
            if (!seenNames.add(name)) {
                throw SpecError("Duplicate command name '$name' in scope '$parentScope'")
            }

            val aliases = strings(array(commandNode, "aliases"))
            for (alias in aliases) {
                if (!seenNames.add(alias)) {
                    throw SpecError("Duplicate command name/alias '$alias' in scope '$parentScope'")
                }
            }

            val description = text(commandNode, "description")?.takeIf { it.isNotBlank() }
                ?: throw SpecError("Command '$id' in scope '$parentScope' is missing 'description'")

            val scope = parseScope(commandNode, "$parentScope.$name", globalFlagIds)
            commands += CommandDef(
                id = id,
                name = name,
                description = description,
                aliases = aliases,
                inheritGlobalFlags = booleanValue(commandNode, "inherit_global_flags", true),
                flags = scope.flags,
                arguments = scope.arguments,
                commands = scope.commands,
                mutuallyExclusiveGroups = scope.groups,
            )
        }

        return commands
    }

    private fun parseFlags(flagsNode: ArrayNode, scopeName: String, availableIds: Set<String>?): List<FlagDef> {
        val flags = mutableListOf<FlagDef>()
        val seenIds = mutableSetOf<String>()

        for (flagNode in flagsNode) {
            val id = text(flagNode, "id")?.takeIf { it.isNotBlank() }
                ?: throw SpecError("Flag in scope '$scopeName' is missing required field 'id'")
            if (!seenIds.add(id)) {
                throw SpecError("Duplicate flag id '$id' in scope '$scopeName'")
            }

            val shortName = text(flagNode, "short")
            val longName = text(flagNode, "long")
            val singleDashLong = text(flagNode, "single_dash_long")
            if (shortName == null && longName == null && singleDashLong == null) {
                throw SpecError("Flag '$id' in scope '$scopeName' must have at least one of 'short', 'long', or 'single_dash_long'")
            }

            val description = text(flagNode, "description")?.takeIf { it.isNotBlank() }
                ?: throw SpecError("Flag '$id' in scope '$scopeName' is missing 'description'")
            val type = text(flagNode, "type")
            if (type == null || type !in validTypes) {
                throw SpecError("Flag '$id' has invalid type '$type'. Must be one of: $validTypes")
            }

            val enumValues = strings(array(flagNode, "enum_values"))
            if (type == "enum" && enumValues.isEmpty()) {
                throw SpecError("Flag '$id' has type 'enum' but 'enum_values' is missing or empty in scope '$scopeName'")
            }

            val defaultWhenPresent = text(flagNode, "default_when_present")
            if (defaultWhenPresent != null) {
                if (type != "enum") {
                    throw SpecError("Flag '$id' in scope '$scopeName' has 'default_when_present' but type is '$type' (only 'enum' supports this field)")
                }
                if (defaultWhenPresent !in enumValues) {
                    throw SpecError("Flag '$id' in scope '$scopeName' has default_when_present='$defaultWhenPresent' which is not in enum_values: $enumValues")
                }
            }

            val conflictsWith = strings(array(flagNode, "conflicts_with"))
            val requires = strings(array(flagNode, "requires"))
            val requiredUnless = strings(array(flagNode, "required_unless"))
            availableIds?.let {
                conflictsWith.forEach { refId ->
                    if (refId !in it) {
                        throw SpecError("Flag '$id' in scope '$scopeName' references unknown flag '$refId' in 'conflicts_with'")
                    }
                }
                requires.forEach { refId ->
                    if (refId !in it) {
                        throw SpecError("Flag '$id' in scope '$scopeName' references unknown flag '$refId' in 'requires'")
                    }
                }
            }

            flags += FlagDef(
                id = id,
                shortName = shortName,
                longName = longName,
                singleDashLong = singleDashLong,
                description = description,
                type = type,
                required = booleanValue(flagNode, "required", false),
                defaultValue = value(flagNode["default"]),
                valueName = text(flagNode, "value_name"),
                enumValues = enumValues,
                conflictsWith = conflictsWith,
                requires = requires,
                requiredUnless = requiredUnless,
                repeatable = booleanValue(flagNode, "repeatable", false),
                defaultWhenPresent = defaultWhenPresent,
            )
        }

        return flags
    }

    private fun parseArguments(argumentsNode: ArrayNode, scopeName: String): List<ArgumentDef> {
        val arguments = mutableListOf<ArgumentDef>()
        val seenIds = mutableSetOf<String>()
        var seenVariadic = false

        for (argumentNode in argumentsNode) {
            val id = text(argumentNode, "id")?.takeIf { it.isNotBlank() }
                ?: throw SpecError("Argument in scope '$scopeName' is missing required field 'id'")
            if (!seenIds.add(id)) {
                throw SpecError("Duplicate argument id '$id' in scope '$scopeName'")
            }

            val displayName = text(argumentNode, "display_name") ?: text(argumentNode, "name")
            if (displayName.isNullOrBlank()) {
                throw SpecError("Argument '$id' in scope '$scopeName' is missing 'display_name'")
            }
            val description = text(argumentNode, "description")?.takeIf { it.isNotBlank() }
                ?: throw SpecError("Argument '$id' in scope '$scopeName' is missing 'description'")
            val type = text(argumentNode, "type")
            if (type == null || type !in validTypes) {
                throw SpecError("Argument '$id' has invalid type '$type'. Must be one of: $validTypes")
            }

            val enumValues = strings(array(argumentNode, "enum_values"))
            if (type == "enum" && enumValues.isEmpty()) {
                throw SpecError("Argument '$id' has type 'enum' but 'enum_values' is missing or empty in scope '$scopeName'")
            }

            val required = booleanValue(argumentNode, "required", true)
            val variadic = booleanValue(argumentNode, "variadic", false)
            if (variadic) {
                if (seenVariadic) {
                    throw SpecError("Scope '$scopeName' has more than one variadic argument. At most one argument per scope may be variadic.")
                }
                seenVariadic = true
            }

            arguments += ArgumentDef(
                id = id,
                displayName = displayName,
                description = description,
                type = type,
                required = required,
                variadic = variadic,
                variadicMin = intValue(argumentNode, "variadic_min", if (required) 1 else 0),
                variadicMax = nullableInt(argumentNode["variadic_max"]),
                defaultValue = value(argumentNode["default"]),
                enumValues = enumValues,
                requiredUnlessFlag = strings(array(argumentNode, "required_unless_flag")),
            )
        }

        return arguments
    }

    private fun parseGroups(groupsNode: ArrayNode, scopeName: String, availableIds: Set<String>): List<ExclusiveGroupDef> =
        buildList {
            for (groupNode in groupsNode) {
                val id = text(groupNode, "id")?.takeIf { it.isNotBlank() }
                    ?: throw SpecError("Exclusive group in scope '$scopeName' is missing 'id'")
                val flagIds = strings(array(groupNode, "flag_ids"))
                if (flagIds.isEmpty()) {
                    throw SpecError("Exclusive group '$id' in scope '$scopeName' has empty 'flag_ids'")
                }
                for (refId in flagIds) {
                    if (refId !in availableIds) {
                        throw SpecError("Exclusive group '$id' in scope '$scopeName' references unknown flag '$refId'")
                    }
                }
                add(ExclusiveGroupDef(id, flagIds, booleanValue(groupNode, "required", false)))
            }
        }

    private fun validateRequiresCycles(flags: List<FlagDef>, scopeName: String) {
        val ids = flags.mapTo(linkedSetOf()) { it.id }
        for (flag in flags) {
            val visited = mutableSetOf<String>()
            val stack = ArrayDeque<String>()
            stack.add(flag.id)
            while (stack.isNotEmpty()) {
                val current = stack.removeLast()
                if (!visited.add(current)) continue
                flags.firstOrNull { it.id == current }?.requires?.forEach { dependency ->
                    if (dependency !in ids) return@forEach
                    if (dependency == flag.id) {
                        throw SpecError("Circular 'requires' dependency detected in scope '$scopeName'. Check for flags that mutually require each other.")
                    }
                    stack.add(dependency)
                }
            }
        }
    }

    private fun text(node: JsonNode?, fieldName: String): String? =
        node?.get(fieldName)?.takeUnless { it.isNull || it.isMissingNode }?.asText()

    private fun array(node: JsonNode?, fieldName: String): ArrayNode {
        val child = node?.get(fieldName)
        if (child == null || child.isNull || child.isMissingNode) {
            return mapper.createArrayNode()
        }
        if (!child.isArray) {
            throw SpecError("Field '$fieldName' must be a JSON array")
        }
        return child as ArrayNode
    }

    private fun booleanValue(node: JsonNode?, fieldName: String, defaultValue: Boolean): Boolean =
        node?.get(fieldName)?.takeUnless { it.isNull || it.isMissingNode }?.asBoolean(defaultValue) ?: defaultValue

    private fun intValue(node: JsonNode?, fieldName: String, defaultValue: Int): Int =
        node?.get(fieldName)?.takeUnless { it.isNull || it.isMissingNode }?.asInt(defaultValue) ?: defaultValue

    private fun nullableInt(node: JsonNode?): Int? =
        if (node == null || node.isNull || node.isMissingNode) null else node.asInt()

    private fun collectIds(nodes: ArrayNode, fieldName: String): Set<String> =
        nodes.mapNotNullTo(linkedSetOf()) { text(it, fieldName) }

    private fun strings(arrayNode: ArrayNode): List<String> =
        arrayNode.filterNot { it.isNull }.map { it.asText() }

    private fun value(node: JsonNode?): Any? = when {
        node == null || node.isNull || node.isMissingNode -> null
        node.isBoolean -> node.asBoolean()
        node.isIntegralNumber -> node.asLong()
        node.isFloatingPointNumber -> node.asDouble()
        node.isArray -> node.map { value(it) }
        else -> node.asText()
    }

    private data class ScopeParts(
        val flags: List<FlagDef>,
        val arguments: List<ArgumentDef>,
        val commands: List<CommandDef>,
        val groups: List<ExclusiveGroupDef>,
    )
}
