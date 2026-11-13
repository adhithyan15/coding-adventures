package recfmt

// Decoding and canonical encoding.  Everything here works on bytes or an
// io.Reader; nothing touches the disk.

import (
	"bufio"
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"reflect"
	"strings"
	"unicode/utf8"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
)

// Limits are the size caps of records.jsonl (KILN04 section 7.1): a line over
// MaxLine, or a file over MaxFile, is refused, never truncated.
type Limits struct {
	MaxLine int64
	MaxFile int64
}

// DefaultLimits are the caps the shard recorder also applies.
var DefaultLimits = Limits{MaxLine: 256 << 20, MaxFile: 2 << 30}

// ---------------------------------------------------------------------------
// Strict decoding.

// decodeStrict is legacy.PolicyDecode plus two more refusals:
//
//   - every key not tagged omitempty must be PRESENT (Go would leave an
//     absent one zero, and a missing observation list reads as "observed
//     nothing");
//   - a slice may not be null, for the same reason.
func decodeStrict(data []byte, v any) error {
	// One canonical spelling: valid UTF-8 (the decoder would turn a bad byte
	// into U+FFFD and the merge would then see a path the shard never wrote),
	// and no whitespace before or after the value (BOM, CR, padding).
	if !utf8.Valid(data) {
		return fmt.Errorf("not valid UTF-8")
	}
	if n := len(data); n > 0 && (data[0] <= ' ' || data[n-1] <= ' ') {
		return fmt.Errorf("leading or trailing whitespace")
	}
	if err := legacy.PolicyDecode(data, v); err != nil {
		return err
	}
	return requireKeys(data, reflect.TypeOf(v))
}

func requireKeys(raw []byte, t reflect.Type) error {
	for t.Kind() == reflect.Pointer {
		t = t.Elem()
	}
	raw = bytes.TrimSpace(raw)
	switch t.Kind() {
	case reflect.Struct:
		var members map[string]json.RawMessage
		if err := json.Unmarshal(raw, &members); err != nil || members == nil {
			return fmt.Errorf("expected an object")
		}
		for i := 0; i < t.NumField(); i++ {
			f := t.Field(i)
			if !f.IsExported() || f.Tag.Get("json") == "-" {
				continue
			}
			name, opts, _ := strings.Cut(f.Tag.Get("json"), ",")
			if name == "" {
				name = f.Name
			}
			val, ok := members[name]
			if ok && string(bytes.TrimSpace(val)) == "null" && strings.Contains(opts, "omitempty") {
				return fmt.Errorf("%s: null is not a value (leave the key out)", name)
			}
			if !ok {
				if strings.Contains(opts, "omitempty") {
					continue
				}
				return fmt.Errorf("required key %q is missing", name)
			}
			if err := requireKeys(val, f.Type); err != nil {
				return fmt.Errorf("%s: %w", name, err)
			}
		}
	case reflect.Slice:
		if string(raw) == "null" {
			return fmt.Errorf("a list may not be null")
		}
		if t.Elem().Kind() == reflect.Struct {
			var items []json.RawMessage
			if err := json.Unmarshal(raw, &items); err != nil {
				return err
			}
			for _, it := range items {
				if err := requireKeys(it, t.Elem()); err != nil {
					return err
				}
			}
		}
	}
	return nil
}

// DecodeBinding strictly decodes and validates pre/binding.json.
func DecodeBinding(data []byte) (Binding, error) {
	var b Binding
	if err := decodeStrict(data, &b); err != nil {
		return Binding{}, fmt.Errorf("binding: %w", err)
	}
	return b, b.Validate()
}

// DecodeHostFacts strictly decodes and validates pre/host-facts.json.
func DecodeHostFacts(data []byte) (HostFacts, error) {
	var f HostFacts
	if err := decodeStrict(data, &f); err != nil {
		return HostFacts{}, fmt.Errorf("host facts: %w", err)
	}
	return f, f.Validate()
}

// DecodeInventory strictly decodes and validates pre/inventory.json.
func DecodeInventory(data []byte) (Inventory, error) {
	var i Inventory
	if err := decodeStrict(data, &i); err != nil {
		return Inventory{}, fmt.Errorf("inventory: %w", err)
	}
	return i, i.Validate()
}

// DecodeHeader strictly decodes and validates one header line.
func DecodeHeader(line []byte) (ShardHeader, error) {
	var h ShardHeader
	if err := decodeStrict(line, &h); err != nil {
		return ShardHeader{}, fmt.Errorf("shard header: %w", err)
	}
	return h, h.Validate()
}

// DecodeRecord strictly decodes and validates one record line.
func DecodeRecord(line []byte) (ShardRecord, error) {
	var r ShardRecord
	if err := decodeStrict(line, &r); err != nil {
		return ShardRecord{}, fmt.Errorf("record: %w", err)
	}
	return r, r.Validate()
}

