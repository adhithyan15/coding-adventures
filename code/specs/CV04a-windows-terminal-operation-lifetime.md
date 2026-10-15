# CV04a — Windows terminal operation lifetime

This bounded repair follows the c351402c transport import at source0389f753.
Independent e5b9163a review withheld acceptance: destruction of the provider
releases boxed OVERLAPPED structures and buffers even when closesocket has only
requested cancellation. This is a static applicable OS-contract defect, not a
claimed executed undefined-behavior reproduction. Miri is DEFERRED/PENDING.

## Submission and completion contract

AcceptEx returns BOOL: zero means failure. Capture WSAGetLastError immediately
after zero; only WSA_IO_PENDING permits retaining a submitted pending operation.
Nonzero synchronous success also retains storage until its IOCP packet arrives.
Do not record a failed nonpending submission or increment pending_accepts for it.
WSARecv/WSASend retain their existing SOCKET_ERROR contract.

GetQueuedCompletionStatus returning FALSE with a nonnull OVERLAPPED is a failed
terminal packet, including when its error value equals WAIT_TIMEOUT. Recover
that identity before classifying null-pointer timeout/error. A null timeout
does not terminate an operation; unknown/null packets cannot discharge a box.

## Destruction barrier

Provider destruction first removes all stream/listener states, closes their
sockets and completed-but-unadopted accepted sockets, and closes sockets owned
inside pending AcceptEx operations. Preserve the existing send shutdown before
stream closure. No application callbacks, adoption, rearming or replenishment
are permitted during teardown.

Keep the completion port and every remaining operation allocation alive until
their actual terminal packets are dequeued. A shared internal drain helper used
by Drop returns Drained with exact accept/stream terminal counts only when both
maps are empty through matching packets. Poll timeouts continue waiting and
cannot release storage. Unknown packets do not increment those counts.

If the completion port reports a fatal wait error without a terminal identity,
return Retained with outstanding counts and that error. Leave outstanding maps
intact in the helper; Drop explicitly retains their allocations and a completion
port reference for process lifetime. This exceptional resource retention is a
safe fallback, not completion, and must never be reported as Drained. There is
no fabricated cancellation success or timeout-based release. Unbounded waiting
for an OS terminal packet is possible; sibling shutdown coordination must
signal before entering this provider destructor.

## Controls and remaining gates

Unit controls pin AcceptEx synchronous success, pending zero and immediate
zero failure, plus failed-packet/non-null timeout precedence. Native Windows
controls use actual pending accepts and reads, include already closed sockets
and completed-but-unadopted accepts, call the same destructor helper, and pin
the exact submitted/terminal accounting. Inject a fatal wait error into that
helper and verify retained outstanding storage before genuinely draining it;
test timeout and unrelated packet handling without dropping pending storage.
Preserve existing final-bytes/EOF, external adoption, IPv6 port ownership and
all platform/resource controls. Run full IOCP and transport suites and strict
Clippy; repeat the complete reverse cone after all required repairs.

Separate required slices remain OPEN: bounded pending adoption synchronized
with closure and reservation ownership through drained commands/unwind, setter
shrink policy, worker/acceptor/partial-launch failure shutdown, and numerical
spacing safety. Stop cannot preempt a permanently blocked application callback;
Windows currently has no cross-thread wake_handle, so bounded poll fallback
remains and no immediate Windows wake is claimed. No new crates.io dependencies.

Independent exact-head security review must accept the implementation before
publication. Whole456/general P2/current-native3OS/required exact-head CI,
conflict and full Closure parity/lineage gates remain OPEN. Frozen Miri checkout,
inputs, target and receipts remain untouched; deliver first, then obtain genuine
final-source Miri acceptance. Full objective incomplete; heartbeat ACTIVE.
