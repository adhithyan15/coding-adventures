package jvmclassfile

import (
	"bytes"
	"encoding/binary"
	"fmt"
	"math"
)

// The JVM class-file format is a binary tree of references. The parser below
// only models the small subset our repository needs first:
//
//   class file
//     -> constant pool
//     -> methods
//     -> Code attribute
//
// That is enough to prove we can both create and inspect real `.class` files
// from Go before we teach a higher layer to lower IR into them.

const (
	constantUTF8        = 1
	constantInteger     = 3
	constantLong        = 5
	constantDouble      = 6
	constantClass       = 7
	constantString      = 8
	constantFieldref    = 9
	constantMethodref   = 10
	constantNameAndType = 12

	// These are the access-flag bits the MVP builder needs.
	ACC_PUBLIC = 0x0001
	ACC_STATIC = 0x0008
	ACC_SUPER  = 0x0020
)

// ClassFileFormatError is returned when bytes do not describe the class-file
// structure this package understands.
type ClassFileFormatError struct {
	Message string
}

func (e ClassFileFormatError) Error() string {
	return e.Message
}

// JVMClassVersion records the major/minor class-file version pair.
type JVMClassVersion struct {
	Major uint16
	Minor uint16
}

func (v JVMClassVersion) String() string {
	return fmt.Sprintf("%d.%d", v.Major, v.Minor)
}

// Each constant-pool entry gets its own struct. That keeps parsing and later
// type assertions readable for people learning the format.
type JVMConstantPoolEntry interface {
	isJVMConstantPoolEntry()
}

type JVMUtf8Info struct {
	Value string
}

func (JVMUtf8Info) isJVMConstantPoolEntry() {}

type JVMIntegerInfo struct {
	Value int32
}

func (JVMIntegerInfo) isJVMConstantPoolEntry() {}

type JVMLongInfo struct {
	Value int64
}

func (JVMLongInfo) isJVMConstantPoolEntry() {}

type JVMDoubleInfo struct {
	Value float64
}

func (JVMDoubleInfo) isJVMConstantPoolEntry() {}

type JVMClassInfo struct {
	NameIndex uint16
}

func (JVMClassInfo) isJVMConstantPoolEntry() {}

type JVMStringInfo struct {
	StringIndex uint16
}

func (JVMStringInfo) isJVMConstantPoolEntry() {}

type JVMNameAndTypeInfo struct {
	NameIndex       uint16
	DescriptorIndex uint16
}

func (JVMNameAndTypeInfo) isJVMConstantPoolEntry() {}

type JVMFieldrefInfo struct {
	ClassIndex       uint16
	NameAndTypeIndex uint16
}

func (JVMFieldrefInfo) isJVMConstantPoolEntry() {}

type JVMMethodrefInfo struct {
	ClassIndex       uint16
	NameAndTypeIndex uint16
}

func (JVMMethodrefInfo) isJVMConstantPoolEntry() {}

// Resolved references are friendlier than raw pool indices in tests and later
// backend assertions.
type JVMFieldReference struct {
	ClassName  string
	Name       string
	Descriptor string
}

type JVMMethodReference struct {
	ClassName  string
	Name       string
	Descriptor string
}

// Attributes are opaque by default, except for Code, which the package decodes
// because method bodies are what later compiler backends care about most.
type JVMMethodAttribute interface {
	isJVMMethodAttribute()
}

type JVMAttributeInfo struct {
	Name string
	Info []byte
}

func (JVMAttributeInfo) isJVMMethodAttribute() {}

type JVMCodeAttribute struct {
	Name             string
	MaxStack         uint16
	MaxLocals        uint16
	Code             []byte
	NestedAttributes []JVMAttributeInfo
}

func (JVMCodeAttribute) isJVMMethodAttribute() {}

type JVMMethodInfo struct {
	AccessFlags uint16
	Name        string
	Descriptor  string
	Attributes  []JVMMethodAttribute
}

