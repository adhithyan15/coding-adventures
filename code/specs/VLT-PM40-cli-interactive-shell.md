# VLT-PM40 — Foreground Interactive Shell

## Status

Normative Phase 1A contract for the foreground interactive session host over
the existing one-shot `vault-pm` command boundary.

## 1. Purpose and boundary

Every `vault-pm` command shipped so far is one operating-system process. It
parses an argument vector, collects a passphrase on the controlling terminal,
unlocks, performs exactly one operation, and exits. Process exit is what wipes
the keys.

That model is correct and stays. It is also unusable for a working session: a
person auditing twenty items answers twenty passphrase prompts, each paying a
full Argon2id derivation.

This contract adds the second host shape VLT-PM00 §14.5 already promised:

```text
vault-pm [--vault NAME] shell
```

It adds **no capability**. Every command available inside a session is a
command that already exists, reached through the same parser, the same
`vault-pm-application` use-case boundary, the same publish-before-release audit
ordering, and the same closed error taxonomy. The shell owns exactly three
things a one-shot process does not need: a command-line reader, a retained
authenticator, and the policy that decides when to drop it.

Out of scope, and explicitly not implied by this slice: a background auto-lock
timer, a local agent or socket (VLT-PM00 §23 item 12), clipboard delivery,
command history, completion, line editing, scripting, aliases, and any
non-interactive batch mode.

## 2. Host placement

```text
code/packages/rust/vault-pm-cli-host   ControllingTerminal::read_command_line
code/packages/rust/vault-pm-cli        src/shell.rs — session loop and policy
code/programs/rust/vault-pm-cli        unchanged; `shell` is one more verb
```

The loop lives in `vault-pm-cli`, not in `vault-pm-cli-host`, because it must
call the crate's own parser and driver, and `vault-pm-cli-host` sits *beneath*
that crate (VLT-PM08 §9 forbids it from depending on a CLI parser). The
terminal primitive lives in `vault-pm-cli-host` because it is controlling
terminal I/O, which that crate owns exclusively.

No new package is introduced. VLT-PM00 §14.1's package list is unchanged, and
the executable stays a thin composition root: `vault-pm shell` reaches the loop
through the ordinary `run` entry point.

## 3. Session model

### 3.1 What is retained, and what is not

The naive design — unlock once, keep the unlocked session, run many commands
against it — is unavailable, and the reason is a rule this product already
enforces. A VLT-PM05 session pins the repository heads it observed, and every
access and mutation boundary consumes the session **by value** so a stale pin
can never be reused (VLT-PM00 §23 items 7a and 9b-2c-5b-1). Reusing one session
across commands would reintroduce exactly the failure that rule exists to
prevent.

The shell therefore retains the smallest thing that removes the repeated
prompt:

| State | Lifetime in a one-shot process | Lifetime in a shell session |
|---|---|---|
| passphrase | one command | until `lock`, idle bound, rejection, or exit |
| derived keys, VRK, item DEKs | one command | one command |
| decrypted catalog, revisions | one command | one command |
| search index projection | one command | one command |
| repository verifier and pinned heads | one command | one command |
| cross-process writer lock | one command | one command |

Only the first row changes. Each command inside a session performs its own
complete verified open, obtains fresh pinned heads, and drops its session
synchronously when it finishes. The decrypted-vault exposure window inside a
shell is identical to the one-shot window.

### 3.2 Binding one vault

A session binds exactly one vault at start: the name given by the leading
`--vault NAME` selector, or the configured default as it stood when the session
began. The name is resolved once, and every delegated command carries it as an
explicit selector.

This is a security decision, not ergonomics. A retained authenticator belongs to
one vault. If a later command could name a different target, the shell would
present a passphrase collected in one context against a target chosen in
another.

A session refuses to start when configuration is absent (invalid input) or when
the named vault is not configured (not found).

The binding is by **name**, and each delegated command re-resolves that name
against configuration as it stands when the command runs. If the name-to-locator
mapping changed mid-session, the retained authenticator would be presented to a
different vault than the one it was collected for. No shipped command rebinds an
existing name — `vault create` only adds new ones, and it is refused inside a
session — so this requires an attacker who can already rewrite the configuration
file as the same operating-system user, which VLT-PM08 §2 places outside this
boundary. It is recorded here as a residual rather than claimed as prevented,
because the session does not pin the resolved locator.

### 3.3 Lazy collection

The authenticator is collected on the first command that actually needs to
unlock, not at session start. A session that only runs `status` or `help` holds
no secret at all.

### 3.4 Dropping the authenticator

The retained value is wiped, synchronously and on drop, when any of these
happen:

1. the user runs `lock`;
2. a command returns the `locked` exit class — a rejected passphrase must not
   turn one mistake into a session that can never succeed again;
3. the configured `auto_lock_seconds` has elapsed when a command is submitted,
   or when the authenticator is handed to an unlock;
