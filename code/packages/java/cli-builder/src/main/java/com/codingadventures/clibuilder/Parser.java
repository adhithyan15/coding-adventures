package com.codingadventures.clibuilder;

import java.nio.file.Path;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

/** Three-phase CLI parser backed by the repo's JSON CLI Builder spec. */
public final class Parser {
    private final CliSpec spec;
    private final List<String> argv;

    public Parser(String specFilePath, List<String> argv) {
        this(Path.of(specFilePath), argv);
    }

    public Parser(Path specFilePath, List<String> argv) {
        this.spec = new SpecLoader().load(specFilePath);
        this.argv = List.copyOf(argv);
    }

    public ParseOutcome parse() {
        if (argv.isEmpty()) {
            throw new ParseErrors(List.of(new ParseError(
                    "missing_required_argument",
                    "argv must not be empty (must include program name)",
                    null,
                    List.of()
            )));
        }

        String program = argv.getFirst();
        List<String> tokens = argv.subList(1, argv.size());

        RoutingResult routing = phase1Routing(program, tokens);
        List<FlagDef> activeFlags = collectActiveFlags(routing.commandPath(), routing.leaf());

        ParseOutcome quick = checkQuickHelpVersion(tokens, activeFlags, routing.commandPath());
        if (quick != null) {
            return quick;
        }
        if (!routing.errors().isEmpty()) {
            throw new ParseErrors(routing.errors());
        }

        ScanResult scan = phase2Scanning(tokens, routing, activeFlags);
        if (scan.specialResult() != null) {
            return scan.specialResult();
        }

        List<ParseError> errors = new ArrayList<>(scan.errors());
        PositionalResolver.Resolution arguments = new PositionalResolver(routing.leaf().arguments())
                .resolve(scan.positionalTokens(), scan.parsedFlags(), routing.commandPath());
        errors.addAll(arguments.errors());
        errors.addAll(new FlagValidator(activeFlags, routing.leaf().mutuallyExclusiveGroups())
                .validate(scan.parsedFlags(), routing.commandPath()));

        if (!errors.isEmpty()) {
            throw new ParseErrors(errors);
        }

        return new ParseResult(
                program,
                routing.commandPath(),
                applyFlagDefaults(activeFlags, scan.parsedFlags()),
                arguments.arguments(),
                scan.explicitFlags()
        );
    }

    private RoutingResult phase1Routing(String program, List<String> tokens) {
        List<String> commandPath = new ArrayList<>();
        commandPath.add(program);
        ScopeDef current = spec;
        List<ParseError> errors = new ArrayList<>();
        Set<Integer> consumedIndices = new LinkedHashSet<>();
        String parsingMode = spec.parsingMode();

        List<FlagDef> scopeFlags = collectActiveFlags(commandPath, current);
        Map<String, FlagDef> flagLookup = buildFlagLookup(scopeFlags);

        int index = 0;
        while (index < tokens.size()) {
            String token = tokens.get(index);
            if ("--".equals(token)) {
                break;
            }
            if (token.startsWith("-")) {
                index += skipFlag(token, flagLookup);
                continue;
            }

            CommandDef matched = resolveCommand(current.commands(), token);
            if (matched != null) {
                commandPath.add(matched.name());
                consumedIndices.add(index);
                current = matched;
                scopeFlags = collectActiveFlags(commandPath, current);
                flagLookup = buildFlagLookup(scopeFlags);
                index += 1;
                continue;
            }

            if ("subcommand_first".equals(parsingMode)) {
                List<String> validNames = new ArrayList<>();
                for (CommandDef command : current.commands()) {
                    validNames.add(command.name());
                    validNames.addAll(command.aliases());
                }
                errors.add(new ParseError(
                        "unknown_command",
                        "Unknown command '" + token + "'",
                        fuzzySuggest(token, validNames),
                        List.copyOf(commandPath)
                ));
            }
            break;
        }

        return new RoutingResult(List.copyOf(commandPath), current, errors, consumedIndices);
    }

