package com.codingadventures.clibuilder

import java.nio.file.Path
import kotlin.math.min

class Parser(specFilePath: String, private val argv: List<String>) {
    private val spec = SpecLoader().load(specFilePath)

    constructor(specFilePath: Path, argv: List<String>) : this(specFilePath.toString(), argv)

    fun parse(): ParseOutcome {
        if (argv.isEmpty()) {
            throw ParseErrors(listOf(ParseError("missing_required_argument", "argv must not be empty (must include program name)")))
        }

        val program = argv.first()
        val tokens = argv.drop(1)
        val routing = phase1Routing(program, tokens)
        val activeFlags = collectActiveFlags(routing.commandPath, routing.leaf)

        checkQuickHelpVersion(tokens, activeFlags, routing.commandPath)?.let { return it }
        if (routing.errors.isNotEmpty()) {
            throw ParseErrors(routing.errors)
        }

        val scan = phase2Scanning(tokens, routing, activeFlags)
        scan.specialResult?.let { return it }

        val errors = scan.errors.toMutableList()
        val arguments = PositionalResolver(routing.leaf.arguments).resolve(scan.positionalTokens, scan.parsedFlags, routing.commandPath)
        errors += arguments.errors
        errors += FlagValidator(activeFlags, routing.leaf.mutuallyExclusiveGroups).validate(scan.parsedFlags, routing.commandPath)
        if (errors.isNotEmpty()) {
            throw ParseErrors(errors)
        }

        return ParseResult(
            program = program,
            commandPath = routing.commandPath,
            flags = applyFlagDefaults(activeFlags, scan.parsedFlags),
            arguments = arguments.arguments,
            explicitFlags = scan.explicitFlags,
        )
    }

    private fun phase1Routing(program: String, tokens: List<String>): RoutingResult {
        val commandPath = mutableListOf(program)
        var current: ScopeDef = spec
        val errors = mutableListOf<ParseError>()
        val consumedIndices = linkedSetOf<Int>()

        var scopeFlags = collectActiveFlags(commandPath, current)
        var flagLookup = buildFlagLookup(scopeFlags)
        var index = 0
        while (index < tokens.size) {
            val token = tokens[index]
            if (token == "--") break
            if (token.startsWith("-")) {
                index += skipFlag(token, flagLookup)
                continue
            }

            val matched = resolveCommand(current.commands, token)
            if (matched != null) {
                commandPath += matched.name
                consumedIndices += index
                current = matched
                scopeFlags = collectActiveFlags(commandPath, current)
                flagLookup = buildFlagLookup(scopeFlags)
                index += 1
                continue
            }

            if (spec.parsingMode == "subcommand_first") {
                val validNames = current.commands.flatMap { listOf(it.name) + it.aliases }
                errors += ParseError("unknown_command", "Unknown command '$token'", fuzzySuggest(token, validNames), commandPath.toList())
            }
            break
        }

        return RoutingResult(commandPath.toList(), current, errors, consumedIndices)
    }

