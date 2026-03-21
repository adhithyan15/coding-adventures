package clibuilder

// =========================================================================
// Parser — the main entry point for CLI argument parsing.
// =========================================================================
//
// # Three-phase parsing algorithm (§6)
//
// Phase 1 — Routing (Directed Graph):
//   Walk the command routing graph G_cmd by consuming subcommand tokens
//   from argv. Each recognized subcommand advances the current node.
//   Flags encountered during routing are skipped (they belong to Phase 2).
//   The phase ends when a token is not a subcommand or when argv is
//   exhausted. The result is command_path and the resolved command node.
//
// Phase 2 — Scanning (Modal State Machine):
//   Walk argv again, skipping command_path tokens. Classify each token
//   using the TokenClassifier. The Modal State Machine tracks parse mode:
//   SCANNING → FLAG_VALUE (after non-boolean flag) → SCANNING
//   SCANNING → END_OF_FLAGS (after "--") → stays END_OF_FLAGS
//   In END_OF_FLAGS mode, all tokens are positional.
//
// Phase 3 — Validation:
//   Assign positional tokens to argument slots (PositionalResolver).
//   Validate flag constraints (FlagValidator).
//   Collect all errors and return them as *ParseErrors.
//
// # Help and version short-circuits
//
// If --help or -h is encountered in Phase 2, Parse() immediately returns
// a *HelpResult without completing scanning or validation. Similarly for
// --version.

import (
	"fmt"
	"strings"

	statemachine "github.com/adhithyan15/coding-adventures/code/packages/go/state-machine"
)

// Parse mode constants used as mode names in the Modal State Machine.
const (
	modeSCANNING    = "SCANNING"
	modeFLAG_VALUE  = "FLAG_VALUE"
	modeEND_OF_FLAGS = "END_OF_FLAGS"
)

// Parser is the main CLI argument parser.
//
// Create a Parser with NewParser, then call Parse() to process argv.
// A Parser is not safe for concurrent use.
type Parser struct {
	spec    map[string]any
	argv    []string // argv[0] is the program name; argv[1:] are the tokens to parse
}

// NewParser creates a Parser from a spec file and an argv slice.
//
// specFilePath is the path to the JSON specification file. It is read and
// validated at construction time — if the spec is invalid, NewParser returns
// a *SpecError immediately before any argv processing.
//
// argv should be os.Args (the full argument vector including the program
// name at argv[0]).
//
// Returns a *SpecError if the spec is invalid, or a standard error if the
// spec file cannot be read or parsed.
func NewParser(specFilePath string, argv []string) (*Parser, error) {
	spec, err := LoadSpec(specFilePath)
	if err != nil {
		return nil, err
	}
	return &Parser{spec: spec, argv: argv}, nil
}

// NewParserFromBytes creates a Parser from a JSON spec byte slice and argv.
//
// Useful in tests where the spec is embedded as a constant string.
func NewParserFromBytes(specJSON []byte, argv []string) (*Parser, error) {
	spec, err := LoadSpecFromBytes(specJSON)
	if err != nil {
		return nil, err
	}
	return &Parser{spec: spec, argv: argv}, nil
}

