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
	} {
		t.Run(path, func(t *testing.T) {
			if mustEvaluate(t, reg, map[string]bool{}, []string{path}, false)[gate] {
				t.Fatal("unrelated path selected conformance")
			}
		})
	}
	for _, pkg := range []string{"python/neural-framework", "typescript/neural-framework", "python/matrix", "typescript/matrix"} {
		if !mustEvaluate(t, reg, map[string]bool{pkg: true}, []string{}, false)[gate] {
			t.Fatalf("affected package %s omitted conformance", pkg)
		}
	}
	if !mustEvaluate(t, reg, map[string]bool{}, []string{}, true)[gate] {
		t.Fatal("forced main build omitted conformance")
	}
}