    private fun phase2Scanning(tokens: List<String>, routing: RoutingResult, activeFlags: List<FlagDef>): ScanResult {
        val classifier = TokenClassifier(activeFlags)
        val parsedFlags = linkedMapOf<String, Any?>()
        val flagCounts = mutableMapOf<String, Int>()
        val positionalTokens = mutableListOf<String>()
        val errors = mutableListOf<ParseError>()
        val explicitFlags = mutableListOf<String>()
        val consumedIndices = routing.consumedIndices.toMutableSet()

        var pendingFlag: FlagDef? = null
        var endOfFlags = false
        var traditionalFirstDone = spec.parsingMode != "traditional"

        tokens.forEachIndexed { index, token ->
            if (index in consumedIndices) return@forEachIndexed
            if (endOfFlags) {
                positionalTokens += token
                return@forEachIndexed
            }

            pendingFlag?.let { flag ->
                val coercion = PositionalResolver.coerceValue(
                    raw = token,
                    argType = flag.type,
                    enumValues = flag.enumValues,
                    context = routing.commandPath,
                    argName = flag.longName ?: flag.id,
                )
                if (coercion.error != null) {
                    errors += coercion.error
                } else {
                    setFlag(parsedFlags, flagCounts, flag, coercion.value, errors, routing.commandPath, explicitFlags)
                }
                pendingFlag = null
                return@forEachIndexed
            }

            if (!traditionalFirstDone && !token.startsWith("-")) {
                traditionalFirstDone = true
                val traditionalFlags = tryTraditional(token, activeFlags)
                if (traditionalFlags != null) {
                    traditionalFlags.forEach { flag ->
                        setFlag(parsedFlags, flagCounts, flag, true, errors, routing.commandPath, explicitFlags)
                    }
                    return@forEachIndexed
                }
            }

            if (!token.startsWith("-") || token == "-") {
                if (spec.parsingMode == "posix") endOfFlags = true
                positionalTokens += token
                return@forEachIndexed
            }

            val event = classifier.classify(token)
            when (event.type) {
                TokenEventType.END_OF_FLAGS -> endOfFlags = true
                TokenEventType.POSITIONAL -> {
                    if (spec.parsingMode == "posix") endOfFlags = true
                    positionalTokens += requireNotNull(event.value)
                }

                TokenEventType.LONG_FLAG,
                TokenEventType.SINGLE_DASH_LONG,
                TokenEventType.SHORT_FLAG,
                -> {
                    val flag = requireNotNull(event.flagDef)
                    handleBuiltin(flag, routing.commandPath)?.let {
                        return ScanResult(parsedFlags, positionalTokens, errors, explicitFlags, it)
                    }
                    when {
                        TokenClassifier.isValuelessType(flag.type) -> {
                            setFlag(parsedFlags, flagCounts, flag, true, errors, routing.commandPath, explicitFlags)
                        }

                        flag.defaultWhenPresent != null -> {
                            val nextIndex = nextUnconsumedIndex(tokens, consumedIndices, index + 1)
                            if (nextIndex < tokens.size && tokens[nextIndex] in flag.enumValues) {
                                consumedIndices += nextIndex
                                setFlag(parsedFlags, flagCounts, flag, tokens[nextIndex], errors, routing.commandPath, explicitFlags)
                            } else {
                                setFlag(parsedFlags, flagCounts, flag, flag.defaultWhenPresent, errors, routing.commandPath, explicitFlags)
                            }
                        }

                        else -> pendingFlag = flag
                    }
                }

                TokenEventType.LONG_FLAG_WITH_VALUE,
                TokenEventType.SHORT_FLAG_WITH_VALUE,
                -> {
                    val flag = requireNotNull(event.flagDef)
                    val coercion = PositionalResolver.coerceValue(
                        raw = requireNotNull(event.value),
                        argType = flag.type,
                        enumValues = flag.enumValues,
                        context = routing.commandPath,
                        argName = flag.longName ?: flag.id,
                    )
                    if (coercion.error != null) errors += coercion.error else setFlag(parsedFlags, flagCounts, flag, coercion.value, errors, routing.commandPath, explicitFlags)
                }

                TokenEventType.STACKED_FLAGS -> {
                    event.flagDefs.forEachIndexed { stackIndex, flag ->
                        val isLast = stackIndex == event.flagDefs.lastIndex
                        when {
                            TokenClassifier.isValuelessType(flag.type) -> setFlag(parsedFlags, flagCounts, flag, true, errors, routing.commandPath, explicitFlags)
                            isLast && event.trailingValue != null -> {
                                val coercion = PositionalResolver.coerceValue(
                                    raw = event.trailingValue,
                                    argType = flag.type,
                                    enumValues = flag.enumValues,
                                    context = routing.commandPath,
                                    argName = flag.shortName ?: flag.id,
                                )
                                if (coercion.error != null) errors += coercion.error else setFlag(parsedFlags, flagCounts, flag, coercion.value, errors, routing.commandPath, explicitFlags)
                            }

                            else -> pendingFlag = flag
                        }
                    }
                }

                TokenEventType.UNKNOWN_FLAG -> errors += ParseError(
                    errorType = "unknown_flag",
                    message = "Unknown flag '${event.token}'",
                    suggestion = withDashes(fuzzySuggest(event.token.removePrefix("-").removePrefix("-"), allFlagNames(activeFlags))),
                    context = routing.commandPath,
                )
            }
        }

        pendingFlag?.let { flag ->
            errors += ParseError("missing_flag_value", "${flag.display()} requires a value", context = routing.commandPath)
        }

        return ScanResult(parsedFlags, positionalTokens, errors, explicitFlags, null)
    }

