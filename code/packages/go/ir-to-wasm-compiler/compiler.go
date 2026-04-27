package irtowasmcompiler

import (
	"fmt"
	"regexp"
	"sort"
	"strings"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	wasmleb128 "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-leb128"
	wasmopcodes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-opcodes"
	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
)

var (
	loopStartRe       = regexp.MustCompile(`^loop_\d+_start$`)
	ifElseRe          = regexp.MustCompile(`^if_\d+_else$`)
	functionCommentRe = regexp.MustCompile(`^function:\s*([A-Za-z_][A-Za-z0-9_]*)\((.*)\)$`)
	opcode            = struct {
		nop       byte
		block     byte
		loop      byte
		if_       byte
		else_     byte
		end       byte
		br        byte
		brIf      byte
		return_   byte
		call      byte
		localGet  byte
		localSet  byte
		i32Load   byte
		i32Load8U byte
		i32Store  byte
		i32Store8 byte
		i32Const  byte
		i32Eqz    byte
		i32Eq     byte
		i32Ne     byte
		i32LtS    byte
		i32GtS    byte
		i32Add    byte
		i32Sub    byte
		i32And    byte
	}{
		nop:       mustOpcode("nop"),
		block:     mustOpcode("block"),
		loop:      mustOpcode("loop"),
		if_:       mustOpcode("if"),
		else_:     mustOpcode("else"),
		end:       mustOpcode("end"),
		br:        mustOpcode("br"),
		brIf:      mustOpcode("br_if"),
		return_:   mustOpcode("return"),
		call:      mustOpcode("call"),
		localGet:  mustOpcode("local.get"),
		localSet:  mustOpcode("local.set"),
		i32Load:   mustOpcode("i32.load"),
		i32Load8U: mustOpcode("i32.load8_u"),
		i32Store:  mustOpcode("i32.store"),
		i32Store8: mustOpcode("i32.store8"),
		i32Const:  mustOpcode("i32.const"),
		i32Eqz:    mustOpcode("i32.eqz"),
		i32Eq:     mustOpcode("i32.eq"),
		i32Ne:     mustOpcode("i32.ne"),
		i32LtS:    mustOpcode("i32.lt_s"),
		i32GtS:    mustOpcode("i32.gt_s"),
		i32Add:    mustOpcode("i32.add"),
		i32Sub:    mustOpcode("i32.sub"),
		i32And:    mustOpcode("i32.and"),
	}
)

const (
	syscallWrite = 1
	syscallRead  = 2
	syscallExit  = 10
	syscallArg0  = 4

	wasiModule      = "wasi_snapshot_preview1"
	wasiIovecOffset = 0
	wasiCountOffset = 8
	wasiByteOffset  = 12
	wasiScratchSize = 16

	regScratch = 1
	regVarBase = 2
)

type WasmLoweringError struct {
	Message string
}

func (e *WasmLoweringError) Error() string {
	return e.Message
}

type FunctionSignature struct {
	Label      string
	ParamCount int
	ExportName string
}

type functionIR struct {
	Label        string
	Instructions []ir.IrInstruction
	Signature    FunctionSignature
	MaxReg       int
}

type wasiImport struct {
	SyscallNumber int
	Name          string
	FuncType      wasmtypes.FuncType
	TypeKey       string
}

type wasiContext struct {
	FunctionIndices map[int]uint32
	ScratchBase     *int
}

type IrToWasmCompiler struct{}

func NewIrToWasmCompiler() *IrToWasmCompiler {
	return &IrToWasmCompiler{}
}

