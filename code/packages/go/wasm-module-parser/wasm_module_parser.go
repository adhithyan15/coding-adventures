// Package wasmmoduleparser provides a decoder for WebAssembly binary modules.
//
// This package is part of the coding-adventures monorepo, a ground-up
// implementation of the computing stack from transistors to operating systems.
//
// # What this Package Does
//
// It parses a raw .wasm binary ([]byte) into a structured WasmModule value.
// This is the decoder layer — it takes bytes and produces data. It does NOT
// execute, validate, or optimise anything beyond structural parsing.
//
// # The .wasm Binary Format
//
// A .wasm file is a sequence of sections, each with an ID byte and a
// LEB128-encoded size, following an 8-byte header:
//
//	┌─────────────────────────────────────────────────────────────────────┐
//	│  WebAssembly binary module (.wasm)                                  │
//	├─────────────────────────────────────────────────────────────────────┤
//	│  Offset 0: magic  0x00 0x61 0x73 0x6D  ("\0asm")                   │
//	│  Offset 4: version 0x01 0x00 0x00 0x00  (1, little-endian u32)     │
//	├──────────────────────┬──────────────────────────────────────────────┤
//	│ [id:u8][size:LEB128][payload:size bytes]  ← section envelope        │
//	│ ... repeated for each section ...                                   │
//	└──────────────────────┴──────────────────────────────────────────────┘
//
// # Section ID Table
//
//	┌────┬──────────────────────┬─────────────────────────────────────────┐
//	│ ID │ Name                 │ Contains                                │
//	├────┼──────────────────────┼─────────────────────────────────────────┤
//	│  0 │ Custom               │ name + arbitrary bytes                  │
//	│  1 │ Type                 │ []FuncType                              │
//	│  2 │ Import               │ []Import                                │
//	│  3 │ Function             │ []uint32 (type indices)                 │
//	│  4 │ Table                │ []TableType                             │
//	│  5 │ Memory               │ []MemoryType                            │
//	│  6 │ Global               │ []Global                                │
//	│  7 │ Export               │ []Export                                │
//	│  8 │ Start                │ *uint32 (nil = absent)                  │
//	│  9 │ Element              │ []Element                               │
//	│ 10 │ Code                 │ []FunctionBody                          │
//	│ 11 │ Data                 │ []DataSegment                           │
//	└────┴──────────────────────┴─────────────────────────────────────────┘
//
// # Public API
//
//	parser := wasmmoduleparser.New()
//	module, err := parser.Parse(data)
//	if err != nil {
//	    var pe *wasmmoduleparser.ParseError
//	    if errors.As(err, &pe) {
//	        fmt.Printf("parse error at byte %d: %s\n", pe.Offset, pe.Message)
//	    }
//	}
package wasmmoduleparser

import (
	"fmt"

	wasmleb128 "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-leb128"
	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
)

// ---------------------------------------------------------------------------
// HEADER CONSTANTS
//
// The 4-byte magic "\0asm" was chosen deliberately:
//   - The null byte at position 0 prevents text editors from misidentifying
//     the file as plain text (most text readers bail on a null byte).
//   - "asm" is a memorable mnemonic for "assembly."
//
// The version is a little-endian u32. Version 1 is the only version in
// widespread use; future versions would change this field.
// ---------------------------------------------------------------------------

var wasmMagic = [4]byte{0x00, 0x61, 0x73, 0x6D}   // "\0asm"
var wasmVersion = [4]byte{0x01, 0x00, 0x00, 0x00}  // version 1

// Section ID constants. These are specified by the WASM 1.0 binary format.
//
// Sections 1–11 must appear in ascending ID order within the binary. Custom
// sections (ID 0) may appear anywhere, including before section 1 or between
// any two non-custom sections.
const (
	sectionCustom   byte = 0
	sectionType     byte = 1
	sectionImport   byte = 2
	sectionFunction byte = 3
	sectionTable    byte = 4
	sectionMemory   byte = 5
	sectionGlobal   byte = 6
	sectionExport   byte = 7
	sectionStart    byte = 8
	sectionElement  byte = 9
	sectionCode     byte = 10
	sectionData     byte = 11
)

// ---------------------------------------------------------------------------
// PARSE ERROR
//
// ParseError carries both a human-readable message and the byte offset where
// the error was detected. The offset is invaluable when debugging binary
// files — you can jump directly to that offset in a hex editor.
// ---------------------------------------------------------------------------