    private fun checkQuickHelpVersion(tokens: List<String>, activeFlags: List<FlagDef>, commandPath: List<String>): ParseOutcome? {
        val userDefinedShortH = activeFlags.any { it.shortName == "h" && it.id != "__help__" }
        tokens.forEach { token ->
            if (spec.builtinFlags.help && token == "--help") {
                return HelpResult(HelpGenerator(spec, commandPath).generate(), commandPath)
            }
            if (spec.builtinFlags.help && token == "-h" && !userDefinedShortH) {
                return HelpResult(HelpGenerator(spec, commandPath).generate(), commandPath)
            }
            if (spec.builtinFlags.version && spec.version != null && token == "--version") {
                return VersionResult(spec.version)
            }
        }
        return null
    }

    private fun handleBuiltin(flag: FlagDef, commandPath: List<String>): ParseOutcome? = when {
        spec.builtinFlags.help && (flag.longName == "help" || flag.id == "__help__") ->
            HelpResult(HelpGenerator(spec, commandPath).generate(), commandPath)

        spec.builtinFlags.version && spec.version != null && (flag.longName == "version" || flag.id == "__version__") ->
            VersionResult(spec.version)

        else -> null
    }

    private fun collectActiveFlags(commandPath: List<String>, leaf: ScopeDef): List<FlagDef> {
        val flags = mutableListOf<FlagDef>()
        val seen = linkedSetOf<String>()
        if (leaf !is CommandDef || leaf.inheritGlobalFlags) {
            spec.globalFlags.forEach { if (seen.add(it.id)) flags += it }
        }

        var current: ScopeDef = spec
        commandPath.drop(1).forEach { name ->
            val next = resolveCommand(current.commands, name) ?: return@forEach
            next.flags.forEach { if (seen.add(it.id)) flags += it }
            current = next
        }

        if (leaf is CliSpec) {
            leaf.flags.forEach { if (seen.add(it.id)) flags += it }
        }
        return flags
    }

    private fun buildFlagLookup(flags: List<FlagDef>): Map<String, FlagDef> = buildMap {
        flags.forEach { flag ->
            flag.longName?.let { put(it, flag) }
            flag.shortName?.let { put(it, flag) }
            flag.singleDashLong?.let { put(it, flag) }
        }
    }

    private fun skipFlag(token: String, flagLookup: Map<String, FlagDef>): Int {
        if (token.startsWith("--") && "=" in token) return 1
        if (token.startsWith("--")) {
            val flag = flagLookup[token.removePrefix("--")]
            return if (flag != null && !TokenClassifier.isValuelessType(flag.type) && flag.defaultWhenPresent == null) 2 else 1
        }
        if (token.startsWith("-") && token.length == 2) {
            val flag = flagLookup[token.removePrefix("-")]
            if (flag != null && !TokenClassifier.isValuelessType(flag.type)) return 2
        }
        if (token.startsWith("-") && !token.startsWith("--")) {
            val flag = flagLookup[token.removePrefix("-")]
            if (flag != null && !TokenClassifier.isValuelessType(flag.type)) return 2
        }
        return 1
    }

    private fun resolveCommand(commands: List<CommandDef>, token: String): CommandDef? =
        commands.firstOrNull { it.name == token || token in it.aliases }

