// Secure process wrappers.
package capabilitycage

import (
	"os"
	"os/exec"
	"strconv"
)

// Exec checks proc:exec:{name} against m, then returns a configured Cmd.
func Exec(m *Manifest, name string, args ...string) (*exec.Cmd, error) {
	return StartNew[*exec.Cmd]("capability-cage.Exec", nil,
		func(op *Operation[*exec.Cmd], rf *ResultFactory[*exec.Cmd]) *OperationResult[*exec.Cmd] {
			op.AddProperty("name", name)
			if err := m.Check(CategoryProc, ActionExec, name); err != nil {
				return rf.Fail(nil, err)
			}
			return rf.Generate(true, false, defaultBackend.Command(name, args...))
		}).GetResult()
}

// Signal checks proc:signal:{pid} against m, then sends sig to the process.
func Signal(m *Manifest, pid int, sig os.Signal) error {
	_, err := StartNew[struct{}]("capability-cage.Signal", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("pid", pid)
			pidStr := strconv.Itoa(pid)
			if err := m.Check(CategoryProc, ActionSignal, pidStr); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			if err := defaultBackend.Kill(pid, sig); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}
