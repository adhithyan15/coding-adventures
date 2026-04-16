package com.codingadventures.clibuilder;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

final class FlagValidator {
    private final List<FlagDef> activeFlags;
    private final List<ExclusiveGroupDef> exclusiveGroups;
    private final Map<String, FlagDef> byId;
    private final Map<String, List<String>> requiresGraph;

    FlagValidator(List<FlagDef> activeFlags, List<ExclusiveGroupDef> exclusiveGroups) {
        this.activeFlags = List.copyOf(activeFlags);
        this.exclusiveGroups = List.copyOf(exclusiveGroups);
        this.byId = new HashMap<>();
        this.requiresGraph = new HashMap<>();
        for (FlagDef flag : activeFlags) {
            byId.put(flag.id(), flag);
            requiresGraph.put(flag.id(), List.copyOf(flag.requires()));
        }
    }

    List<ParseError> validate(Map<String, Object> parsedFlags, List<String> context) {
        List<ParseError> errors = new ArrayList<>();
        Set<String> present = new LinkedHashSet<>();
        for (Map.Entry<String, Object> entry : parsedFlags.entrySet()) {
            Object value = entry.getValue();
            if (value == null || Boolean.FALSE.equals(value)) {
                continue;
            }
            present.add(entry.getKey());
        }

        errors.addAll(checkConflicts(present, context));
        errors.addAll(checkRequires(present, context));
        errors.addAll(checkRequired(present, parsedFlags, context));
        errors.addAll(checkExclusiveGroups(present, context));
        return errors;
    }

    private List<ParseError> checkConflicts(Set<String> present, List<String> context) {
        List<ParseError> errors = new ArrayList<>();
        Set<Set<String>> seenPairs = new HashSet<>();
        for (String flagId : present) {
            FlagDef flag = byId.get(flagId);
            if (flag == null) {
                continue;
            }
            for (String otherId : flag.conflictsWith()) {
                if (!present.contains(otherId)) {
                    continue;
                }
                Set<String> pair = Set.of(flagId, otherId);
                if (seenPairs.add(pair)) {
                    errors.add(new ParseError(
                            "conflicting_flags",
                            byId.get(flagId).display() + " and " + byId.get(otherId).display() + " cannot be used together",
                            null,
                            context
                    ));
                }
            }
        }
        return errors;
    }

    private List<ParseError> checkRequires(Set<String> present, List<String> context) {
        List<ParseError> errors = new ArrayList<>();
        for (String flagId : present) {
            for (String dependency : transitiveRequires(flagId)) {
                if (!present.contains(dependency)) {
                    errors.add(new ParseError(
                            "missing_dependency_flag",
                            byId.get(flagId).display() + " requires " + display(dependency),
                            null,
                            context
                    ));
                }
            }
        }
        return errors;
    }

    private List<ParseError> checkRequired(Set<String> present, Map<String, Object> parsedFlags, List<String> context) {
        List<ParseError> errors = new ArrayList<>();
        for (FlagDef flag : activeFlags) {
            if (!flag.required() || present.contains(flag.id())) {
                continue;
            }
            boolean exempt = false;
            for (String unlessId : flag.requiredUnless()) {
                Object value = parsedFlags.get(unlessId);
                if (value != null && !Boolean.FALSE.equals(value)) {
                    exempt = true;
                    break;
                }
            }
            if (!exempt) {
                errors.add(new ParseError(
                        "missing_required_flag",
                        flag.display() + " is required",
                        null,
                        context
                ));
            }
        }
        return errors;
    }

    private List<ParseError> checkExclusiveGroups(Set<String> present, List<String> context) {
        List<ParseError> errors = new ArrayList<>();
        for (ExclusiveGroupDef group : exclusiveGroups) {
            List<String> presentIds = group.flagIds().stream().filter(present::contains).toList();
            if (presentIds.size() > 1) {
                errors.add(new ParseError(
                        "exclusive_group_violation",
                        "Only one of " + group.flagIds().stream().map(this::display).reduce((a, b) -> a + ", " + b).orElse("") + " may be used at a time",
                        null,
                        context
                ));
            } else if (group.required() && presentIds.isEmpty()) {
                errors.add(new ParseError(
                        "missing_exclusive_group",
                        "One of " + group.flagIds().stream().map(this::display).reduce((a, b) -> a + ", " + b).orElse("") + " is required",
                        null,
                        context
                ));
            }
        }
        return errors;
    }

    private Set<String> transitiveRequires(String start) {
        Set<String> visited = new LinkedHashSet<>();
        ArrayDeque<String> queue = new ArrayDeque<>(requiresGraph.getOrDefault(start, List.of()));
        while (!queue.isEmpty()) {
            String current = queue.pop();
            if (visited.add(current)) {
                queue.addAll(requiresGraph.getOrDefault(current, List.of()));
            }
        }
        return visited;
    }

    private String display(String flagId) {
        FlagDef flag = byId.get(flagId);
        return flag == null ? "--" + flagId : flag.display();
    }
}
