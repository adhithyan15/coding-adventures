package jvmclassfile

import (
	"encoding/binary"
	"encoding/hex"
	"math"
	"strings"
	"testing"
)

var helloWorldClassBytes = mustHexBytes(
	"cafebabe00000041001d0a000200030700040c000500060100106a6176612f6c" +
		"616e672f4f626a6563740100063c696e69743e010003282956090008000907000a" +
		"0c000b000c0100106a6176612f6c616e672f53797374656d0100036f7574010015" +
		"4c6a6176612f696f2f5072696e7453747265616d3b08000e01000d48656c6c6f2c" +
		"20776f726c64210a001000110700120c001300140100136a6176612f696f2f5072" +
		"696e7453747265616d0100077072696e746c6e010015284c6a6176612f6c616e672f" +
		"537472696e673b295607001601000a48656c6c6f576f726c64010004436f64650100" +
		"0f4c696e654e756d6265725461626c650100046d61696e010016285b4c6a6176612f" +
		"6c616e672f537472696e673b295601000a536f7572636546696c6501000f48656c6c" +
		"6f576f726c642e6a61766100210015000200000000000200010005000600010017" +
		"0000001d00010001000000052ab70001b100000001001800000006000100000001" +
		"00090019001a00010017000000250002000100000009b20007120db6000fb10000" +
		"000100180000000a000200000003000800040001001b00000002001c",
)

func TestParseMinimalClassFile(t *testing.T) {
	classBytes, err := BuildMinimalClassFile(BuildMinimalClassFileParams{
		ClassName:  "Example",
		MethodName: "compute",
		Descriptor: "()I",
		Code:       []byte{0x04, 0x05, 0x60, 0xAC},
		MaxStack:   2,
		MaxLocals:  0,
		Constants:  []any{int32(300)},
	})
	if err != nil {
		t.Fatalf("build failed: %v", err)
	}

	parsed, err := ParseClassFile(classBytes)
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}

	if got := parsed.Version.String(); got != "61.0" {
		t.Fatalf("version mismatch: got %s", got)
	}
	if parsed.ThisClassName != "Example" {
		t.Fatalf("expected Example, got %s", parsed.ThisClassName)
	}
	if parsed.SuperClassName != "java/lang/Object" {
		t.Fatalf("expected java/lang/Object, got %s", parsed.SuperClassName)
	}

	method := parsed.FindMethod("compute", "()I")
	if method == nil {
		t.Fatal("expected compute method")
	}
	codeAttribute := method.CodeAttribute()
	if codeAttribute == nil {
		t.Fatal("expected Code attribute")
	}
	if codeAttribute.MaxStack != 2 {
		t.Fatalf("expected max_stack=2, got %d", codeAttribute.MaxStack)
	}
	if string(codeAttribute.Code) != string([]byte{0x04, 0x05, 0x60, 0xAC}) {
		t.Fatalf("unexpected method code: %v", codeAttribute.Code)
	}
	if len(parsed.LdcConstants()) == 0 {
		t.Fatal("expected at least one ldc constant")
	}
}

