### Fixed — Material's default button minimum overrode authored padding (#14858)

`ButtonStyle` carried the authored padding but not `minimumSize`, so Flutter's
own `Size(64, 36)` floor decided the button box instead. Measured on Trestle's
generated Flutter app before changing anything:

- **every** button came back exactly `48.0` high
- the two narrowest were exactly `64.0` wide
- the task-row toggle, one glyph with `padding: 3`, was `64.0 x 48.0`

Neither number is in the `.msl`; both are Material defaults.

Adding `minimumSize: WidgetStatePropertyAll(Size.zero)` hands the size back to
the authored padding:

| toggle `○` | tap target | painted box |
| --- | --- | --- |
| before | 64.0 x 48.0 | 64.0 x 48.0 |
| after | **48.0 x 48.0** | **20.1 x 26.0** |

**This deliberately does not set `tapTargetSize: shrinkWrap`.** That would also
shrink the *touch* target below the 48dp accessibility minimum. The two are
separate properties precisely so the painted box can follow the design while the
hit area stays accessible, and the measurement above shows both holding: the
toggle paints at 20x26 and remains 48x48 tappable. A test asserts `shrinkWrap`
is absent, so a later "fix" cannot quietly trade the tap target away.

Sizes were measured by walking to each button's `Material` ink surface in a
widget test, not read off the emitted Dart — the padding was always *in* the
source, which is exactly why this survived.