func (c *IrToWasmCompiler) Compile(program *ir.IrProgram, functionSignatures ...FunctionSignature) (*wasmtypes.WasmModule, error) {
	signatures := InferFunctionSignaturesFromComments(program)
	for _, signature := range functionSignatures {
		signatures[signature.Label] = signature
	}

	functions, err := c.splitFunctions(program, signatures)
	if err != nil {
		return nil, err
	}
	imports, err := c.collectWasiImports(program)
	if err != nil {
		return nil, err
	}
	typeIndices, types := c.buildTypeTable(functions, imports)
	dataOffsets := c.layoutData(program.Data)

	var scratchBase *int
	if c.needsWasiScratch(program) {
		offset := alignUp(totalDataSize(program.Data), 4)
		scratchBase = &offset
	}

	module := &wasmtypes.WasmModule{}
	module.Types = append(module.Types, types...)
	for _, entry := range imports {
		module.Imports = append(module.Imports, wasmtypes.Import{
			ModuleName: wasiModule,
			Name:       entry.Name,
			Kind:       wasmtypes.ExternalKindFunction,
			TypeInfo:   typeIndices[entry.TypeKey],
		})
	}

	functionIndexBase := uint32(len(imports))
	functionIndices := make(map[string]uint32, len(functions))
	for index, fn := range functions {
		functionIndices[fn.Label] = functionIndexBase + uint32(index)
		module.Functions = append(module.Functions, typeIndices[fn.Label])
	}

	totalBytes := totalDataSize(program.Data)
	if scratchBase != nil {
		totalBytes = max(totalBytes, *scratchBase+wasiScratchSize)
	}

	if c.needsMemory(program) || scratchBase != nil {
		pageCount := 1
		if totalBytes > 0 {
			pageCount = max(1, (totalBytes+65535)/65536)
		}
		module.Memories = append(module.Memories, wasmtypes.MemoryType{
			Limits: wasmtypes.Limits{Min: uint32(pageCount)},
		})
		module.Exports = append(module.Exports, wasmtypes.Export{
			Name:  "memory",
			Kind:  wasmtypes.ExternalKindMemory,
			Index: 0,
		})
		for _, decl := range program.Data {
			module.Data = append(module.Data, wasmtypes.DataSegment{
				MemoryIndex: 0,
				OffsetExpr:  constExpr(dataOffsets[decl.Label]),
				Data:        bytesOfSize(decl.Size, byte(decl.Init&0xFF)),
			})
		}
	}

	wasiCtx := wasiContext{
		FunctionIndices: make(map[int]uint32, len(imports)),
		ScratchBase:     scratchBase,
	}
	for index, entry := range imports {
		wasiCtx.FunctionIndices[entry.SyscallNumber] = uint32(index)
	}

	for _, fn := range functions {
		body, err := newFunctionLowerer(functionLowererOptions{
			Fn:              fn,
			Signatures:      signatures,
			FunctionIndices: functionIndices,
			DataOffsets:     dataOffsets,
			WasiContext:     wasiCtx,
		}).Lower()
		if err != nil {
			return nil, err
		}
		module.Code = append(module.Code, *body)
		if fn.Signature.ExportName != "" {
			module.Exports = append(module.Exports, wasmtypes.Export{
				Name:  fn.Signature.ExportName,
				Kind:  wasmtypes.ExternalKindFunction,
				Index: functionIndices[fn.Label],
			})
		}
	}

	return module, nil
}

func (c *IrToWasmCompiler) buildTypeTable(functions []functionIR, imports []wasiImport) (map[string]uint32, []wasmtypes.FuncType) {
	seen := make(map[string]uint32)
	typeIndices := make(map[string]uint32)
	types := make([]wasmtypes.FuncType, 0)

	rememberType := func(key string, funcType wasmtypes.FuncType) {
		signatureKey := funcTypeKey(funcType)
		index, ok := seen[signatureKey]
		if !ok {
			index = uint32(len(types))
			types = append(types, funcType)
			seen[signatureKey] = index
		}
		typeIndices[key] = index
	}

	for _, entry := range imports {
		rememberType(entry.TypeKey, entry.FuncType)
	}
	for _, fn := range functions {
		params := make([]wasmtypes.ValueType, fn.Signature.ParamCount)
		for index := range params {
			params[index] = wasmtypes.ValueTypeI32
		}
		rememberType(fn.Label, wasmtypes.FuncType{
			Params:  params,
			Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
		})
	}

	return typeIndices, types
}

func (c *IrToWasmCompiler) layoutData(decls []ir.IrDataDecl) map[string]int {
	offsets := make(map[string]int, len(decls))
	cursor := 0
	for _, decl := range decls {
		offsets[decl.Label] = cursor
		cursor += decl.Size
	}
	return offsets
}

func (c *IrToWasmCompiler) needsMemory(program *ir.IrProgram) bool {
	if len(program.Data) > 0 {
		return true
	}
	for _, instruction := range program.Instructions {
		switch instruction.Opcode {
		case ir.OpLoadAddr, ir.OpLoadByte, ir.OpStoreByte, ir.OpLoadWord, ir.OpStoreWord:
			return true
		}
	}
	return false
}

func (c *IrToWasmCompiler) needsWasiScratch(program *ir.IrProgram) bool {
	for _, instruction := range program.Instructions {
		if instruction.Opcode != ir.OpSyscall || len(instruction.Operands) == 0 {
			continue
		}
		syscall, err := expectImmediate(instruction.Operands[0], "SYSCALL number")
		if err != nil {
			return false
		}
		if syscall.Value == syscallWrite || syscall.Value == syscallRead {
			return true
		}
	}
	return false
}

