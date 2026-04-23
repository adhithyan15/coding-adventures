package com.codingadventures.clibuilder;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

final class TokenClassifier {
    private final Map<String, FlagDef> byShort = new HashMap<>();
    private final Map<String, FlagDef> byLong = new HashMap<>();
    private final Map<String, FlagDef> bySingleDashLong = new HashMap<>();

    TokenClassifier(List<FlagDef> activeFlags) {
        for (FlagDef flag : activeFlags) {
            if (flag.shortName() != null) {
                byShort.put(flag.shortName(), flag);
            }
            if (flag.longName() != null) {
                byLong.put(flag.longName(), flag);
            }
            if (flag.singleDashLong() != null) {
                bySingleDashLong.put(flag.singleDashLong(), flag);
            }
        }
    }

    TokenEvent classify(String token) {
        if ("--".equals(token)) {
            return new TokenEvent(TokenEventType.END_OF_FLAGS, null, null, null, List.of(), List.of(), null, token);
        }
        if (token.startsWith("--")) {
            return classifyLong(token);
        }
        if ("-".equals(token)) {
            return new TokenEvent(TokenEventType.POSITIONAL, null, "-", null, List.of(), List.of(), null, token);
        }
        if (token.startsWith("-")) {
            return classifySingleDash(token);
        }
        return new TokenEvent(TokenEventType.POSITIONAL, null, token, null, List.of(), List.of(), null, token);
    }

    static boolean isValuelessType(String type) {
        return "boolean".equals(type) || "count".equals(type);
    }

    private TokenEvent classifyLong(String token) {
        String body = token.substring(2);
        int equalsIndex = body.indexOf('=');
        if (equalsIndex >= 0) {
            String name = body.substring(0, equalsIndex);
            String value = body.substring(equalsIndex + 1);
            FlagDef flag = byLong.get(name);
            if (flag == null) {
                return new TokenEvent(TokenEventType.UNKNOWN_FLAG, null, null, null, List.of(), List.of(), null, token);
            }
            return new TokenEvent(TokenEventType.LONG_FLAG_WITH_VALUE, name, value, flag, List.of(), List.of(), null, token);
        }

        FlagDef flag = byLong.get(body);
        if (flag == null) {
            return new TokenEvent(TokenEventType.UNKNOWN_FLAG, null, null, null, List.of(), List.of(), null, token);
        }
        return new TokenEvent(TokenEventType.LONG_FLAG, body, null, flag, List.of(), List.of(), null, token);
    }

    private TokenEvent classifySingleDash(String token) {
        String suffix = token.substring(1);

        FlagDef singleDashLong = bySingleDashLong.get(suffix);
        if (singleDashLong != null) {
            return new TokenEvent(TokenEventType.SINGLE_DASH_LONG, suffix, null, singleDashLong, List.of(), List.of(), null, token);
        }

        if (suffix.length() == 1) {
            FlagDef flag = byShort.get(suffix);
            if (flag == null) {
                return new TokenEvent(TokenEventType.UNKNOWN_FLAG, null, null, null, List.of(), List.of(), null, token);
            }
            return new TokenEvent(TokenEventType.SHORT_FLAG, suffix, null, flag, List.of(), List.of(), null, token);
        }

        String firstChar = suffix.substring(0, 1);
        FlagDef firstFlag = byShort.get(firstChar);
        if (firstFlag != null) {
            if (!isValuelessType(firstFlag.type())) {
                String remainder = suffix.substring(1);
                if (!remainder.isEmpty()) {
                    return new TokenEvent(TokenEventType.SHORT_FLAG_WITH_VALUE, firstChar, remainder, firstFlag, List.of(), List.of(), null, token);
                }
                return new TokenEvent(TokenEventType.SHORT_FLAG, firstChar, null, firstFlag, List.of(), List.of(), null, token);
            }
            if (!suffix.substring(1).isEmpty()) {
                return classifyStacked(suffix, token);
            }
        }

        return classifyStacked(suffix, token);
    }

    private TokenEvent classifyStacked(String suffix, String token) {
        List<String> chars = new ArrayList<>();
        List<FlagDef> flags = new ArrayList<>();
        String trailingValue = null;

        for (int index = 0; index < suffix.length(); index += 1) {
            String ch = suffix.substring(index, index + 1);
            FlagDef flag = byShort.get(ch);
            if (flag == null) {
                return new TokenEvent(TokenEventType.UNKNOWN_FLAG, null, null, null, List.of(), List.of(), null, token);
            }

            boolean isLast = index == suffix.length() - 1;
            if (isValuelessType(flag.type())) {
                chars.add(ch);
                flags.add(flag);
                continue;
            }

            chars.add(ch);
            flags.add(flag);
            if (!isLast) {
                trailingValue = suffix.substring(index + 1);
            }
            break;
        }

        if (chars.size() == 1 && trailingValue == null && isValuelessType(flags.getFirst().type())) {
            return new TokenEvent(TokenEventType.SHORT_FLAG, chars.getFirst(), null, flags.getFirst(), List.of(), List.of(), null, token);
        }

        return new TokenEvent(TokenEventType.STACKED_FLAGS, null, null, null, List.copyOf(chars), List.copyOf(flags), trailingValue, token);
    }
}

enum TokenEventType {
    END_OF_FLAGS,
    LONG_FLAG,
    LONG_FLAG_WITH_VALUE,
    SINGLE_DASH_LONG,
    SHORT_FLAG,
    SHORT_FLAG_WITH_VALUE,
    STACKED_FLAGS,
    POSITIONAL,
    UNKNOWN_FLAG
}

record TokenEvent(
        TokenEventType type,
        String name,
        String value,
        FlagDef flagDef,
        List<String> chars,
        List<FlagDef> flagDefs,
        String trailingValue,
        String token
) {
}
