package com.codingadventures.clibuilder;

import java.util.List;

sealed interface ScopeDef permits CliSpec, CommandDef {
    List<FlagDef> flags();
    List<ArgumentDef> arguments();
    List<CommandDef> commands();
    List<ExclusiveGroupDef> mutuallyExclusiveGroups();
    String description();
}

record CliSpec(
        String name,
        String displayName,
        String description,
        String version,
        String parsingMode,
        BuiltinFlags builtinFlags,
        List<FlagDef> globalFlags,
        List<FlagDef> flags,
        List<ArgumentDef> arguments,
        List<CommandDef> commands,
        List<ExclusiveGroupDef> mutuallyExclusiveGroups
) implements ScopeDef {
    CliSpec {
        globalFlags = List.copyOf(globalFlags);
        flags = List.copyOf(flags);
        arguments = List.copyOf(arguments);
        commands = List.copyOf(commands);
        mutuallyExclusiveGroups = List.copyOf(mutuallyExclusiveGroups);
    }
}

record BuiltinFlags(boolean help, boolean version) {
}

record FlagDef(
        String id,
        String shortName,
        String longName,
        String singleDashLong,
        String description,
        String type,
        boolean required,
        Object defaultValue,
        String valueName,
        List<String> enumValues,
        List<String> conflictsWith,
        List<String> requires,
        List<String> requiredUnless,
        boolean repeatable,
        String defaultWhenPresent
) {
    FlagDef {
        enumValues = List.copyOf(enumValues);
        conflictsWith = List.copyOf(conflictsWith);
        requires = List.copyOf(requires);
        requiredUnless = List.copyOf(requiredUnless);
    }

    String display() {
        StringBuilder builder = new StringBuilder();
        if (shortName != null) {
            builder.append("-").append(shortName);
        }
        if (longName != null) {
            if (!builder.isEmpty()) {
                builder.append("/");
            }
            builder.append("--").append(longName);
        }
        if (singleDashLong != null) {
            if (!builder.isEmpty()) {
                builder.append("/");
            }
            builder.append("-").append(singleDashLong);
        }
        return builder.isEmpty() ? "--" + id : builder.toString();
    }
}

record ArgumentDef(
        String id,
        String displayName,
        String description,
        String type,
        boolean required,
        boolean variadic,
        int variadicMin,
        Integer variadicMax,
        Object defaultValue,
        List<String> enumValues,
        List<String> requiredUnlessFlag
) {
    ArgumentDef {
        enumValues = List.copyOf(enumValues);
        requiredUnlessFlag = List.copyOf(requiredUnlessFlag);
    }
}

record CommandDef(
        String id,
        String name,
        String description,
        List<String> aliases,
        boolean inheritGlobalFlags,
        List<FlagDef> flags,
        List<ArgumentDef> arguments,
        List<CommandDef> commands,
        List<ExclusiveGroupDef> mutuallyExclusiveGroups
) implements ScopeDef {
    CommandDef {
        aliases = List.copyOf(aliases);
        flags = List.copyOf(flags);
        arguments = List.copyOf(arguments);
        commands = List.copyOf(commands);
        mutuallyExclusiveGroups = List.copyOf(mutuallyExclusiveGroups);
    }
}

record ExclusiveGroupDef(String id, List<String> flagIds, boolean required) {
    ExclusiveGroupDef {
        flagIds = List.copyOf(flagIds);
    }
}
