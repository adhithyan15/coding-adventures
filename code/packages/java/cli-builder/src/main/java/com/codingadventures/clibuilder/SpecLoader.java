package com.codingadventures.clibuilder;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Objects;
import java.util.Set;

final class SpecLoader {
    private static final ObjectMapper MAPPER = new ObjectMapper();
    private static final Set<String> VALID_TYPES = Set.of(
            "boolean", "count", "string", "integer", "float", "path", "file", "directory", "enum"
    );
    private static final Set<String> VALID_PARSING_MODES = Set.of(
            "posix", "gnu", "subcommand_first", "traditional"
    );

    CliSpec load(String specFilePath) {
        return load(Path.of(specFilePath));
    }

    CliSpec load(Path specFilePath) {
        try {
            return loadString(Files.readString(specFilePath));
        } catch (IOException error) {
            throw new SpecError("Cannot read spec file '" + specFilePath + "': " + error.getMessage(), error);
        }
    }

    CliSpec loadString(String json) {
        try {
            JsonNode root = MAPPER.readTree(json);
            if (root == null || !root.isObject()) {
                throw new SpecError("Spec must be a JSON object at the top level");
            }
            return parseSpec(root);
        } catch (SpecError error) {
            throw error;
        } catch (IOException error) {
            throw new SpecError("Spec file is not valid JSON: " + error.getMessage(), error);
        }
    }

    private CliSpec parseSpec(JsonNode root) {
        String version = text(root, "cli_builder_spec_version");
        if (version == null) {
            throw new SpecError("Missing required field: 'cli_builder_spec_version'");
        }
        if (!Objects.equals(version, "1.0")) {
            throw new SpecError("Unsupported spec version '" + version + "'. Expected '1.0'.");
        }

        String name = text(root, "name");
        if (name == null || name.isBlank()) {
            throw new SpecError("Missing required field: 'name'");
        }
        String description = text(root, "description");
        if (description == null || description.isBlank()) {
            throw new SpecError("Missing required field: 'description'");
        }

        String displayName = defaulted(text(root, "display_name"), name);
        String parsingMode = defaulted(text(root, "parsing_mode"), "gnu");
        if (!VALID_PARSING_MODES.contains(parsingMode)) {
            throw new SpecError("Invalid parsing_mode '" + parsingMode + "'. Must be one of: " + VALID_PARSING_MODES);
        }

        BuiltinFlags builtinFlags = parseBuiltinFlags(root.get("builtin_flags"));
        List<FlagDef> globalFlags = parseFlags(array(root, "global_flags"), "global_flags", null);
        Set<String> globalFlagIds = ids(globalFlags);
        ScopeParts scope = parseScope(root, "root", globalFlagIds);

        return new CliSpec(
                name,
                displayName,
                description,
                text(root, "version"),
                parsingMode,
                builtinFlags,
                globalFlags,
                scope.flags(),
                scope.arguments(),
                scope.commands(),
                scope.groups()
        );
    }

    private BuiltinFlags parseBuiltinFlags(JsonNode node) {
        if (node == null || node.isMissingNode() || node.isNull()) {
            return new BuiltinFlags(true, true);
        }
        return new BuiltinFlags(booleanValue(node, "help", true), booleanValue(node, "version", true));
    }

    private ScopeParts parseScope(JsonNode node, String scopeName, Set<String> globalFlagIds) {
        ArrayNode flagNodes = array(node, "flags");
        Set<String> localFlagIds = collectIds(flagNodes, "id");
        Set<String> availableIds = new LinkedHashSet<>(globalFlagIds);
        availableIds.addAll(localFlagIds);

        List<FlagDef> flags = parseFlags(flagNodes, scopeName, availableIds);
        List<ArgumentDef> arguments = parseArguments(array(node, "arguments"), scopeName);
        List<ExclusiveGroupDef> groups = parseGroups(array(node, "mutually_exclusive_groups"), scopeName, availableIds);
        validateRequiresCycles(flags, scopeName);
        List<CommandDef> commands = parseCommands(array(node, "commands"), scopeName, globalFlagIds);
        return new ScopeParts(flags, arguments, commands, groups);
    }