// ParseError is returned by Parser.Parse when the binary is malformed.
//
// It implements the error interface via the Error() method.
//
// Example:
//
//	_, err := parser.Parse([]byte("NOTAWASM"))
//	if err != nil {
//	    var pe *ParseError
//	    errors.As(err, &pe)
//	    fmt.Println(pe.Offset, pe.Message)
//	}
type ParseError struct {
	Message string // human-readable description
	Offset  int    // byte offset in the input where the error was detected
}

// Error implements the error interface. Returns the Message field.
func (e *ParseError) Error() string {
	return e.Message
}

func parseError(offset int, format string, args ...any) *ParseError {
	return &ParseError{
		Message: fmt.Sprintf(format, args...),
		Offset:  offset,
	}
}

// ---------------------------------------------------------------------------
// PARSER
//
// Parser is a stateless struct. Each call to Parse is independent.
// We use a cursor-based approach: a pos integer tracks our position within
// the []byte slice. All helpers accept (data []byte, pos int) and return
// (result, newPos, error).
// ---------------------------------------------------------------------------

// Parser decodes WebAssembly binary modules.
//
// Create one with New() and call Parse for each binary you want to decode.
// The Parser itself is stateless — you can reuse it across calls.
//
// Example:
//
//	p := wasmmoduleparser.New()
//	module, err := p.Parse(wasmBytes)
//	if err != nil {
//	    log.Fatal(err)
//	}
//	for _, ft := range module.Types {
//	    fmt.Printf("type: params=%v results=%v\n", ft.Params, ft.Results)
//	}
type Parser struct{}

// New creates a new Parser.
func New() *Parser {
	result, _ := StartNew[*Parser]("wasm-module-parser.New", nil,
		func(op *Operation[*Parser], rf *ResultFactory[*Parser]) *OperationResult[*Parser] {
			return rf.Generate(true, false, &Parser{})
		}).GetResult()
	return result
}

// Parse decodes a complete .wasm binary into a WasmModule.
//
// Parameters:
//   - data: the raw bytes of a .wasm file (e.g., from os.ReadFile)
//
// Returns:
//   - *WasmModule: populated module on success
//   - error: *ParseError on malformed input, nil on success
//
// The returned WasmModule is always non-nil on success. All slice fields
// default to nil if the corresponding section was absent; nil slices behave
// identically to empty slices when ranging or appending.
//
// Example:
//
//	data, _ := os.ReadFile("hello.wasm")
//	module, err := wasmmoduleparser.New().Parse(data)
func (p *Parser) Parse(data []byte) (*wasmtypes.WasmModule, error) {
	return StartNew[*wasmtypes.WasmModule]("wasm-module-parser.Parse", nil,
		func(op *Operation[*wasmtypes.WasmModule], rf *ResultFactory[*wasmtypes.WasmModule]) *OperationResult[*wasmtypes.WasmModule] {
			op.AddProperty("dataLen", len(data))
			module, err := p.parseInternal(data)
			if err != nil {
				return rf.Fail(nil, err)
			}
			return rf.Generate(true, false, module)
		}).GetResult()
}

