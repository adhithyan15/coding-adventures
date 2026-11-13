package traced

import (
	"debug/elf"
	"encoding/binary"
	"strings"
	"testing"
)

// FuzzELFInterpreter feeds raw bytes to the ELF program-header reader.  It must
// never panic, and an interpreter it returns must be an absolute path with no
// NUL inside the input's bounds.
func FuzzELFInterpreter(f *testing.F) {
	f.Add(buildELF(loader, elf.ELFCLASS64, binary.LittleEndian))
	f.Add(buildELF(loader, elf.ELFCLASS32, binary.BigEndian))
	f.Add(buildELFStatic())
	f.Add([]byte("\x7fELF"))
	f.Add([]byte{})
	f.Fuzz(func(t *testing.T, data []byte) {
		interp, ok := elfInterpreter(data)
		if !ok {
			return
		}
		if interp != "" && (!strings.HasPrefix(interp, "/") || strings.IndexByte(interp, 0) >= 0) {
			t.Fatalf("bad interpreter %q", interp)
		}
		if interp != "" && !strings.Contains(string(data), interp) {
			t.Fatalf("interpreter %q is not in the bytes", interp)
		}
	})
}

// FuzzShebang feeds raw bytes to the "#!" line reader.  It must never panic,
// and an interpreter it returns is non-empty, has no blank or NUL, and comes
// from the first 255 bytes of the first line.
func FuzzShebang(f *testing.F) {
	f.Add([]byte("#!/bin/sh\necho"))
	f.Add([]byte("#! /usr/bin/env python3 -u\n"))
	f.Add([]byte("#!"))
	f.Add([]byte("#!\t\t\n"))
	f.Fuzz(func(t *testing.T, data []byte) {
		if len(data) < 2 || string(data[:2]) != "#!" {
			return
		}
		s, ok := shebangInterpreter(data)
		if !ok {
			return
		}
		if s == "" || strings.ContainsAny(s, " \t\n\x00") || len(s) > 253 {
			t.Fatalf("bad interpreter %q", s)
		}
		if !strings.Contains(string(data[:min(len(data), 255)]), s) {
			t.Fatalf("interpreter %q is not in the first 255 bytes", s)
		}
	})
}
