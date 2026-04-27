package com.codingadventures.clibuilder

internal class FlagValidator(
    activeFlags: List<FlagDef>,
    private val exclusiveGroups: List<ExclusiveGroupDef>,
) {
    private val flags = activeFlags
    private val byId = activeFlags.associateBy { it.id }
    private val requiresGraph = activeFlags.associate { it.id to it.requires }

    fun validate(parsedFlags: Map<String, Any?>, context: List<String>): List<ParseError> {
        val present = parsedFlags.filterValues { it != null && it != false }.keys
        return buildList {
            addAll(checkConflicts(present, context))
            addAll(checkRequires(present, context))
            addAll(checkRequired(present, parsedFlags, context))
            addAll(checkExclusiveGroups(present, context))
        }
    }

    private fun checkConflicts(present: Set<String>, context: List<String>): List<ParseError> {
        val seenPairs = mutableSetOf<Set<String>>()
        return buildList {
            present.forEach { flagId ->
                val flag = byId[flagId] ?: return@forEach
                flag.conflictsWith.forEach { otherId ->
                    if (otherId in present) {
                        val pair = setOf(flagId, otherId)
                        if (seenPairs.add(pair)) {
                            add(ParseError("conflicting_flags", "${flag.display()} and ${byId.getValue(otherId).display()} cannot be used together", context = context))
                        }
                    }
                }
            }
        }
    }

    private fun checkRequires(present: Set<String>, context: List<String>): List<ParseError> =
        buildList {
            present.forEach { flagId ->
                transitiveRequires(flagId).forEach { dependency ->
                    if (dependency !in present) {
                        add(ParseError("missing_dependency_flag", "${byId.getValue(flagId).display()} requires ${display(dependency)}", context = context))
                    }
                }
            }
        }

    private fun checkRequired(present: Set<String>, parsedFlags: Map<String, Any?>, context: List<String>): List<ParseError> =
        buildList {
            flags.forEach { flag ->
                if (!flag.required || flag.id in present) return@forEach
                val exempt = flag.requiredUnless.any { parsedFlags[it] != null && parsedFlags[it] != false }
                if (!exempt) {
                    add(ParseError("missing_required_flag", "${flag.display()} is required", context = context))
                }
            }
        }

    private fun checkExclusiveGroups(present: Set<String>, context: List<String>): List<ParseError> =
        buildList {
            exclusiveGroups.forEach { group ->
                val presentIds = group.flagIds.filter { it in present }
                val displays = group.flagIds.joinToString(", ") { display(it) }
                when {
                    presentIds.size > 1 -> add(ParseError("exclusive_group_violation", "Only one of $displays may be used at a time", context = context))
                    group.required && presentIds.isEmpty() -> add(ParseError("missing_exclusive_group", "One of $displays is required", context = context))
                }
            }
        }

    private fun transitiveRequires(start: String): Set<String> {
        val visited = linkedSetOf<String>()
        val queue = ArrayDeque(requiresGraph[start].orEmpty())
        while (queue.isNotEmpty()) {
            val current = queue.removeLast()
            if (visited.add(current)) {
                queue.addAll(requiresGraph[current].orEmpty())
            }
        }
        return visited
    }

    private fun display(flagId: String): String = byId[flagId]?.display() ?: "--$flagId"
}
