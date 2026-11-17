package parser

// ===========================================================================
// THE COMPACT API: Tree and ParseInto
// ===========================================================================
//
// Parse hands back a tree of *ASTNode. That is convenient, and it is also
// where most of a fast parse's allocations go: one ASTNode per node, one
// children slice, one boxed 64-byte lexer.Token per leaf. A caller that only
// wants to LOOK at the tree (walk it once, pull out a few fields, throw it
// away) pays for all of that and then lets the garbage collector pay again.
//
// The engine never builds *ASTNodes while it parses (see the flat-tree
// comment in compiled.go): it builds two integer arrays. ParseInto stops
// there and gives the caller those arrays, wrapped in a Tree:
//
//	var t parser.Tree                  // reuse it for every parse
//	for _, line := range lines {
//	    if err := c.ParseInto(&t, line, parser.Options{}); err != nil {
//	        continue                   // t is empty
//	    }
//	    walk(&t, t.Root())
//	}
//
// In steady state, with one Tree reused, a parse allocates nothing: the
// arrays are the Tree's own and are recycled, and the memo table and scratch
// stack come from the Compiled's pool.
//
// WHAT A TREE LOOKS LIKE
//
//	nodes[i] = {rule, kids[start:end]}     one entry per rule result
//	kids[j]  >= 0   a token: an index into the token list
//	         <  0   a node reference: ^kid = node<<1 | memo
//
// A NodeID is that node reference (node<<1 | memo), so it is opaque but
// comparable. memo is 0 for a node the engine parsed and 1 for a MEMO HIT: a
// place where the packrat memo table replayed an earlier result.
//
// MEMO HITS, AND WHY THEY SHOW UP HERE
//
// Parse has always returned, for a replayed result, a distinct *ASTNode that
// SHARES the first parse's Children slice and has ZERO positions (the
// positions are computed only when a rule actually parses). Code downstream
// can depend on that, so the compact tree exposes exactly the same structure
// rather than quietly "fixing" it: the child slot of a replay holds a NodeID
// with IsReplay true, whose RuleID, NumChildren and Child are those of the
// node it replays, and whose Position is zero. Span, which is not part of
// the legacy ASTNode contract, reports the real token range for it (the
// range of the replayed node), because a token range is a fact about the
// tokens and has no reason to be unknown. ToAST turns each replay back into
// its own position-less *ASTNode sharing the replayed run, one per slot, so
// ToAST() is indistinguishable from Parse's tree.
//
// OWNERSHIP AND SAFETY
//
//   - A Tree owns its nodes and kids arrays outright. ParseInto swaps buffers
//     with the (pooled) parse state rather than copying out of it, so a Tree
//     never aliases storage another parse can overwrite.
//   - A Tree keeps a reference to the token slice it was parsed from (after
//     any PreParse hooks), because Token and ToAST read from it. The caller
//     must not modify those tokens while it uses the Tree. The Tree does not
//     copy them.
//   - A Tree is NOT safe for concurrent use, including concurrent reads, since
//     Span and Position build a cache on first use. Distinct Trees on one
//     Compiled are independent and may be used from different goroutines.
//   - A Tree is either complete or empty. Any error, limit refusal or
//     recovered panic leaves it empty (Root reports NoNode, ToAST nil);
//     a partial result is never visible.
//   - A reused Tree keeps at most maxPooledFlat entries of each buffer between
//     parses (the same cap that bounds the pooled parse state), so one huge
//     parse does not pin its memory for the life of the process: the next
//     ParseInto, or Reset, drops an oversized buffer. Until then the Tree of
//     course holds its (large) result.

