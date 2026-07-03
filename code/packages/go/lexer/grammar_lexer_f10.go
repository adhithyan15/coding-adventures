package lexer

// grammar_lexer_f10.go — F10 declarative lexer mode transitions
//
// F10 adds a `transitions:` section to `.tokens` grammar files that replaces
// hand-written on-token callbacks for switching lexer modes (groups). Each
// transition rule matches a token type (and optionally the active mode or a
// specific value) and fires a sequence of actions to update the group stack.
//
// ## Why declarative transitions?
//
// Before F10, changing lexer modes required an OnTokenCallback that called
// ctx.PushGroup / ctx.PopGroup. This works, but it couples the grammar
// description to host-language callback code. F10 moves that coupling into
// the .tokens file itself:
//
//     transitions:
//       on SLASH in default -> set-mode div
//       on SLASH in div     -> set-mode default
//
// The lexer evaluates the table after every emitted token; the first matching
// rule fires.
//
// ## Flat-mode inheritance
//
// Two kinds of mode transitions exist:
//
//   - set_mode M — replaces the active group in place (flat toggle). The
//     target group is an "inheriting" mode: it includes the default group's
//     patterns as a fallthrough after its own, so tokens that are valid in
//     the default mode are still recognised.
//
//   - push G / pop — nested region save/restore (F04 semantics). The pushed
//     group is "exclusive": only its own patterns apply, no default fallthrough.
//
// computeInheritingModes computes which groups are inheriting from the
// transitions table so tryMatchTokenInGroup can handle the pattern merging.
//
// ## Integration points
//
// applyTransitions is called by tokenizeStandard and tokenizeIndentation after
// each token is emitted. It reads the current group stack, evaluates the
// transition table in order, and fires the first matching rule's actions.

import grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"

// computeInheritingModes returns the set of group names that should inherit
// the default group's patterns (F10 flat-mode inheritance).
//
// A mode is "inheriting" when it is targeted by at least one set_mode action
// AND is not targeted by any push action. set_mode is a flat toggle — it
// switches context while keeping the default patterns available as a
// fallthrough. push, by contrast, enters an exclusive nested context where
// only the group's own patterns apply.
func computeInheritingModes(transitions []grammartools.ModeTransition) map[string]bool {
	pushTargets := make(map[string]bool)
	setModeTargets := make(map[string]bool)

	for _, rule := range transitions {
		for _, action := range rule.Actions {
			if action.Target == "" {
				continue
			}
			switch action.Kind {
			case "push":
				pushTargets[action.Target] = true
			case "set_mode":
				setModeTargets[action.Target] = true
			}
		}
	}

	// A group inherits from default when it is a set_mode target but not a
	// push target, and it is not the default group itself.
	inheriting := make(map[string]bool)
	for name := range setModeTargets {
		if name != "default" && !pushTargets[name] {
			inheriting[name] = true
		}
	}
	return inheriting
}

// applyTransitions fires the first matching declarative transition rule (F10)
// after a token is emitted. Rules are evaluated in priority order (first
// matching rule fires, then stops).
//
// Matching criteria (all must hold):
//  1. The token's TypeName must appear in rule.OnTokens.
//  2. If rule.InMode is set, it must equal the current active group.
//  3. If rule.OnValue is set, it must equal the token's Value.
//
// Actions are applied in the order they appear in the matched rule:
//   - set_mode TARGET: replace the top of the group stack with TARGET (flat toggle).
//   - push TARGET: push TARGET onto the group stack (nested region).
//   - pop: pop the group stack, clamped so the stack never becomes empty.
//   - enable_skip: re-enable skip-pattern processing.
//   - disable_skip: suspend skip-pattern processing.
//
// This method is a no-op when the grammar has no transitions table (pre-F10
// grammars are unaffected).
func (l *GrammarLexer) applyTransitions(tok Token) {
	if len(l.transitions) == 0 {
		return
	}

	// The active group is the top of the group stack.
	active := l.groupStack[len(l.groupStack)-1]

	for _, rule := range l.transitions {
		// 1. Token-type guard: does the rule trigger on this token's type?
		typeMatched := false
		for _, onType := range rule.OnTokens {
			if onType == tok.TypeName {
				typeMatched = true
				break
			}
		}
		if !typeMatched {
			continue
		}

		// 2. In-mode guard: rule must be for the current active group
		//    (or have no in-mode constraint).
		if rule.InMode != "" && rule.InMode != active {
			continue
		}

		// 3. Value guard: rule must match the token's value
		//    (or have no value constraint).
		if rule.OnValue != "" && rule.OnValue != tok.Value {
			continue
		}

		// First matching rule fires: apply all actions in order.
		for _, action := range rule.Actions {
			switch action.Kind {
			case "set_mode":
				// Flat toggle: replace the active group in place.
				l.groupStack[len(l.groupStack)-1] = action.Target

			case "push":
				// Nested region: push a new exclusive group onto the stack.
				l.groupStack = append(l.groupStack, action.Target)

			case "pop":
				// Close a nested region. Clamp to keep at least one entry
				// so the stack never becomes completely empty.
				if len(l.groupStack) > 1 {
					l.groupStack = l.groupStack[:len(l.groupStack)-1]
				}

			case "enable_skip":
				l.skipEnabled = true

			case "disable_skip":
				l.skipEnabled = false
			}
		}

		// First-match-wins: stop after the first matching rule.
		break
	}
}
