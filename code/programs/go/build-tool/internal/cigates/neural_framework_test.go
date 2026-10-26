package cigates

import "testing"

// A fixture or validator can change without mapping to any native package.
// Conversely, affected-package closure can select a lane without a direct
// source change. Exercise both inputs through the production evaluator.
func TestNeuralFrameworkConformanceSelection(t *testing.T) {
	reg := loadRealRegistry(t)
	const gate = "neural-framework-conformance"
	for _, path := range []string{
		"code/packages/python/neural-framework/src/neural_framework/__init__.py",
		"code/packages/typescript/neural-framework/src/index.ts",
		"code/packages/python/matrix/src/matrix/__init__.py",
		"code/packages/typescript/matrix/src/index.ts",
		"code/packages/typescript/matrix-rust-napi/index.js",
		"code/packages/c/neural-framework/src/neural_framework.c",
		"code/packages/c/neural-framework/tools/run.sh",
		"code/packages/c/neural-framework/tools/run.ps1",
		"code/packages/c/matrix/src/matrix.c",
		"code/packages/c/float-math/src/float_math.c",
		"code/packages/c/bounded-json/src/bounded_json.c",
		"code/packages/c/iso-harness/lib/iso-lib.sh",
		"code/packages/c/iso-harness/lib/iso-lib.ps1",
		"code/scripts/validate_neural_framework_c.py",
		"code/scripts/tests/test_neural_framework_c.py",
		"code/scripts/validate_neural_framework_rust.py",
		"code/scripts/tests/test_neural_framework_rust.py",
		"code/scripts/validate_neural_framework_go.py",
		"code/scripts/tests/test_neural_framework_go.py",
		"code/scripts/validate_neural_framework_ruby.py",
		"code/scripts/tests/test_neural_framework_ruby.py",
		"code/scripts/tests/test_neural_framework_interchange.py",
		"code/specs/fixtures/neural-framework-v1/dense.json",
		"code/scripts/validate_neural_framework_interchange.py",
		"code/scripts/tests/test_neural_framework_ci_gate.py",
		".github/workflows/neural-framework.yml",
		".github/workflows/ci.yml",
		"code/specs/data/ci-gates.json",
	} {
		t.Run(path, func(t *testing.T) {
			if !mustEvaluate(t, reg, map[string]bool{}, []string{path}, false)[gate] {
				t.Fatal("native or shared-input change omitted required conformance")
			}
		})
	}
	for _, path := range []string{
		"README.md",
		"code/learning/machine-learning/curriculum.md",
		"code/specs/fixtures/neural-framework-v10/dense.json",
		"code/scripts/validate_neural_framework_interchange.py.bak",
		"code/packages/c/neural-framework-other/src/neural_framework.c",
		"code/scripts/validate_neural_framework_c.py.bak",
		"code/scripts/tests/test_neural_framework_c.py.bak",
		"code/scripts/validate_other_framework.py",
	} {
		t.Run(path, func(t *testing.T) {
			if mustEvaluate(t, reg, map[string]bool{}, []string{path}, false)[gate] {
				t.Fatal("unrelated path selected conformance")
			}
		})
	}
	for _, pkg := range []string{
		"python/neural-framework", "typescript/neural-framework", "python/matrix", "typescript/matrix",
		"rust/neural-framework", "rust/matrix", "rust/bounded-json", "go/neural-framework", "go/matrix",
		"ruby/neural-framework", "ruby/matrix", "c/neural-framework", "c/matrix", "c/float-math",
		"c/bounded-json", "c/iso-harness",
	} {
		if !mustEvaluate(t, reg, map[string]bool{pkg: true}, []string{}, false)[gate] {
			t.Fatalf("affected package %s omitted conformance", pkg)
		}
	}
	if !mustEvaluate(t, reg, map[string]bool{}, []string{}, true)[gate] {
		t.Fatal("forced main build omitted conformance")
	}
}