    private List<CommandDef> parseCommands(ArrayNode commandsNode, String parentScope, Set<String> globalFlagIds) {
        List<CommandDef> commands = new ArrayList<>();
        Set<String> seenIds = new HashSet<>();
        Set<String> seenNames = new HashSet<>();

        for (JsonNode cmdNode : commandsNode) {
            String id = text(cmdNode, "id");
            if (id == null || id.isBlank()) {
                throw new SpecError("Command in scope '" + parentScope + "' is missing required field 'id'");
            }
            if (!seenIds.add(id)) {
                throw new SpecError("Duplicate command id '" + id + "' in scope '" + parentScope + "'");
            }

            String name = text(cmdNode, "name");
            if (name == null || name.isBlank()) {
                throw new SpecError("Command '" + id + "' in scope '" + parentScope + "' is missing 'name'");
            }
            if (!seenNames.add(name)) {
                throw new SpecError("Duplicate command name '" + name + "' in scope '" + parentScope + "'");
            }

            List<String> aliases = strings(array(cmdNode, "aliases"));
            for (String alias : aliases) {
                if (!seenNames.add(alias)) {
                    throw new SpecError("Duplicate command name/alias '" + alias + "' in scope '" + parentScope + "'");
                }
            }

            String description = text(cmdNode, "description");
            if (description == null || description.isBlank()) {
                throw new SpecError("Command '" + id + "' in scope '" + parentScope + "' is missing 'description'");
            }

            String scopeName = parentScope + "." + name;
            ScopeParts scope = parseScope(cmdNode, scopeName, globalFlagIds);
            commands.add(new CommandDef(
                    id,
                    name,
                    description,
                    aliases,
                    booleanValue(cmdNode, "inherit_global_flags", true),
                    scope.flags(),
                    scope.arguments(),
                    scope.commands(),
                    scope.groups()
            ));
        }

        return commands;
    }

    private List<FlagDef> parseFlags(ArrayNode flagsNode, String scopeName, Set<String> availableIds) {
        List<FlagDef> flags = new ArrayList<>();
        Set<String> seenIds = new HashSet<>();

        for (JsonNode flagNode : flagsNode) {
            String id = text(flagNode, "id");
            if (id == null || id.isBlank()) {
                throw new SpecError("Flag in scope '" + scopeName + "' is missing required field 'id'");
            }
            if (!seenIds.add(id)) {
                throw new SpecError("Duplicate flag id '" + id + "' in scope '" + scopeName + "'");
            }

            String shortName = text(flagNode, "short");
            String longName = text(flagNode, "long");
            String singleDashLong = text(flagNode, "single_dash_long");
            if (shortName == null && longName == null && singleDashLong == null) {
                throw new SpecError(
                        "Flag '" + id + "' in scope '" + scopeName
                                + "' must have at least one of 'short', 'long', or 'single_dash_long'"
                );
            }

            String description = text(flagNode, "description");
            if (description == null || description.isBlank()) {
                throw new SpecError("Flag '" + id + "' in scope '" + scopeName + "' is missing 'description'");
            }
            String type = text(flagNode, "type");
            if (type == null || !VALID_TYPES.contains(type)) {
                throw new SpecError("Flag '" + id + "' has invalid type '" + type + "'. Must be one of: " + VALID_TYPES);
            }

            List<String> enumValues = strings(array(flagNode, "enum_values"));
            if ("enum".equals(type) && enumValues.isEmpty()) {
                throw new SpecError(
                        "Flag '" + id + "' has type 'enum' but 'enum_values' is missing or empty in scope '" + scopeName + "'"
                );
            }

            String defaultWhenPresent = text(flagNode, "default_when_present");
            if (defaultWhenPresent != null) {
                if (!"enum".equals(type)) {
                    throw new SpecError(
                            "Flag '" + id + "' in scope '" + scopeName + "' has 'default_when_present' but type is '"
                                    + type + "' (only 'enum' supports this field)"
                    );
                }
                if (!enumValues.contains(defaultWhenPresent)) {
                    throw new SpecError(
                            "Flag '" + id + "' in scope '" + scopeName + "' has default_when_present='"
                                    + defaultWhenPresent + "' which is not in enum_values: " + enumValues
                    );
                }
            }

            List<String> conflictsWith = strings(array(flagNode, "conflicts_with"));
            List<String> requires = strings(array(flagNode, "requires"));
            List<String> requiredUnless = strings(array(flagNode, "required_unless"));
            if (availableIds != null) {
                for (String refId : conflictsWith) {
                    if (!availableIds.contains(refId)) {
                        throw new SpecError(
                                "Flag '" + id + "' in scope '" + scopeName
                                        + "' references unknown flag '" + refId + "' in 'conflicts_with'"
                        );
                    }
                }
                for (String refId : requires) {
                    if (!availableIds.contains(refId)) {
                        throw new SpecError(
                                "Flag '" + id + "' in scope '" + scopeName
                                        + "' references unknown flag '" + refId + "' in 'requires'"
                        );
                    }
                }
            }

            flags.add(new FlagDef(
                    id,
                    shortName,
                    longName,
                    singleDashLong,
                    description,
                    type,
                    booleanValue(flagNode, "required", false),
                    value(flagNode.get("default")),
                    text(flagNode, "value_name"),
                    enumValues,
                    conflictsWith,
                    requires,
                    requiredUnless,
                    booleanValue(flagNode, "repeatable", false),
                    defaultWhenPresent
            ));
        }

        return flags;
    }

