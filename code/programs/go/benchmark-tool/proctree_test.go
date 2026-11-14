package main

import (
	"os"
	"path/filepath"
	"testing"
)

// A service command runs through a shell, so Stop must take down what the shell started,
// not just the shell. Without that, the real server survives as an orphan and, on Windows,
// keeps the service log open so the results directory cannot be deleted (the failure the
// first scheduled cross-OS build showed in TestRunManifestStartsTCPServiceSubject).
func TestStopKillsTheWholeProcessTree(t *testing.T) {
	dir := t.TempDir()
	logPath := filepath.Join(dir, "service.log")
	logFile, err := os.Create(logPath)
	if err != nil {
		t.Fatal(err)
	}

	cmd := shellCommand(treeCommand(dir))
	cmd.Dir = dir
	cmd.Stdout = logFile
	cmd.Stderr = logFile
	configureProcessTree(cmd)
	if err := cmd.Start(); err != nil {
		t.Fatal(err)
	}
	service := &RunningService{cmd: cmd, tree: attachProcessTree(cmd), done: make(chan error, 1), log: logFile}
	go func() { service.done <- cmd.Wait() }()

	waitForTreeChild(t, dir)
	if err := service.Stop(); err != nil {
		t.Fatalf("stop: %v", err)
	}
	assertTreeKilled(t, dir, logPath)
}
