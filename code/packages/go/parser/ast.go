package parser

// Node acts as the top-level parent bounding structural constraints inherently across expressions and blocks.
type Node interface {
	isNode()
}

// Statement captures imperative code evaluations terminating line instructions implicitly or explicitly.
type Statement interface {
	Node
	isStatement()
}

// Expression captures values actively resolving directly upon stack pops iteratively building computations.
type Expression interface {
	Node
	isExpression()
}

type NumberLiteral struct {
	Value int
}

func (NumberLiteral) isNode()       {}
func (NumberLiteral) isExpression() {}

type StringLiteral struct {
	Value string
}

func (StringLiteral) isNode()       {}
func (StringLiteral) isExpression() {}

type Name struct {
	Name string
}

func (Name) isNode()       {}
func (Name) isExpression() {}

type BinaryOp struct {
	Left  Expression
	Op    string
	Right Expression
}

func (BinaryOp) isNode()       {}
func (BinaryOp) isExpression() {}

type Assignment struct {
	Target Name
	Value  Expression
}

func (Assignment) isNode()      {}
func (Assignment) isStatement() {}

type ExpressionStmt struct {
	Expression Expression
}

func (ExpressionStmt) isNode()      {}
func (ExpressionStmt) isStatement() {}

type Program struct {
	Statements []Statement
}

func (Program) isNode() {}
