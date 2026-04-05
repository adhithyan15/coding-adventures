// Package wasmvalidator provides structural validation for WASM 1.0 modules.
//
// ════════════════════════════════════════════════════════════════════════
// WHY VALIDATE?
// ════════════════════════════════════════════════════════════════════════
//
// A parsed WASM module is just a bag of sections.  Validation ensures:
//
//   - All type indices are within bounds (function declarations point to
//     actual entries in the type section).
//   - All function, table, memory, and global indices are consistent.
//   - The module has at most one memory and one table (WASM 1.0 rule).
//   - Memory limits don't exceed 65536 pages (4 GiB).
//   - Export names are unique.
//   - Data/element segment offsets reference valid memories/tables.
//   - The start function (if present) has the right signature: () -> ().
//
// This package performs *structural* validation.  Full bytecode-level
// type-checking (stack typing) is deferred to a future enhancement.
//
// ════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ════════════════════════════════════════════════════════════════════════
//
//	validated, err := wasmvalidator.Validate(module)
//	if err != nil {
//	    // err is a *ValidationError with Kind and Message
//	}
//	// validated.Module, validated.FuncTypes, validated.IndexSpaces
package wasmvalidator

import (
	"fmt"

	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
)

// ════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ════════════════════════════════════════════════════════════════════════

// MaxMemoryPages is the WASM 1.0 limit: 65536 pages = 4 GiB.
const MaxMemoryPages = 65536

// ════════════════════════════════════════════════════════════════════════
// ERROR TYPES
// ════════════════════════════════════════════════════════════════════════

// ValidationErrorKind categorizes validation failures.
type ValidationErrorKind string

const (
	ErrInvalidTypeIndex     ValidationErrorKind = "invalid_type_index"
	ErrInvalidFuncIndex     ValidationErrorKind = "invalid_func_index"
	ErrInvalidTableIndex    ValidationErrorKind = "invalid_table_index"
	ErrInvalidMemoryIndex   ValidationErrorKind = "invalid_memory_index"
	ErrInvalidGlobalIndex   ValidationErrorKind = "invalid_global_index"
	ErrMultipleMemories     ValidationErrorKind = "multiple_memories"
	ErrMultipleTables       ValidationErrorKind = "multiple_tables"
	ErrMemoryLimitExceeded  ValidationErrorKind = "memory_limit_exceeded"
	ErrMemoryLimitOrder     ValidationErrorKind = "memory_limit_order"
	ErrTableLimitOrder      ValidationErrorKind = "table_limit_order"
	ErrDuplicateExportName  ValidationErrorKind = "duplicate_export_name"
	ErrExportIndexOutOfRange ValidationErrorKind = "export_index_out_of_range"
	ErrStartFunctionBadType ValidationErrorKind = "start_function_bad_type"
)

// ValidationError is returned when a module fails validation.
type ValidationError struct {
	Kind    ValidationErrorKind
	Message string
}

func (e *ValidationError) Error() string {
	return fmt.Sprintf("ValidationError(%s): %s", e.Kind, e.Message)
}

// ════════════════════════════════════════════════════════════════════════
// INDEX SPACES
// ════════════════════════════════════════════════════════════════════════

// IndexSpaces holds the merged index spaces for a module.  Each space
// combines imported entities with locally defined ones.
//
// In WASM, the function index space is:
//
//	[imported function 0, ..., imported function N-1, local function 0, ...]
//
// The same pattern applies to tables, memories, and globals.
type IndexSpaces struct {
	FuncTypes          []wasmtypes.FuncType
	NumImportedFuncs   int
	TableTypes         []wasmtypes.TableType
	NumImportedTables  int
	MemoryTypes        []wasmtypes.MemoryType
	NumImportedMemories int
	GlobalTypes        []wasmtypes.GlobalType
	NumImportedGlobals int
	NumTypes           int
}

// ════════════════════════════════════════════════════════════════════════
// VALIDATED MODULE
// ════════════════════════════════════════════════════════════════════════

// ValidatedModule wraps a module that has passed structural validation.
type ValidatedModule struct {
	Module      *wasmtypes.WasmModule
	FuncTypes   []wasmtypes.FuncType
	IndexSpaces *IndexSpaces
}

// ════════════════════════════════════════════════════════════════════════
// VALIDATE — Top-level entry point
// ════════════════════════════════════════════════════════════════════════

// Validate checks a parsed WASM module for structural correctness.
//
// This validates index bounds, limits, export uniqueness, and the start
// function signature.  It does NOT perform bytecode-level type checking.
//
// Returns a ValidatedModule on success or a *ValidationError on failure.
func Validate(module *wasmtypes.WasmModule) (*ValidatedModule, error) {
	spaces, err := ValidateStructure(module)
	if err != nil {
		return nil, err
	}

	return &ValidatedModule{
		Module:      module,
		FuncTypes:   spaces.FuncTypes,
		IndexSpaces: spaces,
	}, nil
}

