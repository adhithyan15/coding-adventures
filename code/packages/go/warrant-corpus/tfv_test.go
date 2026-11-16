package warrantcorpus

import (
	"strings"
	"testing"
)

func TestParseTFVRealRecordShapes(t *testing.T) {
	text := "tfv 1\n" +
		"a/b.c@1#2-x R4 Kop S6571 Ka X Kb X00ff Kexpect T\n" +
		"id2 R2 Kn N Kv I-5\n" +
		"id3 R2 Kl L2 I1 F Km M1 S6b X01\n"
	records := parseTFV(text)
	if len(records) != 3 || records[0].ID != "a/b.c@1#2-x" {
		t.Fatalf("records: %+v", records)
	}
	first := records[0].Fields
	if first["op"].Kind != "text" || first["op"].Text != "eq" || len(first["a"].Bytes) != 0 || len(first["b"].Bytes) != 2 || !first["expect"].Bool {
		t.Fatalf("fields: %+v", first)
	}
	second := records[1].Fields
	if second["n"].Kind != "null" || second["v"].Text != "-5" {
		t.Fatalf("fields: %+v", second)
	}
	third := records[2].Fields
	if len(third["l"].List) != 2 || third["l"].List[1].Kind != "bool" || third["l"].List[1].Bool {
		t.Fatalf("list: %+v", third["l"])
	}
	if pairs := third["m"].Pairs; len(pairs) != 1 || pairs[0][0].Text != "k" || pairs[0][1].Bytes[0] != 1 {
		t.Fatalf("map: %+v", third["m"])
	}
}

func TestParseTFVRefusals(t *testing.T) {
	bad := map[string]string{
		"no header":         "id R0\n",
		"wrong header":      "tfv 2\nid R0\n",
		"no final newline":  "tfv 1\nid R0",
		"blank line":        "tfv 1\n\nid R0\n",
		"CR":                "tfv 1\r\nid R0\r\n",
		"short record":      "tfv 1\nid\n",
		"bad id":            "tfv 1\nbad!id R0\n",
		"duplicate id":      "tfv 1\nid R0\nid R0\n",
		"bad count":         "tfv 1\nid X0\n",
		"leading zero":      "tfv 1\nid R01\n",
		"missing field key": "tfv 1\nid R1 Nope\n",
		"bad field name":    "tfv 1\nid R1 KBad N\n",
		"duplicate field":   "tfv 1\nid R2 Ka N Ka N\n",
		"trailing tokens":   "tfv 1\nid R1 Ka N N\n",
		"too few fields":    "tfv 1\nid R2 Ka N\n",
		"value missing":     "tfv 1\nid R1 Ka\n",
		"empty token":       "tfv 1\nid R1 Ka  \n",
		"bad int":           "tfv 1\nid R1 Ka Ix\n",
		"negative zero":     "tfv 1\nid R1 Ka I-0\n",
		"leading zero int":  "tfv 1\nid R1 Ka I01\n",
		"bad bytes":         "tfv 1\nid R1 Ka X0\n",
		"upper hex":         "tfv 1\nid R1 Ka XAB\n",
		"bad text":          "tfv 1\nid R1 Ka Sfffe\n",
		"bad text hex":      "tfv 1\nid R1 Ka Sz\n",
		"unknown kind":      "tfv 1\nid R1 Ka Q\n",
		"null with body":    "tfv 1\nid R1 Ka Nx\n",
		"bad list length":   "tfv 1\nid R1 Ka Lx\n",
		"huge list":         "tfv 1\nid R1 Ka L999 N\n",
		"list too short":    "tfv 1\nid R1 Ka L2 N\n",
		"map too short":     "tfv 1\nid R1 Ka M1 N\n",
	}
	for name, text := range bad {
		text := text
		if code := laneCode(t, func() { parseTFV(text) }); code != "tfv" {
			t.Errorf("%s: code %q", name, code)
		}
	}
}

func TestParseTFVDepthLimit(t *testing.T) {
	deep := func(levels int) string {
		return "tfv 1\nid R1 Ka " + strings.Repeat("L1 ", levels) + "N\n"
	}
	parseTFV(deep(tfvMaxDepth))
	wantCode(t, "tfv", func() { parseTFV(deep(tfvMaxDepth + 2)) })
}