    private fun setFlag(
        parsedFlags: MutableMap<String, Any?>,
        flagCounts: MutableMap<String, Int>,
        flag: FlagDef,
        value: Any?,
        errors: MutableList<ParseError>,
        context: List<String>,
        explicitFlags: MutableList<String>,
    ) {
        explicitFlags += flag.id
        val count = flagCounts.getOrDefault(flag.id, 0) + 1
        flagCounts[flag.id] = count

        if (flag.type == "count") {
            parsedFlags[flag.id] = ((parsedFlags[flag.id] as? Number)?.toLong() ?: 0L) + 1L
            return
        }
        if (flag.repeatable) {
            @Suppress("UNCHECKED_CAST")
            val values = parsedFlags.getOrPut(flag.id) { mutableListOf<Any?>() } as MutableList<Any?>
            values += value
            return
        }
        if (count > 1) {
            errors += ParseError("duplicate_flag", "${flag.display()} specified more than once", context = context)
            return
        }
        parsedFlags[flag.id] = value
    }

    private fun applyFlagDefaults(activeFlags: List<FlagDef>, parsedFlags: Map<String, Any?>): Map<String, Any?> =
        buildMap {
            putAll(parsedFlags)
            activeFlags.forEach { flag ->
                if (flag.id !in this) {
                    put(
                        flag.id,
                        when (flag.type) {
                            "boolean" -> flag.defaultValue ?: false
                            "count" -> flag.defaultValue ?: 0L
                            else -> flag.defaultValue
                        },
                    )
                }
            }
        }

    private fun allFlagNames(activeFlags: List<FlagDef>): List<String> =
        buildList {
            activeFlags.forEach { flag ->
                flag.longName?.let { add(it) }
                flag.shortName?.let { add(it) }
                flag.singleDashLong?.let { add(it) }
            }
        }

    private fun tryTraditional(token: String, activeFlags: List<FlagDef>): List<FlagDef>? {
        val byShort = activeFlags.mapNotNull { it.shortName?.let { short -> short to it } }.toMap()
        return buildList {
            token.forEach { char ->
                val flag = byShort[char.toString()] ?: return null
                if (!TokenClassifier.isValuelessType(flag.type)) return null
                add(flag)
            }
        }
    }

    private fun nextUnconsumedIndex(tokens: List<String>, consumedIndices: Set<Int>, start: Int): Int {
        var index = start
        while (index < tokens.size && index in consumedIndices) {
            index += 1
        }
        return index
    }

    private fun fuzzySuggest(token: String, candidates: List<String>): String? {
        var best: String? = null
        var bestDistance = 3
        candidates.forEach { candidate ->
            val distance = levenshtein(token, candidate)
            if (distance < bestDistance) {
                best = candidate
                bestDistance = distance
            }
        }
        return best?.takeIf { bestDistance <= 2 }
    }

    private fun withDashes(suggestion: String?): String? =
        suggestion?.let { if (it.length == 1) "-$it" else "--$it" }

    private fun levenshtein(a: String, b: String): Int {
        if (a == b) return 0
        if (a.isEmpty()) return b.length
        if (b.isEmpty()) return a.length
        if (a.length > b.length) return levenshtein(b, a)

        var previous = IntArray(a.length + 1) { it }
        for (row in b.indices) {
            val current = IntArray(a.length + 1)
            current[0] = row + 1
            for (column in a.indices) {
                val insert = current[column] + 1
                val delete = previous[column + 1] + 1
                val replace = previous[column] + if (a[column] == b[row]) 0 else 1
                current[column + 1] = min(insert, min(delete, replace))
            }
            previous = current
        }
        return previous[a.length]
    }

    private data class RoutingResult(
        val commandPath: List<String>,
        val leaf: ScopeDef,
        val errors: List<ParseError>,
        val consumedIndices: Set<Int>,
    )

    private data class ScanResult(
        val parsedFlags: Map<String, Any?>,
        val positionalTokens: List<String>,
        val errors: List<ParseError>,
        val explicitFlags: List<String>,
        val specialResult: ParseOutcome?,
    )
}