// ════════════════════════════════════════════════════════════════════════
// VALIDATE STRUCTURE — Build and check index spaces
// ════════════════════════════════════════════════════════════════════════

// ValidateStructure builds the combined index spaces for the module and
// validates all structural constraints.
func ValidateStructure(module *wasmtypes.WasmModule) (*IndexSpaces, error) {
	spaces := buildIndexSpaces(module)

	// WASM 1.0: at most one table.
	if len(spaces.TableTypes) > 1 {
		return nil, &ValidationError{
			Kind:    ErrMultipleTables,
			Message: fmt.Sprintf("WASM 1.0 allows at most one table, found %d", len(spaces.TableTypes)),
		}
	}

	// WASM 1.0: at most one memory.
	if len(spaces.MemoryTypes) > 1 {
		return nil, &ValidationError{
			Kind:    ErrMultipleMemories,
			Message: fmt.Sprintf("WASM 1.0 allows at most one memory, found %d", len(spaces.MemoryTypes)),
		}
	}

	// Validate memory limits.
	for _, memType := range spaces.MemoryTypes {
		if err := validateMemoryLimits(memType.Limits); err != nil {
			return nil, err
		}
	}

	// Validate table limits.
	for _, tableType := range spaces.TableTypes {
		if err := validateTableLimits(tableType.Limits); err != nil {
			return nil, err
		}
	}

	// Validate that each local function references a valid type index.
	for i, typeIdx := range module.Functions {
		if int(typeIdx) >= len(module.Types) {
			return nil, &ValidationError{
				Kind:    ErrInvalidTypeIndex,
				Message: fmt.Sprintf("function %d references type index %d, but only %d types exist", i, typeIdx, len(module.Types)),
			}
		}
	}

	// Validate export names are unique and indices are in bounds.
	if err := validateExports(module, spaces); err != nil {
		return nil, err
	}

	// Validate data segments reference valid memories.
	for i, seg := range module.Data {
		if int(seg.MemoryIndex) >= len(spaces.MemoryTypes) {
			return nil, &ValidationError{
				Kind:    ErrInvalidMemoryIndex,
				Message: fmt.Sprintf("data segment %d references memory %d, but only %d memories exist", i, seg.MemoryIndex, len(spaces.MemoryTypes)),
			}
		}
	}

	// Validate element segments reference valid tables.
	for i, elem := range module.Elements {
		if int(elem.TableIndex) >= len(spaces.TableTypes) {
			return nil, &ValidationError{
				Kind:    ErrInvalidTableIndex,
				Message: fmt.Sprintf("element segment %d references table %d, but only %d tables exist", i, elem.TableIndex, len(spaces.TableTypes)),
			}
		}
		// Validate that function indices in element segments are in bounds.
		for j, funcIdx := range elem.FunctionIndices {
			if int(funcIdx) >= len(spaces.FuncTypes) {
				return nil, &ValidationError{
					Kind:    ErrInvalidFuncIndex,
					Message: fmt.Sprintf("element segment %d entry %d references function %d, but only %d functions exist", i, j, funcIdx, len(spaces.FuncTypes)),
				}
			}
		}
	}

	// Validate start function.
	if module.Start != nil {
		startIdx := int(*module.Start)
		if startIdx >= len(spaces.FuncTypes) {
			return nil, &ValidationError{
				Kind:    ErrInvalidFuncIndex,
				Message: fmt.Sprintf("start function index %d out of bounds (only %d functions)", startIdx, len(spaces.FuncTypes)),
			}
		}
		ft := spaces.FuncTypes[startIdx]
		if len(ft.Params) != 0 || len(ft.Results) != 0 {
			return nil, &ValidationError{
				Kind:    ErrStartFunctionBadType,
				Message: fmt.Sprintf("start function must have type () -> (), got (%d params) -> (%d results)", len(ft.Params), len(ft.Results)),
			}
		}
	}

	return spaces, nil
}

// ════════════════════════════════════════════════════════════════════════
// HELPERS
// ════════════════════════════════════════════════════════════════════════

