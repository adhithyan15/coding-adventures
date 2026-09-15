- `earth-science/seismic-wave-arrival-order.adj` (new) -- a new
  `seismic_wave_arrival_order(wave, description)` table names the two
  named seismic body waves and which one an earthquake sends out first
  (p_wave->are_the_first_waves_to_arrive_after_an_earthquake,
  s_wave->are_the_next_waves_to_arrive_after_p_waves), each quoted
  verbatim from its own standalone, parallel-worded sentence in Cal OES
  (California Governor's Office of Emergency Services) News' "What Are
  P-Waves and S-Waves?" article -- `trust authoritative` (a California
  state government .gov source). Distinct from the sibling
  `physics/wave-types.adj`, which classifies waves into the
  mechanical/electromagnetic FAMILY axis (and explicitly abstains on
  seismic waves itself), not this arrival-order axis. Picked after
  checking several not-yet-reviewed science tables this window
  (galaxy-types.adj, wave-types.adj -- both exhaustive fixed
  classifications from a single source sentence, non-extendable), then
  researching seismic waves as a fresh topic -- USGS bundles P/S wave
  facts together in comparative sentences ("P waves travel through solid
  and liquid, but S waves do not"), MTU and Wikipedia bundle multiple
  facts or use a different framing dimension, before Cal OES News
  succeeded with clean parallel single-fact sentences. Honest abstention
  on surface_wave: a real third seismic wave commonly grouped with these
  two, but every source checked either bundles its definition with a
  second distinct fact or uses a different framing than the
  arrival-order pattern the two tabled here share. New manifest objective
  `adj.science.6to8.seismic_wave_arrival_order` (162 objectives total, up
  from 161). New e2e test `facts_seismicwavearrivalorder_e2e.rs` (3
  tests: direct recall, reverse binding, honest abstention).