    private List<ArgumentDef> parseArguments(ArrayNode argumentsNode, String scopeName) {
        List<ArgumentDef> arguments = new ArrayList<>();
        Set<String> seenIds = new HashSet<>();
        boolean seenVariadic = false;

        for (JsonNode argumentNode : argumentsNode) {
            String id = text(argumentNode, "id");
            if (id == null || id.isBlank()) {
                throw new SpecError("Argument in scope '" + scopeName + "' is missing required field 'id'");
            }
            if (!seenIds.add(id)) {
                throw new SpecError("Duplicate argument id '" + id + "' in scope '" + scopeName + "'");
            }

            String displayName = defaulted(text(argumentNode, "display_name"), text(argumentNode, "name"));
            if (displayName == null || displayName.isBlank()) {
                throw new SpecError("Argument '" + id + "' in scope '" + scopeName + "' is missing 'display_name'");
            }
            String description = text(argumentNode, "description");
            if (description == null || description.isBlank()) {
                throw new SpecError("Argument '" + id + "' in scope '" + scopeName + "' is missing 'description'");
            }
            String type = text(argumentNode, "type");
            if (type == null || !VALID_TYPES.contains(type)) {
                throw new SpecError(
                        "Argument '" + id + "' has invalid type '" + type + "'. Must be one of: " + VALID_TYPES
                );
            }

            List<String> enumValues = strings(array(argumentNode, "enum_values"));
            if ("enum".equals(type) && enumValues.isEmpty()) {
                throw new SpecError(
                        "Argument '" + id + "' has type 'enum' but 'enum_values' is missing or empty in scope '"
                                + scopeName + "'"
                );
            }

            boolean required = booleanValue(argumentNode, "required", true);
            boolean variadic = booleanValue(argumentNode, "variadic", false);
            if (variadic) {
                if (seenVariadic) {
                    throw new SpecError(
                            "Scope '" + scopeName + "' has more than one variadic argument. At most one argument per scope may be variadic."
                    );
                }
                seenVariadic = true;
            }

            int variadicMin = intValue(argumentNode, "variadic_min", required ? 1 : 0);
            Integer variadicMax = nullableInt(argumentNode.get("variadic_max"));
            arguments.add(new ArgumentDef(
                    id,
                    displayName,
                    description,
                    type,
                    required,
                    variadic,
                    variadicMin,
                    variadicMax,
                    value(argumentNode.get("default")),
                    enumValues,
                    strings(array(argumentNode, "required_unless_flag"))
            ));
        }

        return arguments;
    }

    private List<ExclusiveGroupDef> parseGroups(ArrayNode groupsNode, String scopeName, Set<String> availableIds) {
        List<ExclusiveGroupDef> groups = new ArrayList<>();
        for (JsonNode groupNode : groupsNode) {
            String id = text(groupNode, "id");
            if (id == null || id.isBlank()) {
                throw new SpecError("Exclusive group in scope '" + scopeName + "' is missing 'id'");
            }
            List<String> flagIds = strings(array(groupNode, "flag_ids"));
            if (flagIds.isEmpty()) {
                throw new SpecError("Exclusive group '" + id + "' in scope '" + scopeName + "' has empty 'flag_ids'");
            }
            for (String refId : flagIds) {
                if (!availableIds.contains(refId)) {
                    throw new SpecError(
                            "Exclusive group '" + id + "' in scope '" + scopeName + "' references unknown flag '" + refId + "'"
                    );
                }
            }
            groups.add(new ExclusiveGroupDef(id, flagIds, booleanValue(groupNode, "required", false)));
        }
        return groups;
    }

