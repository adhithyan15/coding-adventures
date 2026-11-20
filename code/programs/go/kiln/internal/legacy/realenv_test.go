package legacy

import (
	"os"
	"testing"
)

// KILN04 decision D38: GOWORK is a keyed variable in the real env.json, so
// the census can set GOWORK=off and the value is part of the key.
func TestRealEnvPolicyKeysGOWORK(t *testing.T) {
	src, err := os.ReadFile("../../../../../kiln/legacy/env.json")
	if err != nil {
		t.Fatal(err)
	}
	p, err := ParseEnvPolicy(src)
	if err != nil {
		t.Fatal(err)
	}
	vars, err := p.EnvFor([]string{"GOWORK=off", "GOWORKX=1", "OTHER=1"})
	if err != nil {
		t.Fatal(err)
	}
	if len(vars) != 1 || vars[0].Name != "GOWORK" || vars[0].Value != "off" {
		t.Fatalf("want exactly GOWORK=off keyed, got %v", vars)
	}
}