// Parse parses argv against the spec and returns one of:
//   - *ParseResult   — successful parse; use r.Flags, r.Arguments, r.CommandPath
//   - *HelpResult    — user passed --help; print r.Text and exit 0
//   - *VersionResult — user passed --version; print r.Version and exit 0
//
// On error, Parse returns (nil, *ParseErrors) where the ParseErrors value
// contains all errors encountered. The caller should print the error and
// exit 1.
func (p *Parser) Parse() (any, error) {
	if len(p.argv) == 0 {
		return nil, &ParseErrors{Errors: []ParseError{{
			ErrorType: ErrMissingRequiredArgument,
			Message:   "argv is empty (no program name)",
		}}}
	}

	program := p.argv[0]
	tokens := p.argv[1:] // strip argv[0] (the program name)

	parsingMode := stringField(p.spec, "parsing_mode")
	if parsingMode == "" {
		parsingMode = "gnu"
	}

	// -----------------------------------------------------------------------
	// Traditional mode preprocessing (§5.3 / §6.1)
	//
	// If parsing_mode is "traditional" and argv[1] does not start with "-"
	// and does not match any known subcommand, treat it as stacked short flags.
	// We handle this by rewriting tokens[0] into flag tokens.
	// -----------------------------------------------------------------------
	var traditionalPrefix []string
	if parsingMode == "traditional" && len(tokens) > 0 && !strings.HasPrefix(tokens[0], "-") {
		// Get root-level commands to check for subcommand match
		rootCommands := sliceOfMaps(p.spec["commands"])
		knownSubs := make(map[string]bool)
		for _, cmd := range rootCommands {
			knownSubs[stringField(cmd, "name")] = true
			for _, alias := range sliceOfStrings(cmd["aliases"]) {
				knownSubs[alias] = true
			}
		}
		if !knownSubs[tokens[0]] {
			// Treat as stacked flags: each character is a short flag
			for _, ch := range tokens[0] {
				traditionalPrefix = append(traditionalPrefix, "-"+string(ch))
			}
			tokens = append(traditionalPrefix, tokens[1:]...)
		}
	}

	// -----------------------------------------------------------------------
	// Phase 1 — Routing
	// -----------------------------------------------------------------------
	commandPath, resolvedNode, remainingTokens, routingErrs := p.phaseRouting(program, tokens, parsingMode)

	// -----------------------------------------------------------------------
	// Assemble active flags for Phase 2
	// -----------------------------------------------------------------------
	activeFlags := p.buildActiveFlags(commandPath)

	// Inject builtin flags (help, version)
	builtinHelp, builtinVersion := p.builtinFlags()
	if builtinHelp {
		activeFlags = append(activeFlags, map[string]any{
			"id":          "help",
			"short":       "h",
			"long":        "help",
			"description": "Show this help message and exit.",
			"type":        "boolean",
		})
	}
	if builtinVersion {
		activeFlags = append(activeFlags, map[string]any{
			"id":          "version",
			"long":        "version",
			"description": "Show version and exit.",
			"type":        "boolean",
		})
	}

	// -----------------------------------------------------------------------
	// Phase 2 — Scanning (Modal State Machine + TokenClassifier)
	// -----------------------------------------------------------------------
	tc := NewTokenClassifier(activeFlags)
	parsedFlags, positionalTokens, scanErrs, helpRequested, versionRequested :=
		p.phaseScanning(remainingTokens, commandPath, tc, parsingMode)

	// Short-circuit for --help and --version
	if helpRequested {
		hg := NewHelpGenerator(p.spec, commandPath)
		return &HelpResult{
			Text:        hg.Generate(),
			CommandPath: commandPath,
		}, nil
	}
	if versionRequested {
		ver := stringField(p.spec, "version")
		if ver == "" {
			ver = "(unknown)"
		}
		return &VersionResult{Version: ver}, nil
	}

	// Apply default values for absent flags
	parsedFlags = p.applyFlagDefaults(activeFlags, parsedFlags)

	// -----------------------------------------------------------------------
	// Phase 3 — Validation
	// -----------------------------------------------------------------------
	var allErrs []ParseError
	allErrs = append(allErrs, routingErrs...)
	allErrs = append(allErrs, scanErrs...)

	// 3a. Positional argument resolution
	argDefs := sliceOfMaps(resolvedNode["arguments"])
	pr := NewPositionalResolver(argDefs)
	parsedArgs, posErrs := pr.Resolve(positionalTokens, parsedFlags)
	allErrs = append(allErrs, posErrs...)

	// 3b. Flag constraint validation
	exclusiveGroups := sliceOfMaps(resolvedNode["mutually_exclusive_groups"])
	fv := NewFlagValidator(activeFlags, exclusiveGroups)
	flagErrs := fv.Validate(parsedFlags)
	allErrs = append(allErrs, flagErrs...)

	// Attach command context to all errors
	for i := range allErrs {
		if allErrs[i].Context == nil {
			allErrs[i].Context = commandPath
		}
	}

	if len(allErrs) > 0 {
		return nil, &ParseErrors{Errors: allErrs}
	}

	return &ParseResult{
		Program:     program,
		CommandPath: commandPath,
		Flags:       parsedFlags,
		Arguments:   parsedArgs,
	}, nil
}

