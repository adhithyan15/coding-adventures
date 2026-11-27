// Package ossimulator provides the OS-neutral HOS03 event-machine boundary.
// Profiles own policy; Machine owns virtual ordering, JSON isolation, and replay.
package ossimulator

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"math"
	"unicode/utf16"
	"unicode/utf8"
)

const (
	SchemaVersion = 1
	MaxJSONDepth  = 64
	MaxJSONNodes  = 100_000
	MaxStringSize = 1_000_000
	MaxTextSize   = 10_000_000
	MaxRunBudget  = 1_000_000
)

// Stimulus is one portable external input. Data must contain JSON values only.
type Stimulus struct {
	Kind string         `json:"kind"`
	Data map[string]any `json:"data"`
}

// Effect is a profile-owned portable event before machine stamps tick/profile.
type Effect struct {
	Kind string         `json:"kind"`
	Data map[string]any `json:"data"`
}

// TraceEvent records one effect at its logical transition.
type TraceEvent struct {
	Tick    uint64         `json:"tick"`
	Profile string         `json:"profile"`
	Kind    string         `json:"kind"`
	Data    map[string]any `json:"data"`
}

// Profile supplies initial JSON state and a deterministic state reducer.
// Transition receives isolated copies; only a fully valid result is committed.
type Profile interface {
	ID() string
	InitialState() any
	Transition(state any, tick uint64, stimulus Stimulus) (nextState any, effects []Effect, err error)
}

// Snapshot stores the replay anchor and any stimuli not yet consumed.
type Snapshot struct {
	SchemaVersion int        `json:"schemaVersion"`
	Profile       string     `json:"profile"`
	Tick          uint64     `json:"tick"`
	State         any        `json:"state"`
	Pending       []Stimulus `json:"pending"`
	History       []Stimulus `json:"history"`
}

// Machine is a deterministic event reducer. It has no wall clock or host I/O.
type Machine struct {
	profile Profile
	tick    uint64
	state   any
	pending []Stimulus
	history []Stimulus
	trace   []TraceEvent
}

type jsonBudget struct{ nodes, text int }

func NewMachine(profile Profile) (*Machine, error) {
	if profile == nil || profile.ID() == "" {
		return nil, errors.New("profile ID is required")
	}
	state, err := normalizeJSON(profile.InitialState(), "profile state", 0, &jsonBudget{})
	if err != nil {
		return nil, err
	}
	return &Machine{profile: profile, state: state, pending: []Stimulus{}, history: []Stimulus{}, trace: []TraceEvent{}}, nil
}

func (m *Machine) Tick() uint64      { return m.tick }
func (m *Machine) ProfileID() string { return m.profile.ID() }

func (m *Machine) State() any {
	state, _ := cloneJSON(m.state, "profile state")
	return state
}

func (m *Machine) Events() []TraceEvent {
	return cloneEvents(m.trace)
}

func (m *Machine) Inject(stimulus Stimulus) error {
	clean, err := normalizeStimulus(stimulus)
	if err != nil {
		return err
	}
	m.pending = append(m.pending, clean)
	return nil
}

// Step processes exactly one queued stimulus or synthesized clock.tick.
func (m *Machine) Step() ([]TraceEvent, error) {
	start := len(m.trace)
	if err := m.transitionOnce(nil); err != nil {
		return nil, err
	}
	return cloneEvents(m.trace[start:]), nil
}

// Run executes a finite transition budget; previous successful transitions
// remain committed if a later transition in the same budget returns an error.
func (m *Machine) Run(budget int) ([]TraceEvent, error) {
	if budget < 0 || budget > MaxRunBudget {
		return nil, fmt.Errorf("run budget must be in [0,%d]", MaxRunBudget)
	}
	start := len(m.trace)
	for i := 0; i < budget; i++ {
		if err := m.transitionOnce(nil); err != nil {
			return cloneEvents(m.trace[start:]), err
		}
	}
	return cloneEvents(m.trace[start:]), nil
}

func (m *Machine) Snapshot() Snapshot {
	state, _ := cloneJSON(m.state, "profile state")
	return Snapshot{SchemaVersion: SchemaVersion, Profile: m.profile.ID(), Tick: m.tick, State: state, Pending: cloneStimuli(m.pending), History: cloneStimuli(m.history)}
}

func (m *Machine) Serialize() ([]byte, error) {
	return json.Marshal(m.Snapshot()) // encoding/json sorts map keys for stable snapshots.
}

