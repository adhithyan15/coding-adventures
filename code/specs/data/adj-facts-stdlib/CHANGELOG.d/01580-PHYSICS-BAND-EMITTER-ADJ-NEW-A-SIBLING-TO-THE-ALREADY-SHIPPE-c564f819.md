- `physics/band-emitter.adj` (new) — a sibling to the already-shipped `em-spectrum.adj`
  (`band_use(band, application)`, a representative everyday use / effect / detector NASA associates
  with each of the seven EM bands — radio → radio_stations, x_ray → teeth, etc.). That table's own
  header already quotes, verbatim, three NASA sentences that separately state WHO or WHAT emits the
  band: "Ultraviolet radiation is emitted by the Sun...", "Night vision goggles pick up the infrared
  light emitted by our skin and objects with heat.", "Our eyes detect visible light. Fireflies,
  light bulbs, and stars all emit visible light." New `band_emitter(band, emitter)` table:
  ultraviolet → sun, infrared → skin_and_heat_objects, visible → fireflies (the first of three
  listed emitters, per this stdlib's own established convention of carrying the first item a source
  lists when more than one is given). Honestly narrower than its parent: honest abstention on radio,
  microwave, x_ray, and gamma_ray, whose already-cited spans state a USE (already captured by
  `em-spectrum.adj`'s own column) but never an emitter. New e2e test file `facts_bandemitter_e2e.rs`
  (3 tests: forward recall with citation, backward recall of the sun-emitted band, honest abstention
  on radio). No manifest objective, matching `em-spectrum.adj`'s own precedent. Second slice from a
  direct re-examination of the physics/ MODERATE candidates the sweep had deprioritized — closer
  inspection confirmed this one was genuinely buildable, just narrower than the parent's full
  seven-band domain.
