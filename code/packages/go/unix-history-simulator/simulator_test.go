package unixhistory

import (
	"encoding/json"
	"os"
	"reflect"
	"testing"
)

func TestSharedTraces(t *testing.T) {
	data, err := os.ReadFile("../../../specs/fixtures/historical-unix-v1/file-read.json")
	if err != nil {
		t.Fatal(err)
	}
	var cases []struct {
		Command string
		Result
	}
	if err := json.Unmarshal(data, &cases); err != nil {
		t.Fatal(err)
	}
	for _, fixture := range cases {
		t.Run(fixture.Command, func(t *testing.T) {
			output, err := json.Marshal(RunCommand(fixture.Command))
			if err != nil {
				t.Fatal(err)
			}
			var actual Result
			if err := json.Unmarshal(output, &actual); err != nil {
				t.Fatal(err)
			}
			if !reflect.DeepEqual(actual, fixture.Result) {
				t.Fatalf("got %s, want %+v", output, fixture.Result)
			}
		})
	}
}

func TestIndependentOffsets(t *testing.T) {
	table := NewDescriptorTable(2)
	first, ok := table.Open(2)
	if !ok || first != 0 {
		t.Fatalf("first open: %d, %v", first, ok)
	}
	second, ok := table.Open(2)
	if !ok || second != 1 {
		t.Fatalf("second open: %d, %v", second, ok)
	}
	table.Advance(first, 4)
	other, _ := table.Get(second)
	if other.Offset != 0 || table.Close(-1) || !table.Close(first) {
		t.Fatal("unexpected offset or close result")
	}
	fd, ok := table.Open(3)
	slot, _ := table.Get(fd)
	if !ok || fd != 0 || slot != (Descriptor{Inode: 3}) {
		t.Fatal("open did not reuse and reset the lowest free slot")
	}
}