func TestInvalidMagicRaises(t *testing.T) {
	_, err := ParseClassFile([]byte("nope"))
	if err == nil {
		t.Fatal("expected invalid magic error")
	}
	if !strings.Contains(err.Error(), "Invalid class-file magic") {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestResolveStringConstantAndLookupHelpers(t *testing.T) {
	classBytes, err := BuildMinimalClassFile(BuildMinimalClassFileParams{
		ClassName:  "Example",
		MethodName: "compute",
		Descriptor: "()I",
		Code:       []byte{0x04, 0xAC},
		MaxStack:   1,
		MaxLocals:  0,
		Constants:  []any{"hello", int32(7)},
	})
	if err != nil {
		t.Fatalf("build failed: %v", err)
	}

	parsed, err := ParseClassFile(classBytes)
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}

	ldcConstants := parsed.LdcConstants()
	var stringIndex uint16
	for index, value := range ldcConstants {
		if value == "hello" {
			stringIndex = index
		}
	}
	if stringIndex == 0 {
		t.Fatal("expected hello string constant")
	}

	value, err := parsed.ResolveConstant(stringIndex)
	if err != nil {
		t.Fatalf("resolve constant failed: %v", err)
	}
	if value != "hello" {
		t.Fatalf("expected hello, got %#v", value)
	}
	if parsed.FindMethod("missing") != nil {
		t.Fatal("missing method should not resolve")
	}

	var classIndex uint16
	var utf8Index uint16
	for index, entry := range parsed.ConstantPool {
		switch value := entry.(type) {
		case JVMClassInfo:
			if classIndex == 0 {
				classIndex = uint16(index)
			}
		case JVMUtf8Info:
			if value.Value == "Example" {
				utf8Index = uint16(index)
			}
		}
	}

	if _, err := parsed.ResolveConstant(classIndex); err == nil || !strings.Contains(err.Error(), "not a loadable constant") {
		t.Fatalf("expected non-loadable constant error, got %v", err)
	}
	utf8Value, err := parsed.GetUTF8(utf8Index)
	if err != nil {
		t.Fatalf("GetUTF8 failed: %v", err)
	}
	if utf8Value != "Example" {
		t.Fatalf("expected Example, got %s", utf8Value)
	}
}

func TestResolveFieldAndMethodReferencesFromRealClass(t *testing.T) {
	parsed, err := ParseClassFile(helloWorldClassBytes)
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}

	fieldref, err := parsed.ResolveFieldref(7)
	if err != nil {
		t.Fatalf("resolve fieldref failed: %v", err)
	}
	if fieldref != (JVMFieldReference{
		ClassName:  "java/lang/System",
		Name:       "out",
		Descriptor: "Ljava/io/PrintStream;",
	}) {
		t.Fatalf("unexpected fieldref: %#v", fieldref)
	}

	methodref, err := parsed.ResolveMethodref(15)
	if err != nil {
		t.Fatalf("resolve methodref failed: %v", err)
	}
	if methodref != (JVMMethodReference{
		ClassName:  "java/io/PrintStream",
		Name:       "println",
		Descriptor: "(Ljava/lang/String;)V",
	}) {
		t.Fatalf("unexpected methodref: %#v", methodref)
	}

	className, err := parsed.ResolveClassName(21)
	if err != nil {
		t.Fatalf("resolve class name failed: %v", err)
	}
	if className != "HelloWorld" {
		t.Fatalf("expected HelloWorld, got %s", className)
	}

	name, descriptor, err := parsed.ResolveNameAndType(17)
	if err != nil {
		t.Fatalf("resolve name and type failed: %v", err)
	}
	if name != "println" || descriptor != "(Ljava/lang/String;)V" {
		t.Fatalf("unexpected name/descriptor: %s %s", name, descriptor)
	}
}

