package resp

import "testing"

func TestEncodeDecodeRoundTrip(t *testing.T) {
	values := []Value{
		SimpleString("OK"),
		ErrorValue("ERR boom"),
		Integer(42),
		BulkString([]byte("hello")),
		Array([]Value{BulkString([]byte("PING")), BulkString([]byte("world"))}),
		NullBulkString(),
		NullArray(),
	}

	encoded, err := EncodeMany(values)
	if err != nil {
		t.Fatalf("encode failed: %v", err)
	}

	decoded, err := DecodeAll(encoded)
	if err != nil {
		t.Fatalf("decode failed: %v", err)
	}

	if len(decoded) != len(values) {
		t.Fatalf("expected %d values, got %d", len(values), len(decoded))
	}
	for i := range values {
		if !Equal(values[i], decoded[i]) {
			t.Fatalf("value %d mismatch: want %#v got %#v", i, values[i], decoded[i])
		}
	}
}