func (c *IrToWasmCompiler) collectWasiImports(program *ir.IrProgram) ([]wasiImport, error) {
	required := map[int]bool{}
	for _, instruction := range program.Instructions {
		if instruction.Opcode != ir.OpSyscall || len(instruction.Operands) == 0 {
			continue
		}
		syscall, err := expectImmediate(instruction.Operands[0], "SYSCALL number")
		if err != nil {
			return nil, err
		}
		required[syscall.Value] = true
	}

	ordered := []wasiImport{
		{
			SyscallNumber: syscallWrite,
			Name:          "fd_write",
			FuncType: wasmtypes.FuncType{
				Params: []wasmtypes.ValueType{
					wasmtypes.ValueTypeI32,
					wasmtypes.ValueTypeI32,
					wasmtypes.ValueTypeI32,
					wasmtypes.ValueTypeI32,
				},
				Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
			},
			TypeKey: "wasi::fd_write",
		},
		{
			SyscallNumber: syscallRead,
			Name:          "fd_read",
			FuncType: wasmtypes.FuncType{
				Params: []wasmtypes.ValueType{
					wasmtypes.ValueTypeI32,
					wasmtypes.ValueTypeI32,
					wasmtypes.ValueTypeI32,
					wasmtypes.ValueTypeI32,
				},
				Results: []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
			},
			TypeKey: "wasi::fd_read",
		},
		{
			SyscallNumber: syscallExit,
			Name:          "proc_exit",
			FuncType: wasmtypes.FuncType{
				Params:  []wasmtypes.ValueType{wasmtypes.ValueTypeI32},
				Results: nil,
			},
			TypeKey: "wasi::proc_exit",
		},
	}

	supported := map[int]bool{}
	for _, entry := range ordered {
		supported[entry.SyscallNumber] = true
	}
	unsupported := make([]int, 0)
	for syscall := range required {
		if !supported[syscall] {
			unsupported = append(unsupported, syscall)
		}
	}
	sort.Ints(unsupported)
	if len(unsupported) > 0 {
		parts := make([]string, len(unsupported))
		for index, value := range unsupported {
			parts[index] = fmt.Sprintf("%d", value)
		}
		return nil, &WasmLoweringError{Message: fmt.Sprintf("unsupported SYSCALL number(s): %s", strings.Join(parts, ", "))}
	}

	imports := make([]wasiImport, 0)
	for _, entry := range ordered {
		if required[entry.SyscallNumber] {
			imports = append(imports, entry)
		}
	}
	return imports, nil
}

func (c *IrToWasmCompiler) splitFunctions(program *ir.IrProgram, signatures map[string]FunctionSignature) ([]functionIR, error) {
	functions := make([]functionIR, 0)
	startIndex := -1
	startLabel := ""

	for index, instruction := range program.Instructions {
		labelName := functionLabelName(instruction)
		if labelName == "" {
			continue
		}

		if startLabel != "" && startIndex >= 0 {
			fn, err := makeFunctionIR(startLabel, append([]ir.IrInstruction{}, program.Instructions[startIndex:index]...), signatures)
			if err != nil {
				return nil, err
			}
			functions = append(functions, fn)
		}

		startLabel = labelName
		startIndex = index
	}

	if startLabel != "" && startIndex >= 0 {
		fn, err := makeFunctionIR(startLabel, append([]ir.IrInstruction{}, program.Instructions[startIndex:]...), signatures)
		if err != nil {
			return nil, err
		}
		functions = append(functions, fn)
	}

	return functions, nil
}

type functionLowererOptions struct {
	Fn              functionIR
	Signatures      map[string]FunctionSignature
	FunctionIndices map[string]uint32
	DataOffsets     map[string]int
	WasiContext     wasiContext
}

type functionLowerer struct {
	options      functionLowererOptions
	paramCount   int
	bytes        []byte
	instructions []ir.IrInstruction
	labelToIndex map[string]int
}

func newFunctionLowerer(options functionLowererOptions) *functionLowerer {
	labelToIndex := make(map[string]int)
	for index, instruction := range options.Fn.Instructions {
		if instruction.Opcode != ir.OpLabel || len(instruction.Operands) == 0 {
			continue
		}
		if label, ok := instruction.Operands[0].(ir.IrLabel); ok {
			labelToIndex[label.Name] = index
		}
	}

	return &functionLowerer{
		options:      options,
		paramCount:   options.Fn.Signature.ParamCount,
		instructions: options.Fn.Instructions,
		labelToIndex: labelToIndex,
	}
}

func (l *functionLowerer) Lower() (*wasmtypes.FunctionBody, error) {
	l.copyParamsIntoIRRegisters()
	if err := l.emitRegion(1, len(l.instructions)); err != nil {
		return nil, err
	}
	l.emitOpcode(opcode.end)

	locals := make([]wasmtypes.ValueType, l.options.Fn.MaxReg+1)
	for index := range locals {
		locals[index] = wasmtypes.ValueTypeI32
	}

	return &wasmtypes.FunctionBody{
		Locals: locals,
		Code:   append([]byte{}, l.bytes...),
	}, nil
}

func (l *functionLowerer) copyParamsIntoIRRegisters() {
	for paramIndex := 0; paramIndex < l.paramCount; paramIndex++ {
		l.emitOpcode(opcode.localGet)
		l.emitU32(uint32(paramIndex))
		l.emitOpcode(opcode.localSet)
		l.emitU32(uint32(l.localIndex(regVarBase + paramIndex)))
	}
}