func Restore(profile Profile, encoded []byte) (*Machine, error) {
	if profile == nil || profile.ID() == "" {
		return nil, errors.New("profile ID is required")
	}
	var snapshot Snapshot
	if err := json.Unmarshal(encoded, &snapshot); err != nil {
		return nil, fmt.Errorf("decode snapshot: %w", err)
	}
	if snapshot.SchemaVersion != SchemaVersion || snapshot.Profile != profile.ID() {
		return nil, errors.New("snapshot schema or profile mismatch")
	}
	if snapshot.Pending == nil || snapshot.History == nil {
		return nil, errors.New("snapshot arrays are required")
	}
	machine, err := NewMachine(profile)
	if err != nil {
		return nil, err
	}
	for _, stimulus := range snapshot.History {
		if err := machine.transitionOnce(&stimulus); err != nil {
			return nil, fmt.Errorf("replay snapshot history: %w", err)
		}
	}
	if machine.tick != snapshot.Tick {
		return nil, errors.New("snapshot tick does not match history")
	}
	state, err := normalizeJSON(snapshot.State, "snapshot state", 0, &jsonBudget{})
	if err != nil {
		return nil, err
	}
	want, _ := json.Marshal(state)
	got, _ := json.Marshal(machine.state)
	if !bytes.Equal(want, got) {
		return nil, errors.New("snapshot state does not match replayed history")
	}
	for _, stimulus := range snapshot.Pending {
		clean, err := normalizeStimulus(stimulus)
		if err != nil {
			return nil, err
		}
		machine.pending = append(machine.pending, clean)
	}
	return machine, nil
}

func Replay(profile Profile, history []Stimulus) (*Machine, error) {
	machine, err := NewMachine(profile)
	if err != nil {
		return nil, err
	}
	for _, stimulus := range history {
		clean, err := normalizeStimulus(stimulus)
		if err != nil {
			return nil, err
		}
		if err := machine.transitionOnce(&clean); err != nil {
			return nil, err
		}
	}
	return machine, nil
}

func (m *Machine) transitionOnce(explicit *Stimulus) error {
	if m.tick == math.MaxUint64 {
		return errors.New("logical tick limit reached")
	}
	var stimulus Stimulus
	consume := false
	if explicit != nil {
		stimulus = *explicit
	} else if len(m.pending) > 0 {
		stimulus, consume = m.pending[0], true
	} else {
		stimulus = Stimulus{Kind: "clock.tick", Data: map[string]any{}}
	}
	cleanInput, err := normalizeStimulus(stimulus)
	if err != nil {
		return err
	}
	stateCopy, err := cloneJSON(m.state, "profile state")
	if err != nil {
		return err
	}
	inputCopy, err := normalizeStimulus(cleanInput)
	if err != nil {
		return err
	}
	next, effects, err := m.profile.Transition(stateCopy, m.tick+1, inputCopy)
	if err != nil {
		return fmt.Errorf("profile transition: %w", err)
	}
	next, err = normalizeJSON(next, "profile state", 0, &jsonBudget{})
	if err != nil {
		return err
	}
	newEvents := make([]TraceEvent, 0, len(effects))
	for _, effect := range effects {
		if effect.Kind == "" {
			return errors.New("effect kind is required")
		}
		budget := &jsonBudget{}
		kindValue, err := normalizeJSON(effect.Kind, "effect kind", 0, budget)
		if err != nil {
			return err
		}
		data, err := normalizeRecordWithBudget(effect.Data, "effect data", budget)
		if err != nil {
			return err
		}
		newEvents = append(newEvents, TraceEvent{Tick: m.tick + 1, Profile: m.profile.ID(), Kind: kindValue.(string), Data: data})
	}
	if consume {
		m.pending = m.pending[1:]
	}
	m.tick++
	m.state = next
	m.history = append(m.history, cleanInput)
	m.trace = append(m.trace, newEvents...)
	return nil
}

func normalizeStimulus(input Stimulus) (Stimulus, error) {
	if input.Kind == "" {
		return Stimulus{}, errors.New("stimulus kind is required")
	}
	budget := &jsonBudget{}
	kindValue, err := normalizeJSON(input.Kind, "stimulus kind", 0, budget)
	if err != nil {
		return Stimulus{}, err
	}
	data, err := normalizeRecordWithBudget(input.Data, "stimulus data", budget)
	if err != nil {
		return Stimulus{}, err
	}
	return Stimulus{Kind: kindValue.(string), Data: data}, nil
}

