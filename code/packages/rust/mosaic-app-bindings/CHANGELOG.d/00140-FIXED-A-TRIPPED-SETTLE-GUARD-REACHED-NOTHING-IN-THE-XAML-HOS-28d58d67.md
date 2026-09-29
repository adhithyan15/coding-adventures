### Fixed -- a tripped settle guard reached nothing in the XAML host

The other four hosts return the settled update to their caller, so a guard that
gives up arrives as an `error` key. This host's `Dispatch` returns void and
projects props onto a component, and `error` is not a prop -- so the guard
reported into `latestUpdate` and **nothing ever read it**. That is precisely the
dead-field shape the Compose port shipped with `effectWarning`, in a different
place, found this time by writing the test that would have to observe it.

`Status()` now carries the reason, under the lock and consumed once: the
fields are written by whichever thread is settling, so an unlocked read could
hand one caller a guard failure belonging to another's dispatch, and leaving it
set would decorate every later unrelated `ApplyProps` with a stale reason. The
warnings beside it are conditions rather than events, so those persist.

It is kept separate from `effectWarning`
rather than folded in: that one means persistence is off for good, whereas a
guard that trips and then drains successfully leaves nothing pending, so this
one is cleared at the start of each top-level settle instead of being sticky.
Nested frames leave it alone, so an inner guard's reason survives to the outer
frame's caller.

