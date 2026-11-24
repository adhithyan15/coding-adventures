//go:build unix

package record

// The recorder's side of the exec-head sidecar (KILN04 D48, step 4): what the
// step sidecar says about the file, and what `collect` does with it.  The
// supervisor itself is tested against a real kernel in the root package
// (TestSandboxExecHead*) and its format in internal/exechead.

import (
	"crypto/sha256"
	"encoding/hex"
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

// `collect` binds the supervisor's file to the step sidecar (KILN04 D48 step 5):
// the file is read and must be the one the sidecar's nonce, digest and length
// name, whole and strictly decodable, exactly as the epilogue judged it.
func (f *cfix) bindExecHead(data []byte) {
	f.t.Helper()
	if err := os.WriteFile(StepFile(f.p.dir, ca, 0, "exechead"), data, 0o600); err != nil {
		f.t.Fatal(err)
	}
	sum := sha256.Sum256(data)
	f.step(ca, 0, 0, func(sc *StepSidecar) {
		sc.ExecHeadNonce, sc.ExecHeadSHA256, sc.ExecHeadBytes = ehNonce, hex.EncodeToString(sum[:]), int64(len(data))
	})
}

func TestCollectBindsTheExecHeadFileToTheStepSidecar(t *testing.T) {
	good := func(t *testing.T) []byte {
		return ehBytes(t, ehHeader(ehNonce, ca, 0), ehConfirmedLine("/tmp/x/t.test", []byte("\x7fELF")))
	}
	t.Run("a file that matches its sidecar is accepted", func(t *testing.T) {
		// A shard whose steps carry heads is one sealed with --exec-head (D48
		// step 6): every started step has one, a's with a confirmed line.
		f := newSealedCfix(t, true)
		f.runWithHeads(func(string, int) bool { return true })
		f.bindExecHead(good(t))
		_, sh := f.collect()
		requireKnown(t, sh, ca)
	})
	t.Run("a file nobody claims is refused", func(t *testing.T) {
		f := newCfix(t)
		f.good()
		if err := os.WriteFile(StepFile(f.p.dir, ca, 0, "exechead"), good(t), 0o600); err != nil {
			t.Fatal(err)
		}
		_, sh := f.collect()
		requireUnknown(t, sh, ca, "exechead-unclaimed")
	})
	t.Run("a file that changed after the sidecar was written is refused", func(t *testing.T) {
		f := newCfix(t)
		f.good()
		data := good(t)
		f.bindExecHead(data)
		data[len(data)-3] ^= 1
		if err := os.WriteFile(StepFile(f.p.dir, ca, 0, "exechead"), data, 0o600); err != nil {
			t.Fatal(err)
		}
		_, sh := f.collect()
		requireUnknown(t, sh, ca, "exechead-digest")
	})
	t.Run("a file that is missing is refused", func(t *testing.T) {
		f := newCfix(t)
		f.good()
		f.bindExecHead(good(t))
		os.Remove(StepFile(f.p.dir, ca, 0, "exechead"))
		_, sh := f.collect()
		requireUnknown(t, sh, ca, "exechead-missing")
	})
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
