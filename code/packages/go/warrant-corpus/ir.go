package warrantcorpus

import (
	"bytes"
	"math"
	"math/bits"
)

// # The model IR: compile a language-neutral expression tree to a closure
//
// Every claim body in the corpus is a tiny expression tree, for example:
//
//	["eq", ["var", "a"], ["var", "b"]]                       a == b
//	["ok", ["if", ["var","choice"], ["var","a"], ["var","b"]]]
//	["subject_op", "eq", [["var","a"], ["var","b"]]]         call the real export
//
// There are exactly two kinds of call, and keeping them apart is what keeps the
// oracle outside the library (VER04i 2.5):
//
//   - subject_op dispatches through the binding to the REAL export. It is allowed
//     only in a claim's subject side, never in a model.
//   - model_call invokes a named, pure model (`same`, `same_u64`).
//
// # No eval, no plugin loading
//
// The tree is interpreted by compilation to closures: each node becomes a small
// func over a slice of variable slots, built before any case runs. Nothing here
// turns corpus text into code, so a hostile corpus can only choose among the node
// kinds this file implements. An unknown or malformed node is a corpus error
// immediately, never a skipped case.
//
// # Bounds
//
//   - Nesting deeper than maxIRDepth is refused.
//   - maxIRCost is a STATIC upper bound on node evaluations per case, counting
//     every model call at the full cost of its body. A chain of models that each
//     call the next twice costs 2^n; it is refused at compile time, before any
//     case runs, instead of burning CPU for hours.
//   - Arithmetic (add sub mul) is checked, not wrapped: this profile has no U64
//     arithmetic in any model, and a silent wrap would hide a corpus that needed
//     one. A result outside 0 .. 2^64-1 is a corpus error. (The TypeScript lane
//     also range-checks operands first because a bigint can grow without limit;
//     a uint64 cannot, so only the result needs the check.)
//
// Values are canonical (see values.go): bool, uint64, []byte and Pair.
const (
	maxIRDepth = 64
	// maxIRCost is the static bound on node evaluations per expression and case.
	maxIRCost = 2_000
)

type compiled func(env []any) any

// subjectCall is `(op name, evaluated arguments, static width) -> canonical result`.
type subjectCall func(op string, args []any, width int, hasWidth bool) any

func isNode(value any) ([]any, bool) {
	node, ok := value.([]any)
	if !ok || len(node) == 0 {
		return nil, false
	}
	_, isString := node[0].(string)
	return node, isString
}

func asBool(value any) bool {
	b, ok := value.(bool)
	need(ok, "ir-type", "a boolean was expected")
	return b
}

func asUint(value any) uint64 {
	n, ok := value.(uint64)
	need(ok, "ir-overflow", "arithmetic operand is outside U64")
	return n
}

func ordering(left, right any) int {
	if l, ok := left.(uint64); ok {
		if r, ok := right.(uint64); ok {
			switch {
			case l < r:
				return -1
			case l > r:
				return 1
			}
			return 0
		}
	}
	if l, ok := left.([]byte); ok {
		if r, ok := right.([]byte); ok {
			return bytes.Compare(l, r)
		}
	}
	fail("ir-type", "ordering needs two integers or two byte strings")
	return 0
}

var binaryOps = map[string]func(a, b any) any{
	"eq": func(a, b any) any { return valuesEqual(a, b) },
	"ne": func(a, b any) any { return !valuesEqual(a, b) },
	"lt": func(a, b any) any { return ordering(a, b) < 0 },
	"le": func(a, b any) any { return ordering(a, b) <= 0 },
	"gt": func(a, b any) any { return ordering(a, b) > 0 },
	"ge": func(a, b any) any { return ordering(a, b) >= 0 },
	// Both operands are evaluated before the connective runs (as in the other
	// lanes), but the type check short-circuits: a non-boolean right side is
	// only a type error when the left side does not already decide the answer.
	"and":     func(a, b any) any { return asBool(a) && asBool(b) },
	"or":      func(a, b any) any { return asBool(a) || asBool(b) },
	"implies": func(a, b any) any { return !asBool(a) || asBool(b) },
}

var arithmeticOps = map[string]func(a, b uint64) uint64{
	"add": func(a, b uint64) uint64 {
		sum, carry := bits.Add64(a, b, 0)
		need(carry == 0, "ir-overflow", "model arithmetic left the U64 range")
		return sum
	},
	"sub": func(a, b uint64) uint64 {
		need(b <= a, "ir-overflow", "model arithmetic left the U64 range")
		return a - b
	},
	"mul": func(a, b uint64) uint64 {
		high, low := bits.Mul64(a, b)
		need(high == 0, "ir-overflow", "model arithmetic left the U64 range")
		return low
	},
}

