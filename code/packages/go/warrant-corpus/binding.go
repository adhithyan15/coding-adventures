package warrantcorpus

import (
	"path/filepath"
	"reflect"
	"regexp"
	"runtime"
	"strconv"
	"strings"
	"unicode/utf8"
)

// # The binding table: Warrant operations -> this lane's callables
//
// `evidence.bindings` is DATA. It says which public export implements each
// Warrant operation and how a returned error becomes a declared `Err` -- and
// nothing else. It cannot hold logic (a computed answer, a retry, a fallback):
// that is precisely why it is a flat file of directives and not Go. See VER04i
// section 5.3 for the grammar:
//
//	lane go
//	package go/ct-compare
//	module ct_compare
//	import github.com/adhithyan15/coding-adventures/code/packages/go/ct-compare
//	source .
//	seed 0x5741_5252_414e_5421
//	values Bool=bool Bytes=[]byte Nat=uint64 U64=uint64
//	op eq export=CTEq
//	op select_bytes export=CTSelectBytes errors=CT_SELECT_LENGTH_MISMATCH:ErrLengthMismatch
//
// # Go cannot load a package by name at run time
//
// The other lanes `import()` or `importlib` the library named by the binding. A
// Go program has no such thing: the set of exports is fixed when the entry
// program is compiled. So the entry program passes a Registry -- a table of
// adapters -- and this file checks that the registry and the binding describe the
// same code. Two guards make that trustworthy at the boundary:
//
//   - Resolution at start-up. Every export must exist, be callable, and every
//     sentinel error must exist, BEFORE a single case runs (a lazily-missing
//     export would otherwise surface as a pass on whichever suites never reached
//     it).
//   - Provenance. For each export, the registry names the real library function
//     (Impl). The runner never calls it; it asks the Go runtime what it IS:
//     runtime.FuncForPC must give exactly `<import>.<export>` (so a closure, a
//     method or another package's function is refused) and the function's
//     recorded source file must lie under the declared `source` root. The
//     manifest's source_sha256 digests that root, so the digest describes code
//     that was compiled into the binary that ran.
//
// Error normalisation is closed (VER04i 5, rule 2): an adapter returns the value
// or a sentinel error. A returned error that IS a mapped sentinel (identity, not
// errors.Is, not a message) becomes Err(variant); anything else, and any panic
// that escapes an adapter, is `unexpected-error`.
//
// # Mutable bytes
//
// A []byte is mutable. So every Bytes argument is handed to the library as ITS
// OWN fresh copy (the enumerated domain can never be corrupted by a library that
// writes to its input), and for a `fresh` op the result is checked by mutation
// (rule 4): flipping the result must not change an argument, and flipping an
// argument must not change the result.
const maxBindingBytes = 64 * 1024

var (
	nameRE     = regexp.MustCompile(`^[a-z][a-z0-9_]*$`)
	exportRE   = regexp.MustCompile(`^[A-Za-z_][A-Za-z0-9_]*$`)
	variantRE  = regexp.MustCompile(`^[A-Z][A-Z0-9_]*$`)
	sentinelRE = regexp.MustCompile(`^Err[A-Za-z0-9_]*$`)
	seedRE     = regexp.MustCompile(`^(?:0x[0-9a-fA-F_]+|[0-9][0-9_]*)$`)
	importRE   = regexp.MustCompile(`^[A-Za-z0-9_.~-]+(?:/[A-Za-z0-9_.~-]+)*$`)
	sourceRE   = regexp.MustCompile(`^(?:\.|[A-Za-z0-9_.@-]+(?:/[A-Za-z0-9_.@-]+)*)$`)
	singles    = []string{"lane", "package", "module", "import", "source", "seed"}
)

// allowedValues is the only Go type each Warrant scalar may have (VER04i 5.3).
var allowedValues = map[string]string{
	"Bool":  "bool",
	"Bytes": "[]byte",
	"U8":    "uint8",
	"U64":   "uint64",
	"Nat":   "uint64",
}

// Width modes of an op binding.
const (
	widthNone     = "none"
	widthImplicit = "implicit"
	widthRuntime  = "runtime"
)