func (p *Parser) parseInternal(data []byte) (*wasmtypes.WasmModule, error) {
	pos := 0

	// ------------------------------------------------------------------
	// STEP 1: VALIDATE THE 8-BYTE HEADER
	//
	// The header is exactly 8 bytes and must match the fixed magic and
	// version values. This is the simplest sanity check — if this fails,
	// the input is definitely not a valid WASM module.
	//
	// Layout:
	//
	//   byte offset: 0    1    2    3    4    5    6    7
	//   value:       0x00 0x61 0x73 0x6D 0x01 0x00 0x00 0x00
	//                ↑─────────────↑    ↑────────────────↑
	//                magic "\0asm"       version 1
	// ------------------------------------------------------------------
	if len(data) < 8 {
		return nil, parseError(0,
			"truncated header: need 8 bytes, got %d", len(data))
	}

	var magic [4]byte
	copy(magic[:], data[0:4])
	if magic != wasmMagic {
		return nil, parseError(0,
			"bad magic bytes at offset 0: expected \\x00asm, got %v", data[0:4])
	}

	var version [4]byte
	copy(version[:], data[4:8])
	if version != wasmVersion {
		return nil, parseError(4,
			"unsupported version at offset 4: expected \\x01\\x00\\x00\\x00, got %v",
			data[4:8])
	}

	pos = 8

	// ------------------------------------------------------------------
	// STEP 2: PARSE SECTIONS
	//
	// After the header, sections follow one after another until end of file.
	// Each section has this envelope:
	//
	//   section_id:   u8          (1 byte)
	//   section_size: u32 LEB128  (variable, 1–5 bytes)
	//   payload:      bytes       (section_size bytes)
	//
	// We track lastNonCustomID to enforce that non-custom sections appear
	// in ascending order (custom sections are exempt from this rule).
	// ------------------------------------------------------------------
	module := &wasmtypes.WasmModule{}
	lastNonCustomID := byte(0)

	for pos < len(data) {
		// Read the section ID (1 byte).
		if pos >= len(data) {
			return nil, parseError(pos, "unexpected end of input reading section ID")
		}
		sectionID := data[pos]
		pos++

		// Read the section size (LEB128).
		sectionSize, consumed, err := wasmleb128.DecodeUnsigned(data, pos)
		if err != nil {
			return nil, parseError(pos, "error reading section size: %s", err)
		}
		pos += consumed

		// Verify the section payload fits in the remaining data.
		if pos+int(sectionSize) > len(data) {
			return nil, parseError(pos,
				"truncated section %d: declared size %d, only %d bytes remain",
				sectionID, sectionSize, len(data)-pos)
		}

		// Extract the payload slice.
		payload := data[pos : pos+int(sectionSize)]
		payloadBase := pos
		pos += int(sectionSize)

		// Enforce section ordering for non-custom sections.
		if sectionID != sectionCustom {
			if sectionID < lastNonCustomID {
				return nil, parseError(payloadBase,
					"section ordering violation: section %d appears after section %d",
					sectionID, lastNonCustomID)
			}
			lastNonCustomID = sectionID
		}

		// Dispatch to the appropriate parser.
		switch sectionID {
		case sectionCustom:
			cs, err := parseCustomSection(payload, payloadBase)
			if err != nil {
				return nil, err
			}
			module.Customs = append(module.Customs, cs)

		case sectionType:
			types, err := parseTypeSection(payload, payloadBase)
			if err != nil {
				return nil, err
			}
			module.Types = types

		case sectionImport:
			imports, err := parseImportSection(payload, payloadBase)
			if err != nil {
				return nil, err
			}
			module.Imports = imports

		case sectionFunction:
			funcs, err := parseFunctionSection(payload, payloadBase)
			if err != nil {
				return nil, err
			}
			module.Functions = funcs

		case sectionTable:
			tables, err := parseTableSection(payload, payloadBase)
			if err != nil {
				return nil, err
			}
			module.Tables = tables

		case sectionMemory:
			mems, err := parseMemorySection(payload, payloadBase)
			if err != nil {
				return nil, err
			}
			module.Memories = mems

		case sectionGlobal:
			globals, err := parseGlobalSection(payload, payloadBase)
			if err != nil {
				return nil, err
			}
			module.Globals = globals

		case sectionExport:
			exports, err := parseExportSection(payload, payloadBase)
			if err != nil {
				return nil, err
			}
			module.Exports = exports

		case sectionStart:
			idx, err := parseStartSection(payload, payloadBase)
			if err != nil {
				return nil, err
			}
			module.Start = &idx

		case sectionElement:
			elems, err := parseElementSection(payload, payloadBase)
			if err != nil {
				return nil, err
			}
			module.Elements = elems

		case sectionCode:
			bodies, err := parseCodeSection(payload, payloadBase)
			if err != nil {
				return nil, err
			}
			module.Code = bodies

		case sectionData:
			segs, err := parseDataSection(payload, payloadBase)
			if err != nil {
				return nil, err
			}
			module.Data = segs

		default:
			return nil, parseError(payloadBase-2,
				"unknown section ID %d at offset %d", sectionID, payloadBase-2)
		}
	}

	return module, nil
}

// ---------------------------------------------------------------------------
// LOW-LEVEL HELPERS
//
// These helpers wrap LEB128 decoding, bounds checking, name reading, limits
// reading, and expression reading.  Every helper returns (result, newPos, err)
// or (result, newPos) in the style that allows chained reads without deep
// nesting.
// ---------------------------------------------------------------------------

// readU32 reads one unsigned LEB128 u32 from data[pos:] and returns
// (value, newPos, err).  On error, the error is a *ParseError.
func readU32(data []byte, pos, base int) (uint32, int, error) {
	if pos >= len(data) {
		return 0, pos, parseError(base+pos,
			"unexpected end of input at offset %d: need LEB128 byte", base+pos)
	}
	v, consumed, err := wasmleb128.DecodeUnsigned(data, pos)
	if err != nil {
		return 0, pos, parseError(base+pos, "LEB128 decode error: %s", err)
	}
	return uint32(v), pos + consumed, nil
}

