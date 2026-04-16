package com.codingadventures.clibuilder

internal class HelpGenerator(
    private val spec: CliSpec,
    private val commandPath: List<String>,
) {
    private val node: ScopeDef = resolveNode()

    fun generate(): String = buildList {
        add(usageSection())
        add("DESCRIPTION\n  ${node.description}")
        if (node.commands.isNotEmpty()) add(commandsSection())
        if (node.flags.isNotEmpty()) add(optionsSection("OPTIONS", node.flags))
        val globalFlags = spec.globalFlags + builtinFlags()
        if (globalFlags.isNotEmpty()) add(optionsSection("GLOBAL OPTIONS", globalFlags))
        if (node.arguments.isNotEmpty()) add(argumentsSection())
    }.joinToString("\n\n")

    private fun resolveNode(): ScopeDef {
        var current: ScopeDef = spec
        commandPath.drop(1).forEach { name ->
            val next = current.commands.firstOrNull { it.name == name } ?: return current
            current = next
        }
        return current
    }

    private fun usageSection(): String {
        val parts = mutableListOf<String>()
        parts += commandPath
        if (node.flags.isNotEmpty() || spec.globalFlags.isNotEmpty() || builtinFlags().isNotEmpty()) {
            parts += "[OPTIONS]"
        }
        if (node.commands.isNotEmpty()) {
            parts += "[COMMAND]"
        }
        node.arguments.forEach { parts += argumentUsage(it) }
        return "USAGE\n  ${parts.joinToString(" ")}"
    }

    private fun commandsSection(): String {
        val width = maxOf(12, node.commands.maxOfOrNull { it.name.length + 2 } ?: 12)
        return buildList {
            add("COMMANDS")
            node.commands.forEach { command -> add("  ${padRight(command.name, width)}${command.description}") }
        }.joinToString("\n")
    }

    private fun optionsSection(heading: String, flags: List<FlagDef>): String =
        buildList {
            add(heading)
            flags.forEach { flag ->
                val signature = flagSignature(flag)
                val description = if (flag.defaultValue != null && !flag.required) {
                    "${flag.description} [default: ${flag.defaultValue}]"
                } else {
                    flag.description
                }
                if (signature.length < COLUMN_WIDTH - 2) {
                    add("  ${padRight(signature, COLUMN_WIDTH)}$description")
                } else {
                    add("  $signature")
                    add("  ${" ".repeat(COLUMN_WIDTH)}$description")
                }
            }
        }.joinToString("\n")

    private fun argumentsSection(): String =
        buildList {
            add("ARGUMENTS")
            node.arguments.forEach { argument ->
                val usage = argumentUsage(argument)
                val description = "${argument.description} ${if (argument.required) "Required." else "Optional."}"
                if (usage.length < COLUMN_WIDTH - 2) {
                    add("  ${padRight(usage, COLUMN_WIDTH)}$description")
                } else {
                    add("  $usage")
                    add("  ${" ".repeat(COLUMN_WIDTH)}$description")
                }
            }
        }.joinToString("\n")

    private fun flagSignature(flag: FlagDef): String {
        val parts = buildList {
            flag.shortName?.let { add("-$it") }
            flag.longName?.let { add("--$it") }
            flag.singleDashLong?.let { add("-$it") }
        }
        var signature = parts.joinToString(", ")
        if (!TokenClassifier.isValuelessType(flag.type)) {
            val valueName = flag.valueName ?: flag.type.uppercase()
            signature += if (flag.defaultWhenPresent == null) " <$valueName>" else " [=$valueName]"
        }
        return signature
    }

    private fun argumentUsage(argument: ArgumentDef): String =
        if (argument.variadic) {
            if (argument.required) "<${argument.displayName}...>" else "[${argument.displayName}...]"
        } else {
            if (argument.required) "<${argument.displayName}>" else "[${argument.displayName}]"
        }

    private fun builtinFlags(): List<FlagDef> = buildList {
        if (spec.builtinFlags.help) {
            add(FlagDef("__help__", "h", "help", null, "Show this help message and exit.", "boolean", false, null, null, emptyList(), emptyList(), emptyList(), emptyList(), false, null))
        }
        if (spec.builtinFlags.version && spec.version != null) {
            add(FlagDef("__version__", null, "version", null, "Show version and exit.", "boolean", false, null, null, emptyList(), emptyList(), emptyList(), emptyList(), false, null))
        }
    }

    companion object {
        private const val COLUMN_WIDTH = 28

        private fun padRight(value: String, width: Int): String =
            if (value.length >= width) value else value + " ".repeat(width - value.length)
    }
}