// ErrorRow maps a declared Err variant to the name of a sentinel error.
type ErrorRow struct {
	Variant  string
	Sentinel string
}

// OpBinding says how one Warrant op maps to an export.
type OpBinding struct {
	Name       string
	ExportName string
	Width      string
	Errors     []ErrorRow
}

// Binding is a parsed `evidence.bindings`.
type Binding struct {
	Lane       string
	Package    string
	Module     string
	ImportPath string
	Source     string
	Seed       uint64
	Values     map[string]string
	Ops        map[string]*OpBinding
	Raw        []byte
}

func parseOp(parts []string) *OpBinding {
	if len(parts) < 2 || !nameRE.MatchString(parts[0]) {
		bindingFail("syntax", "malformed op directive")
	}
	name := parts[0]
	options := map[string]string{}
	for _, item := range parts[1:] {
		key, value, found := strings.Cut(item, "=")
		_, duplicate := options[key]
		if !found || value == "" || duplicate {
			bindingFail("syntax", "malformed or duplicate op option `%s`", key)
		}
		options[key] = value
	}
	for key := range options {
		if key != "export" && key != "width" && key != "errors" {
			bindingFail("syntax", "op `%s` needs export= and only known options", name)
		}
	}
	exportName, hasExport := options["export"]
	if !hasExport {
		bindingFail("syntax", "op `%s` needs export= and only known options", name)
	}
	if !exportRE.MatchString(exportName) {
		bindingFail("syntax", "export must be an identifier")
	}
	width := options["width"]
	switch width {
	case "":
		width = widthNone
	case widthImplicit, widthRuntime:
	default:
		bindingFail("syntax", "width must be implicit or runtime")
	}
	op := &OpBinding{Name: name, ExportName: exportName, Width: width}
	for _, row := range strings.Split(options["errors"], ",") {
		if row == "" {
			continue
		}
		variant, sentinel, found := strings.Cut(row, ":")
		if !found || !variantRE.MatchString(variant) {
			bindingFail("syntax", "errors rows are VARIANT:SentinelName")
		}
		if !sentinelRE.MatchString(sentinel) {
			bindingFail("bad-error", "`%s` is not an allowed sentinel name (Err followed by an identifier)", sentinel)
		}
		op.Errors = append(op.Errors, ErrorRow{Variant: variant, Sentinel: sentinel})
	}
	return op
}

// parseSeed reads a u64 written in decimal or 0x hex, with `_` separators.
func parseSeed(text string) uint64 {
	if !seedRE.MatchString(text) {
		bindingFail("syntax", "seed must be a decimal or 0x hex u64")
	}
	digits := strings.ReplaceAll(text, "_", "")
	base := 10
	if strings.HasPrefix(digits, "0x") || strings.HasPrefix(digits, "0X") {
		base, digits = 16, digits[2:]
	}
	seed, err := strconv.ParseUint(digits, base, 64)
	if err != nil {
		bindingFail("syntax", "seed must be a decimal or 0x hex u64 inside u64")
	}
	return seed
}

func validPathValue(key, value string) bool {
	if key == "import" && !importRE.MatchString(value) {
		return false
	}
	if key == "source" && !sourceRE.MatchString(value) {
		return false
	}
	if value == "." {
		return key == "source"
	}
	for _, part := range strings.Split(value, "/") {
		if part == "." || part == ".." {
			return false
		}
	}
	return true
}

