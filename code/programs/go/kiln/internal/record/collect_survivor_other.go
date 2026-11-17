//go:build !linux

package record

import "errors"

// liveSurvivors cannot look: there is no /proc to read here, and the recorder
// runs on Linux only. The error makes the shard carry `survivor` (fail closed).
func liveSurvivors(SurvivorQuery) ([]string, error) {
	return nil, errors.New("the survivor scan needs /proc, which this platform does not have")
}
