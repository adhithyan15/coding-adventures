package warrantcorpus

import (
	"bytes"
	"testing"
)

func TestDomainSizesMatchTheirEnumeration(t *testing.T) {
	for _, name := range []string{"bool", "u8_all", "u64_edge", "nat_0_64", "bytes_d1",
		"bytes_n(0)", "bytes_n(1)", "bytes_n(2)", "bytes_n(3)", "bytes_n(4)", "bytes_n(16)", "bytes_n(100)"} {
		size, typeText := domainShape(name)
		domain := domainValues(name)
		if len(domain.Values) != size || domain.Type != typeText {
			t.Errorf("%s: shape (%d, %s), enumeration (%d, %s)", name, size, typeText, len(domain.Values), domain.Type)
		}
	}
}

func TestDomainContents(t *testing.T) {
	u64 := domainValues("u64_edge").Values
	if len(u64) != 377 || u64[0] != uint64(0) || u64[255] != uint64(255) || u64[256] != uint64(1)<<8 || u64[311] != uint64(1)<<63 {
		t.Fatalf("u64_edge head: %v %v %v", u64[0], u64[256], u64[311])
	}
	if u64[312] != ^uint64(0) || u64[313] != ^uint64(0)^1 || u64[376] != ^uint64(0)^(uint64(1)<<63) {
		t.Fatal("u64_edge tail")
	}
	seen := map[uint64]bool{}
	for _, v := range u64 {
		seen[v.(uint64)] = true
	}
	if len(seen) != 377 {
		t.Fatalf("u64_edge has %d distinct values, want 377 (MAX and MAX^1 must not collapse)", len(seen))
	}
	d1 := domainValues("bytes_d1").Values
	if len(d1) != 347 || len(d1[0].([]byte)) != 0 || !bytes.Equal(d1[1].([]byte), []byte{0}) || !bytes.Equal(d1[256].([]byte), []byte{255}) {
		t.Fatal("bytes_d1 head")
	}
	if !bytes.Equal(d1[257].([]byte), []byte{0, 0}) || !bytes.Equal(d1[258].([]byte), []byte{0, 1}) || !bytes.Equal(d1[273].([]byte), []byte{0, 0, 0}) {
		t.Fatal("bytes_d1 alphabet order")
	}
	last := d1[346].([]byte)
	if len(last) != 8 || last[0] != 0xa5 {
		t.Fatalf("bytes_d1 tail: %x", last)
	}
	n4 := domainValues("bytes_n(4)").Values
	if !bytes.Equal(n4[0].([]byte), []byte{0, 0, 0, 0}) || !bytes.Equal(n4[3].([]byte), []byte{255, 255, 255, 255}) ||
		!bytes.Equal(n4[4].([]byte), []byte{1, 0, 0, 0}) || !bytes.Equal(n4[6].([]byte), []byte{255, 0, 0, 0}) || !bytes.Equal(n4[7].([]byte), []byte{0, 1, 0, 0}) {
		t.Fatal("bytes_n(4) order")
	}
	if got := domainValues("bytes_n(2)").Values; len(got) != 16 {
		t.Fatalf("bytes_n(2) has %d values", len(got))
	}
}

func TestDomainRefusals(t *testing.T) {
	for _, name := range []string{"", "bytes", "bytes_n()", "bytes_n(x)", "bytes_n(-1)", "u16_all", "bytes_n(4097)", "bytes_n(99999999999999999999)"} {
		name := name
		if code := laneCode(t, func() { domainValues(name) }); code != "unsupported-domain" && code != "descriptor" {
			t.Errorf("domainValues(%q): %q", name, code)
		}
		if code := laneCode(t, func() { domainShape(name) }); code != "unsupported-domain" && code != "descriptor" {
			t.Errorf("domainShape(%q): %q", name, code)
		}
	}
	wantCode(t, "descriptor", func() { domainShape("bytes_n(4097)") })
	wantCode(t, "descriptor", func() { bytesN(4097) })
	if size, _ := domainShape("bytes_n(4096)"); size != 4+3*4096 {
		t.Fatal("4096 is allowed")
	}
}

func TestBinderShape(t *testing.T) {
	name, size, typeText := binderShape(map[string]any{"domain": "bytes_d1", "type": "Bytes"})
	if name != "bytes_d1" || size != 347 || typeText != "Bytes" {
		t.Fatal("binderShape")
	}
	if _, _, got := binderShape(map[string]any{"domain": "bool"}); got != "Bool" {
		t.Fatal("type is optional")
	}
	for _, bad := range []any{
		"bool", nil, map[string]any{}, map[string]any{"domain": int64(1)}, map[string]any{"domain": "bool", "extra": "x"},
		map[string]any{"domain": "bool", "type": "U64"},
	} {
		bad := bad
		if code := laneCode(t, func() { binderShape(bad) }); code != "descriptor" {
			t.Errorf("%v: %q", bad, code)
		}
	}
}

func TestProductOrderAndEarlyStop(t *testing.T) {
	domains := [][]any{{"a", "b"}, {int64(1), int64(2), int64(3)}}
	var got []string
	product(domains, func(env []any) bool {
		got = append(got, env[0].(string)+string(rune('0'+env[1].(int64))))
		return true
	})
	want := []string{"a1", "a2", "a3", "b1", "b2", "b3"} // last binder varies fastest
	if len(got) != len(want) {
		t.Fatalf("%v", got)
	}
	for i := range want {
		if got[i] != want[i] {
			t.Fatalf("%v, want %v", got, want)
		}
	}
	count := 0
	product(domains, func([]any) bool { count++; return count < 4 })
	if count != 4 {
		t.Fatalf("early stop visited %d", count)
	}
	count = 0
	product([][]any{{1}, {}}, func([]any) bool { count++; return true })
	if count != 0 {
		t.Fatal("an empty domain has an empty product")
	}
	count = 0
	product(nil, func([]any) bool { count++; return count < 2 })
	if count != 1 {
		t.Fatalf("no binders is one empty tuple, visited %d", count)
	}
}
