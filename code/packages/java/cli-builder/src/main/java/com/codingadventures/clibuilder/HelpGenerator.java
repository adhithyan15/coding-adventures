package com.codingadventures.clibuilder;

import java.util.ArrayList;
import java.util.List;

final class HelpGenerator {
    private static final int COLUMN_WIDTH = 28;

    private final CliSpec spec;
    private final List<String> commandPath;
    private final ScopeDef node;

    HelpGenerator(CliSpec spec, List<String> commandPath) {
        this.spec = spec;
        this.commandPath = List.copyOf(commandPath);
        this.node = resolveNode();
    }

    String generate() {
        List<String> sections = new ArrayList<>();
        sections.add(usageSection());
        sections.add("DESCRIPTION\n  " + node.description());

        if (!node.commands().isEmpty()) {
            sections.add(commandsSection());
        }
        if (!node.flags().isEmpty()) {
            sections.add(optionsSection("OPTIONS", node.flags()));
        }
        List<FlagDef> global = new ArrayList<>(spec.globalFlags());
        global.addAll(builtinFlags());
        if (!global.isEmpty()) {
            sections.add(optionsSection("GLOBAL OPTIONS", global));
        }
        if (!node.arguments().isEmpty()) {
            sections.add(argumentsSection());
        }
        return String.join("\n\n", sections);
    }

    private ScopeDef resolveNode() {
        ScopeDef current = spec;
        for (int index = 1; index < commandPath.size(); index += 1) {
            String name = commandPath.get(index);
            CommandDef next = null;
            for (CommandDef command : current.commands()) {
                if (command.name().equals(name)) {
                    next = command;
                    break;
                }
            }
            if (next == null) {
                return current;
            }
            current = next;
        }
        return current;
    }

    private String usageSection() {
        List<String> parts = new ArrayList<>(commandPath);
        if (!node.flags().isEmpty() || !spec.globalFlags().isEmpty() || !builtinFlags().isEmpty()) {
            parts.add("[OPTIONS]");
        }
        if (!node.commands().isEmpty()) {
            parts.add("[COMMAND]");
        }
        for (ArgumentDef argument : node.arguments()) {
            parts.add(argumentUsage(argument));
        }
        return "USAGE\n  " + String.join(" ", parts);
    }

    private String commandsSection() {
        List<String> lines = new ArrayList<>();
        lines.add("COMMANDS");
        int width = 12;
        for (CommandDef command : node.commands()) {
            width = Math.max(width, command.name().length() + 2);
        }
        for (CommandDef command : node.commands()) {
            lines.add("  " + padRight(command.name(), width) + command.description());
        }
        return String.join("\n", lines);
    }

    private String optionsSection(String heading, List<FlagDef> flags) {
        List<String> lines = new ArrayList<>();
        lines.add(heading);
        for (FlagDef flag : flags) {
            String signature = flagSignature(flag);
            String description = flag.description();
            if (flag.defaultValue() != null && !flag.required()) {
                description = description + " [default: " + flag.defaultValue() + "]";
            }
            if (signature.length() < COLUMN_WIDTH - 2) {
                lines.add("  " + padRight(signature, COLUMN_WIDTH) + description);
            } else {
                lines.add("  " + signature);
                lines.add("  " + " ".repeat(COLUMN_WIDTH) + description);
            }
        }
        return String.join("\n", lines);
    }

    private String argumentsSection() {
        List<String> lines = new ArrayList<>();
        lines.add("ARGUMENTS");
        for (ArgumentDef argument : node.arguments()) {
            String usage = argumentUsage(argument);
            String description = argument.description() + " " + (argument.required() ? "Required." : "Optional.");
            if (usage.length() < COLUMN_WIDTH - 2) {
                lines.add("  " + padRight(usage, COLUMN_WIDTH) + description);
            } else {
                lines.add("  " + usage);
                lines.add("  " + " ".repeat(COLUMN_WIDTH) + description);
            }
        }
        return String.join("\n", lines);
    }

    private String flagSignature(FlagDef flag) {
        List<String> parts = new ArrayList<>();
        if (flag.shortName() != null) {
            parts.add("-" + flag.shortName());
        }
        if (flag.longName() != null) {
            parts.add("--" + flag.longName());
        }
        if (flag.singleDashLong() != null) {
            parts.add("-" + flag.singleDashLong());
        }
        String signature = String.join(", ", parts);
        if (!TokenClassifier.isValuelessType(flag.type())) {
            String valueName = flag.valueName() == null ? flag.type().toUpperCase() : flag.valueName();
            signature += flag.defaultWhenPresent() == null ? " <" + valueName + ">" : " [=" + valueName + "]";
        }
        return signature;
    }

    private String argumentUsage(ArgumentDef argument) {
        String displayName = argument.displayName();
        if (argument.variadic()) {
            return argument.required() ? "<" + displayName + "...>" : "[" + displayName + "...]";
        }
        return argument.required() ? "<" + displayName + ">" : "[" + displayName + "]";
    }

    private List<FlagDef> builtinFlags() {
        List<FlagDef> builtins = new ArrayList<>();
        if (spec.builtinFlags().help()) {
            builtins.add(new FlagDef("__help__", "h", "help", null, "Show this help message and exit.", "boolean", false, null, null, List.of(), List.of(), List.of(), List.of(), false, null));
        }
        if (spec.builtinFlags().version() && spec.version() != null) {
            builtins.add(new FlagDef("__version__", null, "version", null, "Show version and exit.", "boolean", false, null, null, List.of(), List.of(), List.of(), List.of(), false, null));
        }
        return builtins;
    }

    private static String padRight(String value, int width) {
        if (value.length() >= width) {
            return value;
        }
        return value + " ".repeat(width - value.length());
    }
}