    private ScanResult phase2Scanning(List<String> tokens, RoutingResult routing, List<FlagDef> activeFlags) {
        TokenClassifier classifier = new TokenClassifier(activeFlags);
        Map<String, Object> parsedFlags = new LinkedHashMap<>();
        Map<String, Integer> flagCounts = new HashMap<>();
        List<String> positionalTokens = new ArrayList<>();
        List<ParseError> errors = new ArrayList<>();
        List<String> explicitFlags = new ArrayList<>();
        Set<Integer> consumedIndices = new HashSet<>(routing.consumedIndices());

        FlagDef pendingFlag = null;
        boolean endOfFlags = false;
        boolean traditionalFirstDone = !"traditional".equals(spec.parsingMode());

        for (int index = 0; index < tokens.size(); index += 1) {
            if (consumedIndices.contains(index)) {
                continue;
            }

            String token = tokens.get(index);
            if (endOfFlags) {
                positionalTokens.add(token);
                continue;
            }

            if (pendingFlag != null) {
                PositionalResolver.CoercionResult coercion = PositionalResolver.coerceValue(
                        token,
                        pendingFlag.type(),
                        pendingFlag.enumValues(),
                        routing.commandPath(),
                        pendingFlag.longName() != null ? pendingFlag.longName() : pendingFlag.id()
                );
                if (coercion.error() != null) {
                    errors.add(coercion.error());
                } else {
                    setFlag(parsedFlags, flagCounts, pendingFlag, coercion.value(), errors, routing.commandPath(), explicitFlags);
                }
                pendingFlag = null;
                continue;
            }

            if (!traditionalFirstDone && !token.startsWith("-")) {
                traditionalFirstDone = true;
                List<FlagDef> traditionalFlags = tryTraditional(token, activeFlags);
                if (traditionalFlags != null) {
                    for (FlagDef flag : traditionalFlags) {
                        setFlag(parsedFlags, flagCounts, flag, Boolean.TRUE, errors, routing.commandPath(), explicitFlags);
                    }
                    continue;
                }
            }

            if (!token.startsWith("-") || "-".equals(token)) {
                if ("posix".equals(spec.parsingMode())) {
                    endOfFlags = true;
                }
                positionalTokens.add(token);
                continue;
            }

            TokenEvent event = classifier.classify(token);
            switch (event.type()) {
                case END_OF_FLAGS -> endOfFlags = true;
                case POSITIONAL -> {
                    if ("posix".equals(spec.parsingMode())) {
                        endOfFlags = true;
                    }
                    positionalTokens.add(event.value());
                }
                case LONG_FLAG, SINGLE_DASH_LONG, SHORT_FLAG -> {
                    FlagDef flag = event.flagDef();
                    ParseOutcome builtin = handleBuiltin(flag, routing.commandPath());
                    if (builtin != null) {
                        return new ScanResult(parsedFlags, positionalTokens, errors, explicitFlags, builtin);
                    }
                    if (TokenClassifier.isValuelessType(flag.type())) {
                        setFlag(parsedFlags, flagCounts, flag, Boolean.TRUE, errors, routing.commandPath(), explicitFlags);
                    } else if (flag.defaultWhenPresent() != null) {
                        int nextIndex = nextUnconsumedIndex(tokens, consumedIndices, index + 1);
                        if (nextIndex < tokens.size() && flag.enumValues().contains(tokens.get(nextIndex))) {
                            consumedIndices.add(nextIndex);
                            setFlag(parsedFlags, flagCounts, flag, tokens.get(nextIndex), errors, routing.commandPath(), explicitFlags);
                        } else {
                            setFlag(parsedFlags, flagCounts, flag, flag.defaultWhenPresent(), errors, routing.commandPath(), explicitFlags);
                        }
                    } else {
                        pendingFlag = flag;
                    }
                }
                case LONG_FLAG_WITH_VALUE, SHORT_FLAG_WITH_VALUE -> {
                    FlagDef flag = event.flagDef();
                    PositionalResolver.CoercionResult coercion = PositionalResolver.coerceValue(
                            event.value(),
                            flag.type(),
                            flag.enumValues(),
                            routing.commandPath(),
                            flag.longName() != null ? flag.longName() : flag.id()
                    );
                    if (coercion.error() != null) {
                        errors.add(coercion.error());
                    } else {
                        setFlag(parsedFlags, flagCounts, flag, coercion.value(), errors, routing.commandPath(), explicitFlags);
                    }
                }
                case STACKED_FLAGS -> {
                    for (int stackIndex = 0; stackIndex < event.flagDefs().size(); stackIndex += 1) {
                        FlagDef flag = event.flagDefs().get(stackIndex);
                        boolean isLast = stackIndex == event.flagDefs().size() - 1;
                        if (TokenClassifier.isValuelessType(flag.type())) {
                            setFlag(parsedFlags, flagCounts, flag, Boolean.TRUE, errors, routing.commandPath(), explicitFlags);
                        } else if (isLast && event.trailingValue() != null) {
                            PositionalResolver.CoercionResult coercion = PositionalResolver.coerceValue(
                                    event.trailingValue(),
                                    flag.type(),
                                    flag.enumValues(),
                                    routing.commandPath(),
                                    flag.shortName() != null ? flag.shortName() : flag.id()
                            );
                            if (coercion.error() != null) {
                                errors.add(coercion.error());
                            } else {
                                setFlag(parsedFlags, flagCounts, flag, coercion.value(), errors, routing.commandPath(), explicitFlags);
                            }
                        } else {
                            pendingFlag = flag;
                        }
                    }
                }
                case UNKNOWN_FLAG -> errors.add(new ParseError(
                        "unknown_flag",
                        "Unknown flag '" + event.token() + "'",
                        withDashes(fuzzySuggest(event.token().replaceFirst("^-+", ""), allFlagNames(activeFlags))),
                        routing.commandPath()
                ));
            }
        }

        if (pendingFlag != null) {
            errors.add(new ParseError(
                    "missing_flag_value",
                    pendingFlag.display() + " requires a value",
                    null,
                    routing.commandPath()
            ));
        }

        return new ScanResult(parsedFlags, positionalTokens, errors, explicitFlags, null);
    }