// ReadLines calls fn for every line of r (newline stripped), refusing an empty
// line, a line over lim.MaxLine and a stream over lim.MaxFile.  The line
// buffer is never allowed to grow past the cap, so a hostile unterminated
// stream costs at most MaxLine bytes.  The final newline is optional.
func ReadLines(r io.Reader, lim Limits, fn func(line []byte) error) error {
	br := bufio.NewReaderSize(r, 1<<16)
	var total int64
	var line []byte
	for n := 1; ; n++ {
		line = line[:0]
		for {
			chunk, err := br.ReadSlice('\n')
			total += int64(len(chunk))
			if total > lim.MaxFile {
				return fmt.Errorf("records file is over %d bytes", lim.MaxFile)
			}
			if int64(len(line)+len(chunk)) > lim.MaxLine+1 {
				return fmt.Errorf("line %d is over %d bytes", n, lim.MaxLine)
			}
			line = append(line, chunk...)
			if errors.Is(err, bufio.ErrBufferFull) {
				continue
			}
			if err != nil && err != io.EOF {
				return err
			}
			if err == io.EOF && len(line) == 0 {
				return nil
			}
			break
		}
		line = bytes.TrimSuffix(line, []byte("\n"))
		if len(line) == 0 {
			return fmt.Errorf("line %d is empty", n)
		}
		if int64(len(line)) > lim.MaxLine {
			return fmt.Errorf("line %d is over %d bytes", n, lim.MaxLine)
		}
		if err := fn(line); err != nil {
			return fmt.Errorf("line %d: %w", n, err)
		}
	}
}

// ShardFiles is the raw content of one shard artifact.
type ShardFiles struct {
	Binding, HostFacts, Inventory []byte
	Records                       io.Reader
}

func sum(b []byte) string {
	d := sha256.Sum256(b)
	return hex.EncodeToString(d[:])
}

// DecodeShard decodes a whole shard artifact.  artifactIndex is the index the
// CALLER read from the artifact's name.  The hashes of the binding, host facts
// and inventory files are computed here, from the exact bytes, so the binding
// check in recmerge compares against what was really uploaded.
func DecodeShard(artifactIndex int, f ShardFiles, lim Limits) (*Shard, error) {
	s := &Shard{ArtifactIndex: artifactIndex}
	var err error
	if s.Binding, err = DecodeBinding(f.Binding); err != nil {
		return nil, err
	}
	s.BindingID = sum(f.Binding)
	if s.Facts, err = DecodeHostFacts(f.HostFacts); err != nil {
		return nil, err
	}
	s.HostFactsSHA256 = sum(f.HostFacts)
	if s.Inventory, err = DecodeInventory(f.Inventory); err != nil {
		return nil, err
	}
	s.InventorySHA256 = sum(f.Inventory)
	if f.Records == nil {
		return nil, fmt.Errorf("records: no stream")
	}
	first := true
	err = ReadLines(f.Records, lim, func(line []byte) error {
		if first {
			first = false
			h, err := DecodeHeader(line)
			s.Header = h
			return err
		}
		r, err := DecodeRecord(line)
		if err != nil {
			return err
		}
		s.Records = append(s.Records, r)
		return nil
	})
	if err != nil {
		return nil, fmt.Errorf("records: %w", err)
	}
	if first {
		return nil, fmt.Errorf("records: no header line")
	}
	return s, nil
}

// ---------------------------------------------------------------------------
// Canonical encoding.  A shard writes these; tests use them to build files.

func marshal(v any) ([]byte, error) {
	var buf bytes.Buffer
	enc := json.NewEncoder(&buf)
	enc.SetEscapeHTML(false)
	if err := enc.Encode(v); err != nil {
		return nil, err
	}
	return bytes.TrimSuffix(buf.Bytes(), []byte("\n")), nil
}

// EncodeBinding validates and encodes a binding (no trailing newline).
func EncodeBinding(b Binding) ([]byte, error) {
	if err := b.Validate(); err != nil {
		return nil, err
	}
	return marshal(b)
}

// EncodeHostFacts validates and encodes host facts.
func EncodeHostFacts(f HostFacts) ([]byte, error) {
	if f.ExecutorArgs == nil {
		f.ExecutorArgs = []string{}
	}
	if f.Env == nil {
		f.Env = []string{}
	}
	if err := f.Validate(); err != nil {
		return nil, err
	}
	return marshal(f)
}

// EncodeInventory validates and encodes an inventory.
func EncodeInventory(i Inventory) ([]byte, error) {
	if i.Paths == nil {
		i.Paths = []string{}
	}
	if err := i.Validate(); err != nil {
		return nil, err
	}
	return marshal(i)
}

// EncodeHeader validates and encodes a header line.
func EncodeHeader(h ShardHeader) ([]byte, error) {
	if h.Void == nil {
		h.Void = []string{}
	}
	if err := h.Validate(); err != nil {
		return nil, err
	}
	return marshal(h)
}

// EncodeRecord validates and encodes a record line.  Nil lists become empty
// lists, because the decoder refuses null.
func EncodeRecord(r ShardRecord) ([]byte, error) {
	r.Record = Normalize(r.Record)
	if r.Steps == nil {
		r.Steps = []Step{}
	}
	if err := r.Validate(); err != nil {
		return nil, err
	}
	return marshal(r)
}

// Normalize replaces nil lists by empty ones and returns the record.
func Normalize(r RawRecord) RawRecord {
	for _, p := range []*[]string{&r.Entries, &r.Writes, &r.TrackedWrites, &r.AbsentObservations,
		&r.Binds, &r.UntrackedObservations, &r.Traversed, &r.Reasons} {
		if *p == nil {
			*p = []string{}
		}
	}
	return r
}

// EncodeShardRecords writes header and records as one records.jsonl stream.
func EncodeShardRecords(h ShardHeader, recs []ShardRecord) ([]byte, error) {
	var out bytes.Buffer
	line, err := EncodeHeader(h)
	if err != nil {
		return nil, err
	}
	out.Write(line)
	out.WriteByte('\n')
	for _, r := range recs {
		if line, err = EncodeRecord(r); err != nil {
			return nil, err
		}
		out.Write(line)
		out.WriteByte('\n')
	}
	return out.Bytes(), nil
}