// parseBinding parses the directive file; any irregularity is a refusal.
func parseBinding(data []byte) *Binding {
	if len(data) > maxBindingBytes {
		bindingFail("input-limit", "binding file is too large")
	}
	if !utf8.Valid(data) {
		bindingFail("syntax", "binding is not UTF-8")
	}
	text := string(data)
	single := map[string]string{}
	values := map[string]string{}
	ops := map[string]*OpBinding{}
	normalized := strings.NewReplacer("\r\n", "\n", "\r", "\n").Replace(text)
	for _, rawLine := range strings.Split(normalized, "\n") {
		line := strings.TrimSpace(rawLine)
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		fields := strings.Fields(line)
		directive, parts := fields[0], fields[1:]
		switch {
		case directive == "op":
			op := parseOp(parts)
			if _, duplicate := ops[op.Name]; duplicate {
				bindingFail("syntax", "duplicate op `%s`", op.Name)
			}
			ops[op.Name] = op
		case directive == "values":
			for _, item := range parts {
				key, value, found := strings.Cut(item, "=")
				_, duplicate := values[key]
				if !found || duplicate {
					bindingFail("syntax", "malformed or duplicate values row")
				}
				values[key] = value
			}
		case containsString(singles, directive):
			_, duplicate := single[directive]
			if len(parts) != 1 || duplicate {
				bindingFail("syntax", "`%s` takes one value, once", directive)
			}
			single[directive] = parts[0]
		default:
			bindingFail("syntax", "unknown directive `%s`", directive)
		}
	}
	var missing []string
	for _, key := range singles {
		if _, has := single[key]; !has {
			missing = append(missing, key)
		}
	}
	if len(missing) > 0 || len(ops) == 0 {
		if len(missing) == 0 {
			missing = []string{"ops"}
		}
		bindingFail("incomplete", "binding lacks %s", strings.Join(missing, ", "))
	}
	for _, key := range []string{"import", "source"} {
		if !validPathValue(key, single[key]) {
			bindingFail("syntax", "%s must be a normalised value (a Go import path, or a package-relative directory)", key)
		}
	}
	seed := parseSeed(single["seed"])
	// Rule 3: width representations are stated, and forbidden ones are refused.
	for scalar, laneType := range values {
		if allowed, known := allowedValues[scalar]; !known || allowed != laneType {
			bindingFail("values", "`%s=%s` is not an allowed Go representation", scalar, laneType)
		}
	}
	return &Binding{
		Lane:       single["lane"],
		Package:    single["package"],
		Module:     single["module"],
		ImportPath: single["import"],
		Source:     single["source"],
		Seed:       seed,
		Values:     values,
		Ops:        ops,
		Raw:        data,
	}
}

// ---------------------------------------------------------------------------
// The registry: what the entry program compiles in
// ---------------------------------------------------------------------------

// Export is one library function as the entry program supplies it.
//
// Call is the ADAPTER: it receives library-form arguments (bool, uint8, uint64,
// or a fresh []byte per Warrant Bool, U8, U64/Nat, Bytes) and returns the
// library's value, or a sentinel error. It may translate values and errors and
// nothing else: no computed answer, no retry, no fallback. A panic that leaves it
// is `unexpected-error`.
//
// Impl is the real library function the adapter wraps. The runner never calls it;
// it asks the runtime for its symbol name and source file (provenance, see the
// file comment).
type Export struct {
	Call func(args []any) (any, error)
	Impl any
}

// Registry is the table the entry program passes to Main: its exports by export
// name, and its sentinel errors by name.
type Registry struct {
	Exports map[string]Export
	Errors  map[string]error
}

// callable is a resolved export.
type callable func(args []any) (any, error)

// symbolOf is how resolveExports asks the runtime about an export. It is a
// variable only so tests can report a scratch source file for the reference
// library; production code never changes it.
var symbolOf = funcSymbol

// funcSymbol returns the runtime's name and source file for a top-level function
// value, or ok == false when impl is not a function.
func funcSymbol(impl any) (name, file string, ok bool) {
	value := reflect.ValueOf(impl)
	if !value.IsValid() || value.Kind() != reflect.Func || value.IsNil() {
		return "", "", false
	}
	fn := runtime.FuncForPC(value.Pointer())
	if fn == nil {
		return "", "", false
	}
	file, _ = fn.FileLine(fn.Entry())
	return fn.Name(), file, true
}