    private ParseOutcome checkQuickHelpVersion(List<String> tokens, List<FlagDef> activeFlags, List<String> commandPath) {
        boolean userDefinedShortH = activeFlags.stream().anyMatch(flag -> "h".equals(flag.shortName()) && !"__help__".equals(flag.id()));
        for (String token : tokens) {
            if (spec.builtinFlags().help() && "--help".equals(token)) {
                return new HelpResult(new HelpGenerator(spec, commandPath).generate(), commandPath);
            }
            if (spec.builtinFlags().help() && "-h".equals(token) && !userDefinedShortH) {
                return new HelpResult(new HelpGenerator(spec, commandPath).generate(), commandPath);
            }
            if (spec.builtinFlags().version() && spec.version() != null && "--version".equals(token)) {
                return new VersionResult(spec.version());
            }
        }
        return null;
    }

    private ParseOutcome handleBuiltin(FlagDef flag, List<String> commandPath) {
        if (spec.builtinFlags().help() && ("help".equals(flag.longName()) || "__help__".equals(flag.id()))) {
            return new HelpResult(new HelpGenerator(spec, commandPath).generate(), commandPath);
        }
        if (spec.builtinFlags().version() && spec.version() != null
                && ("version".equals(flag.longName()) || "__version__".equals(flag.id()))) {
            return new VersionResult(spec.version());
        }
        return null;
    }

    private List<FlagDef> collectActiveFlags(List<String> commandPath, ScopeDef leaf) {
        List<FlagDef> flags = new ArrayList<>();
        Set<String> seen = new LinkedHashSet<>();
        if (!(leaf instanceof CommandDef command) || command.inheritGlobalFlags()) {
            for (FlagDef flag : spec.globalFlags()) {
                if (seen.add(flag.id())) {
                    flags.add(flag);
                }
            }
        }
        ScopeDef current = spec;
        for (int index = 1; index < commandPath.size(); index += 1) {
            CommandDef next = resolveCommand(current.commands(), commandPath.get(index));
            if (next == null) {
                break;
            }
            for (FlagDef flag : next.flags()) {
                if (seen.add(flag.id())) {
                    flags.add(flag);
                }
            }
            current = next;
        }

        if (leaf instanceof CliSpec root) {
            for (FlagDef flag : root.flags()) {
                if (seen.add(flag.id())) {
                    flags.add(flag);
                }
            }
        }
        return flags;
    }

    private Map<String, FlagDef> buildFlagLookup(List<FlagDef> flags) {
        Map<String, FlagDef> lookup = new HashMap<>();
        for (FlagDef flag : flags) {
            if (flag.longName() != null) {
                lookup.put(flag.longName(), flag);
            }
            if (flag.shortName() != null) {
                lookup.put(flag.shortName(), flag);
            }
            if (flag.singleDashLong() != null) {
                lookup.put(flag.singleDashLong(), flag);
            }
        }
        return lookup;
    }

    private int skipFlag(String token, Map<String, FlagDef> flagLookup) {
        if (token.startsWith("--") && token.contains("=")) {
            return 1;
        }
        if (token.startsWith("--")) {
            FlagDef flag = flagLookup.get(token.substring(2));
            if (flag != null && !TokenClassifier.isValuelessType(flag.type()) && flag.defaultWhenPresent() == null) {
                return 2;
            }
            return 1;
        }
        if (token.startsWith("-") && token.length() == 2) {
            FlagDef flag = flagLookup.get(token.substring(1));
            if (flag != null && !TokenClassifier.isValuelessType(flag.type())) {
                return 2;
            }
        }
        if (token.startsWith("-") && !token.startsWith("--")) {
            FlagDef flag = flagLookup.get(token.substring(1));
            if (flag != null && !TokenClassifier.isValuelessType(flag.type())) {
                return 2;
            }
        }
        return 1;
    }

    private CommandDef resolveCommand(List<CommandDef> commands, String token) {
        for (CommandDef command : commands) {
            if (command.name().equals(token) || command.aliases().contains(token)) {
                return command;
            }
        }
        return null;
    }