// buildIndexSpaces constructs the combined index spaces from the module's
// imports and local definitions.
func buildIndexSpaces(module *wasmtypes.WasmModule) *IndexSpaces {
	spaces := &IndexSpaces{
		NumTypes: len(module.Types),
	}

	// Process imports to build the front of each index space.
	for _, imp := range module.Imports {
		switch imp.Kind {
		case wasmtypes.ExternalKindFunction:
			typeIdx, ok := imp.TypeInfo.(uint32)
			if ok && int(typeIdx) < len(module.Types) {
				spaces.FuncTypes = append(spaces.FuncTypes, module.Types[typeIdx])
			}
			spaces.NumImportedFuncs++
		case wasmtypes.ExternalKindTable:
			if tt, ok := imp.TypeInfo.(wasmtypes.TableType); ok {
				spaces.TableTypes = append(spaces.TableTypes, tt)
			}
			spaces.NumImportedTables++
		case wasmtypes.ExternalKindMemory:
			if mt, ok := imp.TypeInfo.(wasmtypes.MemoryType); ok {
				spaces.MemoryTypes = append(spaces.MemoryTypes, mt)
			}
			spaces.NumImportedMemories++
		case wasmtypes.ExternalKindGlobal:
			if gt, ok := imp.TypeInfo.(wasmtypes.GlobalType); ok {
				spaces.GlobalTypes = append(spaces.GlobalTypes, gt)
			}
			spaces.NumImportedGlobals++
		}
	}

	// Append locally defined entities.
	for _, typeIdx := range module.Functions {
		if int(typeIdx) < len(module.Types) {
			spaces.FuncTypes = append(spaces.FuncTypes, module.Types[typeIdx])
		}
	}
	spaces.TableTypes = append(spaces.TableTypes, module.Tables...)
	spaces.MemoryTypes = append(spaces.MemoryTypes, module.Memories...)
	for _, g := range module.Globals {
		spaces.GlobalTypes = append(spaces.GlobalTypes, g.GlobalType)
	}

	return spaces
}

// validateMemoryLimits checks that memory limits are within WASM 1.0 bounds.
func validateMemoryLimits(limits wasmtypes.Limits) error {
	if limits.Min > MaxMemoryPages {
		return &ValidationError{
			Kind:    ErrMemoryLimitExceeded,
			Message: fmt.Sprintf("memory minimum %d exceeds maximum allowed %d pages", limits.Min, MaxMemoryPages),
		}
	}
	if limits.HasMax {
		if limits.Max > MaxMemoryPages {
			return &ValidationError{
				Kind:    ErrMemoryLimitExceeded,
				Message: fmt.Sprintf("memory maximum %d exceeds maximum allowed %d pages", limits.Max, MaxMemoryPages),
			}
		}
		if limits.Min > limits.Max {
			return &ValidationError{
				Kind:    ErrMemoryLimitOrder,
				Message: fmt.Sprintf("memory minimum %d exceeds maximum %d", limits.Min, limits.Max),
			}
		}
	}
	return nil
}

// validateTableLimits checks table limit ordering.
func validateTableLimits(limits wasmtypes.Limits) error {
	if limits.HasMax && limits.Min > limits.Max {
		return &ValidationError{
			Kind:    ErrTableLimitOrder,
			Message: fmt.Sprintf("table minimum %d exceeds maximum %d", limits.Min, limits.Max),
		}
	}
	return nil
}

// validateExports checks that export names are unique and reference valid indices.
func validateExports(module *wasmtypes.WasmModule, spaces *IndexSpaces) error {
	seen := make(map[string]bool)
	for _, exp := range module.Exports {
		if seen[exp.Name] {
			return &ValidationError{
				Kind:    ErrDuplicateExportName,
				Message: fmt.Sprintf("duplicate export name %q", exp.Name),
			}
		}
		seen[exp.Name] = true

		switch exp.Kind {
		case wasmtypes.ExternalKindFunction:
			if int(exp.Index) >= len(spaces.FuncTypes) {
				return &ValidationError{
					Kind:    ErrExportIndexOutOfRange,
					Message: fmt.Sprintf("export %q references function %d, but only %d functions exist", exp.Name, exp.Index, len(spaces.FuncTypes)),
				}
			}
		case wasmtypes.ExternalKindTable:
			if int(exp.Index) >= len(spaces.TableTypes) {
				return &ValidationError{
					Kind:    ErrExportIndexOutOfRange,
					Message: fmt.Sprintf("export %q references table %d, but only %d tables exist", exp.Name, exp.Index, len(spaces.TableTypes)),
				}
			}
		case wasmtypes.ExternalKindMemory:
			if int(exp.Index) >= len(spaces.MemoryTypes) {
				return &ValidationError{
					Kind:    ErrExportIndexOutOfRange,
					Message: fmt.Sprintf("export %q references memory %d, but only %d memories exist", exp.Name, exp.Index, len(spaces.MemoryTypes)),
				}
			}
		case wasmtypes.ExternalKindGlobal:
			if int(exp.Index) >= len(spaces.GlobalTypes) {
				return &ValidationError{
					Kind:    ErrExportIndexOutOfRange,
					Message: fmt.Sprintf("export %q references global %d, but only %d globals exist", exp.Name, exp.Index, len(spaces.GlobalTypes)),
				}
			}
		}
	}
	return nil
}