func (l *functionLowerer) emitRegion(start, end int) error {
	index := start
	for index < end {
		instruction := l.instructions[index]

		if instruction.Opcode == ir.OpComment {
			index++
			continue
		}

		labelName := labelNameFromInstruction(instruction)
		if labelName != "" && loopStartRe.MatchString(labelName) {
			next, err := l.emitLoop(index)
			if err != nil {
				return err
			}
			index = next
			continue
		}

		if (instruction.Opcode == ir.OpBranchZ || instruction.Opcode == ir.OpBranchNz) &&
			len(instruction.Operands) == 2 &&
			isLabelOperand(instruction.Operands[1]) &&
			ifElseRe.MatchString(labelNameFromOperand(instruction.Operands[1])) {
			next, err := l.emitIf(index)
			if err != nil {
				return err
			}
			index = next
			continue
		}

		if instruction.Opcode == ir.OpLabel {
			index++
			continue
		}

		if instruction.Opcode == ir.OpJump || instruction.Opcode == ir.OpBranchZ || instruction.Opcode == ir.OpBranchNz {
			return &WasmLoweringError{Message: fmt.Sprintf("unexpected unstructured control flow in %s", l.options.Fn.Label)}
		}

		if err := l.emitSimple(instruction); err != nil {
			return err
		}
		index++
	}
	return nil
}

func (l *functionLowerer) emitIf(branchIndex int) (int, error) {
	branch := l.instructions[branchIndex]
	condReg, err := expectRegister(branch.Operands[0], "if condition")
	if err != nil {
		return 0, err
	}
	elseLabel, err := expectLabel(branch.Operands[1], "if else label")
	if err != nil {
		return 0, err
	}

	endLabel := elseLabel.Name + "_end"
	if strings.HasSuffix(elseLabel.Name, "_else") {
		endLabel = strings.TrimSuffix(elseLabel.Name, "_else") + "_end"
	}

	elseIndex, err := l.requireLabelIndex(elseLabel.Name)
	if err != nil {
		return 0, err
	}
	endIndex, err := l.requireLabelIndex(endLabel)
	if err != nil {
		return 0, err
	}
	jumpIndex, err := l.findLastJumpToLabel(branchIndex+1, elseIndex, endLabel)
	if err != nil {
		return 0, err
	}

	l.emitLocalGet(condReg.Index)
	if branch.Opcode == ir.OpBranchNz {
		l.emitOpcode(opcode.i32Eqz)
	}
	l.emitOpcode(opcode.if_)
	l.emitByte(byte(wasmtypes.BlockTypeEmpty))

	if err := l.emitRegion(branchIndex+1, jumpIndex); err != nil {
		return 0, err
	}
	if elseIndex+1 < endIndex {
		l.emitOpcode(opcode.else_)
		if err := l.emitRegion(elseIndex+1, endIndex); err != nil {
			return 0, err
		}
	}

	l.emitOpcode(opcode.end)
	return endIndex + 1, nil
}

func (l *functionLowerer) emitLoop(labelIndex int) (int, error) {
	startLabel := labelNameFromInstruction(l.instructions[labelIndex])
	if startLabel == "" {
		return 0, &WasmLoweringError{Message: "loop lowering expected a start label"}
	}
	endLabel := startLabel + "_end"
	if strings.HasSuffix(startLabel, "_start") {
		endLabel = strings.TrimSuffix(startLabel, "_start") + "_end"
	}

	endIndex, err := l.requireLabelIndex(endLabel)
	if err != nil {
		return 0, err
	}
	branchIndex, err := l.findFirstBranchToLabel(labelIndex+1, endIndex, endLabel)
	if err != nil {
		return 0, err
	}
	backedgeIndex, err := l.findLastJumpToLabel(branchIndex+1, endIndex, startLabel)
	if err != nil {
		return 0, err
	}

	branch := l.instructions[branchIndex]
	condReg, err := expectRegister(branch.Operands[0], "loop condition")
	if err != nil {
		return 0, err
	}

	l.emitOpcode(opcode.block)
	l.emitByte(byte(wasmtypes.BlockTypeEmpty))
	l.emitOpcode(opcode.loop)
	l.emitByte(byte(wasmtypes.BlockTypeEmpty))

	if err := l.emitRegion(labelIndex+1, branchIndex); err != nil {
		return 0, err
	}
	l.emitLocalGet(condReg.Index)
	if branch.Opcode == ir.OpBranchZ {
		l.emitOpcode(opcode.i32Eqz)
	}
	l.emitOpcode(opcode.brIf)
	l.emitU32(1)

	if err := l.emitRegion(branchIndex+1, backedgeIndex); err != nil {
		return 0, err
	}
	l.emitOpcode(opcode.br)
	l.emitU32(0)
	l.emitOpcode(opcode.end)
	l.emitOpcode(opcode.end)
	return endIndex + 1, nil
}

