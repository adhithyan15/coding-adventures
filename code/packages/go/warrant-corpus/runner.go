package warrantcorpus

// # Executing the corpus against the real library
//
// A run has three movements, in this order (VER04i section 6):
//
//  1. Examples -- the spec's hand-written `example` lines, one case each.
//  2. Exhaustive suites -- every claim over its whole finite domain. The inputs
//     are ENUMERATED HERE, the expected answers come from interpreting the
//     corpus's model IR here, and both streams are hashed. The hashes must equal
//     the ones the Rust compiler computed. That is the cross-check that makes
//     in-lane enumeration safe: a lane that skips a case, reorders the domain, or
//     evaluates a model differently produces a different digest.
//  3. Negative controls -- the clean pass proved the library agrees with the
//     model; this proves the RUNNER can disagree. One op at a time is broken
//     (faults.go) and the suites that mention it must fail.
//
// # What a suite checks, per case
//
//	actual   = subject side   (calls the REAL export through the binding)
//	expected = reference side (pure model, interpreted from the corpus IR)
//	ok  iff  actual is exactly the declared type  and  actual equals expected
//
// The "exactly the declared type" half happens inside Subject.call, at the
// boundary, so a wrong type is reported where it occurred (`type-mismatch`). The
// expected value never depends on the library, so a wrong library cannot change
// the oracle (VER04i 2.5). A failure prints the suite id and case INDEX only --
// never the input bytes (VER00 section 7.3); the index plus the corpus
// reproduces the case in any lane.

// maxReportedCases is how many failing cases are named per suite (the rest are
// counted).
const maxReportedCases = 10

// FailedCase names one failing case of a suite by index and failure id.
type FailedCase struct {
	Index   int
	Failure string
}

// SuiteResult is the outcome of one exhaustive suite.
type SuiteResult struct {
	ID             string
	Cases          int
	Discarded      int
	Calls          int
	InputsSha256   string
	ExpectedSha256 string
	Failures       int
	FailedCases    []FailedCase
	SuiteFailures  []string
}

// suitePassed is true when no case failed and every count and digest matched.
func suitePassed(result *SuiteResult) bool {
	return result.Failures == 0 && len(result.SuiteFailures) == 0
}

func containsSubject(node any) bool {
	list, ok := node.([]any)
	if !ok {
		return false
	}
	if len(list) > 0 && list[0] == "subject_op" {
		return true
	}
	for _, child := range list {
		if containsSubject(child) {
			return true
		}
	}
	return false
}

// classify returns the failure id for one case, or "" when it passes.
func classify(actual, expected any) string {
	if valuesEqual(actual, expected) {
		return ""
	}
	want, expectingPair := expected.(Pair)
	got, returnedPair := actual.(Pair)
	expectingError := expectingPair && want.Tag == "err"
	returnedError := returnedPair && got.Tag == "err"
	if expectingError && !returnedError {
		return MissingError
	}
	return ResultMismatch
}

// evalSubject runs the compiled subject side of one case. A subjectFailure (the
// library misbehaved) comes back as a failure id; a refusal keeps unwinding.
func evalSubject(run compiled, env []any) (value any, failure string) {
	defer func() {
		if r := recover(); r != nil {
			if sf, isFailure := r.(subjectFailure); isFailure {
				value, failure = nil, sf.id
				return
			}
			panic(r)
		}
	}()
	return run(env), ""
}

