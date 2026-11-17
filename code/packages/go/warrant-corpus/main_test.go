package warrantcorpus

import (
	"os"
	"path/filepath"
	"testing"
)

// childPackageEnv tells a re-executed test binary to behave as the lane's entry
// program (the sabotage command re-runs "the running executable").
const childPackageEnv = "WARRANT_TEST_CHILD_PKG"

func TestMain(m *testing.M) {
	if pkg := os.Getenv(childPackageEnv); pkg != "" {
		symbolOf = func(impl any) (string, string, bool) {
			name, _, ok := funcSymbol(impl)
			return name, filepath.Join(pkg, "lib.go"), ok
		}
		os.Exit(Main(os.Args[1:], ProcessIO(), refRegistry()))
	}
	os.Exit(m.Run())
}
