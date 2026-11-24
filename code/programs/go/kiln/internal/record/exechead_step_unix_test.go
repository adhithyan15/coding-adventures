//go:build unix

package record

// The recorder's side of the exec-head sidecar (KILN04 D48, step 4): what the
// step sidecar says about the file, and what `collect` does with it.  The
// supervisor itself is tested against a real kernel in the root package
// (TestSandboxExecHead*) and its format in internal/exechead.

import (
	"os"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/exechead"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

func TestExecHeadHeadLimitIsTheClassifiers(t *testing.T) {
	if exechead.HeadLimit != traced.HeadLimit {
		t.Fatalf("the sidecar carries %d bytes of head, the classifier reads %d", exechead.HeadLimit, traced.HeadLimit)
	}
}

func execHeadSidecar() StepSidecar {
	return StepSidecar{
		Schema: StepSchema, Action: "code/p", Step: 0, StepCount: 1, Seq: 7, BindingID: strings.Repeat("a", 64),
		LineSHA256: strings.Repeat("b", 64), Started: true, StartedAt: "2026-11-13T00:00:00Z", EndedAt: "2026-11-13T00:00:01Z",
		TraceSHA256: strings.Repeat("c", 64), StepNetNS: true, Clean: true, StraceStdoutEmpty: true, StraceStderrEmpty: true,
	}
}

func TestStepSidecarSchemaIs3AndCarriesTheExecHeadFields(t *testing.T) {
	if StepSchema != "kiln-record-step/3" {
		t.Fatalf("schema %q", StepSchema)
	}
	good := execHeadSidecar()
	good.ExecHeadNonce, good.ExecHeadSHA256, good.ExecHeadBytes = strings.Repeat("1", 32), strings.Repeat("d", 64), 117
	data, err := EncodeStepSidecar(good)
	if err != nil {
		t.Fatal(err)
	}
	for _, want := range []string{`"exechead_nonce":"` + strings.Repeat("1", 32) + `"`, `"exechead_bytes":117`, `"exechead_sha256":"` + strings.Repeat("d", 64) + `"`} {
		if !strings.Contains(string(data), want) {
			t.Errorf("the sidecar lacks %s: %s", want, data)
		}
	}
	back, err := DecodeStepSidecar(data)
	if err != nil || back.ExecHeadNonce != good.ExecHeadNonce || back.ExecHeadBytes != 117 {
		t.Fatalf("%+v %v", back, err)
	}
	// No supervisor: all three empty, and that is a valid sidecar.
	none, err := EncodeStepSidecar(execHeadSidecar())
	if err != nil || !strings.Contains(string(none), `"exechead_nonce":""`) {
		t.Fatalf("%s %v", none, err)
	}
	// A /2 sidecar is refused: the schema string differs and the keys are missing.
	if _, err := DecodeStepSidecar([]byte(strings.Replace(string(none), "step/3", "step/2", 1))); err == nil {
		t.Fatal("a schema/2 sidecar was decoded")
	}
}

func TestStepSidecarExecHeadFieldsAreValidated(t *testing.T) {
	nonce, sum := strings.Repeat("1", 32), strings.Repeat("d", 64)
	for name, mut := range map[string]func(*StepSidecar){
		"a digest without a nonce": func(s *StepSidecar) { s.ExecHeadSHA256 = sum },
		"a length without a nonce": func(s *StepSidecar) { s.ExecHeadBytes = 5 },
		"a short nonce":            func(s *StepSidecar) { s.ExecHeadNonce, s.ExecHeadSHA256, s.ExecHeadBytes = "11", sum, 5 },
		"an upper-case nonce": func(s *StepSidecar) {
			s.ExecHeadNonce, s.ExecHeadSHA256, s.ExecHeadBytes = strings.ToUpper(strings.Repeat("a", 32)), sum, 5
		},
		"a nonce without a digest": func(s *StepSidecar) { s.ExecHeadNonce, s.ExecHeadBytes = nonce, 5 },
		"a bad digest":             func(s *StepSidecar) { s.ExecHeadNonce, s.ExecHeadSHA256, s.ExecHeadBytes = nonce, "zz", 5 },
		"a nonce with no length":   func(s *StepSidecar) { s.ExecHeadNonce, s.ExecHeadSHA256 = nonce, sum },
		"a negative length":        func(s *StepSidecar) { s.ExecHeadNonce, s.ExecHeadSHA256, s.ExecHeadBytes = nonce, sum, -1 },
	} {
		s := execHeadSidecar()
		mut(&s)
		if _, err := EncodeStepSidecar(s); err == nil {
			t.Errorf("%s: encoded", name)
		}
	}
}

// `collect` accepts the supervisor's file as a regular file and never reads it
// (the classifier does not consume it until step 5); anything else of that name
// is not the supervisor's work.
func TestCollectAcceptsTheExecHeadFileAndDoesNotReadIt(t *testing.T) {
	f := newCfix(t)
	f.good()
	// Unreadable on purpose: if collect opened it, the shard would not be clean.
	if err := os.WriteFile(StepFile(f.p.dir, ca, 0, "exechead"), []byte("not even JSON\n"), 0o000); err != nil {
		t.Fatal(err)
	}
	_, sh := f.collect()
	requireKnown(t, sh, ca)
}

func TestCollectRefusesAnExecHeadThatIsNotARegularFile(t *testing.T) {
	f := newCfix(t)
	f.good()
	if err := os.Mkdir(StepFile(f.p.dir, ca, 0, "exechead"), 0o700); err != nil {
		t.Fatal(err)
	}
	_, sh := f.collect()
	requireUnknown(t, sh, ca, "not a regular file")
}