// resolveExports checks the registry against the binding and returns the
// callables, or refuses to start.
func resolveExports(binding *Binding, registry *Registry, packageDir string) (map[string]callable, map[string]error) {
	if registry == nil {
		bindingFail("registry", "no export registry was supplied by the entry program")
	}
	root, err := realPath(filepath.Join(packageDir, filepath.FromSlash(binding.Source)))
	if err != nil {
		bindingFail("import", "cannot resolve the source root `%s` under the package", binding.Source)
	}
	exports := map[string]callable{}
	sentinels := map[string]error{}
	for _, name := range sortedOpNames(binding.Ops) {
		op := binding.Ops[name]
		export, present := registry.Exports[op.ExportName]
		if !present || export.Call == nil {
			bindingFail("export", "op `%s`: `%s` is missing or not callable", name, op.ExportName)
		}
		symbol, file, isFunc := symbolOf(export.Impl)
		if !isFunc {
			bindingFail("export", "op `%s`: `%s` has no library function to check", name, op.ExportName)
		}
		if symbol != binding.ImportPath+"."+op.ExportName {
			bindingFail("source-mismatch", "`%s` is not the function `%s` of the declared import", op.ExportName, op.ExportName)
		}
		resolved, err := realPath(filepath.FromSlash(file))
		if err != nil || !filepath.IsAbs(filepath.FromSlash(file)) {
			bindingFail("source-mismatch", "the source file of `%s` cannot be found (a build with -trimpath cannot run this lane)", op.ExportName)
		}
		if _, inside := relativeInside(root, resolved, hostFlavor); !inside {
			bindingFail("source-mismatch", "`%s` is not inside the declared source root", op.ExportName)
		}
		exports[name] = export.Call
		for _, row := range op.Errors {
			sentinel, found := registry.Errors[row.Sentinel]
			if !found || sentinel == nil {
				bindingFail("bad-error", "op `%s`: sentinel `%s` is not in the registry", name, row.Sentinel)
			}
			sentinels[row.Sentinel] = sentinel
		}
	}
	return exports, sentinels
}

func sortedOpNames(ops map[string]*OpBinding) []string {
	names := make([]string, 0, len(ops))
	for name := range ops {
		names = append(names, name)
	}
	sortStrings(names)
	return names
}

// ---------------------------------------------------------------------------
// The subject
// ---------------------------------------------------------------------------

// subjectFailure is the library misbehaving for one case: a verdict, not a lane
// error. It travels as a panic value so the compiled IR closures stay simple and
// is recovered per case (evalCase).
type subjectFailure struct{ id string }

// Subject is the real library, seen through the binding, with a call counter.
//
// call is the ONLY path from a corpus case to the library. It counts invocations
// (the runner compares the count with the corpus's expected_calls, so a runner
// that never calls the binding is caught), applies an optional fault, type-checks
// the result exactly, and returns it in canonical form: a value, or
// Pair{"err", VARIANT}.
type Subject struct {
	calls     map[string]int
	binding   *Binding
	exports   map[string]callable
	sentinels map[string]error
	sigs      map[string]*OpSig
	errors    ErrorTypes
	fault     *Fault
	faultOp   string
	hasOp     bool
}

func newSubject(binding *Binding, exports map[string]callable, sentinels map[string]error,
	sigs map[string]*OpSig, errors ErrorTypes, fault *Fault, faultOp string) *Subject {
	s := &Subject{
		calls: map[string]int{}, binding: binding, exports: exports, sentinels: sentinels,
		sigs: sigs, errors: errors, fault: fault, faultOp: faultOp, hasOp: faultOp != "",
	}
	for name := range binding.Ops {
		s.calls[name] = 0
	}
	return s
}

// totalCalls is the number of calls made so far to any op in ops.
func (s *Subject) totalCalls(ops []string) int {
	sum := 0
	for _, name := range ops {
		sum += s.calls[name]
	}
	return sum
}

func (s *Subject) callsTo(op string) int { return s.calls[op] }

func (s *Subject) faulted(op string, sig *OpSig) *Fault {
	if s.fault == nil || (s.hasOp && op != s.faultOp) {
		return nil
	}
	if s.fault.Applies(sig) {
		return s.fault
	}
	return nil
}