// -----------------------------------------------------------------------
// Phase 1 implementation
// -----------------------------------------------------------------------

// phaseRouting walks the command routing graph G_cmd, consuming subcommand
// tokens and returning the full command path, the resolved command node
// (as a map), and the remaining tokens for Phase 2.
func (p *Parser) phaseRouting(
	program string,
	tokens []string,
	parsingMode string,
) (commandPath []string, resolvedNode map[string]any, remaining []string, errs []ParseError) {
	commandPath = []string{program}

	// resolvedNode starts as the root spec (the "program node").
	// We use a wrapper map so we can read flags/arguments/commands from it.
	// For the root, "flags" and "arguments" come directly from p.spec.
	resolvedNode = map[string]any{
		"flags":                   p.spec["flags"],
		"arguments":               p.spec["arguments"],
		"commands":                p.spec["commands"],
		"mutually_exclusive_groups": p.spec["mutually_exclusive_groups"],
	}

	// consumedIdx tracks token indices consumed as subcommand names.
	// Flags and positional args are NOT consumed here — they are preserved for
	// Phase 2. We only mark the indices of tokens that were matched as
	// subcommand names so they can be excluded from the remaining slice.
	consumedIdx := make(map[int]bool)

	i := 0
	for i < len(tokens) {
		token := tokens[i]

		// "--" ends routing immediately; leave it (and everything after) in remaining.
		if token == "--" {
			break
		}

		// Skip flags during routing (Phase 1 only routes, Phase 2 scans).
		// Critically: we do NOT add these indices to consumedIdx — the flags
		// must reach Phase 2 intact.
		if strings.HasPrefix(token, "-") {
			// Peek ahead for value-taking flags so we don't accidentally treat
			// the value token as a subcommand on the next iteration.
			if strings.HasPrefix(token, "--") && !strings.Contains(token, "=") {
				rootFlags := p.buildActiveFlags(commandPath)
				flagName := token[2:]
				if isValueTakingLong(flagName, rootFlags) && i+1 < len(tokens) && !strings.HasPrefix(tokens[i+1], "-") {
					i += 2
					continue
				}
			} else if len(token) == 2 && token[0] == '-' && token[1] != '-' {
				rootFlags := p.buildActiveFlags(commandPath)
				char := string(token[1])
				if isValueTakingShort(char, rootFlags) && i+1 < len(tokens) && !strings.HasPrefix(tokens[i+1], "-") {
					i += 2
					continue
				}
			}
			i++
			continue
		}

		// Non-flag token: check if it matches a known subcommand at this level.
		currentCommands := sliceOfMaps(resolvedNode["commands"])
		matched, matchedCmd := findCommand(token, currentCommands)
		if matched {
			consumedIdx[i] = true
			commandPath = append(commandPath, stringField(matchedCmd, "name"))
			resolvedNode = map[string]any{
				"flags":                     matchedCmd["flags"],
				"arguments":                 matchedCmd["arguments"],
				"commands":                  matchedCmd["commands"],
				"mutually_exclusive_groups": matchedCmd["mutually_exclusive_groups"],
				"inherit_global_flags":      matchedCmd["inherit_global_flags"],
			}
			i++
		} else {
			if parsingMode == "subcommand_first" && len(commandPath) == 1 {
				knownNames := commandNames(currentCommands)
				suggestion := ""
				if s, ok := fuzzyMatch(token, knownNames); ok {
					suggestion = fmt.Sprintf("Did you mean %q?", s)
				}
				errs = append(errs, ParseError{
					ErrorType:  ErrUnknownCommand,
					Message:    fmt.Sprintf("unknown command %q", token),
					Suggestion: suggestion,
				})
			}
			break
		}
	}

	// Build remaining: every token that was NOT consumed as a subcommand name.
	// This preserves flags and positional args that appeared before the point
	// where routing stopped (e.g. "ls -lah /tmp" — "-lah" is NOT consumed and
	// must reach Phase 2).
	for j, t := range tokens {
		if !consumedIdx[j] {
			remaining = append(remaining, t)
		}
	}
	return commandPath, resolvedNode, remaining, errs
}