// checkBytes returns a ParseError if data[pos:pos+need] is out of bounds.
func checkBytes(data []byte, pos, need, base int) error {
	if pos+need > len(data) {
		return parseError(base+pos,
			"unexpected end of input at offset %d: need %d bytes, have %d",
			base+pos, need, len(data)-pos)
	}
	return nil
}

// readName reads a length-prefixed UTF-8 string from data[pos:].
//
// Format: <len: LEB128> <utf-8 bytes × len>
//
// WASM uses this for module names, entity names, export names, and custom
// section names throughout the binary.
func readName(data []byte, pos, base int) (string, int, error) {
	length, pos, err := readU32(data, pos, base)
	if err != nil {
		return "", pos, err
	}
	if err := checkBytes(data, pos, int(length), base); err != nil {
		return "", pos, err
	}
	name := string(data[pos : pos+int(length)])
	return name, pos + int(length), nil
}

// readLimits reads a WASM Limits struct from data[pos:].
//
// Binary format:
//
//	0x00  <min>           — only minimum (HasMax = false)
//	0x01  <min>  <max>   — minimum and maximum (HasMax = true)
//
// Limits appear in Table and Memory sections (and their import descriptors).
func readLimits(data []byte, pos, base int) (wasmtypes.Limits, int, error) {
	if err := checkBytes(data, pos, 1, base); err != nil {
		return wasmtypes.Limits{}, pos, err
	}
	flags := data[pos]
	pos++

	minVal, pos, err := readU32(data, pos, base)
	if err != nil {
		return wasmtypes.Limits{}, pos, err
	}

	lim := wasmtypes.Limits{Min: minVal}
	if flags&1 != 0 {
		maxVal, newPos, err := readU32(data, pos, base)
		if err != nil {
			return wasmtypes.Limits{}, pos, err
		}
		pos = newPos
		lim.Max = maxVal
		lim.HasMax = true
	}
	return lim, pos, nil
}

// readExpr reads a WASM constant expression from data[pos:].
//
// A constant expression is a sequence of bytecode ending with 0x0B (end).
// We capture the raw bytes including the terminator.
//
// Why raw bytes?
//
//	The parser's responsibility is structure, not semantics. A separate
//	evaluator can interpret these bytes at instantiation time. Keeping them
//	raw makes the parser simple, fast, and correct by construction.
func readExpr(data []byte, pos, base int) ([]byte, int, error) {
	start := pos
	for {
		if pos >= len(data) {
			return nil, start, parseError(base+start,
				"unterminated init_expr at offset %d: no 0x0B end opcode found",
				base+start)
		}
		if data[pos] == 0x0B {
			pos++ // consume the end opcode
			break
		}
		pos++
	}
	expr := make([]byte, pos-start)
	copy(expr, data[start:pos])
	return expr, pos, nil
}

// ---------------------------------------------------------------------------
// SECTION PARSERS
//
// Each function receives the raw section payload and the absolute byte offset
// of the payload start (used in ParseError.Offset for precise error messages).
// ---------------------------------------------------------------------------

// parseTypeSection parses the Type section (ID 1).
//
// The Type section is a vector of function type descriptors. Each entry
// describes parameter and result value types.  Functions reference types
// by index rather than repeating the full signature.
//
// Binary layout of one FuncType entry:
//
//	0x60         — functype indicator (must be present)
//	count: LEB128  — number of params
//	params: [ValueType × count]
//	count: LEB128  — number of results
//	results: [ValueType × count]
//
// Why have a separate Type section?
//
//	Deduplication: 100 functions with the same (i32, i32)→i32 signature only
//	encode the type once.  The Function section then references it by index.
func parseTypeSection(payload []byte, base int) ([]wasmtypes.FuncType, error) {
	pos := 0
	count, pos, err := readU32(payload, pos, base)
	if err != nil {
		return nil, err
	}

	types := make([]wasmtypes.FuncType, 0, count)
	for i := uint32(0); i < count; i++ {
		if err := checkBytes(payload, pos, 1, base); err != nil {
			return nil, parseError(base+pos,
				"truncated type section: expected %d types, got %d", count, i)
		}
		// Each function type entry starts with 0x60 = functype marker.
		if payload[pos] != 0x60 {
			return nil, parseError(base+pos,
				"expected functype marker 0x60 at offset %d, got 0x%02X",
				base+pos, payload[pos])
		}
		pos++ // consume 0x60

		paramCount, pos2, err := readU32(payload, pos, base)
		if err != nil {
			return nil, err
		}
		pos = pos2

		params := make([]wasmtypes.ValueType, paramCount)
		for j := uint32(0); j < paramCount; j++ {
			if err := checkBytes(payload, pos, 1, base); err != nil {
				return nil, err
			}
			params[j] = wasmtypes.ValueType(payload[pos])
			pos++
		}

		resultCount, pos3, err := readU32(payload, pos, base)
		if err != nil {
			return nil, err
		}
		pos = pos3

		results := make([]wasmtypes.ValueType, resultCount)
		for j := uint32(0); j < resultCount; j++ {
			if err := checkBytes(payload, pos, 1, base); err != nil {
				return nil, err
			}
			results[j] = wasmtypes.ValueType(payload[pos])
			pos++
		}

		types = append(types, wasmtypes.FuncType{Params: params, Results: results})
	}
	return types, nil
}