type model struct {
	params []string
	body   compiled
}

// compiler compiles IR nodes against a model table and an optional subject hook.
type compiler struct {
	models         map[string]any
	subject        subjectCall
	ops            map[string]*OpSig
	compiledModels map[string]*model
	inProgress     map[string]bool
	modelCosts     map[string]int
}

func newCompiler(models map[string]any, subject subjectCall, ops map[string]*OpSig) *compiler {
	return &compiler{
		models:         models,
		subject:        subject,
		ops:            ops,
		compiledModels: map[string]*model{},
		inProgress:     map[string]bool{},
		modelCosts:     map[string]int{},
	}
}

// model compiles (once) and returns the named pure model.
func (c *compiler) model(name string) *model {
	if known, ok := c.compiledModels[name]; ok {
		return known
	}
	raw, ok := c.models[name]
	need(ok, "unsupported-ir", "unknown model call")
	need(!c.inProgress[name], "unsupported-ir", "recursive model")
	fields, ok := raw.(map[string]any)
	_, hasParams := fields["params"]
	_, hasBody := fields["body"]
	need(ok && len(fields) == 2 && hasParams && hasBody, "unsupported-ir", "malformed model")
	rawParams, ok := fields["params"].([]any)
	need(ok, "unsupported-ir", "malformed model parameters")
	params := make([]string, len(rawParams))
	for i, item := range rawParams {
		text, isString := item.(string)
		need(isString, "unsupported-ir", "malformed model parameters")
		params[i] = text
	}
	c.inProgress[name] = true
	// A model body may only call other pure models: no subject hook. It shares
	// the caches, so a model is compiled and costed once.
	body := &compiler{
		models:         c.models,
		subject:        nil,
		ops:            c.ops,
		compiledModels: c.compiledModels,
		inProgress:     c.inProgress,
		modelCosts:     c.modelCosts,
	}
	entry := &model{params: params, body: body.compile(fields["body"], params)}
	delete(c.inProgress, name)
	c.compiledModels[name] = entry
	return entry
}

// compile compiles node; scope lists the variable names, in slot order. A
// top-level call also enforces maxIRCost.
func (c *compiler) compile(node any, scope []string) compiled {
	result := c.compileNode(node, scope, 0)
	c.cost(node.([]any))
	return result
}

// cost is the static upper bound on node evaluations (see maxIRCost).
func (c *compiler) cost(node []any) int {
	tag := node[0].(string)
	var children []any
	switch {
	case tag == "len" || tag == "ok" || tag == "not":
		children = node[1:2]
	case tag == "if":
		children = node[1:4]
	case binaryOps[tag] != nil || arithmeticOps[tag] != nil:
		children = node[1:3]
	case tag == "model_call" || tag == "subject_op":
		children = node[2].([]any)
	}
	total := 1
	for _, child := range children {
		total += c.cost(child.([]any))
	}
	if tag == "model_call" {
		total += c.modelCost(node[1].(string))
	}
	need(total <= maxIRCost, "ir-limit", "model IR cost exceeds the work limit")
	return total
}

func (c *compiler) modelCost(name string) int {
	known, ok := c.modelCosts[name]
	if !ok {
		body := c.models[name].(map[string]any)["body"].([]any)
		known = c.cost(body)
		c.modelCosts[name] = known
	}
	return known
}