// findCommand searches a commands list for a token (checking name and aliases).
// Returns the matched command map and true if found.
func findCommand(token string, commands []map[string]any) (bool, map[string]any) {
	for _, cmd := range commands {
		if stringField(cmd, "name") == token {
			return true, cmd
		}
		for _, alias := range sliceOfStrings(cmd["aliases"]) {
			if alias == token {
				return true, cmd
			}
		}
	}
	return false, nil
}

// commandNames returns a slice of all command names and aliases in a commands list.
func commandNames(commands []map[string]any) []string {
	var names []string
	for _, cmd := range commands {
		names = append(names, stringField(cmd, "name"))
		names = append(names, sliceOfStrings(cmd["aliases"])...)
	}
	return names
}

// isValueTakingLong returns true if the long flag name refers to a non-boolean flag.
func isValueTakingLong(name string, activeFlags []map[string]any) bool {
	for _, f := range activeFlags {
		if stringField(f, "long") == name {
			return stringField(f, "type") != "boolean"
		}
	}
	return false
}

// isValueTakingShort returns true if the short flag char refers to a non-boolean flag.
func isValueTakingShort(char string, activeFlags []map[string]any) bool {
	for _, f := range activeFlags {
		if stringField(f, "short") == char {
			return stringField(f, "type") != "boolean"
		}
	}
	return false
}

// -----------------------------------------------------------------------
// Phase 2 implementation
// -----------------------------------------------------------------------