func (l *functionLowerer) emitSimple(instruction ir.IrInstruction) error {
	switch instruction.Opcode {
	case ir.OpLoadImm:
		dst, err := expectRegister(instruction.Operands[0], "LOAD_IMM dst")
		if err != nil {
			return err
		}
		value, err := expectImmediate(instruction.Operands[1], "LOAD_IMM imm")
		if err != nil {
			return err
		}
		l.emitI32Const(value.Value)
		l.emitLocalSet(dst.Index)
		return nil
	case ir.OpLoadAddr:
		dst, err := expectRegister(instruction.Operands[0], "LOAD_ADDR dst")
		if err != nil {
			return err
		}
		label, err := expectLabel(instruction.Operands[1], "LOAD_ADDR label")
		if err != nil {
			return err
		}
		offset, ok := l.options.DataOffsets[label.Name]
		if !ok {
			return &WasmLoweringError{Message: fmt.Sprintf("unknown data label: %s", label.Name)}
		}
		l.emitI32Const(offset)
		l.emitLocalSet(dst.Index)
		return nil
	case ir.OpLoadByte:
		dst, err := expectRegister(instruction.Operands[0], "LOAD_BYTE dst")
		if err != nil {
			return err
		}
		base, err := expectRegister(instruction.Operands[1], "LOAD_BYTE base")
		if err != nil {
			return err
		}
		offset, err := expectRegister(instruction.Operands[2], "LOAD_BYTE offset")
		if err != nil {
			return err
		}
		l.emitAddress(base.Index, offset.Index)
		l.emitOpcode(opcode.i32Load8U)
		l.emitMemarg(0, 0)
		l.emitLocalSet(dst.Index)
		return nil
	case ir.OpStoreByte:
		src, err := expectRegister(instruction.Operands[0], "STORE_BYTE src")
		if err != nil {
			return err
		}
		base, err := expectRegister(instruction.Operands[1], "STORE_BYTE base")
		if err != nil {
			return err
		}
		offset, err := expectRegister(instruction.Operands[2], "STORE_BYTE offset")
		if err != nil {
			return err
		}
		l.emitAddress(base.Index, offset.Index)
		l.emitLocalGet(src.Index)
		l.emitOpcode(opcode.i32Store8)
		l.emitMemarg(0, 0)
		return nil
	case ir.OpLoadWord:
		dst, err := expectRegister(instruction.Operands[0], "LOAD_WORD dst")
		if err != nil {
			return err
		}
		base, err := expectRegister(instruction.Operands[1], "LOAD_WORD base")
		if err != nil {
			return err
		}
		offset, err := expectRegister(instruction.Operands[2], "LOAD_WORD offset")
		if err != nil {
			return err
		}
		l.emitAddress(base.Index, offset.Index)
		l.emitOpcode(opcode.i32Load)
		l.emitMemarg(2, 0)
		l.emitLocalSet(dst.Index)
		return nil
	case ir.OpStoreWord:
		src, err := expectRegister(instruction.Operands[0], "STORE_WORD src")
		if err != nil {
			return err
		}
		base, err := expectRegister(instruction.Operands[1], "STORE_WORD base")
		if err != nil {
			return err
		}
		offset, err := expectRegister(instruction.Operands[2], "STORE_WORD offset")
		if err != nil {
			return err
		}
		l.emitAddress(base.Index, offset.Index)
		l.emitLocalGet(src.Index)
		l.emitOpcode(opcode.i32Store)
		l.emitMemarg(2, 0)
		return nil
	case ir.OpAdd:
		return l.emitBinaryNumeric(opcode.i32Add, instruction)
	case ir.OpAddImm:
		dst, err := expectRegister(instruction.Operands[0], "ADD_IMM dst")
		if err != nil {
			return err
		}
		src, err := expectRegister(instruction.Operands[1], "ADD_IMM src")
		if err != nil {
			return err
		}
		value, err := expectImmediate(instruction.Operands[2], "ADD_IMM imm")
		if err != nil {
			return err
		}
		l.emitLocalGet(src.Index)
		l.emitI32Const(value.Value)
		l.emitOpcode(opcode.i32Add)
		l.emitLocalSet(dst.Index)
		return nil
	case ir.OpSub:
		return l.emitBinaryNumeric(opcode.i32Sub, instruction)
	case ir.OpAnd:
		return l.emitBinaryNumeric(opcode.i32And, instruction)
	case ir.OpAndImm:
		dst, err := expectRegister(instruction.Operands[0], "AND_IMM dst")
		if err != nil {
			return err
		}
		src, err := expectRegister(instruction.Operands[1], "AND_IMM src")
		if err != nil {
			return err
		}
		value, err := expectImmediate(instruction.Operands[2], "AND_IMM imm")
		if err != nil {
			return err
		}
		l.emitLocalGet(src.Index)
		l.emitI32Const(value.Value)
		l.emitOpcode(opcode.i32And)
		l.emitLocalSet(dst.Index)
		return nil
	case ir.OpCmpEq:
		return l.emitBinaryNumeric(opcode.i32Eq, instruction)
	case ir.OpCmpNe:
		return l.emitBinaryNumeric(opcode.i32Ne, instruction)
	case ir.OpCmpLt:
		return l.emitBinaryNumeric(opcode.i32LtS, instruction)
	case ir.OpCmpGt:
		return l.emitBinaryNumeric(opcode.i32GtS, instruction)
	case ir.OpCall:
		label, err := expectLabel(instruction.Operands[0], "CALL target")
		if err != nil {
			return err
		}
		signature, ok := l.options.Signatures[label.Name]
		if !ok {
			return &WasmLoweringError{Message: fmt.Sprintf("missing function signature for %s", label.Name)}
		}
		functionIndex, ok := l.options.FunctionIndices[label.Name]
		if !ok {
			return &WasmLoweringError{Message: fmt.Sprintf("unknown function label: %s", label.Name)}
		}
		for paramIndex := 0; paramIndex < signature.ParamCount; paramIndex++ {
			l.emitLocalGet(regVarBase + paramIndex)
		}
		l.emitOpcode(opcode.call)
		l.emitU32(functionIndex)
		l.emitLocalSet(regScratch)
		return nil
	case ir.OpRet, ir.OpHalt:
		l.emitLocalGet(regScratch)
		l.emitOpcode(opcode.return_)
		return nil
	case ir.OpNop:
		l.emitOpcode(opcode.nop)
		return nil
	case ir.OpSyscall:
		return l.emitSyscall(instruction)
	default:
		return &WasmLoweringError{Message: fmt.Sprintf("unsupported opcode: %s", instruction.Opcode.String())}
	}
}

