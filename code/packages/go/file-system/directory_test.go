package filesystem

import "testing"

func TestDirectoryEntryCreation(t *testing.T) {
	entry, err := NewDirectoryEntry("hello.txt", 5)
	if err != nil {
		t.Fatal(err)
	}
	if entry.Name != "hello.txt" {
		t.Errorf("expected 'hello.txt', got %q", entry.Name)
	}
	if entry.InodeNumber != 5 {
		t.Errorf("expected 5, got %d", entry.InodeNumber)
	}
}

func TestDirectoryEntryDotEntries(t *testing.T) {
	dot, err := NewDirectoryEntry(".", 0)
	if err != nil {
		t.Fatal(err)
	}
	if dot.Name != "." {
		t.Errorf("expected '.', got %q", dot.Name)
	}

	dotdot, err := NewDirectoryEntry("..", 0)
	if err != nil {
		t.Fatal(err)
	}
	if dotdot.Name != ".." {
		t.Errorf("expected '..', got %q", dotdot.Name)
	}
}

func TestDirectoryEntrySerialize(t *testing.T) {
	entry, _ := NewDirectoryEntry("hello.txt", 5)
	if entry.Serialize() != "hello.txt:5\n" {
		t.Errorf("expected 'hello.txt:5\\n', got %q", entry.Serialize())
	}
}

func TestDeserializeDirectoryEntry(t *testing.T) {
	entry, err := DeserializeDirectoryEntry("hello.txt:5")
	if err != nil {
		t.Fatal(err)
	}
	if entry.Name != "hello.txt" || entry.InodeNumber != 5 {
		t.Errorf("unexpected entry: %+v", entry)
	}
}

func TestDeserializeWithNewline(t *testing.T) {
	entry, err := DeserializeDirectoryEntry("hello.txt:5\n")
	if err != nil {
		t.Fatal(err)
	}
	if entry.Name != "hello.txt" || entry.InodeNumber != 5 {
		t.Errorf("unexpected entry: %+v", entry)
	}
}

func TestSerializeDeserializeRoundtrip(t *testing.T) {
	original, _ := NewDirectoryEntry("notes.txt", 23)
	restored, err := DeserializeDirectoryEntry(original.Serialize())
	if err != nil {
		t.Fatal(err)
	}
	if restored.Name != original.Name || restored.InodeNumber != original.InodeNumber {
		t.Errorf("roundtrip failed: %+v != %+v", restored, original)
	}
}

func TestDirectoryEntryValidation(t *testing.T) {
	_, err := NewDirectoryEntry("", 0)
	if err == nil {
		t.Error("expected error for empty name")
	}

	_, err = NewDirectoryEntry("a/b", 0)
	if err == nil {
		t.Error("expected error for name with /")
	}

	_, err = NewDirectoryEntry("a\x00b", 0)
	if err == nil {
		t.Error("expected error for name with null byte")
	}

	longName := make([]byte, 256)
	for i := range longName {
		longName[i] = 'x'
	}
	_, err = NewDirectoryEntry(string(longName), 0)
	if err == nil {
		t.Error("expected error for name exceeding max length")
	}
}

func TestDirectoryEntryMaxLength(t *testing.T) {
	name := make([]byte, 255)
	for i := range name {
		name[i] = 'x'
	}
	entry, err := NewDirectoryEntry(string(name), 42)
	if err != nil {
		t.Fatal(err)
	}
	if len(entry.Name) != 255 {
		t.Errorf("expected name length 255, got %d", len(entry.Name))
	}
}

func TestDeserializeInvalidFormat(t *testing.T) {
	_, err := DeserializeDirectoryEntry("nocolon")
	if err == nil {
		t.Error("expected error for missing colon")
	}

	_, err = DeserializeDirectoryEntry("")
	if err == nil {
		t.Error("expected error for empty line")
	}

	_, err = DeserializeDirectoryEntry("name:notanumber")
	if err == nil {
		t.Error("expected error for invalid inode number")
	}
}