// phaseScanning processes tokens with the Modal State Machine.
//
// It skips tokens that are already consumed as command path elements,
// classifies each remaining token with the TokenClassifier, and updates
// parsedFlags and positionalTokens accordingly.
//
// Returns parsedFlags, positionalTokens, any scan errors, and whether
// --help or --version was encountered.
func (p *Parser) phaseScanning(
	tokens []string,
	commandPath []string,
	tc *TokenClassifier,
	parsingMode string,
) (parsedFlags map[string]any, positionalTokens []string, errs []ParseError, helpRequested bool, versionRequested bool) {
	parsedFlags = make(map[string]any)

	// ---- Build the Modal State Machine ----
	//
	// We use three modes:
	//   SCANNING     — normal flag/positional scanning
	//   FLAG_VALUE   — expecting the value for a pending non-boolean flag
	//   END_OF_FLAGS — all remaining tokens are positional (after "--")
	//
	// Each mode is a trivial single-state DFA (just one state, one self-loop
	// per token) because the TokenClassifier already does the real work.
	// The Modal State Machine gives us clean mode tracking with a history trace.

	singleStateDFA := func(modeName string) *statemachine.DFA {
		return statemachine.NewDFA(
			[]string{modeName},
			[]string{"token"},
			map[[2]string]string{
				{modeName, "token"}: modeName,
			},
			modeName,
			[]string{modeName},
			nil,
		)
	}

	msm := statemachine.NewModalStateMachine(
		map[string]*statemachine.DFA{
			modeSCANNING:     singleStateDFA(modeSCANNING),
			modeFLAG_VALUE:   singleStateDFA(modeFLAG_VALUE),
			modeEND_OF_FLAGS: singleStateDFA(modeEND_OF_FLAGS),
		},
		map[[2]string]string{
			{modeSCANNING, "to_flag_value"}:    modeFLAG_VALUE,
			{modeSCANNING, "to_end_of_flags"}:  modeEND_OF_FLAGS,
			{modeFLAG_VALUE, "to_scanning"}:     modeSCANNING,
			{modeEND_OF_FLAGS, "stay_eof"}:      modeEND_OF_FLAGS,
		},
		modeSCANNING,
	)

	var pendingFlag map[string]any // set when a non-boolean flag needs its value

	// Track which tokens in commandPath have been consumed (skip them in Phase 2).
	// commandPath[0] is the program name (already stripped). commandPath[1:] are
	// subcommand names we consumed in Phase 1. We need to skip exactly those
	// subcommand tokens in the remaining tokens slice.
	//
	// Strategy: the remaining tokens from Phase 1 already have the routing tokens
	// stripped. We just need to process them all.

	for _, token := range tokens {
		mode := msm.CurrentMode()
		msm.Process("token") // advance the DFA (no-op for our single-state DFAs)

		switch mode {

		case modeEND_OF_FLAGS:
			// All tokens after "--" are positional, no classification needed.
			positionalTokens = append(positionalTokens, token)

		case modeFLAG_VALUE:
			// The entire token is the value for the pending non-boolean flag.
			if pendingFlag != nil {
				flagID := stringField(pendingFlag, "id")
				flagType := stringField(pendingFlag, "type")
				repeatable := boolField(pendingFlag, "repeatable", false)

				val, err := coerceValue(token, flagType, pendingFlag)
				if err != nil {
					// Check if it's an enum error
					if flagType == "enum" {
						errs = append(errs, ParseError{
							ErrorType: ErrInvalidEnumValue,
							Message:   fmt.Sprintf("invalid value %q for %s; must be one of: %v", token, flagLabel(pendingFlag), sliceOfStrings(pendingFlag["enum_values"])),
						})
					} else {
						errs = append(errs, ParseError{
							ErrorType: ErrInvalidValue,
							Message:   fmt.Sprintf("invalid %s for %s: %q", flagType, flagLabel(pendingFlag), token),
						})
					}
				} else {
					setFlagValue(parsedFlags, flagID, val, repeatable, pendingFlag, &errs)
				}
				pendingFlag = nil
			}
			msm.SwitchMode("to_scanning")

		case modeSCANNING:
			ev := tc.Classify(token)

			switch ev.Kind {
			case TokenEndOfFlags:
				msm.SwitchMode("to_end_of_flags")

			case TokenLongFlag:
				// Look up the flag def first; check builtin id (not just name) so
				// user-defined flags named "help"/"version" don't hijack the builtin.
				flagDef := tc.LookupByLong(ev.Name)
				if flagDef != nil && stringField(flagDef, "id") == "help" {
					helpRequested = true
					return
				}
				if flagDef != nil && stringField(flagDef, "id") == "version" {
					versionRequested = true
					return
				}
				if flagDef == nil {
					suggestion := ""
					if s, ok := fuzzyMatch("--"+ev.Name, tc.KnownLongNames()); ok {
						suggestion = fmt.Sprintf("Did you mean %q?", s)
					}
					errs = append(errs, ParseError{
						ErrorType:  ErrUnknownFlag,
						Message:    fmt.Sprintf("unknown flag --%s", ev.Name),
						Suggestion: suggestion,
					})
					continue
				}
				flagID := stringField(flagDef, "id")
				flagType := stringField(flagDef, "type")
				repeatable := boolField(flagDef, "repeatable", false)
				if flagType == "boolean" {
					setFlagValue(parsedFlags, flagID, true, repeatable, flagDef, &errs)
				} else {
					pendingFlag = flagDef
					_ = flagID
					msm.SwitchMode("to_flag_value")
				}

			case TokenLongFlagWithValue:
				flagDef := tc.LookupByLong(ev.Name)
				if flagDef != nil && stringField(flagDef, "id") == "help" {
					helpRequested = true
					return
				}
				if flagDef == nil {
					suggestion := ""
					if s, ok := fuzzyMatch("--"+ev.Name, tc.KnownLongNames()); ok {
						suggestion = fmt.Sprintf("Did you mean %q?", s)
					}
					errs = append(errs, ParseError{
						ErrorType:  ErrUnknownFlag,
						Message:    fmt.Sprintf("unknown flag --%s", ev.Name),
						Suggestion: suggestion,
					})
					continue
				}
				flagID := stringField(flagDef, "id")
				flagType := stringField(flagDef, "type")
				repeatable := boolField(flagDef, "repeatable", false)
				val, coerceErr := coerceValue(ev.Value, flagType, flagDef)
				if coerceErr != nil {
					if flagType == "enum" {
						errs = append(errs, ParseError{
							ErrorType: ErrInvalidEnumValue,
							Message:   fmt.Sprintf("invalid value %q for %s; must be one of: %v", ev.Value, flagLabel(flagDef), sliceOfStrings(flagDef["enum_values"])),
						})
					} else {
						errs = append(errs, ParseError{
							ErrorType: ErrInvalidValue,
							Message:   fmt.Sprintf("invalid %s for %s: %q", flagType, flagLabel(flagDef), ev.Value),
						})
					}
				} else {
					setFlagValue(parsedFlags, flagID, val, repeatable, flagDef, &errs)
				}

			case TokenSingleDashLong:
				flagDef := tc.LookupBySDL(ev.Name)
				if flagDef == nil {
					errs = append(errs, ParseError{
						ErrorType: ErrUnknownFlag,
						Message:   fmt.Sprintf("unknown flag -%s", ev.Name),
					})
					continue
				}
				flagID := stringField(flagDef, "id")
				flagType := stringField(flagDef, "type")
				repeatable := boolField(flagDef, "repeatable", false)
				if flagType == "boolean" {
					setFlagValue(parsedFlags, flagID, true, repeatable, flagDef, &errs)
				} else {
					pendingFlag = flagDef
					_ = flagID
					msm.SwitchMode("to_flag_value")
				}

			case TokenShortFlag:
				// Look up the flag def; check id so user -h flags are not mistaken for builtin.
				flagDef := tc.LookupByShort(ev.Name)
				if flagDef != nil && stringField(flagDef, "id") == "help" {
					helpRequested = true
					return
				}
				if flagDef == nil {
					suggestion := ""
					if s, ok := fuzzyMatch("-"+ev.Name, tc.KnownShortNames()); ok {
						suggestion = fmt.Sprintf("Did you mean %q?", s)
					}
					errs = append(errs, ParseError{
						ErrorType:  ErrUnknownFlag,
						Message:    fmt.Sprintf("unknown flag -%s", ev.Name),
						Suggestion: suggestion,
					})
					continue
				}
				flagID := stringField(flagDef, "id")
				flagType := stringField(flagDef, "type")
				repeatable := boolField(flagDef, "repeatable", false)
				if flagType == "boolean" {
					setFlagValue(parsedFlags, flagID, true, repeatable, flagDef, &errs)
				} else {
					pendingFlag = flagDef
					_ = flagID
					msm.SwitchMode("to_flag_value")
				}

			case TokenShortFlagWithValue:
				flagDef := tc.LookupByShort(ev.Name)
				if flagDef == nil {
					errs = append(errs, ParseError{
						ErrorType: ErrUnknownFlag,
						Message:   fmt.Sprintf("unknown flag -%s", ev.Name),
					})
					continue
				}
				flagID := stringField(flagDef, "id")
				flagType := stringField(flagDef, "type")
				repeatable := boolField(flagDef, "repeatable", false)
				val, coerceErr := coerceValue(ev.Value, flagType, flagDef)
				if coerceErr != nil {
					if flagType == "enum" {
						errs = append(errs, ParseError{
							ErrorType: ErrInvalidEnumValue,
							Message:   fmt.Sprintf("invalid value %q for %s; must be one of: %v", ev.Value, flagLabel(flagDef), sliceOfStrings(flagDef["enum_values"])),
						})
					} else {
						errs = append(errs, ParseError{
							ErrorType: ErrInvalidValue,
							Message:   fmt.Sprintf("invalid %s for %s: %q", flagType, flagLabel(flagDef), ev.Value),
						})
					}
				} else {
					setFlagValue(parsedFlags, flagID, val, repeatable, flagDef, &errs)
				}

			case TokenStackedFlags:
				for _, ch := range ev.Chars {
					flagDef := tc.LookupByShort(ch)
					if flagDef == nil {
						errs = append(errs, ParseError{
							ErrorType: ErrUnknownFlag,
							Message:   fmt.Sprintf("unknown flag -%s in stack %q", ch, ev.Raw),
						})
						continue
					}
					flagID := stringField(flagDef, "id")
					flagType := stringField(flagDef, "type")
					repeatable := boolField(flagDef, "repeatable", false)
					if flagType == "boolean" {
						setFlagValue(parsedFlags, flagID, true, repeatable, flagDef, &errs)
					} else {
						// Last non-boolean flag in a stack — value is the next token
						pendingFlag = flagDef
						_ = flagID
						msm.SwitchMode("to_flag_value")
					}
				}

			case TokenPositional:
				if parsingMode == "posix" {
					// In POSIX mode, the first positional token ends flag scanning.
					// All subsequent tokens are also positional.
					msm.SwitchMode("to_end_of_flags")
					positionalTokens = append(positionalTokens, ev.Name)
				} else {
					positionalTokens = append(positionalTokens, ev.Name)
				}

			case TokenUnknownFlag:
				// Build fuzzy suggestion from all known flags
				allKnown := append(tc.KnownLongNames(), tc.KnownShortNames()...)
				suggestion := ""
				if s, ok := fuzzyMatch(ev.Raw, allKnown); ok {
					suggestion = fmt.Sprintf("Did you mean %q?", s)
				}
				errs = append(errs, ParseError{
					ErrorType:  ErrUnknownFlag,
					Message:    fmt.Sprintf("unknown flag %q", ev.Raw),
					Suggestion: suggestion,
				})
			}
		}
	}

	return parsedFlags, positionalTokens, errs, helpRequested, versionRequested
}