func (m JVMMethodInfo) CodeAttribute() *JVMCodeAttribute {
	for _, attribute := range m.Attributes {
		codeAttribute, ok := attribute.(JVMCodeAttribute)
		if ok {
			copyOfAttribute := codeAttribute
			return &copyOfAttribute
		}
	}
	return nil
}

type JVMClassFile struct {
	Version        JVMClassVersion
	AccessFlags    uint16
	ThisClassName  string
	SuperClassName string
	ConstantPool   []JVMConstantPoolEntry
	Methods        []JVMMethodInfo
}

func (cf *JVMClassFile) GetUTF8(index uint16) (string, error) {
	entry, err := cf.entry(index)
	if err != nil {
		return "", err
	}
	value, ok := entry.(JVMUtf8Info)
	if !ok {
		return "", classFileError("Constant pool entry %d is not a UTF-8 string", index)
	}
	return value.Value, nil
}

func (cf *JVMClassFile) ResolveClassName(index uint16) (string, error) {
	entry, err := cf.entry(index)
	if err != nil {
		return "", err
	}
	classInfo, ok := entry.(JVMClassInfo)
	if !ok {
		return "", classFileError("Constant pool entry %d is not a Class entry", index)
	}
	return cf.GetUTF8(classInfo.NameIndex)
}

func (cf *JVMClassFile) ResolveNameAndType(index uint16) (string, string, error) {
	entry, err := cf.entry(index)
	if err != nil {
		return "", "", err
	}
	nameAndType, ok := entry.(JVMNameAndTypeInfo)
	if !ok {
		return "", "", classFileError("Constant pool entry %d is not a NameAndType entry", index)
	}
	name, err := cf.GetUTF8(nameAndType.NameIndex)
	if err != nil {
		return "", "", err
	}
	descriptor, err := cf.GetUTF8(nameAndType.DescriptorIndex)
	if err != nil {
		return "", "", err
	}
	return name, descriptor, nil
}

func (cf *JVMClassFile) ResolveConstant(index uint16) (any, error) {
	entry, err := cf.entry(index)
	if err != nil {
		return nil, err
	}

	switch value := entry.(type) {
	case JVMUtf8Info:
		return value.Value, nil
	case JVMIntegerInfo:
		return value.Value, nil
	case JVMLongInfo:
		return value.Value, nil
	case JVMDoubleInfo:
		return value.Value, nil
	case JVMStringInfo:
		return cf.GetUTF8(value.StringIndex)
	default:
		return nil, classFileError(
			"Constant pool entry %d is not a loadable constant: %#v",
			index,
			entry,
		)
	}
}

func (cf *JVMClassFile) ResolveFieldref(index uint16) (JVMFieldReference, error) {
	entry, err := cf.entry(index)
	if err != nil {
		return JVMFieldReference{}, err
	}
	fieldref, ok := entry.(JVMFieldrefInfo)
	if !ok {
		return JVMFieldReference{}, classFileError("Constant pool entry %d is not a Fieldref entry", index)
	}
	name, descriptor, err := cf.ResolveNameAndType(fieldref.NameAndTypeIndex)
	if err != nil {
		return JVMFieldReference{}, err
	}
	className, err := cf.ResolveClassName(fieldref.ClassIndex)
	if err != nil {
		return JVMFieldReference{}, err
	}
	return JVMFieldReference{
		ClassName:  className,
		Name:       name,
		Descriptor: descriptor,
	}, nil
}

func (cf *JVMClassFile) ResolveMethodref(index uint16) (JVMMethodReference, error) {
	entry, err := cf.entry(index)
	if err != nil {
		return JVMMethodReference{}, err
	}
	methodref, ok := entry.(JVMMethodrefInfo)
	if !ok {
		return JVMMethodReference{}, classFileError("Constant pool entry %d is not a Methodref entry", index)
	}
	name, descriptor, err := cf.ResolveNameAndType(methodref.NameAndTypeIndex)
	if err != nil {
		return JVMMethodReference{}, err
	}
	className, err := cf.ResolveClassName(methodref.ClassIndex)
	if err != nil {
		return JVMMethodReference{}, err
	}
	return JVMMethodReference{
		ClassName:  className,
		Name:       name,
		Descriptor: descriptor,
	}, nil
}

