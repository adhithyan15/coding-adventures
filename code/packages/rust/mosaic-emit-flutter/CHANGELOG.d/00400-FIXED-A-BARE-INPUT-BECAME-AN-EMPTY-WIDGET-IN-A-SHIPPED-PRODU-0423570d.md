### Fixed — a bare `Input` became an empty widget in a shipped product (#14861)

`Input` is the pre-UI29 spelling of `HostInput`, retained for capabilities the
first `HostInput` contract did not carry. **Every other emitter of the eight
accepts both.** Flutter accepted only `HostInput`, so a bare `Input` fell
through to the unresolved-component fallback:

```dart
/* TODO: component reference 'Input' not yet resolved */ const SizedBox.shrink(),
```

An empty widget where a text field belongs — and **no degradation reported**.

It reached a shipped product. `mosaic-pkg-notes` authors
`Input [ notes-body-input ]`, TaskApp embeds Notes, and TaskApp's Flutter
build had no notes input at all while its report said zero degradations and
its `native-complete` gate passed.

Found while investigating #14861 — that the grid package cannot be emitted on
seven of eight backends. Flutter was the one that "succeeded", and looking at
what it produced is what turned up this.

The fallback itself is unchanged and still deliberate for a genuine
unresolved component reference; `Input` simply is not one.

