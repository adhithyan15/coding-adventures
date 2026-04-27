// ============================================================================
// ParserGrammar.java — Complete contents of a parsed .grammar file
// ============================================================================
//
// A .grammar file uses EBNF to describe how tokens (from a .tokens file)
// combine into valid programs. This class holds the parsed result.
//
// Helper methods extract useful information for cross-validation:
//   - ruleNames()       — all defined rule names
//   - ruleReferences()  — all lowercase names referenced in rule bodies
//   - tokenReferences() — all UPPERCASE names referenced in rule bodies
//
// Layer: TE (text/language layer)
// ============================================================================

package com.codingadventures.grammartools;

import java.util.*;

/**
 * The complete contents of a parsed .grammar file.
 */
public final class ParserGrammar {

    private int version = 0;
    private final List<GrammarRule> rules;

    public ParserGrammar(List<GrammarRule> rules) {
        this.rules = rules;
    }

    public ParserGrammar() {
        this(new ArrayList<>());
    }

    public int getVersion() { return version; }
    public void setVersion(int version) { this.version = version; }

    public List<GrammarRule> getRules() { return rules; }

    /** Return the set of all defined rule names. */
    public Set<String> ruleNames() {
        Set<String> names = new HashSet<>();
        for (GrammarRule rule : rules) {
            names.add(rule.name());
        }
        return names;
    }

    /** Return all lowercase rule names referenced in rule bodies. */
    public Set<String> ruleReferences() {
        Set<String> refs = new HashSet<>();
        for (GrammarRule rule : rules) {
            collectRuleRefs(rule.body(), refs);
        }
        return refs;
    }

    /** Return all UPPERCASE token names referenced in rule bodies. */
    public Set<String> tokenReferences() {
        Set<String> refs = new HashSet<>();
        for (GrammarRule rule : rules) {
            collectTokenRefs(rule.body(), refs);
        }
        return refs;
    }

    // --- Tree walkers for collecting references ---

    private static void collectRuleRefs(GrammarElement element, Set<String> refs) {
        switch (element) {
            case GrammarElement.RuleReference r -> { if (!r.isToken()) refs.add(r.name()); }
            case GrammarElement.Sequence s -> s.elements().forEach(e -> collectRuleRefs(e, refs));
            case GrammarElement.Alternation a -> a.choices().forEach(c -> collectRuleRefs(c, refs));
            case GrammarElement.Repetition r -> collectRuleRefs(r.element(), refs);
            case GrammarElement.Optional o -> collectRuleRefs(o.element(), refs);
            case GrammarElement.Group g -> collectRuleRefs(g.element(), refs);
            case GrammarElement.PositiveLookahead p -> collectRuleRefs(p.element(), refs);
            case GrammarElement.NegativeLookahead n -> collectRuleRefs(n.element(), refs);
            case GrammarElement.OneOrMoreRepetition r -> collectRuleRefs(r.element(), refs);
            case GrammarElement.SeparatedRepetition s -> {
                collectRuleRefs(s.element(), refs);
                collectRuleRefs(s.separator(), refs);
            }
            case GrammarElement.Literal ignored -> {}
        }
    }

    private static void collectTokenRefs(GrammarElement element, Set<String> refs) {
        switch (element) {
            case GrammarElement.RuleReference r -> { if (r.isToken()) refs.add(r.name()); }
            case GrammarElement.Sequence s -> s.elements().forEach(e -> collectTokenRefs(e, refs));
            case GrammarElement.Alternation a -> a.choices().forEach(c -> collectTokenRefs(c, refs));
            case GrammarElement.Repetition r -> collectTokenRefs(r.element(), refs);
            case GrammarElement.Optional o -> collectTokenRefs(o.element(), refs);
            case GrammarElement.Group g -> collectTokenRefs(g.element(), refs);
            case GrammarElement.PositiveLookahead p -> collectTokenRefs(p.element(), refs);
            case GrammarElement.NegativeLookahead n -> collectTokenRefs(n.element(), refs);
            case GrammarElement.OneOrMoreRepetition r -> collectTokenRefs(r.element(), refs);
            case GrammarElement.SeparatedRepetition s -> {
                collectTokenRefs(s.element(), refs);
                collectTokenRefs(s.separator(), refs);
            }
            case GrammarElement.Literal ignored -> {}
        }
    }
}
