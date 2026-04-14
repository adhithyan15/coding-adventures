package typecheckerprotocol

import "strings"

type TypeErrorDiagnostic struct {
	Message string
	Line    int
	Column  int
}

type TypeCheckResult[T any] struct {
	TypedAST T
	Errors   []TypeErrorDiagnostic
	OK       bool
}

type TypeChecker[ASTIn any, ASTOut any] interface {
	Check(ast ASTIn) TypeCheckResult[ASTOut]
}

type HookResult any
type Hook[T any] func(node T, args ...any) HookResult
type Locator func(subject any) (int, int)
type NodeKind[T any] func(node T) string

type notHandledToken struct{}

type GenericTypeChecker[T any] struct {
	hooks    map[string][]Hook[T]
	errors   []TypeErrorDiagnostic
	nodeKind NodeKind[T]
	locate   Locator
}

func NewGenericTypeChecker[T any](nodeKind NodeKind[T], locate Locator) *GenericTypeChecker[T] {
	if locate == nil {
		locate = func(subject any) (int, int) {
			_ = subject
			return 1, 1
		}
	}

	return &GenericTypeChecker[T]{
		hooks:    map[string][]Hook[T]{},
		nodeKind: nodeKind,
		locate:   locate,
	}
}

func (g *GenericTypeChecker[T]) Reset() {
	g.errors = nil
}

func (g *GenericTypeChecker[T]) RegisterHook(phase string, kind string, hook Hook[T]) {
	key := phase + ":" + normalizeKind(kind)
	g.hooks[key] = append(g.hooks[key], hook)
}

func (g *GenericTypeChecker[T]) Dispatch(phase string, node T, args ...any) HookResult {
	kind := ""
	if g.nodeKind != nil {
		kind = normalizeKind(g.nodeKind(node))
	}

	keys := []string{phase + ":" + kind, phase + ":*"}
	for _, key := range keys {
		for _, hook := range g.hooks[key] {
			result := hook(node, args...)
			if _, ok := result.(notHandledToken); !ok {
				return result
			}
		}
	}

	return nil
}

func (g *GenericTypeChecker[T]) NotHandled() HookResult {
	return notHandledToken{}
}

func (g *GenericTypeChecker[T]) Error(message string, subject any) {
	line, column := g.locate(subject)
	g.errors = append(g.errors, TypeErrorDiagnostic{
		Message: message,
		Line:    line,
		Column:  column,
	})
}

func (g *GenericTypeChecker[T]) Errors() []TypeErrorDiagnostic {
	out := make([]TypeErrorDiagnostic, len(g.errors))
	copy(out, g.errors)
	return out
}

func normalizeKind(kind string) string {
	var builder strings.Builder
	lastUnderscore := false

	for _, char := range kind {
		if (char >= 'a' && char <= 'z') || (char >= 'A' && char <= 'Z') || (char >= '0' && char <= '9') {
			builder.WriteRune(char)
			lastUnderscore = false
			continue
		}

		if !lastUnderscore {
			builder.WriteByte('_')
			lastUnderscore = true
		}
	}

	return strings.Trim(builder.String(), "_")
}
