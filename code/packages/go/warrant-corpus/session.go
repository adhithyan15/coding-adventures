package warrantcorpus

import (
	"path/filepath"
	"strings"
)

// # One lane run, start to finish: prepare, execute, report
//
// prepare is where a run is REFUSED (exit 2) -- the paths are contained, the
// corpus loads, the binding parses, and the pieces are checked against each other
// before any case runs. execute is where the library is JUDGED (exit 0 or 1).
//
// # Binding totality (VER04i section 5, rule 1)
//
//   - every op the corpus declares has a binding row, and vice versa;
//   - every error variant declared for an op's Result has a row, and every mapped
//     variant is declared;
//   - an op with a width parameter says how it takes the width, and an op without
//     one does not claim to.
//
// A binding that failed any of these would let some suite quietly skip an
// operation or an error path.

// Environment variables that select sabotage and output (VER04i 4.3).
const (
	EnvFault  = "WARRANT_EVIDENCE_FAULT"
	EnvFilter = "WARRANT_EVIDENCE_FILTER"
	EnvOut    = "WARRANT_EVIDENCE_OUT"
)

// RunConfig says where everything is, and what to break.
type RunConfig struct {
	RepoRoot    string
	CorpusDir   string
	BindingPath string
	PackageDir  string
	// Fault is a portable fault name, or "" for none.
	Fault   string
	FaultOp string
	Filter  string
}

// Prepared is a corpus and binding that agree, with the library's exports resolved.
type Prepared struct {
	Config         RunConfig
	Corpus         *Corpus
	Binding        *Binding
	Exports        map[string]callable
	Sentinels      map[string]error
	CorpusRelative string
}

// Outcome is everything a run learned.
type Outcome struct {
	Examples []ExampleResult
	Suites   []*SuiteResult
	Controls []*ControlResult
	Manifest map[string]any
}

// outcomePassed is true when every example, suite and control passed.
func outcomePassed(outcome *Outcome) bool {
	for _, example := range outcome.Examples {
		if example.Failure != "" {
			return false
		}
	}
	for _, suite := range outcome.Suites {
		if !suitePassed(suite) {
			return false
		}
	}
	for _, control := range outcome.Controls {
		if !controlPassed(control) {
			return false
		}
	}
	return true
}

func noFault(text string) bool { return text == "" || text == "none" }

// parseFaultSpec splits WARRANT_EVIDENCE_FAULT (`<fault>` or `<fault>:<op>`).
// `none` and the empty string mean "no fault" -- a CLEAN run.
func parseFaultSpec(text string) (fault, op string) {
	if noFault(text) {
		return "", ""
	}
	fault, op, _ = strings.Cut(text, ":")
	if _, known := faults[fault]; !known {
		usageFail("unknown-fault", "unknown fault `%s`", fault)
	}
	return fault, op
}

// checkFilterRules is the refusal rule of VER04i 4.3: a filter on its own, or a
// filter beside FAULT=none, would produce a partial CLEAN run that looks like
// evidence. Refuse it.
func checkFilterRules(faultText, filterText string) {
	if filterText != "" && noFault(faultText) {
		usageFail("filter-without-fault", "WARRANT_EVIDENCE_FILTER requires a real WARRANT_EVIDENCE_FAULT")
	}
}

// prepare checks containment, loads, cross-checks and resolves; it refuses by
// panicking with a *LaneError (callers wrap it in guard).
func prepare(config RunConfig, registry *Registry) *Prepared {
	// Containment first: nothing outside the repository is read, not even to
	// find out that it is malformed.
	corpusRelative := requireInside(config.RepoRoot, config.CorpusDir, "corpus directory")
	requireInside(config.RepoRoot, config.BindingPath, "binding file")
	requireInside(config.RepoRoot, config.PackageDir, "package directory")

	corpus := loadCorpus(config.RepoRoot, config.CorpusDir)
	binding := parseBinding(readBytes(config.BindingPath, maxBindingBytes))
	if binding.Module != corpus.Module {
		bindingFail("module", "binding module differs from the corpus module")
	}
	if binding.Lane != "go" || !strings.HasPrefix(binding.Package, "go/") {
		bindingFail("lane", "this runner only runs the go lane")
	}
	sameOps := len(binding.Ops) == len(corpus.Ops)
	for name := range binding.Ops {
		_, known := corpus.Ops[name]
		sameOps = sameOps && known
	}
	if !sameOps {
		bindingFail("totality", "binding ops differ from the corpus ops")
	}
	for _, name := range sortedOpNames(binding.Ops) {
		signature, op := corpus.Ops[name], binding.Ops[name]
		if (len(signature.WidthParams) > 0) != (op.Width != widthNone) {
			bindingFail("width", "op `%s`: width binding disagrees with the spec", name)
		}
		var declared []string
		if signature.Ret.Kind == kindResult {
			declared = corpus.Errors[signature.Ret.Error]
		}
		mapped := make([]string, len(op.Errors))
		for i, row := range op.Errors {
			mapped[i] = row.Variant
		}
		consistent := len(mapped) == len(declared)
		for _, variant := range declared {
			consistent = consistent && containsString(mapped, variant)
		}
		if !consistent {
			bindingFail("totality", "op `%s`: error rows differ from the spec", name)
		}
	}
	exports, sentinels := resolveExports(binding, registry, config.PackageDir)
	return &Prepared{
		Config: config, Corpus: corpus, Binding: binding,
		Exports: exports, Sentinels: sentinels, CorpusRelative: corpusRelative,
	}
}

