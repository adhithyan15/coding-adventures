### Added -- an execution acceptance for the XAML host

`tests/xaml_effect_completion.rs` emits the host into a temporary console
project, compiles it against the same `Windows.UI.Color` value stub the
conformance harness already uses -- so it builds without the Windows App SDK,
on any platform -- and runs it against the conformance runtime, one process and
one state file per scenario.

Seven scenarios. Where the other hosts call `snapshot()` directly, this one has
no such method: it persists internally after every settle. So the assertion is
the user-visible consequence instead -- the runtime refuses to snapshot while an
effect is pending, `PersistSnapshot` catches the refusal, and `Status` carries
it. That is reached through the API this host actually has, rather than adding a
public `Snapshot` for the test's benefit.

The generated project writes an empty `Directory.Build.props` and `.targets`
beside itself, to stop MSBuild's upward search. It looks in every ancestor
directory, and the project lives under the system temp directory -- `/tmp`,
mode 1777, on a Linux build host -- so any local user could pre-plant one and
have their targets run as the build user.

Setting `<ImportDirectoryBuildProps>false</ImportDirectoryBuildProps>` in the
project body does **not** work and looks like it does, which is how the first
attempt at this got it wrong: `Directory.Build.props` is imported by the
implicit `Sdk.props` *before* the body is evaluated, so the property is read too
late. Measured on 9.0.313 -- a hostile props file one directory up still landed
with the property set, while the sentinel files and a command-line `-p:` both
suppressed it. The `.targets` half of that same property does work, which is
what makes the broken half easy to miss.

Mutation-tested: removing the handler-throw guard fails the `throwing` case
(and trips the vacuous-prop detector, because the escaping exception means props
are never applied), and dropping the settle error from `Status` fails "a runaway
chain is reported rather than abandoned quietly". Reverting the deferred unload
does **not** fail anything on macOS, for the reason given above.

