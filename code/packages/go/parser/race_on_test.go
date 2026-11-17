//go:build race

package parser

// raceEnabled: under -race sync.Pool deliberately drops items at random, so
// allocation-count assertions are skipped.
const raceEnabled = true