func normalizeRecord(input any, label string) (map[string]any, error) {
	return normalizeRecordWithBudget(input, label, &jsonBudget{})
}

func normalizeRecordWithBudget(input any, label string, budget *jsonBudget) (map[string]any, error) {
	value, err := normalizeJSON(input, label, 0, budget)
	if err != nil {
		return nil, err
	}
	record, ok := value.(map[string]any)
	if !ok {
		return nil, fmt.Errorf("%s must be a JSON object", label)
	}
	return record, nil
}

// normalizeJSON accepts native JSON-shaped Go values only and creates owned
// copies, so a caller cannot mutate pending stimuli or committed machine state.
func normalizeJSON(value any, label string, depth int, budget *jsonBudget) (any, error) {
	if depth > MaxJSONDepth {
		return nil, fmt.Errorf("%s exceeds maximum JSON depth", label)
	}
	budget.nodes++
	if budget.nodes > MaxJSONNodes {
		return nil, fmt.Errorf("%s exceeds maximum JSON nodes", label)
	}
	switch item := value.(type) {
	case nil, bool:
		return item, nil
	case string:
		if !utf8.ValidString(item) {
			return nil, fmt.Errorf("%s contains invalid UTF-8", label)
		}
		units := utf16Length(item)
		if units > MaxStringSize {
			return nil, fmt.Errorf("%s string exceeds maximum length", label)
		}
		budget.text += units
		if budget.text > MaxTextSize {
			return nil, fmt.Errorf("%s exceeds maximum JSON text", label)
		}
		return item, nil
	case float64:
		if math.IsNaN(item) || math.IsInf(item, 0) {
			return nil, fmt.Errorf("%s numbers must be finite", label)
		}
		if item == 0 {
			return float64(0), nil
		}
		return item, nil
	case float32:
		return normalizeJSON(float64(item), label, depth, budget)
	case int:
		return float64(item), nil
	case int8:
		return float64(item), nil
	case int16:
		return float64(item), nil
	case int32:
		return float64(item), nil
	case int64:
		return float64(item), nil
	case uint:
		return float64(item), nil
	case uint8:
		return float64(item), nil
	case uint16:
		return float64(item), nil
	case uint32:
		return float64(item), nil
	case uint64:
		return float64(item), nil
	case []any:
		if item == nil {
			return nil, nil
		}
		if len(item) > MaxJSONNodes {
			return nil, fmt.Errorf("%s exceeds maximum JSON nodes", label)
		}
		result := make([]any, len(item))
		for i, child := range item {
			clean, err := normalizeJSON(child, label, depth+1, budget)
			if err != nil {
				return nil, err
			}
			result[i] = clean
		}
		return result, nil
	case map[string]any:
		if item == nil {
			return nil, nil
		}
		if len(item) > MaxJSONNodes {
			return nil, fmt.Errorf("%s exceeds maximum JSON nodes", label)
		}
		result := make(map[string]any, len(item))
		for key, child := range item {
			if !utf8.ValidString(key) {
				return nil, fmt.Errorf("%s contains invalid UTF-8 key", label)
			}
			units := utf16Length(key)
			if units > MaxStringSize {
				return nil, fmt.Errorf("%s key exceeds maximum length", label)
			}
			budget.text += units
			if budget.text > MaxTextSize {
				return nil, fmt.Errorf("%s exceeds maximum JSON text", label)
			}
			clean, err := normalizeJSON(child, label, depth+1, budget)
			if err != nil {
				return nil, err
			}
			result[key] = clean
		}
		return result, nil
	default:
		return nil, fmt.Errorf("%s contains unsupported Go value %T", label, value)
	}
}

// Go strings count bytes; HOS03's portable size limit counts UTF-16 code units.
func utf16Length(text string) int { return len(utf16.Encode([]rune(text))) }

func cloneJSON(value any, label string) (any, error) {
	return normalizeJSON(value, label, 0, &jsonBudget{})
}

func cloneEvents(events []TraceEvent) []TraceEvent {
	result := make([]TraceEvent, len(events))
	for index, event := range events {
		data, _ := normalizeRecord(event.Data, "trace data")
		result[index] = TraceEvent{Tick: event.Tick, Profile: event.Profile, Kind: event.Kind, Data: data}
	}
	return result
}

func cloneStimuli(stimuli []Stimulus) []Stimulus {
	result := make([]Stimulus, len(stimuli))
	for index, stimulus := range stimuli {
		result[index], _ = normalizeStimulus(stimulus)
	}
	return result
}