func (cf *JVMClassFile) LdcConstants() map[uint16]any {
	lookup := map[uint16]any{}
	for index := uint16(1); index < uint16(len(cf.ConstantPool)); index++ {
		switch entry := cf.ConstantPool[index].(type) {
		case JVMIntegerInfo:
			lookup[index] = entry.Value
		case JVMStringInfo:
			value, err := cf.GetUTF8(entry.StringIndex)
			if err == nil {
				lookup[index] = value
			}
		}
	}
	return lookup
}

func (cf *JVMClassFile) FindMethod(name string, descriptor ...string) *JVMMethodInfo {
	for index := range cf.Methods {
		method := &cf.Methods[index]
		if method.Name != name {
			continue
		}
		if len(descriptor) > 0 && method.Descriptor != descriptor[0] {
			continue
		}
		return method
	}
	return nil
}

func (cf *JVMClassFile) entry(index uint16) (JVMConstantPoolEntry, error) {
	if index == 0 || int(index) >= len(cf.ConstantPool) {
		return nil, classFileError("Constant pool index %d is out of range", index)
	}
	entry := cf.ConstantPool[index]
	if entry == nil {
		return nil, classFileError("Constant pool index %d points at a reserved wide slot", index)
	}
	return entry, nil
}

// BuildMinimalClassFile keeps emission boring on purpose. The caller supplies
// one method body, and the helper wraps it in the smallest useful class-file
// shell around it.
type BuildMinimalClassFileParams struct {
	ClassName         string
	MethodName        string
	Descriptor        string
	Code              []byte
	MaxStack          uint16
	MaxLocals         uint16
	Constants         []any
	MajorVersion      uint16
	MinorVersion      uint16
	ClassAccessFlags  uint16
	MethodAccessFlags uint16
	SuperClassName    string
}