4. the advisory clock cannot be read at a command boundary, or could not be read
   when the value was first collected, in which case it is never retained;
5. the session ends by `exit`, `quit`, end of input, or terminal failure;
6. the process exits.

### 3.5 The idle bound is a bound, not a timer

Rule 3 above is checked when a command **is submitted** and again when the
authenticator **is handed out**. It is deliberately not checked only before the
prompt is printed: the process then blocks on the terminal for as long as
nobody types, so a value that was fresh when the prompt appeared could be
arbitrarily stale by the time somebody — not necessarily the same somebody —
submits a command. Measuring at submission is what makes the bound mean
anything for an unattended terminal, which is the threat `auto_lock_seconds`
exists to address. The second check at the point of use exists so that
reordering the loop cannot silently reopen that gap.

A session parked at its prompt for an hour therefore re-authenticates on the
very next command it is given; it does not re-lock while nobody is typing, and
nothing wakes up to wipe the value in the meantime. The value used is the
existing `vaults.<name>.auto_lock_seconds` from VLT-PM07 configuration (default
300), which no host enforced before this slice.

A pre-emptive timer that locks an idle session while it waits is deliberately
**not** delivered here. It requires either a background thread holding secret
material or a non-blocking terminal read loop, and VLT-PM00 §23 schedules
auto-lock with the Phase 1B local agent (item 12). This slice does not claim it
and does not contradict it: a bound that only tightens the existing policy
cannot conflict with a stricter one added later.

## 4. Closed shell grammar

### 4.1 Tokenization

A command line is bounded at 1,024 bytes and 8 tokens, must be valid UTF-8, and
must contain no control characters after its terminator is removed.

| Input | Tokens |
|---|---|
| `item show ABC` | `item`, `show`, `ABC` |
| `search "two words"` | `search`, `two words` |
| `item   list` | `item`, `list` |
| `search "unterminated` | rejected |
| `search "a"b` | rejected |
| nine or more tokens | rejected |

Double quotes group one token containing spaces, doing the job an operating
system shell would have done for a one-shot invocation. There are no escapes,
no single quotes, no nesting, no globbing, no variable expansion, no
substitution, and no operators. A line can therefore mean nothing other than
what it visibly says. Rejected lines wipe any tokens already built, because a
line may hold a search query and queries are treated as secret-bearing
everywhere else in this crate.

### 4.2 Built-in verbs

| Verb | Effect |
|---|---|
| *(blank line)* | reprompt; no state change |
| `lock` | wipe the retained authenticator; print `Locked.` |
| `help`, `--help`, `-h` | print the shell built-ins followed by the one-shot usage table verbatim |
| `exit`, `quit` | end the session |
| end of input | end the session |

### 4.3 Refused verbs

`init`, `vault`, `shell`, and any line beginning with `--vault` are refused
inside a session with the ordinary invalid-input class. The first two build a
*different* vault than the one bound; the third would open a second session
with its own retained authenticator over the same terminal; the fourth would
aim the retained authenticator at a target the user never authenticated
against. A refusal never ends the session.

### 4.4 Delegated verbs

Everything else is passed to the unchanged one-shot parser with the bound
vault selector prefixed. No command is intercepted, rewritten, filtered,
reordered, or given different arguments than the same line would produce as a
one-shot invocation.

## 5. Terminal and stream policy

Command lines are read from the process's **controlling terminal**, exactly
where passphrase prompts are collected, never from process standard input. A
redirected or piped stdin therefore cannot drive an unlocked session, preserving
the VLT-PM08 §2 property that redirectable process inputs are not a secret
source — and, now, not a command source either.

The prompt is the fixed constant `vault-pm> `. Like every other prompt in this
product it is compile-time text: no vault name, item title, or previous result
is ever rendered into it, so a stored value cannot counterfeit shell chrome.

Echo is left in the terminal's ordinary line-discipline state for command
lines, because a command line is a selector, not a secret. Secret-bearing input
is unaffected: `item add login`, `conflict merge card`, `export`, `import`, and
every other ceremony collect their hidden fields through the same echo-disabled
`/dev/tty` (or `CONIN$`) path they use one-shot, inside the same session. The
shell has no code path that reads a secret.

Ordinary command output goes to process stdout and stderr, byte-identical to
what the one-shot executable prints for the same command, so `vault-pm shell >
transcript` still captures results. Audited secret reveal continues to write
only to the controlling terminal and never to those streams (VLT-PM25 §5).

End of input — `Ctrl-D` on Unix, `Ctrl-Z` on a Windows console, or a closed
terminal — is a value, not a failure, and ends the session cleanly. Any other
terminal read or write failure ends the session with the corresponding closed
class.

## 6. Output, exit classes, and rendering

A one-shot process makes one command's class its process class. A long-lived
session cannot: it runs many commands and exits once.

1. Each command renders exactly what it would have rendered one-shot, including
   the same fixed, payload-free stderr line for its failure class.
