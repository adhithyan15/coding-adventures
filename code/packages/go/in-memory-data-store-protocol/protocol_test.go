package datastoreprotocol

import (
	"testing"

	resp "github.com/adhithyan15/coding-adventures/code/packages/go/resp-protocol"
)

func TestFrameFromResp(t *testing.T) {
	value := resp.Array([]resp.Value{
		resp.BulkString([]byte("set")),
		resp.BulkString([]byte("k")),
		resp.Integer(1),
	})
	frame, ok := FrameFromResp(value)
	if !ok {
		t.Fatal("expected frame")
	}
	if frame.Command != "SET" || len(frame.Args) != 2 || string(frame.Args[1]) != "1" {
		t.Fatalf("unexpected frame: %#v", frame)
	}
}