func BuildMinimalClassFile(params BuildMinimalClassFileParams) ([]byte, error) {
	if params.ClassName == "" {
		return nil, fmt.Errorf("class name must not be empty")
	}
	if params.MethodName == "" {
		return nil, fmt.Errorf("method name must not be empty")
	}
	if params.Descriptor == "" {
		return nil, fmt.Errorf("descriptor must not be empty")
	}

	majorVersion := params.MajorVersion
	if majorVersion == 0 {
		majorVersion = 61
	}
	minorVersion := params.MinorVersion
	classAccessFlags := params.ClassAccessFlags
	if classAccessFlags == 0 {
		classAccessFlags = ACC_PUBLIC | ACC_SUPER
	}
	methodAccessFlags := params.MethodAccessFlags
	if methodAccessFlags == 0 {
		methodAccessFlags = ACC_PUBLIC | ACC_STATIC
	}
	superClassName := params.SuperClassName
	if superClassName == "" {
		superClassName = "java/lang/Object"
	}

	entries := make([][]byte, 0, 8+len(params.Constants)*2)
	indices := map[string]uint16{}
	addEntry := func(key string, payload []byte) uint16 {
		if index, ok := indices[key]; ok {
			return index
		}
		entries = append(entries, payload)
		index := uint16(len(entries))
		indices[key] = index
		return index
	}

	addUTF8 := func(value string) (uint16, error) {
		encoded := []byte(value)
		if len(encoded) > math.MaxUint16 {
			return 0, fmt.Errorf("UTF-8 constant %q exceeds 65535 bytes", value)
		}
		payload := make([]byte, 0, 3+len(encoded))
		payload = append(payload, constantUTF8)
		payload = appendU2(payload, uint16(len(encoded)))
		payload = append(payload, encoded...)
		return addEntry("Utf8:"+value, payload), nil
	}

	addClass := func(value string) (uint16, error) {
		nameIndex, err := addUTF8(value)
		if err != nil {
			return 0, err
		}
		payload := []byte{constantClass}
		payload = appendU2(payload, nameIndex)
		return addEntry("Class:"+value, payload), nil
	}

	var addConstant func(value any) (uint16, error)
	addConstant = func(value any) (uint16, error) {
		switch typed := value.(type) {
		case int:
			if typed < math.MinInt32 || typed > math.MaxInt32 {
				return 0, fmt.Errorf("integer constant %d is outside JVM Integer range", typed)
			}
			return addConstant(int32(typed))
		case int32:
			payload := []byte{constantInteger}
			payload = appendI4(payload, typed)
			return addEntry(fmt.Sprintf("Integer:%d", typed), payload), nil
		case string:
			stringIndex, err := addUTF8(typed)
			if err != nil {
				return 0, err
			}
			payload := []byte{constantString}
			payload = appendU2(payload, stringIndex)
			return addEntry("String:"+typed, payload), nil
		default:
			return 0, fmt.Errorf("unsupported minimal class constant type %T", value)
		}
	}

	thisClassIndex, err := addClass(params.ClassName)
	if err != nil {
		return nil, err
	}
	superClassIndex, err := addClass(superClassName)
	if err != nil {
		return nil, err
	}
	methodNameIndex, err := addUTF8(params.MethodName)
	if err != nil {
		return nil, err
	}
	descriptorIndex, err := addUTF8(params.Descriptor)
	if err != nil {
		return nil, err
	}
	codeNameIndex, err := addUTF8("Code")
	if err != nil {
		return nil, err
	}

	for _, constant := range params.Constants {
		if _, err := addConstant(constant); err != nil {
			return nil, err
		}
	}

	codeAttributeBody := make([]byte, 0, 12+len(params.Code))
	codeAttributeBody = appendU2(codeAttributeBody, params.MaxStack)
	codeAttributeBody = appendU2(codeAttributeBody, params.MaxLocals)
	codeAttributeBody = appendU4(codeAttributeBody, uint32(len(params.Code)))
	codeAttributeBody = append(codeAttributeBody, params.Code...)
	codeAttributeBody = appendU2(codeAttributeBody, 0)
	codeAttributeBody = appendU2(codeAttributeBody, 0)

	codeAttribute := make([]byte, 0, 6+len(codeAttributeBody))
	codeAttribute = appendU2(codeAttribute, codeNameIndex)
	codeAttribute = appendU4(codeAttribute, uint32(len(codeAttributeBody)))
	codeAttribute = append(codeAttribute, codeAttributeBody...)

	methodInfo := make([]byte, 0, 8+len(codeAttribute))
	methodInfo = appendU2(methodInfo, methodAccessFlags)
	methodInfo = appendU2(methodInfo, methodNameIndex)
	methodInfo = appendU2(methodInfo, descriptorIndex)
	methodInfo = appendU2(methodInfo, 1)
	methodInfo = append(methodInfo, codeAttribute...)

	classBytes := make([]byte, 0, 64+len(entries)*8+len(params.Code))
	classBytes = appendU4(classBytes, 0xCAFEBABE)
	classBytes = appendU2(classBytes, minorVersion)
	classBytes = appendU2(classBytes, majorVersion)
	classBytes = appendU2(classBytes, uint16(len(entries)+1))
	for _, entry := range entries {
		classBytes = append(classBytes, entry...)
	}
	classBytes = appendU2(classBytes, classAccessFlags)
	classBytes = appendU2(classBytes, thisClassIndex)
	classBytes = appendU2(classBytes, superClassIndex)
	classBytes = appendU2(classBytes, 0)
	classBytes = appendU2(classBytes, 0)
	classBytes = appendU2(classBytes, 1)
	classBytes = append(classBytes, methodInfo...)
	classBytes = appendU2(classBytes, 0)
	return classBytes, nil
}