func (p *Prepared) subject(fault *Fault, faultOp string) *Subject {
	return newSubject(p.Binding, p.Exports, p.Sentinels, p.Corpus.Ops, p.Corpus.Errors, fault, faultOp)
}

// execute runs the corpus. emit receives each TAP-style output line.
func execute(prepared *Prepared, emit func(line string)) *Outcome {
	config, corpus, binding := prepared.Config, prepared.Corpus, prepared.Binding
	faulted := config.Fault != ""
	needle := config.Filter
	selected := func(id string) bool { return needle == "" || strings.Contains(id, needle) }

	if needle != "" {
		matched := false
		for _, e := range corpus.Examples {
			matched = matched || selected(e.ID)
		}
		for _, s := range corpus.Suites {
			matched = matched || selected(s.ID)
		}
		if !matched {
			usageFail("empty-filter", "the filter matched no example or suite")
		}
	}

	outcome := &Outcome{}
	var fault *Fault
	if faulted {
		fault = faults[config.Fault]
	}
	subject := prepared.subject(fault, config.FaultOp)

	for _, example := range runExamples(corpus, subject, selected) {
		outcome.Examples = append(outcome.Examples, example)
		if example.Failure == "" {
			emit("ok " + example.ID)
		} else {
			emit("not ok " + example.ID + " " + example.Failure)
		}
	}

	var todo []Suite
	for _, suite := range corpus.Suites {
		if selected(suite.ID) {
			todo = append(todo, suite)
		}
	}
	for _, suite := range shuffled(todo, binding.Seed) {
		result := runSuite(corpus, &suite, subject, faulted)
		outcome.Suites = append(outcome.Suites, result)
		for _, failed := range result.FailedCases {
			emit("not ok " + suite.ID + "@" + itoa(failed.Index) + " " + failed.Failure)
		}
		if result.Failures > len(result.FailedCases) {
			emit("# " + suite.ID + ": " + itoa(result.Failures) + " failing cases in total")
		}
		for _, failure := range result.SuiteFailures {
			emit("not ok " + suite.ID + " " + failure)
		}
		if suitePassed(result) {
			emit("ok " + suite.ID)
			emit("# " + suite.ID + ": " + itoa(result.Cases) + " cases, " + itoa(result.Discarded) +
				" discarded, " + itoa(result.Calls) + " calls")
		} else if result.Failures > 0 {
			emit("not ok " + suite.ID + " " + result.FailedCases[0].Failure)
		}
	}

	if !faulted && needle == "" {
		for index := range corpus.Controls {
			control := &corpus.Controls[index]
			result := runControl(corpus, control, prepared.subject(faults[control.Fault], control.Op))
			outcome.Controls = append(outcome.Controls, result)
			if controlPassed(result) {
				emit("ok " + control.ID)
			} else {
				emit("not ok " + control.ID + " " + UncaughtFault)
			}
		}
		if outcomePassed(outcome) {
			sourceRoot := filepath.Join(config.PackageDir, filepath.FromSlash(binding.Source))
			outcome.Manifest = buildManifest(corpus, binding, prepared.CorpusRelative, sourceRoot, rows(prepared, outcome))
		}
	}
	return outcome
}

func rows(prepared *Prepared, outcome *Outcome) []map[string]any {
	result := make([]map[string]any, 0, len(outcome.Examples)+len(outcome.Suites)+len(outcome.Controls))
	for _, example := range outcome.Examples {
		result = append(result, map[string]any{
			"id": example.ID, "layer": "portable", "kind": "example", "op": example.Op, "cases": 1, "verdict": "pass",
		})
	}
	byID := map[string]*Suite{}
	for i := range prepared.Corpus.Suites {
		byID[prepared.Corpus.Suites[i].ID] = &prepared.Corpus.Suites[i]
	}
	for _, suiteResult := range outcome.Suites {
		suite := byID[suiteResult.ID]
		result = append(result, map[string]any{
			"id":              suite.ID,
			"layer":           "portable",
			"kind":            "exhaustive",
			"op":              suite.Ops[0],
			"claim":           suite.Claim,
			"cases":           suiteResult.Cases,
			"discarded":       suiteResult.Discarded,
			"calls":           suiteResult.Calls,
			"inputs_sha256":   suiteResult.InputsSha256,
			"expected_sha256": suiteResult.ExpectedSha256,
			"domain":          suite.DomainText,
			"verdict":         "pass",
		})
	}
	for _, control := range outcome.Controls {
		result = append(result, map[string]any{
			"id":        control.ID,
			"layer":     "portable",
			"kind":      "negctl",
			"fault":     control.Fault,
			"op":        control.Op,
			"target":    control.TargetKind,
			"caught_by": append([]string(nil), control.CaughtBy...),
			"verdict":   "pass",
		})
	}
	return result
}

// lookupEnv finds KEY in a KEY=VALUE list (the last one wins, as in a real
// environment block).
func lookupEnv(environ []string, key string) string {
	value := ""
	for _, item := range environ {
		if k, v, found := strings.Cut(item, "="); found && k == key {
			value = v
		}
	}
	return value
}