func TestParseRejectsTrailingBytes(t *testing.T) {
	classBytes, err := BuildMinimalClassFile(BuildMinimalClassFileParams{
		ClassName:  "Example",
		MethodName: "compute",
		Descriptor: "()V",
		Code:       []byte{0xB1},
		MaxStack:   0,
		MaxLocals:  0,
	})
	if err != nil {
		t.Fatalf("build failed: %v", err)
	}

	withTrailingByte := append(cloneBytes(classBytes), 0x00)
	_, err = ParseClassFile(withTrailingByte)
	if err == nil {
		t.Fatal("expected trailing-byte error")
	}
	if !strings.Contains(err.Error(), "Trailing bytes after class-file parse") {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestBuildRejectsUnsupportedConstantType(t *testing.T) {
	_, err := BuildMinimalClassFile(BuildMinimalClassFileParams{
		ClassName:  "Example",
		MethodName: "compute",
		Descriptor: "()V",
		Code:       []byte{0xB1},
		Constants:  []any{true},
	})
	if err == nil {
		t.Fatal("expected unsupported constant error")
	}
	if !strings.Contains(err.Error(), "unsupported minimal class constant type") {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestParseWideConstantsAndLookupErrors(t *testing.T) {
	parsed, err := ParseClassFile(buildWideConstantFixtureClass())
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}

	longValue, err := parsed.ResolveConstant(5)
	if err != nil {
		t.Fatalf("resolve long failed: %v", err)
	}
	if longValue != int64(1234567890123) {
		t.Fatalf("unexpected long value: %#v", longValue)
	}

	doubleValue, err := parsed.ResolveConstant(7)
	if err != nil {
		t.Fatalf("resolve double failed: %v", err)
	}
	if doubleValue != 3.5 {
		t.Fatalf("unexpected double value: %#v", doubleValue)
	}

	if _, err := parsed.ResolveConstant(6); err == nil || !strings.Contains(err.Error(), "reserved wide slot") {
		t.Fatalf("expected reserved-slot error, got %v", err)
	}
	if _, err := parsed.GetUTF8(2); err == nil || !strings.Contains(err.Error(), "not a UTF-8 string") {
		t.Fatalf("expected UTF-8 type error, got %v", err)
	}
	if _, err := parsed.ResolveClassName(1); err == nil || !strings.Contains(err.Error(), "not a Class entry") {
		t.Fatalf("expected class-entry error, got %v", err)
	}
	if _, _, err := parsed.ResolveNameAndType(2); err == nil || !strings.Contains(err.Error(), "not a NameAndType entry") {
		t.Fatalf("expected name-and-type error, got %v", err)
	}
	if parsed.FindMethod("run", "()V") == nil {
		t.Fatal("expected run method")
	}
}

func TestCodeAttributeReturnsNilWhenMissing(t *testing.T) {
	method := JVMMethodInfo{
		Name: "helper",
		Attributes: []JVMMethodAttribute{
			JVMAttributeInfo{Name: "Synthetic", Info: []byte{}},
		},
	}

	if method.CodeAttribute() != nil {
		t.Fatal("expected nil Code attribute")
	}
}

func TestUnsupportedConstantPoolTagRejected(t *testing.T) {
	classBytes, err := BuildMinimalClassFile(BuildMinimalClassFileParams{
		ClassName:  "Example",
		MethodName: "compute",
		Descriptor: "()V",
		Code:       []byte{0xB1},
	})
	if err != nil {
		t.Fatalf("build failed: %v", err)
	}

	mutated := cloneBytes(classBytes)
	mutated[10] = 0x7F
	_, err = ParseClassFile(mutated)
	if err == nil {
		t.Fatal("expected unsupported-tag error")
	}
	if !strings.Contains(err.Error(), "Unsupported constant-pool tag") {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestBuildSupportsGoIntAndRejectsOverflow(t *testing.T) {
	classBytes, err := BuildMinimalClassFile(BuildMinimalClassFileParams{
		ClassName:  "Example",
		MethodName: "compute",
		Descriptor: "()V",
		Code:       []byte{0xB1},
		Constants:  []any{7},
	})
	if err != nil {
		t.Fatalf("build failed: %v", err)
	}

	parsed, err := ParseClassFile(classBytes)
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}
	foundSeven := false
	for _, value := range parsed.LdcConstants() {
		if value == int32(7) {
			foundSeven = true
		}
	}
	if !foundSeven {
		t.Fatal("expected integer constant converted from Go int")
	}

	_, err = BuildMinimalClassFile(BuildMinimalClassFileParams{
		ClassName:  "Example",
		MethodName: "compute",
		Descriptor: "()V",
		Code:       []byte{0xB1},
		Constants:  []any{int(math.MaxInt32) + 1},
	})
	if err == nil || !strings.Contains(err.Error(), "outside JVM Integer range") {
		t.Fatalf("expected integer range error, got %v", err)
	}
}

func TestBuildValidatesRequiredFields(t *testing.T) {
	_, err := BuildMinimalClassFile(BuildMinimalClassFileParams{
		MethodName: "compute",
		Descriptor: "()V",
		Code:       []byte{0xB1},
	})
	if err == nil || !strings.Contains(err.Error(), "class name must not be empty") {
		t.Fatalf("expected missing class-name error, got %v", err)
	}

	_, err = BuildMinimalClassFile(BuildMinimalClassFileParams{
		ClassName:  "Example",
		Descriptor: "()V",
		Code:       []byte{0xB1},
	})
	if err == nil || !strings.Contains(err.Error(), "method name must not be empty") {
		t.Fatalf("expected missing method-name error, got %v", err)
	}

	_, err = BuildMinimalClassFile(BuildMinimalClassFileParams{
		ClassName:  "Example",
		MethodName: "compute",
		Code:       []byte{0xB1},
	})
	if err == nil || !strings.Contains(err.Error(), "descriptor must not be empty") {
		t.Fatalf("expected missing descriptor error, got %v", err)
	}
}

func TestLookupHelpersRejectOutOfRangeAndWrongTypes(t *testing.T) {
	classBytes, err := BuildMinimalClassFile(BuildMinimalClassFileParams{
		ClassName:  "Example",
		MethodName: "compute",
		Descriptor: "()V",
		Code:       []byte{0xB1},
	})
	if err != nil {
		t.Fatalf("build failed: %v", err)
	}

	parsed, err := ParseClassFile(classBytes)
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}

	if _, err := parsed.GetUTF8(99); err == nil || !strings.Contains(err.Error(), "out of range") {
		t.Fatalf("expected out-of-range UTF-8 error, got %v", err)
	}
	if _, err := parsed.ResolveClassName(99); err == nil || !strings.Contains(err.Error(), "out of range") {
		t.Fatalf("expected out-of-range class-name error, got %v", err)
	}
	if _, _, err := parsed.ResolveNameAndType(99); err == nil || !strings.Contains(err.Error(), "out of range") {
		t.Fatalf("expected out-of-range name-and-type error, got %v", err)
	}
	if _, err := parsed.ResolveConstant(99); err == nil || !strings.Contains(err.Error(), "out of range") {
		t.Fatalf("expected out-of-range constant error, got %v", err)
	}
	if _, err := parsed.ResolveFieldref(1); err == nil || !strings.Contains(err.Error(), "not a Fieldref entry") {
		t.Fatalf("expected fieldref type error, got %v", err)
	}
	if _, err := parsed.ResolveMethodref(1); err == nil || !strings.Contains(err.Error(), "not a Methodref entry") {
		t.Fatalf("expected methodref type error, got %v", err)
	}
}

func TestParseRejectsTruncatedClassFile(t *testing.T) {
	classBytes, err := BuildMinimalClassFile(BuildMinimalClassFileParams{
		ClassName:  "Example",
		MethodName: "compute",
		Descriptor: "()V",
		Code:       []byte{0xB1},
	})
	if err != nil {
		t.Fatalf("build failed: %v", err)
	}

	_, err = ParseClassFile(classBytes[:len(classBytes)-1])
	if err == nil {
		t.Fatal("expected truncated-input error")
	}
	if !strings.Contains(err.Error(), "Unexpected end of class-file data") {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestClassReaderRejectsNegativeLengths(t *testing.T) {
	reader := &classReader{data: []byte{0x01, 0x02}}
	_, err := reader.read(-1)
	if err == nil {
		t.Fatal("expected negative-length error")
	}
	if !strings.Contains(err.Error(), "Unexpected end of class-file data") {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestParseTreatsNestedCodeAttributeAsOpaque(t *testing.T) {
	parsed, err := ParseClassFile(buildNestedCodeAttributeFixtureClass())
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}

	method := parsed.FindMethod("run", "()V")
	if method == nil {
		t.Fatal("expected run method")
	}
	codeAttribute := method.CodeAttribute()
	if codeAttribute == nil {
		t.Fatal("expected top-level Code attribute")
	}
	if len(codeAttribute.NestedAttributes) != 1 {
		t.Fatalf("expected one nested attribute, got %d", len(codeAttribute.NestedAttributes))
	}
	if codeAttribute.NestedAttributes[0].Name != "Code" {
		t.Fatalf("expected nested Code attribute to remain opaque, got %s", codeAttribute.NestedAttributes[0].Name)
	}
	if len(codeAttribute.NestedAttributes[0].Info) == 0 {
		t.Fatal("expected opaque nested Code payload bytes")
	}
}

func mustHexBytes(encoded string) []byte {
	decoded, err := hex.DecodeString(encoded)
	if err != nil {
		panic(err)
	}
	return decoded
}

func buildWideConstantFixtureClass() []byte {
	entries := [][]byte{
		utf8Entry("WideExample"),
		classEntry(1),
		utf8Entry("java/lang/Object"),
		classEntry(3),
		longEntry(1234567890123),
		doubleEntry(3.5),
		utf8Entry("VALUE"),
		utf8Entry("I"),
		utf8Entry("run"),
		utf8Entry("()V"),
		utf8Entry("Code"),
	}

	codeAttributeBody := []byte{}
	codeAttributeBody = appendU2(codeAttributeBody, 0)
	codeAttributeBody = appendU2(codeAttributeBody, 0)
	codeAttributeBody = appendU4(codeAttributeBody, 1)
	codeAttributeBody = append(codeAttributeBody, 0xB1)
	codeAttributeBody = appendU2(codeAttributeBody, 0)
	codeAttributeBody = appendU2(codeAttributeBody, 0)

	codeAttribute := []byte{}
	codeAttribute = appendU2(codeAttribute, 13)
	codeAttribute = appendU4(codeAttribute, uint32(len(codeAttributeBody)))
	codeAttribute = append(codeAttribute, codeAttributeBody...)

	fieldInfo := []byte{}
	fieldInfo = appendU2(fieldInfo, ACC_PUBLIC|ACC_STATIC)
	fieldInfo = appendU2(fieldInfo, 9)
	fieldInfo = appendU2(fieldInfo, 10)
	fieldInfo = appendU2(fieldInfo, 0)

	methodInfo := []byte{}
	methodInfo = appendU2(methodInfo, ACC_PUBLIC|ACC_STATIC)
	methodInfo = appendU2(methodInfo, 11)
	methodInfo = appendU2(methodInfo, 12)
	methodInfo = appendU2(methodInfo, 1)
	methodInfo = append(methodInfo, codeAttribute...)

	classBytes := []byte{}
	classBytes = appendU4(classBytes, 0xCAFEBABE)
	classBytes = appendU2(classBytes, 0)
	classBytes = appendU2(classBytes, 61)
	classBytes = appendU2(classBytes, 14)
	for _, entry := range entries {
		classBytes = append(classBytes, entry...)
	}
	classBytes = appendU2(classBytes, ACC_PUBLIC|ACC_SUPER)
	classBytes = appendU2(classBytes, 2)
	classBytes = appendU2(classBytes, 4)
	classBytes = appendU2(classBytes, 0)
	classBytes = appendU2(classBytes, 1)
	classBytes = append(classBytes, fieldInfo...)
	classBytes = appendU2(classBytes, 1)
	classBytes = append(classBytes, methodInfo...)
	classBytes = appendU2(classBytes, 0)
	return classBytes
}

func buildNestedCodeAttributeFixtureClass() []byte {
	entries := [][]byte{
		utf8Entry("NestedCodeExample"),
		classEntry(1),
		utf8Entry("java/lang/Object"),
		classEntry(3),
		utf8Entry("run"),
		utf8Entry("()V"),
		utf8Entry("Code"),
	}

	nestedCodePayload := []byte{}
	nestedCodePayload = appendU2(nestedCodePayload, 0)
	nestedCodePayload = appendU2(nestedCodePayload, 0)
	nestedCodePayload = appendU4(nestedCodePayload, 0)
	nestedCodePayload = appendU2(nestedCodePayload, 0)
	nestedCodePayload = appendU2(nestedCodePayload, 0)

	nestedAttribute := []byte{}
	nestedAttribute = appendU2(nestedAttribute, 7)
	nestedAttribute = appendU4(nestedAttribute, uint32(len(nestedCodePayload)))
	nestedAttribute = append(nestedAttribute, nestedCodePayload...)

	codeAttributeBody := []byte{}
	codeAttributeBody = appendU2(codeAttributeBody, 0)
	codeAttributeBody = appendU2(codeAttributeBody, 0)
	codeAttributeBody = appendU4(codeAttributeBody, 1)
	codeAttributeBody = append(codeAttributeBody, 0xB1)
	codeAttributeBody = appendU2(codeAttributeBody, 0)
	codeAttributeBody = appendU2(codeAttributeBody, 1)
	codeAttributeBody = append(codeAttributeBody, nestedAttribute...)

	codeAttribute := []byte{}
	codeAttribute = appendU2(codeAttribute, 7)
	codeAttribute = appendU4(codeAttribute, uint32(len(codeAttributeBody)))
	codeAttribute = append(codeAttribute, codeAttributeBody...)

	methodInfo := []byte{}
	methodInfo = appendU2(methodInfo, ACC_PUBLIC|ACC_STATIC)
	methodInfo = appendU2(methodInfo, 5)
	methodInfo = appendU2(methodInfo, 6)
	methodInfo = appendU2(methodInfo, 1)
	methodInfo = append(methodInfo, codeAttribute...)

	classBytes := []byte{}
	classBytes = appendU4(classBytes, 0xCAFEBABE)
	classBytes = appendU2(classBytes, 0)
	classBytes = appendU2(classBytes, 61)
	classBytes = appendU2(classBytes, 8)
	for _, entry := range entries {
		classBytes = append(classBytes, entry...)
	}
	classBytes = appendU2(classBytes, ACC_PUBLIC|ACC_SUPER)
	classBytes = appendU2(classBytes, 2)
	classBytes = appendU2(classBytes, 4)
	classBytes = appendU2(classBytes, 0)
	classBytes = appendU2(classBytes, 0)
	classBytes = appendU2(classBytes, 1)
	classBytes = append(classBytes, methodInfo...)
	classBytes = appendU2(classBytes, 0)
	return classBytes
}

func utf8Entry(value string) []byte {
	entry := []byte{constantUTF8}
	entry = appendU2(entry, uint16(len(value)))
	entry = append(entry, []byte(value)...)
	return entry
}

func classEntry(nameIndex uint16) []byte {
	entry := []byte{constantClass}
	return appendU2(entry, nameIndex)
}

func longEntry(value int64) []byte {
	entry := []byte{constantLong}
	var storage [8]byte
	binary.BigEndian.PutUint64(storage[:], uint64(value))
	return append(entry, storage[:]...)
}

func doubleEntry(value float64) []byte {
	entry := []byte{constantDouble}
	var storage [8]byte
	binary.BigEndian.PutUint64(storage[:], math.Float64bits(value))
	return append(entry, storage[:]...)
}