// parseImportSection parses the Import section (ID 2).
//
// Each import brings an external entity (function, table, memory, or global)
// into the module. Imports are indexed before local definitions in each
// address space.
//
// Why import?
//   - Access host APIs: JavaScript functions, WASI system calls
//   - Link modules together
//   - Share memory between modules
//
// Binary layout of one Import entry:
//
//	module_name: LEB128-length + UTF-8
//	field_name:  LEB128-length + UTF-8
//	kind:        u8 (0=func, 1=table, 2=memory, 3=global)
//	type_info:   depends on kind
func parseImportSection(payload []byte, base int) ([]wasmtypes.Import, error) {
	pos := 0
	count, pos, err := readU32(payload, pos, base)
	if err != nil {
		return nil, err
	}

	imports := make([]wasmtypes.Import, 0, count)
	for i := uint32(0); i < count; i++ {
		modName, pos2, err := readName(payload, pos, base)
		if err != nil {
			return nil, err
		}
		pos = pos2

		fieldName, pos3, err := readName(payload, pos, base)
		if err != nil {
			return nil, err
		}
		pos = pos3

		if err := checkBytes(payload, pos, 1, base); err != nil {
			return nil, err
		}
		kindByte := payload[pos]
		pos++

		imp := wasmtypes.Import{
			ModuleName: modName,
			Name:       fieldName,
			Kind:       wasmtypes.ExternalKind(kindByte),
		}

		switch wasmtypes.ExternalKind(kindByte) {
		case wasmtypes.ExternalKindFunction:
			// Function import: type index
			idx, pos4, err := readU32(payload, pos, base)
			if err != nil {
				return nil, err
			}
			pos = pos4
			imp.TypeInfo = idx

		case wasmtypes.ExternalKindTable:
			// Table import: element_type + limits
			if err := checkBytes(payload, pos, 1, base); err != nil {
				return nil, err
			}
			elemType := payload[pos]
			pos++
			lim, pos5, err := readLimits(payload, pos, base)
			if err != nil {
				return nil, err
			}
			pos = pos5
			imp.TypeInfo = wasmtypes.TableType{ElementType: elemType, Limits: lim}

		case wasmtypes.ExternalKindMemory:
			// Memory import: limits only
			lim, pos6, err := readLimits(payload, pos, base)
			if err != nil {
				return nil, err
			}
			pos = pos6
			imp.TypeInfo = wasmtypes.MemoryType{Limits: lim}

		case wasmtypes.ExternalKindGlobal:
			// Global import: valtype + mutability
			if err := checkBytes(payload, pos, 2, base); err != nil {
				return nil, err
			}
			valType := wasmtypes.ValueType(payload[pos])
			mutable := payload[pos+1] != 0
			pos += 2
			imp.TypeInfo = wasmtypes.GlobalType{ValueType: valType, Mutable: mutable}

		default:
			return nil, parseError(base+pos-1,
				"unknown import kind 0x%02X at offset %d", kindByte, base+pos-1)
		}

		imports = append(imports, imp)
	}
	return imports, nil
}