// runSuite runs one exhaustive suite; see the file comment for the checks.
func runSuite(corpus *Corpus, suite *Suite, subject *Subject, failFast bool) *SuiteResult {
	names := make([]string, len(suite.Binders))
	// Refuse before running anything: the work this suite will do must be the
	// work it declared (the corpus is untrusted; see maxSuiteProduct). The shapes
	// are counted without building the domains.
	size := 1
	for i, binder := range suite.Binders {
		names[i] = binder.Name
		_, count, _ := binderShape(map[string]any{"domain": binder.Descriptor})
		need(count == 0 || size <= maxSafeInt/count, "work", "suite exceeds the per-suite work cap")
		size *= count
	}
	need(size == suite.Cases+suite.Discarded && size <= maxSuiteProduct,
		"work", "declared cases + discarded differ from the binder domains' product")
	domains := make([][]any, len(suite.Binders))
	for i, binder := range suite.Binders {
		domains[i] = domainValues(binder.Descriptor).Values
	}
	binderTypes := make([]*Type, len(suite.BinderTypes))
	for i, text := range suite.BinderTypes {
		binderTypes[i] = parseType(text)
	}
	resultType := parseType(suite.ResultType)

	pure := newCompiler(corpus.Models, nil, corpus.Ops)
	hooked := newCompiler(corpus.Models, subject.call, corpus.Ops)
	admit := pure.compile(suite.Admit, names)
	lhsSubject, rhsSubject := containsSubject(suite.Lhs), containsSubject(suite.Rhs)
	need(lhsSubject != rhsSubject, "unsupported-ir", "exactly one side of a claim may call the subject")
	subjectSide, referenceSide := suite.Rhs, suite.Lhs
	if lhsSubject {
		subjectSide, referenceSide = suite.Lhs, suite.Rhs
	}
	runSubject := hooked.compile(subjectSide, names)
	runReference := pure.compile(referenceSide, names)

	prefix := streamPrefix(suite.ID, suite.BinderTypes, suite.ResultType)
	inputsHash := newStreamHasher(prefix)
	expectedHash := newStreamHasher(prefix)

	result := &SuiteResult{ID: suite.ID}
	callsBefore := subject.totalCalls(suite.Ops)
	index := 0
	stopped := false
	product(domains, func(env []any) bool {
		if admitted, _ := admit(env).(bool); !admitted {
			result.Discarded++
			return true
		}
		expected := runReference(env)
		for position, value := range env {
			inputsHash.write(value, binderTypes[position])
		}
		expectedHash.write(expected, resultType)
		actual, failure := evalSubject(runSubject, env)
		if failure == "" {
			failure = classify(actual, expected)
		}
		index++
		if failure == "" {
			return true
		}
		result.Failures++
		if len(result.FailedCases) < maxReportedCases {
			result.FailedCases = append(result.FailedCases, FailedCase{index - 1, failure})
		}
		if failFast {
			stopped = true
			return false
		}
		return true
	})

	result.Cases = index
	result.Calls = subject.totalCalls(suite.Ops) - callsBefore
	if stopped {
		// A fail-fast run never saw the whole enumeration, so its counts and
		// digests are not comparable; the failing cases already decide it.
		return result
	}
	result.InputsSha256 = inputsHash.hexdigest()
	result.ExpectedSha256 = expectedHash.hexdigest()
	if result.Cases != suite.Cases || result.Discarded != suite.Discarded {
		result.SuiteFailures = append(result.SuiteFailures, CountMismatch)
	}
	if result.InputsSha256 != suite.InputsSha256 || result.ExpectedSha256 != suite.ExpectedSha256 {
		result.SuiteFailures = append(result.SuiteFailures, StreamMismatch)
	}
	if result.Calls != suite.ExpectedCalls {
		result.SuiteFailures = append(result.SuiteFailures, CallCountMismatch)
	}
	return result
}

// ExampleResult is the outcome of one example case.
type ExampleResult struct {
	ID      string
	Op      string
	Failure string
}

// runExamples runs each example once.
//
// cases.tfv was cross-checked equal to these decoded examples when the corpus
// loaded (loadCorpus), so executing the decoded form executes the TFV records.
func runExamples(corpus *Corpus, subject *Subject, only func(id string) bool) []ExampleResult {
	var results []ExampleResult
	for _, example := range corpus.Examples {
		if !only(example.ID) {
			continue
		}
		run := func(env []any) any {
			return subject.call(example.Op, append([]any(nil), example.Args...), example.Width, example.HasWidth)
		}
		actual, failure := evalSubject(run, nil)
		if failure == "" {
			failure = classify(actual, example.Expected)
		}
		results = append(results, ExampleResult{ID: example.ID, Op: example.Op, Failure: failure})
	}
	return results
}

// ControlResult is the outcome of one negative control.
type ControlResult struct {
	ID            string
	Fault         string
	Op            string
	TargetKind    string
	CaughtBy      []string
	ReachedExport bool
}

// controlPassed: a control passes iff a required claim failed AND the export was
// called.
func controlPassed(result *ControlResult) bool {
	return len(result.CaughtBy) > 0 && result.ReachedExport
}

// runControl breaks one op, reruns exactly the control's target suites, and
// records who noticed.
func runControl(corpus *Corpus, control *Control, subject *Subject) *ControlResult {
	byID := map[string]*Suite{}
	for i := range corpus.Suites {
		byID[corpus.Suites[i].ID] = &corpus.Suites[i]
	}
	caught := []string{}
	for _, suiteID := range control.RequiredClaims {
		if runSuite(corpus, byID[suiteID], subject, true).Failures > 0 {
			caught = append(caught, suiteID)
		}
	}
	return &ControlResult{
		ID:            control.ID,
		Fault:         control.Fault,
		Op:            control.Op,
		TargetKind:    control.TargetKind,
		CaughtBy:      caught,
		ReachedExport: subject.callsTo(control.Op) > 0,
	}
}

// shuffled orders suites by a seeded Fisher-Yates shuffle (SplitMix64).
//
// Suite order is the only thing the evidence seed controls (VER04i 6): case order
// is canonical. Shuffling stops a suite from silently depending on state left
// behind by the one before it. Every step wraps at 64 bits (Go's uint64 does so
// natively), so the order is the same one the Python and TypeScript lanes draw.
func shuffled[T any](items []T, seed uint64) []T {
	state := seed
	next := func() uint64 {
		state += 0x9e3779b97f4a7c15
		z := state
		z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9
		z = (z ^ (z >> 27)) * 0x94d049bb133111eb
		return z ^ (z >> 31)
	}
	order := append([]T(nil), items...)
	for position := len(order) - 1; position > 0; position-- {
		other := int(next() % uint64(position+1))
		order[position], order[other] = order[other], order[position]
	}
	return order
}
