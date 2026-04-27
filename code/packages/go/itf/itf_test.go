package itf

import "testing"

func TestITF(t *testing.T) {
	if _, err := NormalizeITF("12345"); err == nil {
		t.Fatal("expected odd-length input to fail")
	}
	encoded, err := EncodeITF("123456")
	if err != nil {
		t.Fatal(err)
	}
	if len(encoded) != 3 || encoded[0].Pair != "12" {
		t.Fatalf("unexpected encoding %#v", encoded)
	}
	runs, err := ExpandITFRuns("123456")
	if err != nil {
		t.Fatal(err)
	}
	if runs[0].Role != "start" || runs[len(runs)-1].Role != "stop" {
		t.Fatalf("unexpected edge roles %#v %#v", runs[0], runs[len(runs)-1])
	}
}