// parseFunctionSection parses the Function section (ID 3).
//
// The Function section is a vector of type indices, one per locally-defined
// function. Entry i gives the type index for the i-th local function.
//
// Why separate from the Code section?
//
//	Separating type indices from bytecode allows tools to inspect function
//	signatures (section 3) without parsing bytecode (section 10). The Code
//	section is often the largest section; skipping it is a significant win
//	for tools that only need the module's interface.
//
// Binary layout:
//
//	count:        LEB128
//	type_indices: [u32 LEB128 × count]
func parseFunctionSection(payload []byte, base int) ([]uint32, error) {
	pos := 0
	count, pos, err := readU32(payload, pos, base)
	if err != nil {
		return nil, err
	}
	funcs := make([]uint32, 0, count)
	for i := uint32(0); i < count; i++ {
		idx, pos2, err := readU32(payload, pos, base)
		if err != nil {
			return nil, err
		}
		pos = pos2
		funcs = append(funcs, idx)
	}
	return funcs, nil
}

// parseTableSection parses the Table section (ID 4).
//
// A table is an array of opaque references. In WASM 1.0, the only valid
// element type is 0x70 = funcref. Tables are used by call_indirect to
// implement indirect function dispatch (C function pointers, vtables, etc.).
//
// Why tables instead of raw function pointers?
//
//	WASM is sandboxed. call_indirect looks up a function reference in a
//	table by index and verifies the type signature. A bug in sandboxed code
//	cannot cause an arbitrary jump to an unexpected address.
//
// Binary layout of one Table entry:
//
//	element_type: u8 (0x70 = funcref)
//	limits:       flags + min [+ max]
func parseTableSection(payload []byte, base int) ([]wasmtypes.TableType, error) {
	pos := 0
	count, pos, err := readU32(payload, pos, base)
	if err != nil {
		return nil, err
	}
	tables := make([]wasmtypes.TableType, 0, count)
	for i := uint32(0); i < count; i++ {
		if err := checkBytes(payload, pos, 1, base); err != nil {
			return nil, err
		}
		elemType := payload[pos]
		pos++
		lim, pos2, err := readLimits(payload, pos, base)
		if err != nil {
			return nil, err
		}
		pos = pos2
		tables = append(tables, wasmtypes.TableType{ElementType: elemType, Limits: lim})
	}
	return tables, nil
}

// parseMemorySection parses the Memory section (ID 5).
//
// WASM 1.0 allows at most one linear memory per module. The memory is a flat
// byte-addressable array measured in 64-KiB pages (65536 bytes per page).
//
// Why 64-KiB pages?
//
//	A coarse granularity allows the runtime to use OS-level virtual memory
//	mappings (mmap) without excessive bookkeeping. 64 KiB matches the page
//	size of many OSes and is a practical compromise between waste and overhead.
//
// Binary layout of one Memory entry:
//
//	limits: flags + min [+ max]
func parseMemorySection(payload []byte, base int) ([]wasmtypes.MemoryType, error) {
	pos := 0
	count, pos, err := readU32(payload, pos, base)
	if err != nil {
		return nil, err
	}
	mems := make([]wasmtypes.MemoryType, 0, count)
	for i := uint32(0); i < count; i++ {
		lim, pos2, err := readLimits(payload, pos, base)
		if err != nil {
			return nil, err
		}
		pos = pos2
		mems = append(mems, wasmtypes.MemoryType{Limits: lim})
	}
	return mems, nil
}

// parseGlobalSection parses the Global section (ID 6).
//
// Globals are module-level variables with a type, mutability flag, and a
// constant initializer expression.
//
// Common uses in compiled WASM:
//   - __stack_pointer (mutable i32): the C stack pointer
//   - __data_end, __heap_base (immutable i32): linker-generated constants
//
// Binary layout of one Global entry:
//
//	valtype:   u8
//	mutable:   u8 (0 = immutable, 1 = mutable)
//	init_expr: bytes until 0x0B (end) inclusive
func parseGlobalSection(payload []byte, base int) ([]wasmtypes.Global, error) {
	pos := 0
	count, pos, err := readU32(payload, pos, base)
	if err != nil {
		return nil, err
	}
	globals := make([]wasmtypes.Global, 0, count)
	for i := uint32(0); i < count; i++ {
		if err := checkBytes(payload, pos, 2, base); err != nil {
			return nil, err
		}
		valType := wasmtypes.ValueType(payload[pos])
		mutable := payload[pos+1] != 0
		pos += 2

		initExpr, pos2, err := readExpr(payload, pos, base)
		if err != nil {
			return nil, err
		}
		pos = pos2

		globals = append(globals, wasmtypes.Global{
			GlobalType: wasmtypes.GlobalType{ValueType: valType, Mutable: mutable},
			InitExpr:   initExpr,
		})
	}
	return globals, nil
}