    private void setFlag(
            Map<String, Object> parsedFlags,
            Map<String, Integer> flagCounts,
            FlagDef flag,
            Object value,
            List<ParseError> errors,
            List<String> context,
            List<String> explicitFlags
    ) {
        explicitFlags.add(flag.id());
        int count = flagCounts.getOrDefault(flag.id(), 0) + 1;
        flagCounts.put(flag.id(), count);

        if ("count".equals(flag.type())) {
            long current = parsedFlags.get(flag.id()) instanceof Number number ? number.longValue() : 0L;
            parsedFlags.put(flag.id(), current + 1L);
            return;
        }

        if (flag.repeatable()) {
            @SuppressWarnings("unchecked")
            List<Object> values = (List<Object>) parsedFlags.computeIfAbsent(flag.id(), ignored -> new ArrayList<>());
            values.add(value);
            return;
        }

        if (count > 1) {
            errors.add(new ParseError(
                    "duplicate_flag",
                    flag.display() + " specified more than once",
                    null,
                    context
            ));
            return;
        }

        parsedFlags.put(flag.id(), value);
    }

    private Map<String, Object> applyFlagDefaults(List<FlagDef> activeFlags, Map<String, Object> parsedFlags) {
        Map<String, Object> result = new LinkedHashMap<>(parsedFlags);
        for (FlagDef flag : activeFlags) {
            if (result.containsKey(flag.id())) {
                continue;
            }
            if ("boolean".equals(flag.type())) {
                result.put(flag.id(), flag.defaultValue() == null ? Boolean.FALSE : flag.defaultValue());
            } else if ("count".equals(flag.type())) {
                result.put(flag.id(), flag.defaultValue() == null ? 0L : flag.defaultValue());
            } else {
                result.put(flag.id(), flag.defaultValue());
            }
        }
        return Map.copyOf(result);
    }

    private List<String> allFlagNames(List<FlagDef> activeFlags) {
        List<String> names = new ArrayList<>();
        for (FlagDef flag : activeFlags) {
            if (flag.longName() != null) {
                names.add(flag.longName());
            }
            if (flag.shortName() != null) {
                names.add(flag.shortName());
            }
            if (flag.singleDashLong() != null) {
                names.add(flag.singleDashLong());
            }
        }
        return names;
    }

    private List<FlagDef> tryTraditional(String token, List<FlagDef> activeFlags) {
        Map<String, FlagDef> byShort = new HashMap<>();
        for (FlagDef flag : activeFlags) {
            if (flag.shortName() != null) {
                byShort.put(flag.shortName(), flag);
            }
        }
        List<FlagDef> result = new ArrayList<>();
        for (int index = 0; index < token.length(); index += 1) {
            FlagDef flag = byShort.get(token.substring(index, index + 1));
            if (flag == null || !TokenClassifier.isValuelessType(flag.type())) {
                return null;
            }
            result.add(flag);
        }
        return result;
    }

    private int nextUnconsumedIndex(List<String> tokens, Set<Integer> consumedIndices, int start) {
        int index = start;
        while (index < tokens.size() && consumedIndices.contains(index)) {
            index += 1;
        }
        return index;
    }

    private static String fuzzySuggest(String token, List<String> candidates) {
        String best = null;
        int bestDistance = 3;
        for (String candidate : candidates) {
            int distance = levenshtein(token, candidate);
            if (distance < bestDistance) {
                best = candidate;
                bestDistance = distance;
            }
        }
        return bestDistance <= 2 ? best : null;
    }

    private static String withDashes(String suggestion) {
        if (suggestion == null) {
            return null;
        }
        return suggestion.length() == 1 ? "-" + suggestion : "--" + suggestion;
    }

    private static int levenshtein(String a, String b) {
        if (a.equals(b)) {
            return 0;
        }
        if (a.isEmpty()) {
            return b.length();
        }
        if (b.isEmpty()) {
            return a.length();
        }
        if (a.length() > b.length()) {
            return levenshtein(b, a);
        }
        int[] previous = new int[a.length() + 1];
        for (int index = 0; index <= a.length(); index += 1) {
            previous[index] = index;
        }
        for (int row = 0; row < b.length(); row += 1) {
            int[] current = new int[a.length() + 1];
            current[0] = row + 1;
            for (int column = 0; column < a.length(); column += 1) {
                int insert = current[column] + 1;
                int delete = previous[column + 1] + 1;
                int replace = previous[column] + (a.charAt(column) == b.charAt(row) ? 0 : 1);
                current[column + 1] = Math.min(insert, Math.min(delete, replace));
            }
            previous = current;
        }
        return previous[a.length()];
    }

    private record RoutingResult(
            List<String> commandPath,
            ScopeDef leaf,
            List<ParseError> errors,
            Set<Integer> consumedIndices
    ) {
    }

    private record ScanResult(
            Map<String, Object> parsedFlags,
            List<String> positionalTokens,
            List<ParseError> errors,
            List<String> explicitFlags,
            ParseOutcome specialResult
    ) {
    }
}
