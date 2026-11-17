//! Pre-computed cumulative tempo map with binary search (MUS04 §6).

use crate::error::MusicMachineError;
use crate::limits::DEFAULT_TEMPO_BPM;
use crate::types::TempoEvent;

/// Pre-computed cumulative seconds table with O(log T) binary search queries.
#[derive(Debug, Clone, PartialEq)]
pub struct TempoMap {
    ppq: u32,
    start_ticks: Vec<u64>,
    bpms: Vec<f64>,
    cumulative_seconds: Vec<f64>,
}

impl TempoMap {
    /// Construct a new pre-computed tempo map from a slice of tempo events.
    pub fn new(tempo_events: &[TempoEvent], ppq: u32) -> Result<Self, MusicMachineError> {
        if ppq == 0 {
            return Err(MusicMachineError::InvalidArgument("ppq must be > 0".into()));
        }
        let mut tempos = tempo_events.to_vec();
        tempos.sort_by_key(|e| e.start_tick);
        if tempos.is_empty() || tempos[0].start_tick != 0 {
            tempos.insert(
                0,
                TempoEvent {
                    start_tick: 0,
                    bpm: DEFAULT_TEMPO_BPM,
                },
            );
        }

        let mut start_ticks = Vec::with_capacity(tempos.len());
        let mut bpms = Vec::with_capacity(tempos.len());
        let mut cumulative_seconds = Vec::with_capacity(tempos.len());
        let mut cur_sec = 0.0;

        for (i, t) in tempos.iter().enumerate() {
            if i > 0 {
                let prev_tick = start_ticks[i - 1];
                let prev_bpm = bpms[i - 1];
                let delta_ticks = t.start_tick.checked_sub(prev_tick).ok_or_else(|| {
                    MusicMachineError::InvalidArgument("tempo start_ticks out of order".into())
                })?;
                cur_sec += (delta_ticks as f64) * 60.0 / (prev_bpm * (ppq as f64));
            }
            start_ticks.push(t.start_tick);
            bpms.push(t.bpm);
            cumulative_seconds.push(cur_sec);
        }

        Ok(Self {
            ppq,
            start_ticks,
            bpms,
            cumulative_seconds,
        })
    }

    /// Query the timeline position in seconds corresponding to an absolute integer tick.
    pub fn seconds_at_tick(&self, tick: u64) -> f64 {
        let mut low = 0;
        let mut high = self.start_ticks.len();
        while low < high {
            let mid = (low + high) / 2;
            if tick < self.start_ticks[mid] {
                high = mid;
            } else {
                low = mid + 1;
            }
        }
        let idx = if low > 0 { low - 1 } else { 0 };
        let base_tick = self.start_ticks[idx];
        let base_bpm = self.bpms[idx];
        let base_sec = self.cumulative_seconds[idx];
        base_sec + ((tick - base_tick) as f64) * 60.0 / (base_bpm * (self.ppq as f64))
    }
}
