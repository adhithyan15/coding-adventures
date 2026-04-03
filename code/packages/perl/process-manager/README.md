# process-manager (Perl)

Process lifecycle management for the coding-adventures simulated OS.

## What It Does

- `PCB` — Process Control Block with state, registers, signals, and priority
- `Manager` — spawn, fork, exec, wait_child, exit_process, kill, schedule, block/unblock
- Priority-based round-robin scheduler
- Signals: SIGKILL/SIGSTOP uncatchable, SIGCONT resumes blocked processes

## Usage

```perl
use CodingAdventures::ProcessManager;

my $mgr  = CodingAdventures::ProcessManager::Manager->new();
my $init = $mgr->spawn("init");
my $ls   = $mgr->fork($init);
$mgr->exec($ls, "ls", { pc => 0x4000 });
my $chosen = $mgr->schedule();  # runs init or ls
$mgr->exit_process($ls, 0);
my ($status, $ec) = $mgr->wait_child($init, $ls);
```
