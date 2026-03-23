package latticeasttocss

// scope.go — lexical scope chain for Lattice variables, mixins, and functions.
//
// # Why Lexical Scoping?
//
// CSS has no concept of scope — everything is global. Lattice adds variables,
// mixins, and functions, which need scoping rules so inner blocks don't
// accidentally clobber outer names.
//
// Lattice uses lexical (static) scoping: a variable's scope is determined by
// where it appears in the source, not by the runtime call order. This is the
// same model used by JavaScript, Python, Go, and most modern languages.
//
// # How It Works
//
// Each { } block in the Lattice source creates a new child scope. Looking up
// a variable walks up the parent chain until the name is found:
//
//	$color: red;              ← global scope (depth 0)
//	.parent {                 ← child scope (depth 1)
//	    $color: blue;         ← shadows the global $color
//	    color: $color;        → blue (found at depth 1)
//	    .child {              ← grandchild scope (depth 2)
//	        color: $color;    → blue (inherited from depth 1)
//	    }
//	}
//	.sibling {                ← another child at depth 1
//	    color: $color;        → red (global, unaffected by .parent)
//	}
//
// This is a linked list of scope nodes. Each node has a `parent` pointer and
// a `bindings` map. Looking up a name walks the chain upward.
//
// # Special Cases
//
// Mixin expansion: creates a child scope of the caller's scope. Mixins see
// the caller's variables (like JavaScript closures).
//
// Function evaluation: creates an isolated scope whose parent is the global
// scope only, NOT the caller's scope. This prevents functions from depending
// on where they are called from — they only see their parameters and globals.

// ScopeChain is a single node in the lexical scope chain.
//
// Each scope has:
//   - bindings: maps names to values (AST nodes, tokens, or LatticeValues)
//   - parent:   the enclosing scope, nil for the global scope
//
// The zero value is not useful; always use NewScopeChain().
type ScopeChain struct {
	bindings map[string]interface{}
	parent   *ScopeChain
}

// NewScopeChain creates a new scope with the given parent (nil for global scope).
//
// Example:
//
//	global := NewScopeChain(nil)
//	block := NewScopeChain(global)  // block can see global's bindings
func NewScopeChain(parent *ScopeChain) *ScopeChain {
	return &ScopeChain{
		bindings: make(map[string]interface{}),
		parent:   parent,
	}
}

// Get looks up a name in this scope or any ancestor scope.
//
// Walks up the parent chain until the name is found. Returns (nil, false)
// if the name is not found anywhere in the chain.
//
// This is the core of lexical scoping: a variable declared in an outer scope
// is visible in all inner scopes unless shadowed by a local declaration.
//
// Example:
//
//	global.Set("$color", "red")
//	block.Get("$color")  // returns ("red", true) — inherited from global
func (s *ScopeChain) Get(name string) (interface{}, bool) {
	if val, ok := s.bindings[name]; ok {
		return val, true
	}
	if s.parent != nil {
		return s.parent.Get(name)
	}
	return nil, false
}

// Set binds a name to a value in this scope (never in a parent scope).
//
// This means a child scope can shadow a parent's binding without modifying
// the parent. The parent remains unchanged.
//
// Example:
//
//	global.Set("$color", "red")
//	block.Set("$color", "blue")     // block's local binding
//	global.Get("$color")            // still "red" — unaffected
func (s *ScopeChain) Set(name string, value interface{}) {
	s.bindings[name] = value
}

// Has reports whether a name exists in this scope or any ancestor.
//
// Like Get, walks up the parent chain. Returns true if the name is bound
// anywhere, false otherwise.
func (s *ScopeChain) Has(name string) bool {
	_, ok := s.Get(name)
	return ok
}

// HasLocal reports whether a name is bound in this scope only (not ancestors).
//
// Useful for detecting re-declarations and shadowing diagnostics.
func (s *ScopeChain) HasLocal(name string) bool {
	_, ok := s.bindings[name]
	return ok
}

// SetGlobal binds a name to a value in the root (global) scope.
//
// Walks up the parent chain to find the root scope (the one with no parent),
// then sets the binding there. This implements the !global flag in Lattice
// variable declarations.
//
// When !global is used inside a deeply nested scope (e.g., inside a mixin
// inside a @for loop), the variable is set at the top level, making it
// visible everywhere:
//
//	$theme: light;
//	@mixin set-dark {
//	    $theme: dark !global;
//	    // Sets $theme in the root scope, not the mixin scope
//	}
func (s *ScopeChain) SetGlobal(name string, value interface{}) {
	root := s
	for root.parent != nil {
		root = root.parent
	}
	root.bindings[name] = value
}

// Child creates a new child scope with this scope as parent.
//
// The child inherits all bindings from the parent chain via Get, but any
// Set calls on the child only affect the child — the parent is unchanged.
//
// This is how blocks create new scopes in Lattice:
//
//	block := scope.Child()   // entering a new { } block
func (s *ScopeChain) Child() *ScopeChain {
	return NewScopeChain(s)
}

// Depth returns how many levels deep this scope is.
// The global scope has depth 0. Each Child() call adds 1.
//
// Useful for debugging and for the transformer to know whether we are
// inside a block (depth > 0) or at the top level (depth == 0).
func (s *ScopeChain) Depth() int {
	if s.parent == nil {
		return 0
	}
	return 1 + s.parent.Depth()
}