func ParseClassFile(data []byte) (*JVMClassFile, error) {
	reader := &classReader{data: data}

	magic, err := reader.u4()
	if err != nil {
		return nil, err
	}
	if magic != 0xCAFEBABE {
		return nil, classFileError("Invalid class-file magic: 0x%08X", magic)
	}

	minor, err := reader.u2()
	if err != nil {
		return nil, err
	}
	major, err := reader.u2()
	if err != nil {
		return nil, err
	}

	constantPoolCount, err := reader.u2()
	if err != nil {
		return nil, err
	}
	constantPool := make([]JVMConstantPoolEntry, constantPoolCount)

	for index := uint16(1); index < constantPoolCount; {
		tag, err := reader.u1()
		if err != nil {
			return nil, err
		}

		advance := uint16(1)
		switch tag {
		case constantUTF8:
			length, err := reader.u2()
			if err != nil {
				return nil, err
			}
			bytesValue, err := reader.read(int(length))
			if err != nil {
				return nil, err
			}
			constantPool[index] = JVMUtf8Info{Value: string(bytesValue)}
		case constantInteger:
			valueBytes, err := reader.read(4)
			if err != nil {
				return nil, err
			}
			constantPool[index] = JVMIntegerInfo{Value: int32(binary.BigEndian.Uint32(valueBytes))}
		case constantLong:
			valueBytes, err := reader.read(8)
			if err != nil {
				return nil, err
			}
			constantPool[index] = JVMLongInfo{Value: int64(binary.BigEndian.Uint64(valueBytes))}
			advance = 2
		case constantDouble:
			valueBytes, err := reader.read(8)
			if err != nil {
				return nil, err
			}
			constantPool[index] = JVMDoubleInfo{
				Value: math.Float64frombits(binary.BigEndian.Uint64(valueBytes)),
			}
			advance = 2
		case constantClass:
			nameIndex, err := reader.u2()
			if err != nil {
				return nil, err
			}
			constantPool[index] = JVMClassInfo{NameIndex: nameIndex}
		case constantString:
			stringIndex, err := reader.u2()
			if err != nil {
				return nil, err
			}
			constantPool[index] = JVMStringInfo{StringIndex: stringIndex}
		case constantNameAndType:
			nameIndex, err := reader.u2()
			if err != nil {
				return nil, err
			}
			descriptorIndex, err := reader.u2()
			if err != nil {
				return nil, err
			}
			constantPool[index] = JVMNameAndTypeInfo{
				NameIndex:       nameIndex,
				DescriptorIndex: descriptorIndex,
			}
		case constantFieldref:
			classIndex, err := reader.u2()
			if err != nil {
				return nil, err
			}
			nameAndTypeIndex, err := reader.u2()
			if err != nil {
				return nil, err
			}
			constantPool[index] = JVMFieldrefInfo{
				ClassIndex:       classIndex,
				NameAndTypeIndex: nameAndTypeIndex,
			}
		case constantMethodref:
			classIndex, err := reader.u2()
			if err != nil {
				return nil, err
			}
			nameAndTypeIndex, err := reader.u2()
			if err != nil {
				return nil, err
			}
			constantPool[index] = JVMMethodrefInfo{
				ClassIndex:       classIndex,
				NameAndTypeIndex: nameAndTypeIndex,
			}
		default:
			return nil, classFileError("Unsupported constant-pool tag %d at index %d", tag, index)
		}

		index += advance
	}

	accessFlags, err := reader.u2()
	if err != nil {
		return nil, err
	}
	thisClassIndex, err := reader.u2()
	if err != nil {
		return nil, err
	}
	thisClassName, err := resolveClassName(constantPool, thisClassIndex)
	if err != nil {
		return nil, err
	}
	superClassIndex, err := reader.u2()
	if err != nil {
		return nil, err
	}
	superClassName := ""
	if superClassIndex != 0 {
		superClassName, err = resolveClassName(constantPool, superClassIndex)
		if err != nil {
			return nil, err
		}
	}

	interfacesCount, err := reader.u2()
	if err != nil {
		return nil, err
	}
	for index := uint16(0); index < interfacesCount; index++ {
		if _, err := reader.u2(); err != nil {
			return nil, err
		}
	}

	fieldsCount, err := reader.u2()
	if err != nil {
		return nil, err
	}
	for index := uint16(0); index < fieldsCount; index++ {
		if err := skipMember(reader); err != nil {
			return nil, err
		}
	}

	methodsCount, err := reader.u2()
	if err != nil {
		return nil, err
	}
	methods := make([]JVMMethodInfo, 0, methodsCount)
	for index := uint16(0); index < methodsCount; index++ {
		method, err := parseMethod(reader, constantPool)
		if err != nil {
			return nil, err
		}
		methods = append(methods, method)
	}

	classAttributesCount, err := reader.u2()
	if err != nil {
		return nil, err
	}
	for index := uint16(0); index < classAttributesCount; index++ {
		if _, err := parseAttribute(reader, constantPool, false); err != nil {
			return nil, err
		}
	}

	if reader.remaining() != 0 {
		return nil, classFileError("Trailing bytes after class-file parse: %d", reader.remaining())
	}

	return &JVMClassFile{
		Version: JVMClassVersion{
			Major: major,
			Minor: minor,
		},
		AccessFlags:    accessFlags,
		ThisClassName:  thisClassName,
		SuperClassName: superClassName,
		ConstantPool:   constantPool,
		Methods:        methods,
	}, nil
}

