package wasmmoduleencoder

import (
	"fmt"

	wasmleb128 "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-leb128"
	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
)

var (
	WASMMagic   = []byte{0x00, 0x61, 0x73, 0x6D}
	WASMVersion = []byte{0x01, 0x00, 0x00, 0x00}
)

type WasmEncodeError struct {
	Message string
}

func (e *WasmEncodeError) Error() string {
	return e.Message
}

func EncodeModule(module *wasmtypes.WasmModule) ([]byte, error) {
	sections := make([][]byte, 0)

	for _, custom := range module.Customs {
		payload := encodeCustom(custom)
		sections = append(sections, section(0, payload))
	}
	if len(module.Types) > 0 {
		payload, err := vector(module.Types, encodeFuncType)
		if err != nil {
			return nil, err
		}
		sections = append(sections, section(1, payload))
	}
	if len(module.Imports) > 0 {
		payload, err := vector(module.Imports, encodeImport)
		if err != nil {
			return nil, err
		}
		sections = append(sections, section(2, payload))
	}
	if len(module.Functions) > 0 {
		payload, err := vector(module.Functions, func(value uint32) ([]byte, error) {
			return u32(value), nil
		})
		if err != nil {
			return nil, err
		}
		sections = append(sections, section(3, payload))
	}
	if len(module.Tables) > 0 {
		payload, err := vector(module.Tables, func(value wasmtypes.TableType) ([]byte, error) {
			return encodeTableType(value), nil
		})
		if err != nil {
			return nil, err
		}
		sections = append(sections, section(4, payload))
	}
	if len(module.Memories) > 0 {
		payload, err := vector(module.Memories, func(value wasmtypes.MemoryType) ([]byte, error) {
			return encodeMemoryType(value), nil
		})
		if err != nil {
			return nil, err
		}
		sections = append(sections, section(5, payload))
	}
	if len(module.Globals) > 0 {
		payload, err := vector(module.Globals, func(value wasmtypes.Global) ([]byte, error) {
			return encodeGlobal(value), nil
		})
		if err != nil {
			return nil, err
		}
		sections = append(sections, section(6, payload))
	}
	if len(module.Exports) > 0 {
		payload, err := vector(module.Exports, func(value wasmtypes.Export) ([]byte, error) {
			return encodeExport(value), nil
		})
		if err != nil {
			return nil, err
		}
		sections = append(sections, section(7, payload))
	}
	if module.Start != nil {
		sections = append(sections, section(8, u32(*module.Start)))
	}
	if len(module.Elements) > 0 {
		payload, err := vector(module.Elements, func(value wasmtypes.Element) ([]byte, error) {
			return encodeElement(value), nil
		})
		if err != nil {
			return nil, err
		}
		sections = append(sections, section(9, payload))
	}
	if len(module.Code) > 0 {
		payload, err := vector(module.Code, func(value wasmtypes.FunctionBody) ([]byte, error) {
			return encodeFunctionBody(value), nil
		})
		if err != nil {
			return nil, err
		}
		sections = append(sections, section(10, payload))
	}
	if len(module.Data) > 0 {
		payload, err := vector(module.Data, func(value wasmtypes.DataSegment) ([]byte, error) {
			return encodeDataSegment(value), nil
		})
		if err != nil {
			return nil, err
		}
		sections = append(sections, section(11, payload))
	}

	result := make([]byte, 0, len(WASMMagic)+len(WASMVersion))
	result = append(result, WASMMagic...)
	result = append(result, WASMVersion...)
	for _, current := range sections {
		result = append(result, current...)
	}
	return result, nil
}

func section(sectionID byte, payload []byte) []byte {
	result := []byte{sectionID}
	result = append(result, u32(uint32(len(payload)))...)
	result = append(result, payload...)
	return result
}

func u32(value uint32) []byte {
	return wasmleb128.EncodeUnsigned(uint64(value))
}

func name(text string) []byte {
	data := []byte(text)
	result := make([]byte, 0, len(data)+5)
	result = append(result, u32(uint32(len(data)))...)
	result = append(result, data...)
	return result
}

func vector[T any](values []T, encoder func(T) ([]byte, error)) ([]byte, error) {
	result := make([]byte, 0)
	result = append(result, u32(uint32(len(values)))...)
	for _, value := range values {
		encoded, err := encoder(value)
		if err != nil {
			return nil, err
		}
		result = append(result, encoded...)
	}
	return result, nil
}

func valueTypes(types []wasmtypes.ValueType) []byte {
	result := make([]byte, 0, len(types)+5)
	result = append(result, u32(uint32(len(types)))...)
	for _, valueType := range types {
		result = append(result, byte(valueType))
	}
	return result
}

func encodeFuncType(funcType wasmtypes.FuncType) ([]byte, error) {
	result := []byte{0x60}
	result = append(result, valueTypes(funcType.Params)...)
	result = append(result, valueTypes(funcType.Results)...)
	return result, nil
}

