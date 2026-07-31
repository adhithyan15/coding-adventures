// Package capabilitycage provides a compile-time capability manifest system
// that enforces which OS-level resources a package is allowed to access.
package capabilitycage

import "fmt"

const (
	CategoryFS     = "fs"
	CategoryNet    = "net"
	CategoryProc   = "proc"
	CategoryEnv    = "env"
	CategoryFFI    = "ffi"
	CategoryTime   = "time"
	CategoryStdin  = "stdin"
	CategoryStdout = "stdout"

	ActionRead    = "read"
	ActionWrite   = "write"
	ActionCreate  = "create"
	ActionDelete  = "delete"
	ActionList    = "list"
	ActionConnect = "connect"
	ActionListen  = "listen"
	ActionDNS     = "dns"
	ActionExec    = "exec"
	ActionFork    = "fork"
	ActionSignal  = "signal"
	ActionCall    = "call"
	ActionLoad    = "load"
	ActionSleep   = "sleep"
)

// Capability declares a single OS-level permission.
type Capability struct {
	Category      string
	Action        string
	Target        string
	Justification string
}

// Manifest is an immutable, compile-time capability declaration for a package.
type Manifest struct {
	capabilities []Capability
}

// validCapabilityPair implements the closed category/action taxonomy from
// Spec 13.
func validCapabilityPair(category, action string) bool {
	switch category {
	case CategoryFS:
		return action == ActionRead || action == ActionWrite || action == ActionCreate ||
			action == ActionDelete || action == ActionList
	case CategoryNet:
		return action == ActionConnect || action == ActionListen || action == ActionDNS
	case CategoryProc:
		return action == ActionExec || action == ActionFork || action == ActionSignal
	case CategoryEnv:
		return action == ActionRead || action == ActionWrite
	case CategoryFFI:
		return action == ActionCall || action == ActionLoad
	case CategoryTime:
		return action == ActionRead || action == ActionSleep
	case CategoryStdin:
		return action == ActionRead
	case CategoryStdout:
		return action == ActionWrite
	default:
		return false
	}
}

// NewManifest constructs an immutable Manifest from a slice of capabilities.
// It panics if any capability uses a category/action pair outside Spec 13's
// closed taxonomy, so an invalid manifest can never be observed through Has,
// Check, or Capabilities.
func NewManifest(caps []Capability) *Manifest {
	for i, capability := range caps {
		if !validCapabilityPair(capability.Category, capability.Action) {
			panic(fmt.Sprintf(
				"capability %d has invalid category/action pair %q:%q",
				i, capability.Category, capability.Action,
			))
		}
	}

	result, _ := StartNew[*Manifest]("capability-cage.NewManifest", nil,
		func(op *Operation[*Manifest], rf *ResultFactory[*Manifest]) *OperationResult[*Manifest] {
			copied := make([]Capability, len(caps))
			copy(copied, caps)
			return rf.Generate(true, false, &Manifest{capabilities: copied})
		}).GetResult()
	return result
}

// EmptyManifest is the pre-built zero-capability manifest for pure-computation packages.
var EmptyManifest = NewManifest(nil)

// Check returns a CapabilityViolationError if no declared capability covers
// the (category, action, target) triple.
func (m *Manifest) Check(category, action, target string) error {
	_, err := StartNew[struct{}]("capability-cage.Manifest.Check", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("category", category)
			op.AddProperty("action", action)
			op.AddProperty("target", target)
			if m.Has(category, action, target) {
				return rf.Generate(true, false, struct{}{})
			}
			return rf.Fail(struct{}{}, &CapabilityViolationError{
				Category: category,
				Action:   action,
				Target:   target,
				Message: fmt.Sprintf(
					"capability %s:%s:%s not declared; add it to required_capabilities.json then regenerate gen_capabilities.go",
					category, action, target,
				),
			})
		}).GetResult()
	return err
}

// Has returns true if the manifest declares a capability that covers the
// (category, action, target) triple.
func (m *Manifest) Has(category, action, target string) bool {
	result, _ := StartNew[bool]("capability-cage.Manifest.Has", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("category", category)
			op.AddProperty("action", action)
			op.AddProperty("target", target)
			for _, cap := range m.capabilities {
				if cap.Category == category && cap.Action == action {
					if matchTarget(cap.Target, target) {
						return rf.Generate(true, false, true)
					}
				}
			}
			return rf.Generate(true, false, false)
		}).GetResult()
	return result
}

// Capabilities returns a copy of the manifest's capability list.
func (m *Manifest) Capabilities() []Capability {
	result, _ := StartNew[[]Capability]("capability-cage.Manifest.Capabilities", nil,
		func(op *Operation[[]Capability], rf *ResultFactory[[]Capability]) *OperationResult[[]Capability] {
			out := make([]Capability, len(m.capabilities))
			copy(out, m.capabilities)
			return rf.Generate(true, false, out)
		}).GetResult()
	return result
}

// CapabilityViolationError is returned when a package attempts an OS operation
// that was not declared in its manifest.
type CapabilityViolationError struct {
	Category string
	Action   string
	Target   string
	Message  string
}

// Error implements the error interface.
func (e *CapabilityViolationError) Error() string {
	result, _ := StartNew[string]("capability-cage.CapabilityViolationError.Error", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, e.Message)
		}).GetResult()
	return result
}
