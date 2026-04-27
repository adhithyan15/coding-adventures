package com.codingadventures.clibuilder

internal class TokenClassifier(activeFlags: List<FlagDef>) {
    private val byShort = activeFlags.mapNotNull { it.shortName?.let { short -> short to it } }.toMap()
    private val byLong = activeFlags.mapNotNull { it.longName?.let { long -> long to it } }.toMap()
    private val bySingleDashLong = activeFlags.mapNotNull { it.singleDashLong?.let { single -> single to it } }.toMap()

    fun classify(token: String): TokenEvent = when {
        token == "--" -> TokenEvent(TokenEventType.END_OF_FLAGS, token = token)
        token.startsWith("--") -> classifyLong(token)
        token == "-" -> TokenEvent(TokenEventType.POSITIONAL, value = "-", token = token)
        token.startsWith("-") -> classifySingleDash(token)
        else -> TokenEvent(TokenEventType.POSITIONAL, value = token, token = token)
    }

    private fun classifyLong(token: String): TokenEvent {
        val body = token.removePrefix("--")
        val equalsIndex = body.indexOf('=')
        if (equalsIndex >= 0) {
            val name = body.substring(0, equalsIndex)
            val value = body.substring(equalsIndex + 1)
            val flag = byLong[name] ?: return TokenEvent(TokenEventType.UNKNOWN_FLAG, token = token)
            return TokenEvent(TokenEventType.LONG_FLAG_WITH_VALUE, name = name, value = value, flagDef = flag, token = token)
        }

        val flag = byLong[body] ?: return TokenEvent(TokenEventType.UNKNOWN_FLAG, token = token)
        return TokenEvent(TokenEventType.LONG_FLAG, name = body, flagDef = flag, token = token)
    }

    private fun classifySingleDash(token: String): TokenEvent {
        val suffix = token.removePrefix("-")
        bySingleDashLong[suffix]?.let { return TokenEvent(TokenEventType.SINGLE_DASH_LONG, name = suffix, flagDef = it, token = token) }

        if (suffix.length == 1) {
            val flag = byShort[suffix] ?: return TokenEvent(TokenEventType.UNKNOWN_FLAG, token = token)
            return TokenEvent(TokenEventType.SHORT_FLAG, name = suffix, flagDef = flag, token = token)
        }

        val firstChar = suffix.take(1)
        val firstFlag = byShort[firstChar]
        if (firstFlag != null) {
            if (!isValuelessType(firstFlag.type)) {
                val remainder = suffix.drop(1)
                return if (remainder.isNotEmpty()) {
                    TokenEvent(TokenEventType.SHORT_FLAG_WITH_VALUE, name = firstChar, value = remainder, flagDef = firstFlag, token = token)
                } else {
                    TokenEvent(TokenEventType.SHORT_FLAG, name = firstChar, flagDef = firstFlag, token = token)
                }
            }
            if (suffix.drop(1).isNotEmpty()) {
                return classifyStacked(suffix, token)
            }
        }

        return classifyStacked(suffix, token)
    }

    private fun classifyStacked(suffix: String, token: String): TokenEvent {
        val chars = mutableListOf<String>()
        val flags = mutableListOf<FlagDef>()
        var trailingValue: String? = null

        for (index in suffix.indices) {
            val char = suffix.substring(index, index + 1)
            val flag = byShort[char] ?: return TokenEvent(TokenEventType.UNKNOWN_FLAG, token = token)
            val isLast = index == suffix.lastIndex
            chars += char
            flags += flag
            if (!isValuelessType(flag.type)) {
                if (!isLast) {
                    trailingValue = suffix.substring(index + 1)
                }
                break
            }
        }

        if (chars.size == 1 && trailingValue == null && isValuelessType(flags.first().type)) {
            return TokenEvent(TokenEventType.SHORT_FLAG, name = chars.first(), flagDef = flags.first(), token = token)
        }

        return TokenEvent(TokenEventType.STACKED_FLAGS, chars = chars, flagDefs = flags, trailingValue = trailingValue, token = token)
    }

    companion object {
        fun isValuelessType(type: String?): Boolean = type == "boolean" || type == "count"
    }
}

internal enum class TokenEventType {
    END_OF_FLAGS,
    LONG_FLAG,
    LONG_FLAG_WITH_VALUE,
    SINGLE_DASH_LONG,
    SHORT_FLAG,
    SHORT_FLAG_WITH_VALUE,
    STACKED_FLAGS,
    POSITIONAL,
    UNKNOWN_FLAG,
}

internal data class TokenEvent(
    val type: TokenEventType,
    val name: String? = null,
    val value: String? = null,
    val flagDef: FlagDef? = null,
    val chars: List<String> = emptyList(),
    val flagDefs: List<FlagDef> = emptyList(),
    val trailingValue: String? = null,
    val token: String,
)
