package csharpparser

import "testing"

func assertParsesCompilationUnit(t *testing.T, source, version string) {
	t.Helper()

	node, err := ParseCSharp(source, version)
	if err != nil {
		t.Fatalf("failed to parse %q with version %q: %v", source, version, err)
	}
	if node.RuleName != "compilation_unit" {
		t.Fatalf("expected compilation_unit at root, got %s", node.RuleName)
	}
}

func TestParseCSharp(t *testing.T) {
	assertParsesCompilationUnit(t, "public class Foo {}", "")
}

func TestParseCSharpClassDeclaration(t *testing.T) {
	assertParsesCompilationUnit(t, "namespace Demo { public class Foo {} }", "")
}

func TestParseCSharpVersions(t *testing.T) {
	cases := []struct {
		name    string
		version string
		source  string
	}{
		{name: "1_0", version: "1.0", source: "public class Foo {}"},
		{name: "2_0", version: "2.0", source: "public class Foo {}"},
		{name: "3_0", version: "3.0", source: "public class Foo {}"},
		{name: "4_0", version: "4.0", source: "public class Foo {}"},
		{name: "5_0", version: "5.0", source: "public class Foo {}"},
		{name: "6_0", version: "6.0", source: "public class Foo {}"},
		{name: "7_0", version: "7.0", source: "public class Foo {}"},
		{name: "8_0", version: "8.0", source: "public class Foo {}"},
		{name: "9_0", version: "9.0", source: "public class Foo {}"},
		{name: "10_0", version: "10.0", source: "public class Foo {}"},
		{name: "11_0", version: "11.0", source: "public class Foo {}"},
		{name: "12_0", version: "12.0", source: "public class Foo {}"},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			assertParsesCompilationUnit(t, tc.source, tc.version)
		})
	}
}

func TestParseCSharpTopLevelStatements(t *testing.T) {
	cases := []struct {
		name    string
		version string
	}{
		{name: "9_0", version: "9.0"},
		{name: "10_0", version: "10.0"},
		{name: "11_0", version: "11.0"},
		{name: "12_0", version: "12.0"},
		{name: "default", version: ""},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			assertParsesCompilationUnit(t, "int x = 1;", tc.version)
		})
	}
}

func TestParseCSharpUnknownVersion(t *testing.T) {
	_, err := ParseCSharp("public class Foo {}", "99")
	if err == nil {
		t.Fatal("expected error for unknown version, got nil")
	}
}

func TestParseCSharpDefaultVersion(t *testing.T) {
	assertParsesCompilationUnit(t, "public class Foo {}", "")
}

func TestNewCSharpParser(t *testing.T) {
	cp, err := NewCSharpParser("public class Foo {}", "12.0")
	if err != nil {
		t.Fatalf("expected no error, got: %v", err)
	}
	if cp == nil {
		t.Fatal("expected non-nil parser")
	}
}