func encodeLimits(limits wasmtypes.Limits) []byte {
	if !limits.HasMax {
		result := []byte{0x00}
		return append(result, u32(limits.Min)...)
	}

	result := []byte{0x01}
	result = append(result, u32(limits.Min)...)
	result = append(result, u32(limits.Max)...)
	return result
}

func encodeMemoryType(memoryType wasmtypes.MemoryType) []byte {
	return encodeLimits(memoryType.Limits)
}

func encodeTableType(tableType wasmtypes.TableType) []byte {
	result := []byte{tableType.ElementType}
	return append(result, encodeLimits(tableType.Limits)...)
}

func encodeGlobalType(globalType wasmtypes.GlobalType) []byte {
	mutable := byte(0x00)
	if globalType.Mutable {
		mutable = 0x01
	}
	return []byte{byte(globalType.ValueType), mutable}
}

func encodeImport(importValue wasmtypes.Import) ([]byte, error) {
	result := make([]byte, 0)
	result = append(result, name(importValue.ModuleName)...)
	result = append(result, name(importValue.Name)...)
	result = append(result, byte(importValue.Kind))

	switch importValue.Kind {
	case wasmtypes.ExternalKindFunction:
		typeIndex, ok := importValue.TypeInfo.(uint32)
		if !ok {
			return nil, &WasmEncodeError{Message: "function imports require a numeric type index"}
		}
		result = append(result, u32(typeIndex)...)
	case wasmtypes.ExternalKindTable:
		tableType, ok := importValue.TypeInfo.(wasmtypes.TableType)
		if !ok {
			return nil, &WasmEncodeError{Message: "table imports require a TableType"}
		}
		result = append(result, encodeTableType(tableType)...)
	case wasmtypes.ExternalKindMemory:
		memoryType, ok := importValue.TypeInfo.(wasmtypes.MemoryType)
		if !ok {
			return nil, &WasmEncodeError{Message: "memory imports require a MemoryType"}
		}
		result = append(result, encodeMemoryType(memoryType)...)
	case wasmtypes.ExternalKindGlobal:
		globalType, ok := importValue.TypeInfo.(wasmtypes.GlobalType)
		if !ok {
			return nil, &WasmEncodeError{Message: "global imports require a GlobalType"}
		}
		result = append(result, encodeGlobalType(globalType)...)
	default:
		return nil, &WasmEncodeError{Message: fmt.Sprintf("unsupported import kind: %d", importValue.Kind)}
	}

	return result, nil
}

func encodeExport(exportValue wasmtypes.Export) []byte {
	result := make([]byte, 0)
	result = append(result, name(exportValue.Name)...)
	result = append(result, byte(exportValue.Kind))
	result = append(result, u32(exportValue.Index)...)
	return result
}

func encodeGlobal(globalValue wasmtypes.Global) []byte {
	result := make([]byte, 0)
	result = append(result, encodeGlobalType(globalValue.GlobalType)...)
	result = append(result, globalValue.InitExpr...)
	return result
}

func encodeElement(element wasmtypes.Element) []byte {
	result := make([]byte, 0)
	result = append(result, u32(element.TableIndex)...)
	result = append(result, element.OffsetExpr...)
	result = append(result, u32(uint32(len(element.FunctionIndices)))...)
	for _, functionIndex := range element.FunctionIndices {
		result = append(result, u32(functionIndex)...)
	}
	return result
}

func encodeDataSegment(segment wasmtypes.DataSegment) []byte {
	result := make([]byte, 0)
	result = append(result, u32(segment.MemoryIndex)...)
	result = append(result, segment.OffsetExpr...)
	result = append(result, u32(uint32(len(segment.Data)))...)
	result = append(result, segment.Data...)
	return result
}

func encodeFunctionBody(body wasmtypes.FunctionBody) []byte {
	localGroups := groupLocals(body.Locals)
	payload := make([]byte, 0)
	payload = append(payload, u32(uint32(len(localGroups)))...)
	for _, group := range localGroups {
		payload = append(payload, u32(uint32(group.Count))...)
		payload = append(payload, byte(group.ValueType))
	}
	payload = append(payload, body.Code...)

	result := make([]byte, 0)
	result = append(result, u32(uint32(len(payload)))...)
	result = append(result, payload...)
	return result
}

type localGroup struct {
	Count     int
	ValueType wasmtypes.ValueType
}

func groupLocals(locals []wasmtypes.ValueType) []localGroup {
	if len(locals) == 0 {
		return nil
	}

	groups := make([]localGroup, 0)
	currentType := locals[0]
	count := 1
	for index := 1; index < len(locals); index++ {
		if locals[index] == currentType {
			count++
			continue
		}
		groups = append(groups, localGroup{Count: count, ValueType: currentType})
		currentType = locals[index]
		count = 1
	}
	groups = append(groups, localGroup{Count: count, ValueType: currentType})
	return groups
}

func encodeCustom(custom wasmtypes.CustomSection) []byte {
	result := make([]byte, 0)
	result = append(result, name(custom.Name)...)
	result = append(result, custom.Data...)
	return result
}