func (c *compiler) compileNode(raw any, scope []string, depth int) compiled {
	need(depth <= maxIRDepth, "ir-limit", "model IR nests too deeply")
	node, ok := isNode(raw)
	need(ok, "unsupported-ir", "malformed or unknown model IR node")
	tag := node[0].(string)
	sub := func(child any) compiled { return c.compileNode(child, scope, depth+1) }

	switch tag {
	case "lit":
		need(len(node) == 2, "unsupported-ir", "malformed literal")
		value := c.literal(node[1])
		return func([]any) any { return value }
	case "var":
		slot := -1
		if len(node) == 2 {
			if name, isString := node[1].(string); isString {
				for i, candidate := range scope {
					if candidate == name {
						slot = i
						break
					}
				}
			}
		}
		need(len(node) == 2 && slot >= 0, "unsupported-ir", "unknown variable")
		return func(env []any) any { return env[slot] }
	case "len":
		need(len(node) == 2, "unsupported-ir", "malformed len")
		inner := sub(node[1])
		return func(env []any) any {
			data, isBytes := inner(env).([]byte)
			need(isBytes, "ir-type", "len needs a byte string")
			return uint64(len(data))
		}
	case "ok":
		need(len(node) == 2, "unsupported-ir", "malformed ok")
		inner := sub(node[1])
		return func(env []any) any { return Pair{"ok", inner(env)} }
	case "err", "name":
		variant, isString := "", false
		if len(node) == 2 {
			variant, isString = node[1].(string)
		}
		need(isString, "unsupported-ir", "malformed err/name")
		return func([]any) any { return Pair{tag, variant} }
	case "not":
		need(len(node) == 2, "unsupported-ir", "malformed not")
		inner := sub(node[1])
		return func(env []any) any { return !asBool(inner(env)) }
	case "if":
		need(len(node) == 4, "unsupported-ir", "malformed if")
		condition, then, otherwise := sub(node[1]), sub(node[2]), sub(node[3])
		return func(env []any) any {
			if asBool(condition(env)) {
				return then(env)
			}
			return otherwise(env)
		}
	case "model_call":
		return c.modelCall(node, sub)
	case "subject_op":
		return c.subjectOp(node, sub)
	}
	if apply, isBinary := binaryOps[tag]; isBinary {
		need(len(node) == 3, "unsupported-ir", "malformed binary node")
		left, right := sub(node[1]), sub(node[2])
		return func(env []any) any { return apply(left(env), right(env)) }
	}
	if apply, isArithmetic := arithmeticOps[tag]; isArithmetic {
		need(len(node) == 3, "unsupported-ir", "malformed arithmetic node")
		left, right := sub(node[1]), sub(node[2])
		return func(env []any) any { return apply(asUint(left(env)), asUint(right(env))) }
	}
	fail("unsupported-ir", "unknown model IR tag")
	return nil
}

// literal admits a boolean or a non-negative integer. (The corpus has no
// negative literal; a uint64 cannot hold one, so it is refused rather than
// wrapped.)
func (c *compiler) literal(value any) any {
	switch v := value.(type) {
	case bool:
		return v
	case int64:
		need(v >= 0, "unsupported-ir", "unsupported literal")
		return uint64(v)
	}
	fail("unsupported-ir", "unsupported literal")
	return nil
}

func (c *compiler) modelCall(node []any, sub func(any) compiled) compiled {
	name, isString := "", false
	var rawArgs []any
	var isList bool
	if len(node) == 3 {
		name, isString = node[1].(string)
		rawArgs, isList = node[2].([]any)
	}
	need(isString && isList, "unsupported-ir", "malformed model_call")
	entry := c.model(name)
	args := make([]compiled, len(rawArgs))
	for i, argument := range rawArgs {
		args[i] = sub(argument)
	}
	need(len(args) == len(entry.params), "unsupported-ir", "model arity mismatch")
	return func(env []any) any {
		values := make([]any, len(args))
		for i, argument := range args {
			values[i] = argument(env)
		}
		return entry.body(values)
	}
}

func (c *compiler) subjectOp(node []any, sub func(any) compiled) compiled {
	call := c.subject
	need(call != nil, "unsupported-ir", "a subject call is forbidden in a model or admission expression")
	name, isString := "", false
	var rawArgs []any
	var isList bool
	if len(node) == 3 || len(node) == 4 {
		name, isString = node[1].(string)
		rawArgs, isList = node[2].([]any)
	}
	need(isString && isList, "unsupported-ir", "malformed subject_op")
	signature, known := c.ops[name]
	need(known, "unsupported-ir", "subject call to an unknown op")
	width, hasWidth := 0, false
	if len(node) == 4 {
		widths, isWidthList := node[3].([]any)
		need(isWidthList && len(widths) == 1, "unsupported-ir", "malformed static width")
		n, isInt := widths[0].(int64)
		need(isInt && n >= 0 && n <= maxSafeInt && n <= math.MaxInt, "unsupported-ir", "malformed static width")
		if n < 0 || n > math.MaxInt {
			return nil
		}
		width, hasWidth = int(n), true
	}
	need((len(signature.WidthParams) > 0) == hasWidth, "unsupported-ir",
		"a fixed-width op needs its width, and any other op must not state one")
	args := make([]compiled, len(rawArgs))
	for i, argument := range rawArgs {
		args[i] = sub(argument)
	}
	return func(env []any) any {
		values := make([]any, len(args))
		for i, argument := range args {
			values[i] = argument(env)
		}
		return call(name, values, width, hasWidth)
	}
}