func resolveClassName(constantPool []JVMConstantPoolEntry, classIndex uint16) (string, error) {
	if classIndex == 0 || int(classIndex) >= len(constantPool) {
		return "", classFileError("Class constant-pool index %d is out of range", classIndex)
	}
	entry, ok := constantPool[classIndex].(JVMClassInfo)
	if !ok {
		return "", classFileError("Constant pool entry %d is not a Class entry", classIndex)
	}
	return getUTF8(constantPool, entry.NameIndex)
}

func parseMethod(reader *classReader, constantPool []JVMConstantPoolEntry) (JVMMethodInfo, error) {
	accessFlags, err := reader.u2()
	if err != nil {
		return JVMMethodInfo{}, err
	}
	nameIndex, err := reader.u2()
	if err != nil {
		return JVMMethodInfo{}, err
	}
	name, err := getUTF8(constantPool, nameIndex)
	if err != nil {
		return JVMMethodInfo{}, err
	}
	descriptorIndex, err := reader.u2()
	if err != nil {
		return JVMMethodInfo{}, err
	}
	descriptor, err := getUTF8(constantPool, descriptorIndex)
	if err != nil {
		return JVMMethodInfo{}, err
	}
	attributesCount, err := reader.u2()
	if err != nil {
		return JVMMethodInfo{}, err
	}
	attributes := make([]JVMMethodAttribute, 0, attributesCount)
	for index := uint16(0); index < attributesCount; index++ {
		attribute, err := parseAttribute(reader, constantPool, true)
		if err != nil {
			return JVMMethodInfo{}, err
		}
		attributes = append(attributes, attribute)
	}
	return JVMMethodInfo{
		AccessFlags: accessFlags,
		Name:        name,
		Descriptor:  descriptor,
		Attributes:  attributes,
	}, nil
}

func parseAttribute(
	reader *classReader,
	constantPool []JVMConstantPoolEntry,
	allowCode bool,
) (JVMMethodAttribute, error) {
	nameIndex, err := reader.u2()
	if err != nil {
		return nil, err
	}
	name, err := getUTF8(constantPool, nameIndex)
	if err != nil {
		return nil, err
	}
	length, err := reader.u4()
	if err != nil {
		return nil, err
	}
	payload, err := readU4Payload(reader, length)
	if err != nil {
		return nil, err
	}

	if name != "Code" || !allowCode {
		return JVMAttributeInfo{Name: name, Info: payload}, nil
	}

	codeReader := &classReader{data: payload}
	maxStack, err := codeReader.u2()
	if err != nil {
		return nil, err
	}
	maxLocals, err := codeReader.u2()
	if err != nil {
		return nil, err
	}
	codeLength, err := codeReader.u4()
	if err != nil {
		return nil, err
	}
	code, err := readU4Payload(codeReader, codeLength)
	if err != nil {
		return nil, err
	}

	exceptionTableLength, err := codeReader.u2()
	if err != nil {
		return nil, err
	}
	for index := uint16(0); index < exceptionTableLength; index++ {
		if _, err := codeReader.read(8); err != nil {
			return nil, err
		}
	}

	nestedAttributesCount, err := codeReader.u2()
	if err != nil {
		return nil, err
	}
	nestedAttributes := make([]JVMAttributeInfo, 0, nestedAttributesCount)
	for index := uint16(0); index < nestedAttributesCount; index++ {
		attribute, err := parseAttribute(codeReader, constantPool, false)
		if err != nil {
			return nil, err
		}
		rawAttribute, ok := attribute.(JVMAttributeInfo)
		if ok {
			nestedAttributes = append(nestedAttributes, rawAttribute)
		}
	}

	if codeReader.remaining() != 0 {
		return nil, classFileError("Code attribute contained trailing bytes after parsing")
	}

	return JVMCodeAttribute{
		Name:             name,
		MaxStack:         maxStack,
		MaxLocals:        maxLocals,
		Code:             code,
		NestedAttributes: nestedAttributes,
	}, nil
}

