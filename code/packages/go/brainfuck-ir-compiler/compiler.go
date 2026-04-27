package brainfuckircompiler

import (
	"fmt"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	sm "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-source-map"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// ──────────────────────────────────────────────────────────────────────────────
// BrainfuckIrCompiler — translates a Brainfuck AST into IR
//
// The compiler walks the AST produced by the Brainfuck parser and emits
// IR instructions for each node. It also builds the first two segments
// of the source map chain:
//
//   Segment 1: SourceToAst  (source positions → AST node IDs)
//   Segment 2: AstToIr      (AST node IDs → IR instruction IDs)
//
// ──────────────────────────────────────────────────────────────────────────────
// Register allocation
//
// Brainfuck needs very few registers:
//
//   v0 = tape base address (pointer to the start of the tape)
//   v1 = tape pointer offset (current cell index, 0-based)
//   v2 = temporary (cell value for loads/stores)
//   v3 = temporary (for bounds checks)
//   v4 = temporary (for syscall arguments)
//   v5 = max pointer value (tape_size - 1, for bounds checks)
//   v6 = zero constant (for bounds checks)
//
// This fixed allocation maps directly to a small set of physical
// registers in any ISA. Future languages (BASIC) that need more
// registers will use a register allocator in the backend.
//
// ──────────────────────────────────────────────────────────────────────────────

// Register constants — virtual register indices used by the compiler.
const (
	regTapeBase = 0 // v0: base address of the tape
	regTapePtr  = 1 // v1: current cell offset (0-based index)
	regTemp     = 2 // v2: temporary for cell values
	regTemp2    = 3 // v3: temporary for bounds checks
	regSysArg   = 4 // v4: syscall argument
	regMaxPtr   = 5 // v5: tape_size - 1 (for bounds checks)
	regZero     = 6 // v6: constant 0 (for bounds checks)
)

// Syscall numbers — these match the RISC-V simulator's ecall dispatch.
const (
	syscallWrite = 1  // write byte in a0 to stdout
	syscallRead  = 2  // read byte from stdin into a0
	syscallExit  = 10 // halt with exit code in a0
)

// CompileResult holds the outputs of compilation.
type CompileResult struct {
	Program   *ir.IrProgram
	SourceMap *sm.SourceMapChain
}

// Compile takes a Brainfuck AST (from ParseBrainfuck) and a source filename,
// and produces an IrProgram plus source map segments.
//
// The filename is used in source map entries to identify which file the
// source positions refer to.
func Compile(ast *parser.ASTNode, filename string, config BuildConfig) (*CompileResult, error) {
	c := &compiler{
		config:    config,
		filename:  filename,
		idGen:     ir.NewIDGenerator(),
		nodeIDGen: 0,
		program:   ir.NewIrProgram("_start"),
		sourceMap: sm.NewSourceMapChain(),
		loopCount: 0,
	}

	// Validate inputs
	if ast.RuleName != "program" {
		return nil, fmt.Errorf("expected 'program' AST node, got %q", ast.RuleName)
	}
	if config.TapeSize <= 0 {
		return nil, fmt.Errorf("invalid TapeSize %d: must be positive", config.TapeSize)
	}

	// Add tape data declaration
	c.program.AddData(ir.IrDataDecl{
		Label: "tape",
		Size:  config.TapeSize,
		Init:  0,
	})

	// Emit prologue
	c.emitPrologue()

	// Compile the program body
	if err := c.compileProgram(ast); err != nil {
		return nil, err
	}

	// Emit epilogue
	c.emitEpilogue()

	return &CompileResult{
		Program:   c.program,
		SourceMap: c.sourceMap,
	}, nil
}

// ──────────────────────────────────────────────────────────────────────────────
// Internal compiler state
// ──────────────────────────────────────────────────────────────────────────────

type compiler struct {
	config    BuildConfig
	filename  string
	idGen     *ir.IDGenerator
	nodeIDGen int // monotonic AST node ID counter
	program   *ir.IrProgram
	sourceMap *sm.SourceMapChain
	loopCount int // for generating unique loop labels
}

// nextNodeID returns the next unique AST node ID.
func (c *compiler) nextNodeID() int {
	id := c.nodeIDGen
	c.nodeIDGen++
	return id
}

// emit adds an instruction to the program and returns its ID.
func (c *compiler) emit(opcode ir.IrOp, operands ...ir.IrOperand) int {
	id := c.idGen.Next()
	c.program.AddInstruction(ir.IrInstruction{
		Opcode:   opcode,
		Operands: operands,
		ID:       id,
	})
	return id
}

// emitLabel adds a label instruction (labels have ID -1 as they produce
// no machine code).
func (c *compiler) emitLabel(name string) {
	c.program.AddInstruction(ir.IrInstruction{
		Opcode:   ir.OpLabel,
		Operands: []ir.IrOperand{ir.IrLabel{Name: name}},
		ID:       -1,
	})
}

// ──────────────────────────────────────────────────────────────────────────────
// Prologue and Epilogue
//
// The prologue sets up the execution environment:
//   - Load the tape base address into v0
//   - Set the tape pointer to 0 in v1
//   - (debug) Set max pointer and zero constant
//
// The epilogue terminates the program cleanly.
// ──────────────────────────────────────────────────────────────────────────────

func (c *compiler) emitPrologue() {
	c.emitLabel("_start")

	// v0 = &tape (base address of the tape)
	c.emit(ir.OpLoadAddr, ir.IrRegister{Index: regTapeBase}, ir.IrLabel{Name: "tape"})

	// v1 = 0 (tape pointer starts at cell 0)
	c.emit(ir.OpLoadImm, ir.IrRegister{Index: regTapePtr}, ir.IrImmediate{Value: 0})

	// Debug mode: set up bounds check registers
	if c.config.InsertBoundsChecks {
		// v5 = tape_size - 1 (max valid pointer)
		c.emit(ir.OpLoadImm,
			ir.IrRegister{Index: regMaxPtr},
			ir.IrImmediate{Value: c.config.TapeSize - 1})

		// v6 = 0 (for lower bound check)
		c.emit(ir.OpLoadImm,
			ir.IrRegister{Index: regZero},
			ir.IrImmediate{Value: 0})
	}
}

func (c *compiler) emitEpilogue() {
	c.emit(ir.OpHalt)

	// Emit out-of-bounds trap handler if bounds checking is enabled
	if c.config.InsertBoundsChecks {
		c.emitLabel("__trap_oob")
		// Load error exit code into syscall arg register
		c.emit(ir.OpLoadImm,
			ir.IrRegister{Index: regSysArg},
			ir.IrImmediate{Value: 1})
		// Exit with error code 1
		c.emit(ir.OpSyscall, ir.IrImmediate{Value: syscallExit})
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// AST Walking
//
// The AST has this structure (from the parser grammar):
//
//   program     → { instruction }
//   instruction → loop | command
//   loop        → LOOP_START { instruction } LOOP_END
//   command     → RIGHT | LEFT | INC | DEC | OUTPUT | INPUT
//
// The compiler walks this tree recursively, emitting IR for each node.
// ──────────────────────────────────────────────────────────────────────────────

func (c *compiler) compileProgram(node *parser.ASTNode) error {
	for _, child := range node.Children {
		childNode, ok := child.(*parser.ASTNode)
		if !ok {
			continue // skip tokens at the program level
		}
		if err := c.compileNode(childNode); err != nil {
			return err
		}
	}
	return nil
}

func (c *compiler) compileNode(node *parser.ASTNode) error {
	switch node.RuleName {
	case "instruction":
		// An instruction wraps either a loop or a command
		for _, child := range node.Children {
			childNode, ok := child.(*parser.ASTNode)
			if !ok {
				continue
			}
			if err := c.compileNode(childNode); err != nil {
				return err
			}
		}
		return nil

	case "command":
		return c.compileCommand(node)

	case "loop":
		return c.compileLoop(node)

	default:
		return fmt.Errorf("unexpected AST node type: %q", node.RuleName)
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// Command compilation
//
// Each Brainfuck command maps to a specific sequence of IR instructions.
// The mapping is documented in the spec (BF03) and in the compilation
// mapping table below.
//
// ┌────────────────┬──────────────────────────────────────────────────────┐
// │ Command        │ IR Output                                            │
// ├────────────────┼──────────────────────────────────────────────────────┤
// │ > (RIGHT)      │ ADD_IMM v1, v1, 1                                   │
// │ < (LEFT)       │ ADD_IMM v1, v1, -1                                  │
// │ + (INC)        │ LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, 1;           │
// │                │ AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1         │
// │ - (DEC)        │ LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, -1;          │
// │                │ AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1         │
// │ . (OUTPUT)     │ LOAD_BYTE v2, v0, v1; SYSCALL 1                    │
// │ , (INPUT)      │ SYSCALL 2; STORE_BYTE v4, v0, v1                   │
// └────────────────┴──────────────────────────────────────────────────────┘
// ──────────────────────────────────────────────────────────────────────────────

func (c *compiler) compileCommand(node *parser.ASTNode) error {
	// Extract the token from the command node
	tok := c.extractToken(node)
	if tok == nil {
		return fmt.Errorf("command node has no token")
	}

	// Create source map entries
	astNodeID := c.nextNodeID()
	c.sourceMap.SourceToAst.Add(sm.SourcePosition{
		File:   c.filename,
		Line:   tok.Line,
		Column: tok.Column,
		Length: 1,
	}, astNodeID)

	// Track IR instruction IDs for source mapping
	var irIDs []int

	switch tok.Value {
	case ">": // RIGHT: move tape pointer right
		if c.config.InsertBoundsChecks {
			irIDs = append(irIDs, c.emitBoundsCheckRight()...)
		}
		id := c.emit(ir.OpAddImm,
			ir.IrRegister{Index: regTapePtr},
			ir.IrRegister{Index: regTapePtr},
			ir.IrImmediate{Value: 1})
		irIDs = append(irIDs, id)

	case "<": // LEFT: move tape pointer left
		if c.config.InsertBoundsChecks {
			irIDs = append(irIDs, c.emitBoundsCheckLeft()...)
		}
		id := c.emit(ir.OpAddImm,
			ir.IrRegister{Index: regTapePtr},
			ir.IrRegister{Index: regTapePtr},
			ir.IrImmediate{Value: -1})
		irIDs = append(irIDs, id)

	case "+": // INC: increment current cell
		irIDs = append(irIDs, c.emitCellMutation(1)...)

	case "-": // DEC: decrement current cell
		irIDs = append(irIDs, c.emitCellMutation(-1)...)

	case ".": // OUTPUT: write current cell to stdout
		// Load current cell value
		id1 := c.emit(ir.OpLoadByte,
			ir.IrRegister{Index: regTemp},
			ir.IrRegister{Index: regTapeBase},
			ir.IrRegister{Index: regTapePtr})
		irIDs = append(irIDs, id1)
		// Copy to the syscall argument register without depending on v6.
		id2 := c.emit(ir.OpAddImm,
			ir.IrRegister{Index: regSysArg},
			ir.IrRegister{Index: regTemp},
			ir.IrImmediate{Value: 0})
		irIDs = append(irIDs, id2)
		// Syscall 1 = write byte
		id3 := c.emit(ir.OpSyscall, ir.IrImmediate{Value: syscallWrite})
		irIDs = append(irIDs, id3)

	case ",": // INPUT: read byte from stdin into current cell
		// Syscall 2 = read byte
		id1 := c.emit(ir.OpSyscall, ir.IrImmediate{Value: syscallRead})
		irIDs = append(irIDs, id1)
		// Store result (in syscall arg register) to current cell
		id2 := c.emit(ir.OpStoreByte,
			ir.IrRegister{Index: regSysArg},
			ir.IrRegister{Index: regTapeBase},
			ir.IrRegister{Index: regTapePtr})
		irIDs = append(irIDs, id2)

	default:
		return fmt.Errorf("unknown command token: %q", tok.Value)
	}

	// Record AST → IR mapping
	c.sourceMap.AstToIr.Add(astNodeID, irIDs)
	return nil
}

// emitCellMutation emits the IR for incrementing or decrementing the
// current cell by the given delta (+1 or -1).
//
// The sequence is:
//
//	LOAD_BYTE  v2, v0, v1        ← load current cell
//	ADD_IMM    v2, v2, delta      ← increment/decrement
//	AND_IMM    v2, v2, 255        ← mask to byte (if enabled)
//	STORE_BYTE v2, v0, v1        ← store back
func (c *compiler) emitCellMutation(delta int) []int {
	var ids []int

	// Load current cell value
	id := c.emit(ir.OpLoadByte,
		ir.IrRegister{Index: regTemp},
		ir.IrRegister{Index: regTapeBase},
		ir.IrRegister{Index: regTapePtr})
	ids = append(ids, id)

	// Add delta
	id = c.emit(ir.OpAddImm,
		ir.IrRegister{Index: regTemp},
		ir.IrRegister{Index: regTemp},
		ir.IrImmediate{Value: delta})
	ids = append(ids, id)

	// Mask to byte range (0-255) if enabled
	if c.config.MaskByteArithmetic {
		id = c.emit(ir.OpAndImm,
			ir.IrRegister{Index: regTemp},
			ir.IrRegister{Index: regTemp},
			ir.IrImmediate{Value: 255})
		ids = append(ids, id)
	}

	// Store back to cell
	id = c.emit(ir.OpStoreByte,
		ir.IrRegister{Index: regTemp},
		ir.IrRegister{Index: regTapeBase},
		ir.IrRegister{Index: regTapePtr})
	ids = append(ids, id)

	return ids
}

// ──────────────────────────────────────────────────────────────────────────────
// Bounds checking
//
// In debug builds, the compiler inserts range checks before every
// pointer move. If the pointer goes out of bounds, the program jumps
// to the __trap_oob label (which calls exit(1)).
//
// RIGHT (>):
//   ADD_IMM v3, v1, 1         ← predicted ptr after increment
//   CMP_GT  v3, v3, v5        ← is predicted ptr > max?
//   BRANCH_NZ v3, __trap_oob  ← if so, trap
//
// LEFT (<):
//   ADD_IMM v3, v1, -1        ← predicted ptr after decrement
//   CMP_LT  v3, v3, v6        ← is predicted ptr < 0?
//   BRANCH_NZ v3, __trap_oob  ← if so, trap
// ──────────────────────────────────────────────────────────────────────────────

func (c *compiler) emitBoundsCheckRight() []int {
	var ids []int
	id := c.emit(ir.OpAddImm,
		ir.IrRegister{Index: regTemp2},
		ir.IrRegister{Index: regTapePtr},
		ir.IrImmediate{Value: 1})
	ids = append(ids, id)
	id = c.emit(ir.OpCmpGt,
		ir.IrRegister{Index: regTemp2},
		ir.IrRegister{Index: regTemp2},
		ir.IrRegister{Index: regMaxPtr})
	ids = append(ids, id)
	id = c.emit(ir.OpBranchNz,
		ir.IrRegister{Index: regTemp2},
		ir.IrLabel{Name: "__trap_oob"})
	ids = append(ids, id)
	return ids
}

func (c *compiler) emitBoundsCheckLeft() []int {
	var ids []int
	id := c.emit(ir.OpAddImm,
		ir.IrRegister{Index: regTemp2},
		ir.IrRegister{Index: regTapePtr},
		ir.IrImmediate{Value: -1})
	ids = append(ids, id)
	id = c.emit(ir.OpCmpLt,
		ir.IrRegister{Index: regTemp2},
		ir.IrRegister{Index: regTemp2},
		ir.IrRegister{Index: regZero})
	ids = append(ids, id)
	id = c.emit(ir.OpBranchNz,
		ir.IrRegister{Index: regTemp2},
		ir.IrLabel{Name: "__trap_oob"})
	ids = append(ids, id)
	return ids
}

// ──────────────────────────────────────────────────────────────────────────────
// Loop compilation
//
// A Brainfuck loop [body] compiles to:
//
//   LABEL      loop_N_start
//   LOAD_BYTE  v2, v0, v1          ← load current cell
//   BRANCH_Z   v2, loop_N_end      ← skip body if cell == 0
//   ...compile body...
//   JUMP       loop_N_start        ← repeat
//   LABEL      loop_N_end
//
// Loops nest arbitrarily deep. Each loop gets a unique number N
// (from c.loopCount) to make labels unique.
// ──────────────────────────────────────────────────────────────────────────────

func (c *compiler) compileLoop(node *parser.ASTNode) error {
	loopNum := c.loopCount
	c.loopCount++
	startLabel := fmt.Sprintf("loop_%d_start", loopNum)
	endLabel := fmt.Sprintf("loop_%d_end", loopNum)

	// Find the LOOP_START token for source mapping
	astNodeID := c.nextNodeID()
	if node.StartLine > 0 {
		c.sourceMap.SourceToAst.Add(sm.SourcePosition{
			File:   c.filename,
			Line:   node.StartLine,
			Column: node.StartColumn,
			Length: 1,
		}, astNodeID)
	}

	var irIDs []int

	// Emit loop start label
	c.emitLabel(startLabel)

	// Load current cell and branch if zero
	id := c.emit(ir.OpLoadByte,
		ir.IrRegister{Index: regTemp},
		ir.IrRegister{Index: regTapeBase},
		ir.IrRegister{Index: regTapePtr})
	irIDs = append(irIDs, id)

	id = c.emit(ir.OpBranchZ,
		ir.IrRegister{Index: regTemp},
		ir.IrLabel{Name: endLabel})
	irIDs = append(irIDs, id)

	// Compile loop body (instruction children, skipping LOOP_START and LOOP_END tokens)
	for _, child := range node.Children {
		childNode, ok := child.(*parser.ASTNode)
		if !ok {
			continue // skip bracket tokens
		}
		if err := c.compileNode(childNode); err != nil {
			return err
		}
	}

	// Jump back to loop start
	id = c.emit(ir.OpJump, ir.IrLabel{Name: startLabel})
	irIDs = append(irIDs, id)

	// Emit loop end label
	c.emitLabel(endLabel)

	// Record AST → IR mapping for the loop construct
	c.sourceMap.AstToIr.Add(astNodeID, irIDs)

	return nil
}

// ──────────────────────────────────────────────────────────────────────────────
// Token extraction
//
// The AST structure is:
//   command → Token (leaf node wrapping a single token)
//
// The extractToken helper digs through the AST to find the leaf token.
// ──────────────────────────────────────────────────────────────────────────────

func (c *compiler) extractToken(node *parser.ASTNode) *lexer.Token {
	// If the node itself is a leaf, return its token
	if node.IsLeaf() {
		return node.Token()
	}

	// Otherwise, look through children for a leaf node or a raw token
	for _, child := range node.Children {
		switch v := child.(type) {
		case lexer.Token:
			return &v
		case *parser.ASTNode:
			tok := c.extractToken(v)
			if tok != nil {
				return tok
			}
		}
	}
	return nil
}
