# VLT-PM08: CLI Secret Input and Entropy Host

Status: Phase 1A normative contract

## 1. Purpose

VLT-PM05 accepts already-owned zeroizing passphrase and entropy bytes, while
VLT-PM06 intentionally owns only paths, permissions, local persistence, and
process exclusion. A real command must still collect a secret without exposing
it through redirectable process inputs and must obtain fresh operating-system
randomness without leaking platform diagnostics.

This specification defines that narrow CLI-only host boundary. It owns:

1. fixed passphrase prompts written to the controlling terminal;
2. hidden, bounded line collection from that same terminal;
3. constant-time confirmation for a newly selected vault passphrase; and
4. exact caller-buffer filling from the operating-system CSPRNG.

It does not parse commands, resolve paths, persist bytes, select storage,
calibrate Argon2id, prepare vault state, print secrets, or implement clipboard
policy. Web and desktop clients may reuse the same application contracts with
their own UI secret collectors and the common CSPRNG primitive.

## 2. Security objective

A shell pipeline, redirected stdin, inherited environment, process listing,
configuration reader, URL handler, or command history must not become a master
passphrase source. The adapter opens the process's controlling terminal or
attached console directly for every collection. If none is available, it fails
closed rather than reading stdin or accepting an alternate source.

The adapter minimizes secret lifetime and diagnostic exposure. It returns an
owned zeroizing byte vector, never a `String`, borrowed slice, printable secret
wrapper, or error containing input. It does not log prompts, lengths, native
error codes, terminal paths, or collected bytes.

This boundary cannot defend against another process already running as the
same operating-system user, an administrator, kernel compromise, terminal
emulator capture, or a force-kill that prevents in-process cleanup.

## 3. Closed prompt contract

The public prompt selection is a closed enum. V1 contains exactly:

| Purpose | Exact text |
|---|---|
| unlock | `Vault passphrase: ` |
| new vault | `New vault passphrase: ` |
| confirmation | `Confirm vault passphrase: ` |
| portable export | `Export passphrase: ` |
| portable import/open | `Import passphrase: ` |

The API accepts no caller-provided prompt text. This prevents an item name,
provider string, path, locator, or other attacker-controlled value from being
rendered beside secret collection. Prompts go to the controlling terminal, not
stdout or stderr, so ordinary command output remains independently redirectable.

## 4. Secret collection contract

Each call independently opens and verifies the native terminal object, captures
its current mode, disables input echo, writes one fixed prompt, and reads until
the first CR or LF terminator. The terminator is not part of the secret.

The resulting passphrase is:

- non-empty;
- at most 1,024 bytes after native text conversion;
- owned by `Zeroizing<Vec<u8>>` immediately after platform collection; and
- never exposed through ordinary `Debug` or error output.

Unix retains the exact terminal input bytes. Windows reads UTF-16 console input,
rejects an unpaired surrogate, and encodes the resulting Unicode scalar values
as UTF-8 bytes. The maximum is measured in final bytes. A value may therefore
fit the temporary Windows code-unit bound yet fail the byte bound after UTF-8
encoding.

An oversized line is drained through its terminator while echo remains disabled.
This prevents a suffix from becoming the next command's visible input. EOF,
invalid encoding, incomplete input, and native read failure fail closed.

## 5. Native terminal requirements

### 5.1 Unix

The adapter opens `/dev/tty` read-write with close-on-exec and no-follow flags,
then verifies both `isatty` and character-device identity. It captures termios,
clears only `ECHO` and `ECHONL`, and applies the hidden mode before writing the
prompt. It restores the captured mode before writing the post-input newline and
before returning control to the caller.

### 5.2 Windows

The adapter opens `CONIN$` and `CONOUT$` rather than inherited standard handles.
It verifies each with `GetConsoleMode`, captures the input mode, clears only
`ENABLE_ECHO_INPUT`, and uses wide console reads and writes. It restores the
exact captured input mode before writing CRLF and returning.

### 5.3 Restoration

Mode ownership uses a guard. Success, empty input, overflow, encoding failure,
I/O failure after mode capture, explicit early return, and Rust panic unwinding
all attempt restoration. An explicit restoration failure is a terminal-mode
failure and cannot be hidden by a successful read. Abort, `SIGKILL`, kernel
failure, and other termination that prevents destructors from executing are
outside this library's guarantee; the eventual CLI must keep its hidden-input
critical section minimal and install any process-wide graceful-shutdown policy
before invoking it.

## 6. New-passphrase confirmation

New-vault setup performs exactly two independent collections using the fixed new
and confirmation prompts. It compares the complete byte sequences with the
repository constant-time equality primitive. A mismatch returns only the stable
`secrets do not match` class. Both the confirmation and a rejected first value
are wiped on drop. There is no normalization, trimming, case folding, retry
loop, partial-match diagnostic, or length disclosure in this layer.

Command UX may choose to call the two-read operation again after a mismatch,
but each retry must be an explicit bounded CLI policy outside this adapter.

## 7. Entropy contract

The entropy adapter accepts a caller-owned mutable byte slice and either
completely overwrites it with fresh operating-system CSPRNG output or returns a
closed failure. Empty requests are rejected as caller errors. There is no
partial-success result, deterministic seed, user-space pool, cache, retry with
a weaker source, time-based fallback, or platform diagnostic in the public
error.

VLT-PM05 generation-zero initialization requests one 496-byte buffer. Portable
export requests 40 bytes. Other application operations may request their exact
specified lengths through the same adapter. The application layer remains
responsible for partitioning and zeroizing caller-owned entropy.

## 8. Error and redaction contract

V1 errors are a closed, payload-free set:

- terminal unavailable;
- terminal access failed;
- terminal mode failed;
- secret input failed;
- empty secret;
- secret too long;
- secrets do not match;
- invalid entropy request;
- OS entropy unavailable; and
- unsupported platform.

Display text is stable and low resolution. `Debug` contains only the variant.
Native error numbers, paths, handles, input bytes, lengths, prompt payloads, and
random bytes are never attached.

## 9. Dependency and composition rules

`vault-pm-cli-host` may depend on the repository CSPRNG, constant-time compare,
and zeroize primitives plus target-specific terminal APIs. It must not depend
on VLT-PM01 through VLT-PM07, a storage adapter, CLI parser, network SDK,
clipboard, or OS credential store.

The Phase 1A executable composes it as follows:

1. parse and validate a command without accepting inline secrets;
2. resolve and prepare VLT-PM06 roots and acquire the process guard;
3. load and parse VLT-PM07 configuration;
4. collect a required passphrase from this adapter;
5. fill the exact application-requested entropy buffer when mutation requires
   randomness; and
6. transfer owned values into VLT-PM05 and wipe them before exit.

## 10. Acceptance tests

Automated tests must prove:

1. prompt and error text is exact, fixed, and payload-free;
2. empty and over-1,024-byte secrets fail;
3. matching confirmation returns the first owned secret and mismatch fails;
4. Unix pseudo-terminal input is not echoed and the original mode returns;
5. an oversized Unix line is drained before mode restoration;
6. non-terminal Unix objects are rejected;
7. Windows console names are fixed and wide-input decoding rejects malformed
   UTF-16;
8. empty entropy requests fail while independent non-empty requests fill the
   complete caller buffers; and
9. native, Linux, and Windows target builds pass denied-warning linting.

The next `vault-pm-cli` slice must add real executable pseudo-terminal or
ConPTY tests proving redirected stdin cannot satisfy a prompt, command output
remains redirectable, prompt failure maps to the stable CLI exit class, and
`init -> status -> doctor` survives process restart.