func (l *functionLowerer) emitSyscall(instruction ir.IrInstruction) error {
	syscall, err := expectImmediate(instruction.Operands[0], "SYSCALL number")
	if err != nil {
		return err
	}
	switch syscall.Value {
	case syscallWrite:
		return l.emitWasiWrite()
	case syscallRead:
		return l.emitWasiRead()
	case syscallExit:
		return l.emitWasiExit()
	default:
		return &WasmLoweringError{Message: fmt.Sprintf("unsupported SYSCALL number: %d", syscall.Value)}
	}
}

func (l *functionLowerer) emitWasiWrite() error {
	scratchBase, err := l.requireWasiScratch()
	if err != nil {
		return err
	}
	iovecPtr := scratchBase + wasiIovecOffset
	nwrittenPtr := scratchBase + wasiCountOffset
	bytePtr := scratchBase + wasiByteOffset

	l.emitI32Const(bytePtr)
	l.emitLocalGet(syscallArg0)
	l.emitOpcode(opcode.i32Store8)
	l.emitMemarg(0, 0)

	l.emitStoreConstI32(iovecPtr, bytePtr)
	l.emitStoreConstI32(iovecPtr+4, 1)

	l.emitI32Const(1)
	l.emitI32Const(iovecPtr)
	l.emitI32Const(1)
	l.emitI32Const(nwrittenPtr)
	if err := l.emitWasiCall(syscallWrite); err != nil {
		return err
	}
	l.emitLocalSet(regScratch)
	return nil
}

func (l *functionLowerer) emitWasiRead() error {
	scratchBase, err := l.requireWasiScratch()
	if err != nil {
		return err
	}
	iovecPtr := scratchBase + wasiIovecOffset
	nreadPtr := scratchBase + wasiCountOffset
	bytePtr := scratchBase + wasiByteOffset

	l.emitI32Const(bytePtr)
	l.emitI32Const(0)
	l.emitOpcode(opcode.i32Store8)
	l.emitMemarg(0, 0)

	l.emitStoreConstI32(iovecPtr, bytePtr)
	l.emitStoreConstI32(iovecPtr+4, 1)

	l.emitI32Const(0)
	l.emitI32Const(iovecPtr)
	l.emitI32Const(1)
	l.emitI32Const(nreadPtr)
	if err := l.emitWasiCall(syscallRead); err != nil {
		return err
	}
	l.emitLocalSet(regScratch)

	l.emitI32Const(bytePtr)
	l.emitOpcode(opcode.i32Load8U)
	l.emitMemarg(0, 0)
	l.emitLocalSet(syscallArg0)
	return nil
}

func (l *functionLowerer) emitWasiExit() error {
	l.emitLocalGet(syscallArg0)
	if err := l.emitWasiCall(syscallExit); err != nil {
		return err
	}
	l.emitI32Const(0)
	l.emitOpcode(opcode.return_)
	return nil
}

func (l *functionLowerer) emitStoreConstI32(address, value int) {
	l.emitI32Const(address)
	l.emitI32Const(value)
	l.emitOpcode(opcode.i32Store)
	l.emitMemarg(2, 0)
}

func (l *functionLowerer) emitWasiCall(syscallNumber int) error {
	functionIndex, ok := l.options.WasiContext.FunctionIndices[syscallNumber]
	if !ok {
		return &WasmLoweringError{Message: fmt.Sprintf("missing WASI import for SYSCALL %d", syscallNumber)}
	}
	l.emitOpcode(opcode.call)
	l.emitU32(functionIndex)
	return nil
}