    private void validateRequiresCycles(List<FlagDef> flags, String scopeName) {
        Set<String> ids = ids(flags);
        for (FlagDef flag : flags) {
            Set<String> visited = new HashSet<>();
            ArrayDeque<String> stack = new ArrayDeque<>();
            stack.push(flag.id());
            while (!stack.isEmpty()) {
                String current = stack.pop();
                if (!visited.add(current)) {
                    continue;
                }
                for (FlagDef candidate : flags) {
                    if (candidate.id().equals(current)) {
                        for (String dependency : candidate.requires()) {
                            if (!ids.contains(dependency)) {
                                continue;
                            }
                            if (dependency.equals(flag.id())) {
                                throw new SpecError(
                                        "Circular 'requires' dependency detected in scope '" + scopeName
                                                + "'. Check for flags that mutually require each other."
                                );
                            }
                            stack.push(dependency);
                        }
                    }
                }
            }
        }
    }

    private static String text(JsonNode node, String fieldName) {
        if (node == null) {
            return null;
        }
        JsonNode child = node.get(fieldName);
        if (child == null || child.isNull() || child.isMissingNode()) {
            return null;
        }
        return child.asText();
    }

    private static ArrayNode array(JsonNode node, String fieldName) {
        if (node == null) {
            return MAPPER.createArrayNode();
        }
        JsonNode child = node.get(fieldName);
        if (child == null || child.isNull() || child.isMissingNode()) {
            return MAPPER.createArrayNode();
        }
        if (!child.isArray()) {
            throw new SpecError("Field '" + fieldName + "' must be a JSON array");
        }
        return (ArrayNode) child;
    }

    private static String defaulted(String value, String defaultValue) {
        return value == null ? defaultValue : value;
    }

    private static boolean booleanValue(JsonNode node, String fieldName, boolean defaultValue) {
        if (node == null) {
            return defaultValue;
        }
        JsonNode child = node.get(fieldName);
        if (child == null || child.isNull() || child.isMissingNode()) {
            return defaultValue;
        }
        return child.asBoolean(defaultValue);
    }

    private static int intValue(JsonNode node, String fieldName, int defaultValue) {
        if (node == null) {
            return defaultValue;
        }
        JsonNode child = node.get(fieldName);
        if (child == null || child.isNull() || child.isMissingNode()) {
            return defaultValue;
        }
        return child.asInt(defaultValue);
    }

    private static Integer nullableInt(JsonNode node) {
        if (node == null || node.isNull() || node.isMissingNode()) {
            return null;
        }
        return node.asInt();
    }

    private static Set<String> ids(List<FlagDef> flags) {
        Set<String> ids = new LinkedHashSet<>();
        for (FlagDef flag : flags) {
            ids.add(flag.id());
        }
        return ids;
    }

    private static Set<String> collectIds(ArrayNode nodes, String fieldName) {
        Set<String> ids = new LinkedHashSet<>();
        for (JsonNode node : nodes) {
            String id = text(node, fieldName);
            if (id != null) {
                ids.add(id);
            }
        }
        return ids;
    }

    private static List<String> strings(ArrayNode arrayNode) {
        List<String> values = new ArrayList<>();
        for (JsonNode item : arrayNode) {
            if (!item.isNull()) {
                values.add(item.asText());
            }
        }
        return values;
    }

    private static Object value(JsonNode node) {
        if (node == null || node.isNull() || node.isMissingNode()) {
            return null;
        }
        if (node.isBoolean()) {
            return node.asBoolean();
        }
        if (node.isIntegralNumber()) {
            return node.asLong();
        }
        if (node.isFloatingPointNumber()) {
            return node.asDouble();
        }
        if (node.isArray()) {
            List<Object> values = new ArrayList<>();
            for (JsonNode child : node) {
                values.add(value(child));
            }
            return List.copyOf(values);
        }
        return node.asText();
    }

    private record ScopeParts(
            List<FlagDef> flags,
            List<ArgumentDef> arguments,
            List<CommandDef> commands,
            List<ExclusiveGroupDef> groups
    ) {
    }
}