// parseExportSection parses the Export section (ID 7).
//
// Exports make module-internal entities accessible from outside. A WASM
// runtime host calls exported functions by name.
//
// Common exports:
//   - "_start" or "main": entry point for WASI programs
//   - "memory": the linear memory (shared with JavaScript in a browser)
//   - "__wbindgen_*": glue functions generated by wasm-bindgen
//
// Binary layout of one Export entry:
//
//	name:  LEB128-length + UTF-8
//	kind:  u8 (0=func, 1=table, 2=memory, 3=global)
//	index: u32 LEB128 (into the appropriate address space)
func parseExportSection(payload []byte, base int) ([]wasmtypes.Export, error) {
	pos := 0
	count, pos, err := readU32(payload, pos, base)
	if err != nil {
		return nil, err
	}
	exports := make([]wasmtypes.Export, 0, count)
	for i := uint32(0); i < count; i++ {
		name, pos2, err := readName(payload, pos, base)
		if err != nil {
			return nil, err
		}
		pos = pos2

		if err := checkBytes(payload, pos, 1, base); err != nil {
			return nil, err
		}
		kind := wasmtypes.ExternalKind(payload[pos])
		pos++

		idx, pos3, err := readU32(payload, pos, base)
		if err != nil {
			return nil, err
		}
		pos = pos3

		exports = append(exports, wasmtypes.Export{Name: name, Kind: kind, Index: idx})
	}
	return exports, nil
}

// parseStartSection parses the Start section (ID 8).
//
// If present, the Start section names a function to call automatically when
// the module is instantiated. The function must have type () → ().
//
// In practice, C/C++ programs compiled to WASM use the Start section to
// run static constructors and initialise the C runtime before main().
//
// Binary layout:
//
//	function_index: u32 LEB128
func parseStartSection(payload []byte, base int) (uint32, error) {
	pos := 0
	idx, _, err := readU32(payload, pos, base)
	if err != nil {
		return 0, err
	}
	return idx, nil
}

// parseElementSection parses the Element section (ID 9).
//
// Element segments initialise table slots with function references. This is
// required before any call_indirect can safely use those table entries.
//
// Example: a C program's function-pointer table would be populated here.
//
// Binary layout of one Element entry:
//
//	table_index:     u32 LEB128 (always 0 in WASM 1.0)
//	offset_expr:     bytes until 0x0B
//	func_count:      u32 LEB128
//	function_indices:[u32 LEB128 × func_count]
func parseElementSection(payload []byte, base int) ([]wasmtypes.Element, error) {
	pos := 0
	count, pos, err := readU32(payload, pos, base)
	if err != nil {
		return nil, err
	}
	elems := make([]wasmtypes.Element, 0, count)
	for i := uint32(0); i < count; i++ {
		tableIdx, pos2, err := readU32(payload, pos, base)
		if err != nil {
			return nil, err
		}
		pos = pos2

		offsetExpr, pos3, err := readExpr(payload, pos, base)
		if err != nil {
			return nil, err
		}
		pos = pos3

		funcCount, pos4, err := readU32(payload, pos, base)
		if err != nil {
			return nil, err
		}
		pos = pos4

		indices := make([]uint32, 0, funcCount)
		for j := uint32(0); j < funcCount; j++ {
			idx, pos5, err := readU32(payload, pos, base)
			if err != nil {
				return nil, err
			}
			pos = pos5
			indices = append(indices, idx)
		}

		elems = append(elems, wasmtypes.Element{
			TableIndex:      tableIdx,
			OffsetExpr:      offsetExpr,
			FunctionIndices: indices,
		})
	}
	return elems, nil
}