func (l *functionLowerer) requireWasiScratch() (int, error) {
	if l.options.WasiContext.ScratchBase == nil {
		return 0, &WasmLoweringError{Message: "SYSCALL lowering requires WASM scratch memory"}
	}
	return *l.options.WasiContext.ScratchBase, nil
}

func (l *functionLowerer) emitBinaryNumeric(op byte, instruction ir.IrInstruction) error {
	name := instruction.Opcode.String()
	dst, err := expectRegister(instruction.Operands[0], name+" dst")
	if err != nil {
		return err
	}
	left, err := expectRegister(instruction.Operands[1], name+" lhs")
	if err != nil {
		return err
	}
	right, err := expectRegister(instruction.Operands[2], name+" rhs")
	if err != nil {
		return err
	}
	l.emitLocalGet(left.Index)
	l.emitLocalGet(right.Index)
	l.emitOpcode(op)
	l.emitLocalSet(dst.Index)
	return nil
}

func (l *functionLowerer) emitAddress(baseIndex, offsetIndex int) {
	l.emitLocalGet(baseIndex)
	l.emitLocalGet(offsetIndex)
	l.emitOpcode(opcode.i32Add)
}

func (l *functionLowerer) emitLocalGet(regIndex int) {
	l.emitOpcode(opcode.localGet)
	l.emitU32(uint32(l.localIndex(regIndex)))
}

func (l *functionLowerer) emitLocalSet(regIndex int) {
	l.emitOpcode(opcode.localSet)
	l.emitU32(uint32(l.localIndex(regIndex)))
}

func (l *functionLowerer) emitI32Const(value int) {
	l.emitOpcode(opcode.i32Const)
	l.emitBytes(wasmleb128.EncodeSigned(int64(value)))
}

func (l *functionLowerer) emitMemarg(align, offset int) {
	l.emitU32(uint32(align))
	l.emitU32(uint32(offset))
}

func (l *functionLowerer) emitOpcode(value byte) {
	l.bytes = append(l.bytes, value)
}

func (l *functionLowerer) emitByte(value byte) {
	l.bytes = append(l.bytes, value)
}

func (l *functionLowerer) emitU32(value uint32) {
	l.emitBytes(wasmleb128.EncodeUnsigned(uint64(value)))
}

func (l *functionLowerer) emitBytes(bytes []byte) {
	l.bytes = append(l.bytes, bytes...)
}

func (l *functionLowerer) localIndex(regIndex int) int {
	return l.paramCount + regIndex
}

func (l *functionLowerer) requireLabelIndex(label string) (int, error) {
	index, ok := l.labelToIndex[label]
	if !ok {
		return 0, &WasmLoweringError{Message: fmt.Sprintf("missing label %s in %s", label, l.options.Fn.Label)}
	}
	return index, nil
}

func (l *functionLowerer) findFirstBranchToLabel(start, end int, label string) (int, error) {
	for index := start; index < end; index++ {
		instruction := l.instructions[index]
		if instruction.Opcode != ir.OpBranchZ && instruction.Opcode != ir.OpBranchNz {
			continue
		}
		if labelNameFromOperand(instruction.Operands[1]) == label {
			return index, nil
		}
	}
	return 0, &WasmLoweringError{Message: fmt.Sprintf("expected branch to %s in %s", label, l.options.Fn.Label)}
}

func (l *functionLowerer) findLastJumpToLabel(start, end int, label string) (int, error) {
	for index := end - 1; index >= start; index-- {
		instruction := l.instructions[index]
		if instruction.Opcode != ir.OpJump {
			continue
		}
		if labelNameFromOperand(instruction.Operands[0]) == label {
			return index, nil
		}
	}
	return 0, &WasmLoweringError{Message: fmt.Sprintf("expected jump to %s in %s", label, l.options.Fn.Label)}
}

func InferFunctionSignaturesFromComments(program *ir.IrProgram) map[string]FunctionSignature {
	signatures := make(map[string]FunctionSignature)
	pendingComment := ""

	for _, instruction := range program.Instructions {
		if instruction.Opcode == ir.OpComment {
			pendingComment = labelNameFromOperand(instruction.Operands[0])
			continue
		}

		labelName := functionLabelName(instruction)
		if labelName != "" {
			if labelName == "_start" {
				signatures[labelName] = FunctionSignature{Label: labelName, ParamCount: 0, ExportName: "_start"}
			} else if strings.HasPrefix(labelName, "_fn_") && pendingComment != "" {
				exportName := strings.TrimPrefix(labelName, "_fn_")
				match := functionCommentRe.FindStringSubmatch(pendingComment)
				if len(match) == 3 && match[1] == exportName {
					paramsBlob := strings.TrimSpace(match[2])
					paramCount := 0
					if paramsBlob != "" {
						for _, piece := range strings.Split(paramsBlob, ",") {
							if strings.TrimSpace(piece) != "" {
								paramCount++
							}
						}
					}
					signatures[labelName] = FunctionSignature{
						Label:      labelName,
						ParamCount: paramCount,
						ExportName: exportName,
					}
				}
			}
			pendingComment = ""
			continue
		}

		pendingComment = ""
	}

	return signatures
}

