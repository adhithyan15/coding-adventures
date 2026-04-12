package datastore

import (
	"testing"

	datastoreprotocol "github.com/adhithyan15/coding-adventures/code/packages/go/in-memory-data-store-protocol"
	resp "github.com/adhithyan15/coding-adventures/code/packages/go/resp-protocol"
)

func TestDataStorePipeline(t *testing.T) {
	store := New()
	frame := datastoreprotocol.NewCommandFrame("PING", nil)
	if got := store.ExecuteFrame(frame); got.Kind != resp.SimpleStringKind || got.Text != "PONG" {
		t.Fatalf("unexpected ping response: %#v", got)
	}
}