func getUTF8(constantPool []JVMConstantPoolEntry, index uint16) (string, error) {
	if index == 0 || int(index) >= len(constantPool) {
		return "", classFileError("UTF-8 constant-pool index %d is out of range", index)
	}
	entry, ok := constantPool[index].(JVMUtf8Info)
	if !ok {
		return "", classFileError("Constant pool entry %d is not UTF-8", index)
	}
	return entry.Value, nil
}

func skipMember(reader *classReader) error {
	if _, err := reader.u2(); err != nil {
		return err
	}
	if _, err := reader.u2(); err != nil {
		return err
	}
	if _, err := reader.u2(); err != nil {
		return err
	}
	attributesCount, err := reader.u2()
	if err != nil {
		return err
	}
	for index := uint16(0); index < attributesCount; index++ {
		if _, err := reader.u2(); err != nil {
			return err
		}
		attributeLength, err := reader.u4()
		if err != nil {
			return err
		}
		if _, err := readU4Payload(reader, attributeLength); err != nil {
			return err
		}
	}
	return nil
}

type classReader struct {
	data   []byte
	offset int
}

func (r *classReader) remaining() int {
	return len(r.data) - r.offset
}

func (r *classReader) read(length int) ([]byte, error) {
	if length < 0 || length > r.remaining() {
		return nil, classFileError("Unexpected end of class-file data")
	}
	end := r.offset + length
	chunk := r.data[r.offset:end]
	r.offset = end
	return chunk, nil
}

func (r *classReader) u1() (uint8, error) {
	bytesValue, err := r.read(1)
	if err != nil {
		return 0, err
	}
	return bytesValue[0], nil
}

func (r *classReader) u2() (uint16, error) {
	bytesValue, err := r.read(2)
	if err != nil {
		return 0, err
	}
	return binary.BigEndian.Uint16(bytesValue), nil
}

func (r *classReader) u4() (uint32, error) {
	bytesValue, err := r.read(4)
	if err != nil {
		return 0, err
	}
	return binary.BigEndian.Uint32(bytesValue), nil
}

func classFileError(format string, args ...any) error {
	return ClassFileFormatError{Message: fmt.Sprintf(format, args...)}
}

func readU4Payload(reader *classReader, length uint32) ([]byte, error) {
	maxInt := ^uint(0) >> 1
	if uint64(length) > uint64(maxInt) {
		return nil, classFileError("Class-file payload length %d exceeds host int capacity", length)
	}
	return reader.read(int(length))
}

func appendU2(buffer []byte, value uint16) []byte {
	var storage [2]byte
	binary.BigEndian.PutUint16(storage[:], value)
	return append(buffer, storage[:]...)
}

func appendU4(buffer []byte, value uint32) []byte {
	var storage [4]byte
	binary.BigEndian.PutUint32(storage[:], value)
	return append(buffer, storage[:]...)
}

func appendI4(buffer []byte, value int32) []byte {
	return appendU4(buffer, uint32(value))
}

// A tiny helper used by tests to craft one-off malformed class files without
// pulling binary details into every assertion.
func cloneBytes(input []byte) []byte {
	return bytes.Clone(input)
}