import (
	"errors"
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// NodeID identifies a node of a Tree. It is opaque; compare it only for
// equality and with NoNode. A replay of a node (see IsReplay) has a different
// NodeID from the node it replays.
type NodeID int32

// NoNode is the NodeID of "no node": the Root of an empty Tree.
const NoNode NodeID = -1

// Child is one entry of a node's children: a token or a node.
type Child struct{ v int32 }

// IsToken reports whether the child is a token.
func (ch Child) IsToken() bool { return ch.v >= 0 }

// Token returns the index of the child token (valid for Tree.Token), or -1 if
// the child is a node.
func (ch Child) Token() int {
	if ch.v >= 0 {
		return int(ch.v)
	}
	return -1
}

// Node returns the child's NodeID, or NoNode if the child is a token.
func (ch Child) Node() NodeID {
	if ch.v >= 0 {
		return NoNode
	}
	return NodeID(^ch.v)
}

// Tree is a parse result in compact form. The zero Tree is empty and ready
// for ParseInto. See the comment at the top of this file for ownership and
// concurrency.
type Tree struct {
	c      *Compiled
	ok     bool
	root   int32 // reference (node<<1), valid when ok
	tokens []lexer.Token
	eof    lexer.Token
	nodes  []cnode
	kids   []int32

	// Lazily built by Span and Position: the first and last leaf token
	// index under each node, or -1 if it has none.
	first, last []int32
	spans       bool
}

var errNilTree = errors.New("parser: ParseInto needs a non-nil *Tree")

// errPostParseUnsupported: PostParse hooks transform *ASTNode trees, which
// ParseInto never builds. Silently skipping them would hand back a tree the
// caller's hooks never saw, so it is refused.
var errPostParseUnsupported = errors.New("parser: ParseInto does not support Options.PostParse (hooks transform *ASTNode); call ToAST and apply them yourself")

// Reset empties the tree and drops any buffer larger than maxPooledFlat
// entries. ParseInto does this itself; Reset is for a caller that wants to
// release a Tree's memory or its token reference earlier.
func (t *Tree) Reset() {
	t.ok, t.root = false, -1
	t.c, t.tokens = nil, nil
	t.eof = lexer.Token{}
	t.nodes, t.kids = t.nodes[:0], t.kids[:0]
	if cap(t.nodes) > maxPooledFlat {
		t.nodes = nil
	}
	if cap(t.kids) > maxPooledFlat {
		t.kids = nil
	}
	t.first, t.last, t.spans = t.first[:0], t.last[:0], false
	if cap(t.first) > maxPooledFlat {
		t.first = nil
	}
	if cap(t.last) > maxPooledFlat {
		t.last = nil
	}
}

// ParseInto parses tokens like Parse, but leaves the result in t as a compact
// Tree instead of building *ASTNodes. It runs the same engine, so success,
// failure, the error value and every limit (MaxDepth, MaxTokens,
// MaxMemoSlots, HardMaxFrames) are exactly Parse's; t.ToAST() is the tree
// Parse returns.
//
// Any previous contents of t are discarded first. On error t is empty. PreParse
// hooks run; PostParse hooks are refused with an error because they transform
// *ASTNodes (apply them to t.ToAST() if you need them).
func (c *Compiled) ParseInto(t *Tree, tokens []lexer.Token, opts Options) (err error) {
	if t == nil {
		return errNilTree
	}
	t.Reset()
	defer func() {
		if r := recover(); r != nil {
			t.Reset()
			err = fmt.Errorf("parser: internal error: %v", r)
		}
	}()
	if len(opts.PostParse) > 0 {
		return errPostParseUnsupported
	}

	tokens, start, err := c.prepare(tokens, opts)
	if err != nil {
		return err
	}

	// Take the Tree's buffers for the parse and give them back after, so a
	// reused Tree recycles its own arrays. While the parse runs they are the
	// state's alone (t holds none), so a panic cannot leave t aliasing a
	// state that is dropped.
	ownKids, ownNodes := t.kids, t.nodes
	t.kids, t.nodes = nil, nil

	s := c.pool.Get().(*parseState)
	s.reset(tokens, opts)
	poolKids, poolNodes := s.kids, s.nodes
	s.kids, s.nodes = ownKids[:0], ownNodes[:0]

	root, err := s.runFlat(start)

	t.kids, t.nodes = s.kids, s.nodes
	t.eof = s.eof
	s.kids, s.nodes = poolKids, poolNodes
	s.release()
	if err != nil {
		t.Reset()
		return err
	}
	t.c, t.tokens, t.root, t.ok = c, tokens, root, true
	return nil
}

// RuleID returns the id of the named rule, as used by Tree.RuleID, and
// whether the rule exists. With duplicate definitions the last one wins, as
// in Parse.
func (c *Compiled) RuleID(name string) (int32, bool) {
	id, ok := c.byName[name]
	return id, ok
}

// RuleName returns the name of rule id, or "" if id is out of range.
func (c *Compiled) RuleName(id int32) string {
	if id < 0 || int(id) >= len(c.rules) {
		return ""
	}
	return c.rules[id].name
}

// Root returns the root node, or NoNode if the tree is empty.
func (t *Tree) Root() NodeID {
	if !t.ok {
		return NoNode
	}
	return NodeID(t.root)
}

// NumNodes is the number of node slots the engine produced, including nodes
// of failed attempts that are unreachable from Root. It is a size, not a
// count of the tree you can walk.
func (t *Tree) NumNodes() int { return len(t.nodes) }

// NumTokens is the number of real tokens in the list the tree was parsed
// from; Token also accepts the index NumTokens, the synthetic EOF.
func (t *Tree) NumTokens() int { return len(t.tokens) }

func (t *Tree) node(n NodeID) *cnode { return &t.nodes[n>>1] }

// IsReplay reports whether n is a memo hit: a distinct node sharing the
// children of the node it replays and having no Position. See the file
// comment.
func (t *Tree) IsReplay(n NodeID) bool { return n&1 != 0 }

// RuleID is n's rule, an index for Compiled.RuleName.
func (t *Tree) RuleID(n NodeID) int32 { return t.node(n).rule }

// Rule is n's rule name, the same string as ASTNode.RuleName.
func (t *Tree) Rule(n NodeID) string { return t.c.rules[t.node(n).rule].name }

// NumChildren is the length of n's Children.
func (t *Tree) NumChildren(n NodeID) int {
	cn := t.node(n)
	return int(cn.end - cn.start)
}

// Child returns the i'th child of n (0 <= i < NumChildren(n)).
func (t *Tree) Child(n NodeID, i int) Child {
	cn := t.node(n)
	if i < 0 || i >= int(cn.end-cn.start) {
		panic("parser: Tree.Child index out of range")
	}
	return Child{t.kids[int(cn.start)+i]}
}

// Token returns token index i (0 <= i <= NumTokens). Index NumTokens is the
// synthetic EOF the parser sees past the end of the list, which a rule can
// match and which then appears as a child. The pointer is into the caller's
// token slice (or the Tree's own EOF), and must be treated as read-only.
func (t *Tree) Token(i int) *lexer.Token {
	if i < len(t.tokens) {
		return &t.tokens[i]
	}
	if i == len(t.tokens) {
		return &t.eof
	}
	panic("parser: Tree.Token index out of range")
}

// buildSpans computes first and last for every node in one pass. Nodes are
// stored children-first (a child's index is below its parent's), so each
// node needs only a scan of its own run.
func (t *Tree) buildSpans() {
	n := len(t.nodes)
	if cap(t.first) < n {
		t.first = make([]int32, n)
		t.last = make([]int32, n)
	}
	t.first, t.last = t.first[:n], t.last[:n]
	for i := range t.nodes {
		run := t.kids[t.nodes[i].start:t.nodes[i].end]
		f, l := int32(-1), int32(-1)
		for _, k := range run {
			if k >= 0 {
				f = k
			} else if x := t.first[^k>>1]; x >= 0 {
				f = x
			} else {
				continue
			}
			break
		}
		for j := len(run) - 1; j >= 0; j-- {
			k := run[j]
			if k >= 0 {
				l = k
			} else if x := t.last[^k>>1]; x >= 0 {
				l = x
			} else {
				continue
			}
			break
		}
		t.first[i], t.last[i] = f, l
	}
	t.spans = true
}

// Span returns the token indexes of the first and last leaf token under n
// (inclusive), or (-1, -1) if n contains no token. For a replay it is the
// span of the node replayed.
func (t *Tree) Span(n NodeID) (first, last int) {
	if !t.spans {
		t.buildSpans()
	}
	return int(t.first[n>>1]), int(t.last[n>>1])
}

// Position returns what ASTNode's StartLine, StartColumn, EndLine and
// EndColumn are for n in Parse's tree: the line and column of its first and
// last leaf tokens, and all zero if it has none. A replay (IsReplay) is
// always all zero, as in Parse; use Span for its real range.
func (t *Tree) Position(n NodeID) (startLine, startColumn, endLine, endColumn int) {
	if n&1 != 0 {
		return 0, 0, 0, 0
	}
	f, l := t.Span(n)
	if f < 0 {
		return 0, 0, 0, 0
	}
	a, b := t.Token(f), t.Token(l)
	return a.Line, a.Column, b.Line, b.Column
}

// ToAST builds the *ASTNode tree Parse would have returned for the same
// input: same structure, positions, nil-ness of Children, and the same
// sharing (a replay is its own node sharing the replayed Children). It
// returns nil for an empty tree. The result does not alias the Tree's
// buffers (it copies the tokens it boxes), though it is built from the
// token slice the Tree references, so call it before changing those tokens.
func (t *Tree) ToAST() *ASTNode {
	if !t.ok {
		return nil
	}
	sc, _ := t.c.scratch.Get().(*astScratch)
	if sc == nil {
		sc = new(astScratch)
	}
	ast := buildAST(t.c, t.tokens, t.eof, t.nodes, t.kids, sc, t.root)
	sc.release()
	t.c.scratch.Put(sc)
	return ast
}