// call invokes op on canonical args; it returns the exactly-typed, canonical
// result, or panics with a subjectFailure (a verdict) or a *LaneError (a refusal).
func (s *Subject) call(op string, args []any, width int, hasWidth bool) any {
	sig := s.sigs[op]
	binding := s.binding.Ops[op]
	s.calls[op]++
	if hasWidth {
		for _, value := range args {
			if data, isBytes := value.([]byte); isBytes {
				need(len(data) == width, "width", "a Bytes<N> argument does not have length N")
			}
		}
	}
	// Wherever the signature itself states a length (`Bytes<4>`), assert it.
	for i, param := range sig.Params {
		if param.Ty.Kind == kindBytes && param.Ty.HasWidth && param.Ty.WidthName == "" {
			data, isBytes := args[i].([]byte)
			need(isBytes && len(data) == param.Ty.Width, "width", "an argument does not have its declared length")
		}
	}
	given := make([]any, len(args))
	for i, value := range args {
		given[i] = lower(value, sig.Params[i].Ty)
	}
	fault := s.faulted(op, sig)
	passed := given
	if fault != nil {
		passed = fault.OnInput(append([]any(nil), given...), sig)
	}
	if binding.Width == widthRuntime {
		need(hasWidth, "width", "a runtime-width op was called without a static width")
		passed = append(append([]any(nil), passed...), width)
	}
	outcome, failure := s.invoke(op, binding, sig, passed)
	if failure != "" {
		panic(subjectFailure{failure})
	}
	if sig.Fresh {
		requireFresh(outcome, passed)
	}
	final := outcome
	if fault != nil {
		final = fault.OnOutput(outcome, given, sig)
	}
	if !checkValue(final, sig.Ret, s.errors) {
		panic(subjectFailure{TypeMismatch})
	}
	return lift(final, sig.Ret)
}

// invoke runs the adapter. Exactly a mapped sentinel becomes Err(variant);
// anything else (another error, a panic) is a verdict, never a lane error.
func (s *Subject) invoke(op string, binding *OpBinding, sig *OpSig, passed []any) (outcome any, failure string) {
	defer func() {
		if r := recover(); r != nil {
			if lane, isLane := r.(*LaneError); isLane {
				panic(lane)
			}
			outcome, failure = nil, UnexpectedError
		}
	}()
	value, err := s.exports[op](passed)
	if err != nil {
		for _, row := range binding.Errors {
			if err == s.sentinels[row.Sentinel] {
				return Pair{"err", row.Variant}, ""
			}
		}
		return nil, UnexpectedError
	}
	// A library signals success by returning the value itself; the model's
	// Result wraps it. Wrapping here (never trusting a returned pair) is what
	// makes a stray Pair{"err", ...} from the library fail the type check
	// instead of passing as an error.
	if sig.Ret.Kind == kindResult {
		return Pair{"ok", value}, ""
	}
	return value, ""
}

func flip(data []byte) {
	for i := range data {
		data[i] ^= 0xff
	}
}

// requireFresh is rule 4, checked by mutation: a `fresh` result may not share
// memory with any argument it was given.
//
// The runner owns every argument (each is its own copy), so it may scribble on
// them. Flipping the result must leave every argument as the call left it, and
// flipping every argument must leave the result as the call returned it; either
// change means the memory is shared. Both are restored afterwards, because a
// fault's output stage still reads the arguments and the caller still compares
// the result.
func requireFresh(outcome any, passed []any) {
	value := outcome
	if pair, isPair := outcome.(Pair); isPair && pair.Tag == "ok" {
		value = pair.Body
	}
	result, isBytes := value.([]byte)
	if !isBytes {
		return
	}
	type buffer struct{ live, after []byte }
	var buffers []buffer
	for _, arg := range passed {
		if data, isData := arg.([]byte); isData {
			buffers = append(buffers, buffer{data, append([]byte(nil), data...)})
		}
	}
	keep := append([]byte(nil), result...)
	flip(result)
	shared := false
	for _, b := range buffers {
		shared = shared || !bytesEqual(b.live, b.after)
	}
	copy(result, keep)
	for _, b := range buffers {
		flip(b.live)
	}
	shared = shared || !bytesEqual(result, keep)
	for _, b := range buffers {
		copy(b.live, b.after)
	}
	if shared {
		panic(subjectFailure{TypeMismatch})
	}
}