2. A command's failure never ends the session. An interactive user retries.
3. The **process** class of `vault-pm shell` is success when the session ends
   through `exit`, `quit`, or end of input, whatever individual commands did
   inside it; and the closed class of the host failure that ended the session
   otherwise (for example provider when the terminal becomes unreadable,
   invalid input when no configuration exists, not found for an unknown bound
   vault).
4. Callers that need a command's exit class must invoke that command one-shot.
   This is stated so no script depends on the shell's process class carrying
   per-command meaning.

`status` inside an unlocked session still reports `locked`, and that is correct
rather than a gap: it projects durable owner state, and between commands the
vault genuinely is locked. The session retains an authenticator, not an
unlocked vault. No shell-only status label is introduced.

## 7. Security analysis

### 7.1 The one property that changes

Against the one-shot model, a session extends the lifetime of exactly one
secret: the master passphrase, held in a wipe-on-drop buffer between commands.

This is a real trade and is stated plainly rather than minimized. An adversary
who can read this process's memory recovers the master passphrase, where the
same adversary reading a one-shot process mid-command would recover that
vault's derived keys. The passphrase is the more valuable of the two: it
re-derives everything, forever, and may be reused elsewhere.

Retaining a derived unlock capability instead of the passphrase would be
strictly better. It is not available: `VaultAccessV1::unlock` accepts a
passphrase, and there is no key-only reopen path. Creating one is an
application-layer change (VLT-PM05), outside this host-only slice, and is the
right shape for the Phase 1B agent work that will need it anyway.

The mitigations shipped here are: wipe-on-drop custody, lazy collection,
explicit `lock`, the command-boundary idle bound, fail-closed wipe on a rejected
attempt, fail-closed wipe when the clock is unreadable, and a hand-written
`Debug` that cannot print the value. VLT-PM08 §2 already places an attacker
running as the same operating-system user, a kernel compromise, and terminal
emulator capture outside this boundary's guarantee; that remains true.

### 7.2 Properties that must not change, and how they are held

| Invariant | How the shell preserves it |
|---|---|
| publish-before-release audit ordering | untouched — commands run through the same application boundary |
| audit events per operation | one per command, exactly as one-shot; a session emits no event of its own |
| fail-closed error handling | shell-level failures map through the same closed taxonomy |
| no secret in argv, env, config, logs | commands are read from the terminal, never from argv; no secret is accepted as a token |
| hidden secret collection | unchanged host methods; the shell never intercepts a secret prompt |
| secret reveal on the terminal only | unchanged; `CliOutput` still never carries a secret |
| single-writer cross-process rule | the writer lock is held per command, never across a prompt |
| session consumption / no stale pins | each command opens and consumes its own session |
| stdin cannot supply a secret | stdin cannot supply a command either |

The cross-process writer lock deserves emphasis: a session acquires it
per command and releases it before returning to the prompt, so an idle shell
never blocks another process. The lock is also acquired once at session start to
read configuration and released before the loop begins.

## 8. Acceptance gates

The slice is complete only when tests prove:

1. `shell` parses as a bare verb, accepts an optional leading vault selector,
   and rejects any argument;
2. one scripted passphrase satisfies several authenticated commands in one
   session;
3. `lock` wipes the authenticator and the next command re-authenticates;
4. a rejected passphrase is not retained, and the following command may succeed;
5. the configured idle bound drops the authenticator both when the clock
   advances between commands and when it advances entirely inside the blocked
   terminal read, so a stale value is never handed to a submitted command;
6. lifecycle verbs, nested `shell`, a leading `--vault`, unterminated quotes,
   and over-long token lists are refused without ending the session, and the
   refusal is enforced at dispatch as well as at classification;
7. `help` and blank lines need no authentication;
8. a session refuses to start without configuration, with an unknown vault, or
   without a readable terminal, using the closed classes;
9. the tokenizer's accepted and rejected forms are exactly as tabulated;
10. the retained authenticator is unreachable through `Debug`;
11. end of input on a real pseudo-terminal ends a real process cleanly, and a
    real process unlocks once, runs several commands with no second prompt,
    performs a hidden item-creation ceremony inside the session, re-authenticates
    after `lock`, and leaves no secret in the transcript or on disk; and
12. formatting, Clippy, rustdoc, host tests, CLI tests, and the downstream
    executable suite pass.

## 9. References

- `VLT-PM00-local-first-password-manager.md` §14.4, §14.5, §14.7, §23 item 9b-5
- `VLT-PM05-application.md` — session lifecycle and consumption
- `VLT-PM07-config.md` — `auto_lock_seconds`
- `VLT-PM08-cli-host.md` — controlling-terminal collection and closed prompts
- `VLT-PM09-cli-bootstrap.md` — parser, renderer, and exit classes
- `VLT-PM25-cli-secret-reveal.md` — terminal-only secret delivery
