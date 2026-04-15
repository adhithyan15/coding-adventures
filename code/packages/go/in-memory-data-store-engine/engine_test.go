package datastoreengine

import (
	"testing"

	resp "github.com/adhithyan15/coding-adventures/code/packages/go/resp-protocol"
)

func TestStringAndDatabaseCommands(t *testing.T) {
	engine := New()

	if got := engine.ExecuteParts([][]byte{[]byte("PING")}); got.Kind != resp.SimpleStringKind || got.Text != "PONG" {
		t.Fatalf("unexpected ping response: %#v", got)
	}

	if got := engine.ExecuteParts([][]byte{[]byte("SET"), []byte("hello"), []byte("world")}); got.Kind != resp.SimpleStringKind || got.Text != "OK" {
		t.Fatalf("unexpected set response: %#v", got)
	}

	if got := engine.ExecuteParts([][]byte{[]byte("GET"), []byte("hello")}); got.Kind != resp.BulkStringKind || string(got.Bulk) != "world" {
		t.Fatalf("unexpected get response: %#v", got)
	}

	if got := engine.ExecuteParts([][]byte{[]byte("INCRBY"), []byte("counter"), []byte("5")}); got.Kind != resp.IntegerKind || got.Integer != 5 {
		t.Fatalf("unexpected incrby response: %#v", got)
	}

	if got := engine.ExecuteParts([][]byte{[]byte("APPEND"), []byte("hello"), []byte("!")}); got.Kind != resp.IntegerKind || got.Integer != 6 {
		t.Fatalf("unexpected append response: %#v", got)
	}
}

func TestHashSetAndHyperLogLogCommands(t *testing.T) {
	engine := New()

	if got := engine.ExecuteParts([][]byte{
		[]byte("HSET"), []byte("profile"), []byte("name"), []byte("Ada"), []byte("lang"), []byte("Go"),
	}); got.Kind != resp.IntegerKind || got.Integer != 2 {
		t.Fatalf("unexpected hset response: %#v", got)
	}
	if got := engine.ExecuteParts([][]byte{[]byte("HGET"), []byte("profile"), []byte("name")}); got.Kind != resp.BulkStringKind || string(got.Bulk) != "Ada" {
		t.Fatalf("unexpected hget response: %#v", got)
	}
	if got := engine.ExecuteParts([][]byte{[]byte("SADD"), []byte("tags"), []byte("a"), []byte("b"), []byte("a")}); got.Kind != resp.IntegerKind || got.Integer != 2 {
		t.Fatalf("unexpected sadd response: %#v", got)
	}
	if got := engine.ExecuteParts([][]byte{[]byte("SMEMBERS"), []byte("tags")}); got.Kind != resp.ArrayKind || len(got.Array) != 2 {
		t.Fatalf("unexpected smembers response: %#v", got)
	}
	if got := engine.ExecuteParts([][]byte{[]byte("PFADD"), []byte("visitors"), []byte("one"), []byte("two"), []byte("one")}); got.Kind != resp.IntegerKind {
		t.Fatalf("unexpected pfadd response: %#v", got)
	}
	if got := engine.ExecuteParts([][]byte{[]byte("PFCOUNT"), []byte("visitors")}); got.Kind != resp.IntegerKind || got.Integer == 0 {
		t.Fatalf("unexpected pfcount response: %#v", got)
	}
}

func TestSelectAndFlushCommands(t *testing.T) {
	engine := New()
	if got := engine.ExecuteParts([][]byte{[]byte("SELECT"), []byte("3")}); got.Kind != resp.SimpleStringKind || got.Text != "OK" {
		t.Fatalf("unexpected select response: %#v", got)
	}
	if got := engine.ExecuteParts([][]byte{[]byte("SET"), []byte("dbkey"), []byte("value")}); got.Kind != resp.SimpleStringKind {
		t.Fatalf("unexpected set response: %#v", got)
	}
	if got := engine.ExecuteParts([][]byte{[]byte("DBSIZE")}); got.Kind != resp.IntegerKind || got.Integer != 1 {
		t.Fatalf("unexpected dbsize response: %#v", got)
	}
	if got := engine.ExecuteParts([][]byte{[]byte("FLUSHDB")}); got.Kind != resp.SimpleStringKind {
		t.Fatalf("unexpected flushdb response: %#v", got)
	}
	if got := engine.ExecuteParts([][]byte{[]byte("DBSIZE")}); got.Kind != resp.IntegerKind || got.Integer != 0 {
		t.Fatalf("unexpected dbsize after flush: %#v", got)
	}
}