func makeFunctionIR(label string, instructions []ir.IrInstruction, signatures map[string]FunctionSignature) (functionIR, error) {
	signature, ok := signatures[label]
	if !ok && label == "_start" {
		signature = FunctionSignature{Label: label, ParamCount: 0, ExportName: "_start"}
		ok = true
	}
	if !ok {
		return functionIR{}, &WasmLoweringError{Message: fmt.Sprintf("missing function signature for %s", label)}
	}

	maxReg := max(1, regVarBase+max(signature.ParamCount-1, 0))
	hasSyscall := false
	for _, instruction := range instructions {
		if instruction.Opcode == ir.OpSyscall {
			hasSyscall = true
		}
		for _, operand := range instruction.Operands {
			if register, ok := operand.(ir.IrRegister); ok {
				maxReg = max(maxReg, register.Index)
			}
		}
	}
	if hasSyscall {
		maxReg = max(maxReg, syscallArg0)
	}

	return functionIR{
		Label:        label,
		Instructions: instructions,
		Signature:    signature,
		MaxReg:       maxReg,
	}, nil
}

func constExpr(value int) []byte {
	result := []byte{opcode.i32Const}
	result = append(result, wasmleb128.EncodeSigned(int64(value))...)
	result = append(result, opcode.end)
	return result
}

func functionLabelName(instruction ir.IrInstruction) string {
	label := labelNameFromInstruction(instruction)
	if label == "_start" || strings.HasPrefix(label, "_fn_") {
		return label
	}
	return ""
}

func labelNameFromInstruction(instruction ir.IrInstruction) string {
	if instruction.Opcode != ir.OpLabel || len(instruction.Operands) == 0 {
		return ""
	}
	label, ok := instruction.Operands[0].(ir.IrLabel)
	if !ok {
		return ""
	}
	return label.Name
}

func labelNameFromOperand(operand ir.IrOperand) string {
	label, ok := operand.(ir.IrLabel)
	if !ok {
		return ""
	}
	return label.Name
}

func expectRegister(operand ir.IrOperand, context string) (ir.IrRegister, error) {
	value, ok := operand.(ir.IrRegister)
	if !ok {
		return ir.IrRegister{}, &WasmLoweringError{Message: fmt.Sprintf("%s: expected register, got %s", context, describeOperand(operand))}
	}
	return value, nil
}

func expectImmediate(operand ir.IrOperand, context string) (ir.IrImmediate, error) {
	value, ok := operand.(ir.IrImmediate)
	if !ok {
		return ir.IrImmediate{}, &WasmLoweringError{Message: fmt.Sprintf("%s: expected immediate, got %s", context, describeOperand(operand))}
	}
	return value, nil
}

func expectLabel(operand ir.IrOperand, context string) (ir.IrLabel, error) {
	value, ok := operand.(ir.IrLabel)
	if !ok {
		return ir.IrLabel{}, &WasmLoweringError{Message: fmt.Sprintf("%s: expected label, got %s", context, describeOperand(operand))}
	}
	return value, nil
}

func isLabelOperand(operand ir.IrOperand) bool {
	_, ok := operand.(ir.IrLabel)
	return ok
}

func describeOperand(operand ir.IrOperand) string {
	switch value := operand.(type) {
	case ir.IrRegister:
		return fmt.Sprintf("v%d", value.Index)
	case ir.IrImmediate:
		return fmt.Sprintf("%d", value.Value)
	case ir.IrLabel:
		return value.Name
	default:
		return fmt.Sprintf("%T", operand)
	}
}

func alignUp(value, alignment int) int {
	return ((value + alignment - 1) / alignment) * alignment
}

func funcTypeKey(funcType wasmtypes.FuncType) string {
	params := make([]string, len(funcType.Params))
	for index, value := range funcType.Params {
		params[index] = fmt.Sprintf("%d", value)
	}
	results := make([]string, len(funcType.Results))
	for index, value := range funcType.Results {
		results[index] = fmt.Sprintf("%d", value)
	}
	return strings.Join(params, ",") + "=>" + strings.Join(results, ",")
}

func mustOpcode(name string) byte {
	info, ok := wasmopcodes.GetOpcodeByName(name)
	if !ok {
		panic(fmt.Sprintf("missing wasm opcode %q", name))
	}
	return info.Opcode
}

func bytesOfSize(size int, value byte) []byte {
	data := make([]byte, size)
	for index := range data {
		data[index] = value
	}
	return data
}

func totalDataSize(decls []ir.IrDataDecl) int {
	total := 0
	for _, decl := range decls {
		total += decl.Size
	}
	return total
}

func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}
