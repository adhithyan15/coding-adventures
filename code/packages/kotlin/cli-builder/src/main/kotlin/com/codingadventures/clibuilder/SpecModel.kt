package com.codingadventures.clibuilder

internal sealed interface ScopeDef {
    val flags: List<FlagDef>
    val arguments: List<ArgumentDef>
    val commands: List<CommandDef>
    val mutuallyExclusiveGroups: List<ExclusiveGroupDef>
    val description: String
}

internal data class CliSpec(
    val name: String,
    val displayName: String,
    override val description: String,
    val version: String?,
    val parsingMode: String,
    val builtinFlags: BuiltinFlags,
    val globalFlags: List<FlagDef>,
    override val flags: List<FlagDef>,
    override val arguments: List<ArgumentDef>,
    override val commands: List<CommandDef>,
    override val mutuallyExclusiveGroups: List<ExclusiveGroupDef>,
) : ScopeDef

internal data class BuiltinFlags(val help: Boolean, val version: Boolean)

internal data class FlagDef(
    val id: String,
    val shortName: String?,
    val longName: String?,
    val singleDashLong: String?,
    val description: String,
    val type: String,
    val required: Boolean,
    val defaultValue: Any?,
    val valueName: String?,
    val enumValues: List<String>,
    val conflictsWith: List<String>,
    val requires: List<String>,
    val requiredUnless: List<String>,
    val repeatable: Boolean,
    val defaultWhenPresent: String?,
) {
    fun display(): String {
        val parts = buildList {
            shortName?.let { add("-$it") }
            longName?.let { add("--$it") }
            singleDashLong?.let { add("-$it") }
        }
        return if (parts.isEmpty()) "--$id" else parts.joinToString("/")
    }
}

internal data class ArgumentDef(
    val id: String,
    val displayName: String,
    val description: String,
    val type: String,
    val required: Boolean,
    val variadic: Boolean,
    val variadicMin: Int,
    val variadicMax: Int?,
    val defaultValue: Any?,
    val enumValues: List<String>,
    val requiredUnlessFlag: List<String>,
)

internal data class CommandDef(
    val id: String,
    val name: String,
    override val description: String,
    val aliases: List<String>,
    val inheritGlobalFlags: Boolean,
    override val flags: List<FlagDef>,
    override val arguments: List<ArgumentDef>,
    override val commands: List<CommandDef>,
    override val mutuallyExclusiveGroups: List<ExclusiveGroupDef>,
) : ScopeDef

internal data class ExclusiveGroupDef(val id: String, val flagIds: List<String>, val required: Boolean)
