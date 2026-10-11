## Fixed — no lesson announced as drivable asks a driver to read, write or gesture

A lesson whose core is `voice` is announced as "you can do this one in the car", and
only its detachable sections (writing, the letters in a word, script) wait behind the
stop guard. The modality rule now reads the sections a driver hears plainly for a step
that needs eyes or hands (reason `eyes-or-hands-step`, human-language-data); before it,
such steps in the core of a lesson with a letters or writing section were read out at
the wheel.

- **Now not drivable at the core (21), no lesson text changed.** Word decoding in chapters
  24-30 ("Uncover **આજ** once. … Name them in order, then read the word once": GU-C24-aaj,
  GU-C24-raat, GU-C25-atyaare, GU-C25-bapor, GU-C25-divas, GU-C25-mahino, GU-C25-saanj,
  GU-C25-savaar, GU-C27-bhaat, GU-C28-baari, GU-C29-nadi, GU-C30-kaagal) and the R3/R4
  reading-score returns (GU-R15-name-exchange-r3, GU-R16-anand-r4, GU-R16-chhe-r4,
  GU-R16-maarun-r4, GU-R16-my-name-is-r4, GU-R16-naam-r4, GU-R17-kem-r4, GU-R17-shun-r4,
  GU-R17-whats-your-name-r4) exist to read or point at script; deferring the step would leave
  nothing to do in the car, so they are honestly `sight` at the core. All were already `pen`
  in full.
