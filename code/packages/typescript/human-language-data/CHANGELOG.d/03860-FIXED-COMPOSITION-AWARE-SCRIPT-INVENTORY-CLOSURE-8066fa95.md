### Fixed - composition-aware script inventory closure

- Check only characters belonging to a lesson's declared script, so legacy
  romanized headwords do not masquerade as missing Arabic glyphs.
- Compare canonical decomposition on both sides so **أ**, **إ**, and **آ** close
  through their Alif carrier and combining marks.
- Add Arabic Maddah Above composition and enable the now-clean completion gate.