// parseCodeSection parses the Code section (ID 10).
//
// The Code section contains one function body per locally-defined function,
// in the same order as the Function section. Each body is self-contained
// (prefixed with its own byte size) so tools can lazily parse individual
// functions without reading the whole section.
//
// Local variable encoding:
//
//	The binary uses (count, type) groups to compress runs of same-type locals.
//	E.g., 5 i32 locals is stored as (5, i32) rather than 5 × (1, i32).
//	We expand the groups into individual entries here.
//
// Binary layout of one Code entry:
//
//	body_size:        LEB128 (total bytes for this entry, after this field)
//	local_decl_count: LEB128
//	local_decls:      [(count: LEB128, type: u8) × local_decl_count]
//	code:             remaining bytes (bytecode, ending with 0x0B = end)
func parseCodeSection(payload []byte, base int) ([]wasmtypes.FunctionBody, error) {
	pos := 0
	count, pos, err := readU32(payload, pos, base)
	if err != nil {
		return nil, err
	}
	bodies := make([]wasmtypes.FunctionBody, 0, count)
	for i := uint32(0); i < count; i++ {
		bodySize, pos2, err := readU32(payload, pos, base)
		if err != nil {
			return nil, err
		}
		pos = pos2
		bodyStart := pos

		if err := checkBytes(payload, pos, int(bodySize), base); err != nil {
			return nil, parseError(base+pos,
				"truncated code entry %d: need %d bytes, have %d",
				i, bodySize, len(payload)-pos)
		}

		// Parse local variable declaration groups.
		localDeclCount, pos3, err := readU32(payload, pos, base)
		if err != nil {
			return nil, err
		}
		pos = pos3

		var locals []wasmtypes.ValueType
		for j := uint32(0); j < localDeclCount; j++ {
			runCount, pos4, err := readU32(payload, pos, base)
			if err != nil {
				return nil, err
			}
			pos = pos4

			if err := checkBytes(payload, pos, 1, base); err != nil {
				return nil, err
			}
			valType := wasmtypes.ValueType(payload[pos])
			pos++

			// Expand (runCount, valType) into individual local entries.
			for k := uint32(0); k < runCount; k++ {
				locals = append(locals, valType)
			}
		}

		// Remaining bytes in the body are the raw bytecode.
		codeEnd := bodyStart + int(bodySize)
		code := make([]byte, codeEnd-pos)
		copy(code, payload[pos:codeEnd])
		pos = codeEnd

		bodies = append(bodies, wasmtypes.FunctionBody{Locals: locals, Code: code})
	}
	return bodies, nil
}

// parseDataSection parses the Data section (ID 11).
//
// Data segments initialise regions of linear memory with constant byte
// strings. This is how compiled C programs load .rodata and .data sections
// into WASM memory at startup.
//
// Example: string literals, lookup tables, and pre-computed data all end up
// as DataSegment entries.
//
// Binary layout of one Data entry:
//
//	memory_index: u32 LEB128 (always 0 in WASM 1.0)
//	offset_expr:  bytes until 0x0B
//	byte_count:   u32 LEB128
//	data_bytes:   [u8 × byte_count]
func parseDataSection(payload []byte, base int) ([]wasmtypes.DataSegment, error) {
	pos := 0
	count, pos, err := readU32(payload, pos, base)
	if err != nil {
		return nil, err
	}
	segs := make([]wasmtypes.DataSegment, 0, count)
	for i := uint32(0); i < count; i++ {
		memIdx, pos2, err := readU32(payload, pos, base)
		if err != nil {
			return nil, err
		}
		pos = pos2

		offsetExpr, pos3, err := readExpr(payload, pos, base)
		if err != nil {
			return nil, err
		}
		pos = pos3

		byteCount, pos4, err := readU32(payload, pos, base)
		if err != nil {
			return nil, err
		}
		pos = pos4

		if err := checkBytes(payload, pos, int(byteCount), base); err != nil {
			return nil, err
		}
		data := make([]byte, byteCount)
		copy(data, payload[pos:pos+int(byteCount)])
		pos += int(byteCount)

		segs = append(segs, wasmtypes.DataSegment{
			MemoryIndex: memIdx,
			OffsetExpr:  offsetExpr,
			Data:        data,
		})
	}
	return segs, nil
}

// parseCustomSection parses a Custom section (ID 0).
//
// Custom sections carry non-standard extension data. The WASM runtime ignores
// any custom section it does not recognise, making them a safe extension point
// for toolchain metadata, debug information, and future proposals.
//
// Common custom sections:
//   - "name":       WASM Name section (human-readable function/local names)
//   - ".debug_info": DWARF debug information (Emscripten, wasm-pack)
//   - "producers":  Toolchain metadata
//
// Binary layout:
//
//	name_len:   u32 LEB128
//	name_bytes: UTF-8
//	data:       remaining bytes in the section payload
func parseCustomSection(payload []byte, base int) (wasmtypes.CustomSection, error) {
	pos := 0
	name, pos, err := readName(payload, pos, base)
	if err != nil {
		return wasmtypes.CustomSection{}, err
	}
	data := make([]byte, len(payload)-pos)
	copy(data, payload[pos:])
	return wasmtypes.CustomSection{Name: name, Data: data}, nil
}