// setFlagValue sets a flag's parsed value, handling repeatable flags and
// duplicate detection.
func setFlagValue(parsedFlags map[string]any, flagID string, val any, repeatable bool, flagDef map[string]any, errs *[]ParseError) {
	if repeatable {
		existing, ok := parsedFlags[flagID]
		if !ok {
			parsedFlags[flagID] = []any{val}
		} else if arr, isArr := existing.([]any); isArr {
			parsedFlags[flagID] = append(arr, val)
		} else {
			parsedFlags[flagID] = []any{existing, val}
		}
	} else {
		if _, alreadySet := parsedFlags[flagID]; alreadySet {
			*errs = append(*errs, ParseError{
				ErrorType: ErrDuplicateFlag,
				Message:   fmt.Sprintf("%s specified more than once", flagLabel(flagDef)),
			})
		} else {
			parsedFlags[flagID] = val
		}
	}
}

// -----------------------------------------------------------------------
// Helper methods
// -----------------------------------------------------------------------

// buildActiveFlags returns all flags in scope for the given command path:
// local flags for the resolved command + global_flags (if inherit is true).
func (p *Parser) buildActiveFlags(commandPath []string) []map[string]any {
	return getAllFlagsForScope(p.spec, commandPath)
}

// builtinFlags returns whether help and version builtins are enabled.
func (p *Parser) builtinFlags() (help bool, version bool) {
	help = true
	version = stringField(p.spec, "version") != ""

	bi, ok := p.spec["builtin_flags"].(map[string]any)
	if !ok {
		return
	}
	if v, ok := bi["help"].(bool); ok {
		help = v
	}
	if v, ok := bi["version"].(bool); ok {
		version = v
	}
	return
}

// applyFlagDefaults fills in default values for all flags in scope that were
// not seen in argv. Boolean defaults to false; others default to nil or the
// declared default.
func (p *Parser) applyFlagDefaults(activeFlags []map[string]any, parsedFlags map[string]any) map[string]any {
	for _, f := range activeFlags {
		id := stringField(f, "id")
		if id == "" {
			continue
		}
		if _, seen := parsedFlags[id]; seen {
			continue
		}
		flagType := stringField(f, "type")
		if flagType == "boolean" {
			parsedFlags[id] = false
		} else if f["default"] != nil {
			parsedFlags[id] = f["default"]
		} else {
			parsedFlags[id] = nil
		}
	}
	return parsedFlags
}
