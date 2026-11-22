package warrantcorpus

import (
	"math"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"
)

// # Loading and validating a warrant-corpus-v1 directory
//
// The corpus is produced by the Rust compiler (warrant-grammar) and is UNTRUSTED
// input here. Loading it is therefore a long list of "this must be exactly so",
// each failing closed:
//
//  1. the files are size-capped, regular, strictly parsed (no duplicate keys);
//  2. every object has exactly the expected keys (a new field is a new profile,
//     not something to skip);
//  3. spec_sha256 is re-derived from the `.warrant` file on disk (StaleEvidence
//     caught at run time, VER04i 6 step 1);
//  4. each negative control is RE-DERIVED from the fault registry, the op
//     signatures and the suites that mention the op, and must equal the corpus's
//     row -- a corpus cannot quietly omit a control or point it at the wrong
//     claims;
//  5. cases.tfv is cross-checked, record by record, against the examples in
//     corpus.json: two encodings of the same facts must agree.
//
// Nothing here EXECUTES the library; that is runner.go.

// corpusProfile is the profile string every consumer must see.
const corpusProfile = "warrant-corpus-v1"

// Work limits (VER04i section 3; the security review of WD-101). The corpus is
// untrusted, so the Cartesian product of a suite's binder domains must equal the
// cases it declares (admitted + discarded) and stay under a cap BEFORE any case
// runs. The real ct-compare corpus needs 240,818 at most per suite and 812,927
// for one clean pass; with every negative control re-running its target suites
// the total is about 2.1 million.
const (
	// maxSuiteProduct bounds the tuples enumerated by one suite.
	maxSuiteProduct = 1_000_000
	// maxTotalWork bounds all suites once, plus every control's re-runs.
	maxTotalWork = 8_000_000
	maxBinders   = 8
)

var (
	hex64RE     = regexp.MustCompile(`^[0-9a-f]{64}$`)
	identRE     = regexp.MustCompile(`^[a-z][a-z0-9_]*$`)
	topKeys     = []string{"schema_version", "profile", "module", "spec", "spec_sha256", "generator", "types", "ops", "models", "examples", "suites", "faults", "controls"}
	suiteKeys   = []string{"id", "kind", "claim", "ops", "binders", "admit", "lhs", "rhs", "cases", "discarded", "expected_calls", "domain_text", "inputs_sha256", "expected_sha256", "result_type"}
	exampleKeys = []string{"args", "expected", "id", "op", "result_type", "width"}
)

// Example is one `example` from the spec: call Op on Args, expect Expected.
type Example struct {
	ID       string
	Op       string
	Args     []any
	Expected any
	Width    int
	HasWidth bool
}

// Binder is one named, enumerated variable of a suite.
type Binder struct {
	Name       string
	Descriptor string
}

// Suite is one exhaustive suite: a claim over a finite, enumerable domain.
type Suite struct {
	ID             string
	Claim          string
	Ops            []string
	Binders        []Binder
	BinderTypes    []string
	Admit          any
	Lhs            any
	Rhs            any
	ResultType     string
	Cases          int
	Discarded      int
	ExpectedCalls  int
	DomainText     string
	InputsSha256   string
	ExpectedSha256 string
}

// Control is one generated negative control (suite kind, op, fault).
type Control struct {
	ID             string
	Fault          string
	Op             string
	TargetKind     string
	RequiredClaims []string
}

// Corpus is a validated corpus, ready to run.
type Corpus struct {
	Module       string
	Spec         string
	SpecSha256   string
	CorpusSha256 string
	CasesSha256  string
	Errors       ErrorTypes
	Ops          map[string]*OpSig
	Models       map[string]any
	Examples     []Example
	Suites       []Suite
	Controls     []Control
}

// exact checks that obj is an object with exactly the given keys.
func exact(obj any, keys []string, where string) map[string]any {
	object, ok := obj.(map[string]any)
	valid := ok && len(object) == len(keys)
	if valid {
		for _, key := range keys {
			if _, has := object[key]; !has {
				valid = false
			}
		}
	}
	need(valid, "schema", "%s has unexpected fields", where)
	return object
}

func text(value any, where string) string {
	s, ok := value.(string)
	need(ok && s != "", "schema", "%s must be a non-empty string", where)
	return s
}

func count(value any, where string) int {
	n, ok := value.(int64)
	need(ok && n >= 0 && n <= math.MaxInt, "schema", "%s must be a count", where)
	if n < 0 || n > math.MaxInt {
		return 0
	}
	return int(n)
}

func digest(value any, where string) string {
	s, ok := value.(string)
	need(ok && hex64RE.MatchString(s), "schema", "%s must be a lowercase SHA-256", where)
	return s
}

func sortedKeys(object map[string]any) []string {
	keys := make([]string, 0, len(object))
	for key := range object {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	return keys
}

// repoPath resolves a repository-relative path, refusing anything that escapes.
//
// A colon is refused too: on Windows "C:evil" is drive-relative and "a:b" names
// an NTFS alternate data stream, and no corpus path has one.
func repoPath(root, relative string) string {
	valid := !strings.ContainsAny(relative, `\:`) && !strings.HasPrefix(relative, "/")
	if valid {
		for _, part := range strings.Split(relative, "/") {
			if part == "" || part == "." || part == ".." {
				valid = false
			}
		}
	}
	need(valid, "schema", "corpus path must be a normalised repository-relative path")
	return filepath.Join(root, filepath.FromSlash(relative))
}

func signatures(rawOps any) map[string]*OpSig {
	object, ok := rawOps.(map[string]any)
	need(ok && len(object) > 0, "schema", "ops must be a non-empty object")
	ops := map[string]*OpSig{}
	for _, name := range sortedKeys(object) {
		need(identRE.MatchString(name), "schema", "bad op name")
		fields, ok := object[name].(map[string]any)
		valid := ok
		if ok {
			_, hasParams := fields["params"]
			_, hasRet := fields["ret"]
			valid = hasParams && hasRet
			for key := range fields {
				if key != "params" && key != "ret" && key != "width_params" && key != "fresh" {
					valid = false
				}
			}
		}
		need(valid, "schema", "malformed op signature")
		rawParams, ok := fields["params"].([]any)
		need(ok, "schema", "op params must be a list")
		var params []Param
		for _, item := range rawParams {
			pair, isPair := item.([]any)
			var paramName, paramType string
			var nameOK, typeOK bool
			if isPair && len(pair) == 2 {
				paramName, nameOK = pair[0].(string)
				paramType, typeOK = pair[1].(string)
			}
			need(nameOK && typeOK, "schema", "malformed op parameter")
			params = append(params, Param{Name: paramName, Ty: parseType(paramType)})
		}
		var widths []string
		if rawWidths, has := fields["width_params"]; has {
			list, isList := rawWidths.([]any)
			need(isList, "schema", "bad widths")
			for _, item := range list {
				width, isString := item.(string)
				need(isString, "schema", "bad widths")
				widths = append(widths, width)
			}
		}
		fresh, _ := fields["fresh"].(bool)
		ops[name] = &OpSig{
			Name:        name,
			Params:      params,
			Ret:         parseType(text(fields["ret"], "op ret")),
			WidthParams: widths,
			Fresh:       fresh,
		}
	}
	return ops
}

func decodeExample(row any, ops map[string]*OpSig, errors ErrorTypes, module string) Example {
	fields := exact(row, exampleKeys, "example")
	id := text(fields["id"], "example id")
	need(strings.HasPrefix(id, "warrant/"+module+"/example/"), "schema", "example id is outside the module")
	op := text(fields["op"], "example op")
	sig, known := ops[op]
	need(known, "example", "example names an unknown op")
	args, isList := fields["args"].([]any)
	need(isList && len(args) == len(sig.Params), "example", "example arity mismatch")
	need(fields["result_type"] == sig.Ret.Text, "example", "example result type disagrees")
	example := Example{ID: id, Op: op}
	width := fields["width"]
	if len(sig.WidthParams) > 0 {
		n, isInt := width.(int64)
		need(isInt && n >= 0 && n <= math.MaxInt, "example", "fixed-width example lacks a width")
		if n < 0 || n > math.MaxInt {
			return Example{}
		}
		example.Width, example.HasWidth = int(n), true
	} else {
		need(width == nil, "example", "unexpected example width")
	}
	for i, raw := range args {
		ty := sig.Params[i].Ty
		value := decodeJSON(raw, ty, errors)
		if ty.HasWidth && ty.WidthName != "" {
			// Bytes<N>: the tag keeps the literal parameter name and the actual
			// length must equal the example's static width.
			need(len(value.([]byte)) == example.Width, "example", "example buffer is not N bytes")
		}
		example.Args = append(example.Args, value)
	}
	example.Expected = decodeJSON(fields["expected"], sig.Ret, errors)
	return example
}

func decodeSuite(row any, ops map[string]*OpSig, module string) Suite {
	fields := exact(row, suiteKeys, "suite")
	claim := text(fields["claim"], "suite claim")
	id := text(fields["id"], "suite id")
	need(id == "warrant/"+module+"/exhaustive/"+claim && fields["kind"] == "exhaustive",
		"schema", "suite id does not follow the claim")
	rawOps, isList := fields["ops"].([]any)
	need(isList && len(rawOps) > 0, "schema", "suite names an unknown op")
	suite := Suite{ID: id, Claim: claim}
	for _, item := range rawOps {
		name, isString := item.(string)
		_, known := ops[name]
		need(isString && known, "schema", "suite names an unknown op")
		suite.Ops = append(suite.Ops, name)
	}
	binders, isList := fields["binders"].([]any)
	need(isList && len(binders) > 0 && len(binders) <= maxBinders, "schema", "suite needs between 1 and MAX_BINDERS binders")
	seen := map[string]bool{}
	product := 1
	for _, item := range binders {
		pair, isPair := item.([]any)
		var name string
		var nameOK bool
		if isPair && len(pair) == 2 {
			name, nameOK = pair[0].(string)
		}
		need(nameOK, "schema", "bad binder")
		descriptor, size, typeText := binderShape(pair[1])
		// Refuse before the product can overflow.
		need(size == 0 || product <= maxSafeInt/size, "work", "suite exceeds the per-suite work cap")
		product *= size
		need(!seen[name], "schema", "duplicate binder name")
		seen[name] = true
		suite.Binders = append(suite.Binders, Binder{Name: name, Descriptor: descriptor})
		suite.BinderTypes = append(suite.BinderTypes, typeText)
	}
	suite.ResultType = text(fields["result_type"], "suite result type")
	parseType(suite.ResultType)
	suite.Cases = count(fields["cases"], "cases")
	suite.Discarded = count(fields["discarded"], "discarded")
	need(suite.Cases > 0, "schema", "a suite must admit at least one case (no vacuous suites)")
	need(product == suite.Cases+suite.Discarded, "work", "declared cases + discarded differ from the binder domains' product")
	need(product <= maxSuiteProduct, "work", "suite exceeds the per-suite work cap")
	suite.Admit, suite.Lhs, suite.Rhs = fields["admit"], fields["lhs"], fields["rhs"]
	suite.ExpectedCalls = count(fields["expected_calls"], "expected_calls")
	suite.DomainText = text(fields["domain_text"], "domain_text")
	suite.InputsSha256 = digest(fields["inputs_sha256"], "inputs_sha256")
	suite.ExpectedSha256 = digest(fields["expected_sha256"], "expected_sha256")
	return suite
}

// deriveControls re-derives the controls the corpus MUST contain (VER04i 4.1).
func deriveControls(module string, rawFaults any, suites []Suite, ops map[string]*OpSig) []Control {
	list, ok := rawFaults.([]any)
	need(ok && len(list) > 0, "schema", "faults must be a non-empty list")
	var derived []Control
	seen := map[string]bool{}
	for _, row := range list {
		fields := exact(row, []string{"applies", "fault", "op"}, "fault")
		name := text(fields["fault"], "fault")
		op := text(fields["op"], "fault op")
		fault, known := faults[name]
		need(known, "fault", "corpus names a fault the lane does not implement")
		sig, hasOp := ops[op]
		need(hasOp && fault.Applies(sig), "fault", "fault does not apply to op")
		need(fields["applies"] == "input" || fields["applies"] == "output", "fault", "bad fault stage")
		key := name + "\x00" + op
		need(!seen[key], "fault", "duplicate fault row")
		seen[key] = true
		var required []string
		for _, suite := range suites {
			if containsString(suite.Ops, op) {
				required = append(required, suite.ID)
			}
		}
		need(len(required) > 0, "fault", "no claim can catch a registered fault")
		derived = append(derived, Control{
			ID:             "warrant/" + module + "/negctl/exhaustive/" + op + "/" + name,
			Fault:          name,
			Op:             op,
			TargetKind:     "exhaustive",
			RequiredClaims: required,
		})
	}
	// The compiler lists controls sorted by id; so must the corpus.
	sort.Slice(derived, func(i, j int) bool { return derived[i].ID < derived[j].ID })
	return derived
}

// checkTFV cross-checks cases.tfv against corpus.json's examples.
func checkTFV(source string, examples []Example, ops map[string]*OpSig, errors ErrorTypes) {
	records := parseTFV(source)
	need(len(records) == len(examples), "tfv", "TFV record count differs from the examples")
	for index, record := range records {
		example := examples[index]
		need(record.ID == example.ID, "tfv", "TFV example order or identity differs")
		sig := ops[example.Op]
		expected := map[string]bool{"op": true, "expect": true}
		for _, param := range sig.Params {
			expected[param.Name] = true
		}
		if len(sig.WidthParams) > 0 {
			expected["width"] = true
		}
		valid := len(record.Fields) == len(expected)
		for key := range record.Fields {
			valid = valid && expected[key]
		}
		need(valid, "tfv", "TFV fields disagree with the signature")
		op := record.Fields["op"]
		need(op.Kind == "text" && op.Text == example.Op, "tfv", "TFV op differs from the example")
		for position, param := range sig.Params {
			value := decodeTFV(record.Fields[param.Name], param.Ty, errors)
			need(valuesEqual(value, example.Args[position]), "tfv", "TFV argument differs")
		}
		if len(sig.WidthParams) > 0 {
			width := record.Fields["width"]
			need(width.Kind == "int" && example.HasWidth && width.Text == itoa(example.Width), "tfv", "TFV width differs")
		}
		need(valuesEqual(decodeTFV(record.Fields["expect"], sig.Ret, errors), example.Expected), "tfv", "TFV expected value differs")
	}
}

func itoa(n int) string { return strconv.Itoa(n) }

// loadCorpus reads, validates and decodes the corpus in corpusDir.
//
// Every malformation is a refusal (exit status 2). The explicit checks cover the
// shapes we thought of; the guard below turns any shape we did not (a value of
// the wrong Go type where an object belongs) into the same refusal instead of an
// internal error.
func loadCorpus(repoRoot, corpusDir string) *Corpus {
	defer func() {
		if r := recover(); r != nil {
			if _, isLane := r.(*LaneError); isLane {
				panic(r)
			}
			fail("schema", "corpus has a malformed field type")
		}
	}()
	return loadCorpusUnguarded(repoRoot, corpusDir)
}

func loadCorpusUnguarded(repoRoot, corpusDir string) *Corpus {
	corpusBytes := readBytes(filepath.Join(corpusDir, "corpus.json"), maxCorpusBytes)
	casesBytes := readBytes(filepath.Join(corpusDir, "cases.tfv"), maxTextBytes)
	root := exact(parseJSON(corpusBytes), topKeys, "corpus")
	need(root["schema_version"] == int64(1) && root["profile"] == corpusProfile, "schema", "wrong profile")
	module := text(root["module"], "module")
	need(identRE.MatchString(module), "schema", "module must be an identifier")

	spec := text(root["spec"], "spec")
	specPath := repoPath(repoRoot, spec)
	_, inside := resolveInside(repoRoot, specPath, true)
	need(inside, "schema", "spec path escapes the repository")
	specBytes := readBytes(specPath, maxTextBytes)
	specSha256 := digest(root["spec_sha256"], "spec_sha256")
	need(sha256Hex(specBytes) == specSha256, "stale-evidence", "spec_sha256 does not match the Warrant spec on disk")

	rawTypes, ok := root["types"].(map[string]any)
	need(ok, "schema", "types must be an object")
	errors := ErrorTypes{}
	for _, name := range sortedKeys(rawTypes) {
		variants, isList := rawTypes[name].([]any)
		need(errorNameRE.MatchString(name) && isList, "schema", "malformed error type")
		list := []string{}
		for _, variant := range variants {
			text, isString := variant.(string)
			need(isString, "schema", "malformed error type")
			list = append(list, text)
		}
		errors[name] = list
	}
	ops := signatures(root["ops"])
	models, ok := root["models"].(map[string]any)
	need(ok, "schema", "models must be an object")

	rawExamples, ok := root["examples"].([]any)
	need(ok, "schema", "examples must be a list")
	examples := make([]Example, 0, len(rawExamples))
	exampleIDs := map[string]bool{}
	for _, row := range rawExamples {
		example := decodeExample(row, ops, errors, module)
		need(!exampleIDs[example.ID], "schema", "duplicate example id")
		exampleIDs[example.ID] = true
		examples = append(examples, example)
	}

	rawSuites, ok := root["suites"].([]any)
	need(ok && len(rawSuites) > 0, "schema", "suites must be a list")
	suites := make([]Suite, 0, len(rawSuites))
	suiteIDs := map[string]bool{}
	for _, row := range rawSuites {
		suite := decodeSuite(row, ops, module)
		need(!suiteIDs[suite.ID], "schema", "duplicate suite id")
		suiteIDs[suite.ID] = true
		suites = append(suites, suite)
	}

	controls := deriveControls(module, root["faults"], suites, ops)
	rawControls, ok := root["controls"].([]any)
	need(ok, "schema", "controls must be a list")
	for _, row := range rawControls {
		fields, isObj := row.(map[string]any)
		claims, isList := []any(nil), false
		if isObj {
			claims, isList = fields["required_claims"].([]any)
		}
		valid := isObj && isList
		for _, claim := range claims {
			_, isString := claim.(string)
			valid = valid && isString
		}
		need(valid, "schema", "control required_claims must be a list of strings")
	}
	need(len(rawControls) == len(controls), "control", "controls differ from those derived from the faults")
	for index, row := range rawControls {
		fields := exact(row, []string{"fault", "id", "op", "required_claims", "target_kind"}, "control")
		want := controls[index]
		claims := fields["required_claims"].([]any)
		same := fields["id"] == want.ID && fields["fault"] == want.Fault && fields["op"] == want.Op &&
			fields["target_kind"] == want.TargetKind && len(claims) == len(want.RequiredClaims)
		for at, claim := range claims {
			same = same && at < len(want.RequiredClaims) && claim == want.RequiredClaims[at]
		}
		need(same, "control", "controls differ from those derived from the faults")
	}

	sizes := map[string]int{}
	total := 0
	for _, suite := range suites {
		sizes[suite.ID] = suite.Cases + suite.Discarded
		total += suite.Cases + suite.Discarded
	}
	for _, control := range controls {
		for _, claim := range control.RequiredClaims {
			total += sizes[claim]
		}
	}
	need(total <= maxTotalWork, "work", "total work (suites plus control re-runs) exceeds the work cap")
	checkTFV(decodeUTF8(casesBytes, "tfv"), examples, ops, errors)
	return &Corpus{
		Module:       module,
		Spec:         spec,
		SpecSha256:   specSha256,
		CorpusSha256: sha256Hex(corpusBytes),
		CasesSha256:  sha256Hex(casesBytes),
		Errors:       errors,
		Ops:          ops,
		Models:       models,
		Examples:     examples,
		Suites:       suites,
		Controls:     controls,
	}
}

// listControlRows returns `<fault>:<op>` for each control, read from corpus.json
// alone. Used by list-faults and sabotage; it needs neither the spec nor the
// library.
func listControlRows(corpusDir string) []string {
	root, ok := parseJSON(readBytes(filepath.Join(corpusDir, "corpus.json"), maxCorpusBytes)).(map[string]any)
	need(ok, "schema", "no controls")
	controls, ok := root["controls"].([]any)
	need(ok, "schema", "no controls")
	rows := make([]string, 0, len(controls))
	for _, control := range controls {
		fields, isObj := control.(map[string]any)
		need(isObj, "schema", "bad control")
		fault, faultOK := fields["fault"].(string)
		op, opOK := fields["op"].(string)
		need(faultOK && opOK, "schema", "bad control")
		rows = append(rows, fault+":"+op)
	}
	return rows
}
