//! Multi-track polyphonic PCM rendering and mixing pipeline (MUS01 / MUS03 / MUS04 / MUS07).

use std::collections::HashMap;

use musical_instruments::{render_instrument_note, RenderOptions};
use oscillator::sample_count_for_duration;
use pcm_audio::{PcmAudio, PcmFormat};
use percussion_kit::{
    get_drum_kit, render_percussion_sequence, PercussionHitEvent, SequenceRenderOptions,
};

use crate::error::MusicMachineError;
use crate::limits::*;
use crate::tempo::TempoMap;
use crate::types::*;

/// Mix a source PCM sample slice into a target buffer at an offset using saturating 16-bit arithmetic.
///
/// Samples outside $[-32768, 32767]$ saturate to boundary values and increment the clipped count.
pub fn mix_pcm_samples(target: &mut Vec<i16>, start_sample: usize, source: &[i16]) -> usize {
    let required_count = start_sample.saturating_add(source.len());
    if target.len() < required_count {
        target.resize(required_count, 0);
    }

    let mut clipped_count = 0;
    for (offset, &sample) in source.iter().enumerate() {
        let target_index = start_sample + offset;
        let mixed = (target[target_index] as i32) + (sample as i32);
        if mixed > 32_767 {
            target[target_index] = 32_767;
            clipped_count += 1;
        } else if mixed < -32_768 {
            target[target_index] = -32_768;
            clipped_count += 1;
        } else {
            target[target_index] = mixed as i16;
        }
    }
    clipped_count
}

/// Render a parsed beginner `TextScore` into a `RenderedScore`.
pub fn render_score_to_pcm(
    score: &TextScore,
    max_sample_count: usize,
) -> Result<RenderedScore, MusicMachineError> {
    let sample_limit = if max_sample_count == 0 {
        DEFAULT_MAX_SAMPLE_COUNT
    } else {
        max_sample_count
    };
    if sample_limit > MAX_SAMPLE_COUNT {
        return Err(MusicMachineError::BudgetExceeded(format!(
            "max_sample_count {sample_limit} exceeds MAX_SAMPLE_COUNT={MAX_SAMPLE_COUNT}"
        )));
    }

    let total_dur = score.total_duration_seconds();
    if total_dur > MAX_DURATION_SECONDS {
        return Err(MusicMachineError::BudgetExceeded(format!(
            "total score duration {total_dur}s exceeds MAX_DURATION_SECONDS={MAX_DURATION_SECONDS}"
        )));
    }

    let rate = score.sample_rate_hz as f64;
    let pre_samples = (total_dur * rate).round() as usize;
    if pre_samples > sample_limit {
        return Err(MusicMachineError::BudgetExceeded(format!(
            "score render would require {pre_samples} samples, above max_sample_count={sample_limit}"
        )));
    }

    let mut pcm_samples: Vec<i16> = Vec::new();
    let mut rendered_notes = Vec::new();
    let mut rendered_hits = Vec::new();
    let mut clipped_sample_count = 0;

    // Check if score is entirely drum hits and rests to apply hi-hat choke sequence
    let has_hits = score.events.iter().any(|e| e.kind == ScoreEventKind::Hit);
    let all_hits_or_rests = score
        .events
        .iter()
        .all(|e| e.kind == ScoreEventKind::Hit || e.kind == ScoreEventKind::Rest);

    if has_hits && all_hits_or_rests {
        let mut kit_events = Vec::new();
        let mut cur_t = 0.0;
        for ev in &score.events {
            if ev.kind == ScoreEventKind::Hit {
                if let Some(ref hit_id) = ev.hit_id {
                    kit_events.push(PercussionHitEvent::new(
                        hit_id.clone(),
                        cur_t,
                        ev.duration_seconds,
                        score.amplitude,
                        "drums",
                        None,
                    )?);
                }
            }
            cur_t += ev.duration_seconds;
        }

        let kit_name = score.drumkit.as_deref().unwrap_or("standard_kit");
        let kit_prof = get_drum_kit(kit_name)?;
        let seq_opts = SequenceRenderOptions {
            kit: Some(&kit_prof),
            sample_rate_hz: rate,
            max_sample_count: sample_limit,
            ..Default::default()
        };

        let percussion_render = render_percussion_sequence(&kit_events, &seq_opts)?;
        let mut total_samples = percussion_render.pcm_buffer.samples().to_vec();
        let min_required = sample_count_for_duration(total_dur, rate)?;
        if total_samples.len() < min_required {
            total_samples.resize(min_required, 0);
        }

        clipped_sample_count += percussion_render.pcm_buffer.clipped_sample_count();
        let format = PcmFormat::mono_16bit(rate)?;
        let pcm_buffer = PcmAudio::from_i16(total_samples, format, 0.0)?;
        rendered_hits.extend(percussion_render.events);

        return Ok(RenderedScore {
            score: score.clone(),
            pcm_buffer,
            rendered_notes: Vec::new(),
            rendered_hits,
            clipped_sample_count,
        });
    }

    let mut cursor_sample_index: usize = 0;
    let mut elapsed_seconds = 0.0;

    for ev in &score.events {
        match ev.kind {
            ScoreEventKind::Rest => {
                let silence_count = sample_count_for_duration(ev.duration_seconds, rate)?;
                cursor_sample_index = cursor_sample_index
                    .checked_add(silence_count)
                    .ok_or_else(|| MusicMachineError::BudgetExceeded("cursor_sample_index overflow".into()))?;
                if cursor_sample_index > sample_limit {
                    return Err(MusicMachineError::BudgetExceeded(format!(
                        "cursor_sample_index {cursor_sample_index} exceeds max_sample_count={sample_limit}"
                    )));
                }
                if pcm_samples.len() < cursor_sample_index {
                    pcm_samples.resize(cursor_sample_index, 0);
                }
            }
            ScoreEventKind::Hit => {
                let hit_id = ev.hit_id.as_deref().unwrap_or("kick");
                let kit_event = PercussionHitEvent::new(
                    hit_id,
                    0.0,
                    ev.duration_seconds,
                    score.amplitude,
                    "drums",
                    None,
                )?;
                let kit_name = score.drumkit.as_deref().unwrap_or("standard_kit");
                let kit_prof = get_drum_kit(kit_name)?;
                let seq_opts = SequenceRenderOptions {
                    kit: Some(&kit_prof),
                    sample_rate_hz: rate,
                    max_sample_count: sample_limit,
                    ..Default::default()
                };

                let seq_render = render_percussion_sequence(&[kit_event], &seq_opts)?;
                clipped_sample_count += seq_render.pcm_buffer.clipped_sample_count();
                clipped_sample_count += mix_pcm_samples(
                    &mut pcm_samples,
                    cursor_sample_index,
                    seq_render.pcm_buffer.samples(),
                );
                let hit_samples = sample_count_for_duration(ev.duration_seconds, rate)?;
                cursor_sample_index = cursor_sample_index
                    .checked_add(hit_samples)
                    .ok_or_else(|| MusicMachineError::BudgetExceeded("cursor_sample_index overflow".into()))?;
                if cursor_sample_index > sample_limit {
                    return Err(MusicMachineError::BudgetExceeded(format!(
                        "cursor_sample_index {cursor_sample_index} exceeds max_sample_count={sample_limit}"
                    )));
                }
                rendered_hits.extend(seq_render.events);
            }
            ScoreEventKind::Note => {
                let note_str = ev.note.as_deref().unwrap_or("A4");
                let mut opts = RenderOptions::new();
                opts = opts.sample_rate_hz(rate);
                opts = opts.amplitude(score.amplitude);
                opts = opts.instrument_id(&score.instrument_id);
                opts = opts.start_time_seconds(elapsed_seconds);
                opts = opts.max_sample_count(sample_limit);

                let rendered_note = render_instrument_note(note_str, ev.duration_seconds, &opts)?;
                clipped_sample_count += rendered_note.pcm_buffer.clipped_sample_count();
                clipped_sample_count += mix_pcm_samples(
                    &mut pcm_samples,
                    cursor_sample_index,
                    rendered_note.pcm_buffer.samples(),
                );
                let note_samples = sample_count_for_duration(ev.duration_seconds, rate)?;
                cursor_sample_index = cursor_sample_index
                    .checked_add(note_samples)
                    .ok_or_else(|| MusicMachineError::BudgetExceeded("cursor_sample_index overflow".into()))?;
                if cursor_sample_index > sample_limit {
                    return Err(MusicMachineError::BudgetExceeded(format!(
                        "cursor_sample_index {cursor_sample_index} exceeds max_sample_count={sample_limit}"
                    )));
                }
                rendered_notes.push(rendered_note);
            }
        }
        elapsed_seconds += ev.duration_seconds;
    }

    let format = PcmFormat::mono_16bit(rate)?;
    let pcm_buffer = PcmAudio::from_i16(pcm_samples, format, 0.0)?;

    Ok(RenderedScore {
        score: score.clone(),
        pcm_buffer,
        rendered_notes,
        rendered_hits,
        clipped_sample_count,
    })
}

/// Render a parsed portable multi-track `PortableScore` into a `RenderedPortableScore`.
pub fn render_portable_score_to_pcm(
    score: &PortableScore,
    max_sample_count: usize,
) -> Result<RenderedPortableScore, MusicMachineError> {
    let sample_limit = if max_sample_count == 0 {
        DEFAULT_MAX_SAMPLE_COUNT
    } else {
        max_sample_count
    };
    if sample_limit > MAX_SAMPLE_COUNT {
        return Err(MusicMachineError::BudgetExceeded(format!(
            "max_sample_count {sample_limit} exceeds MAX_SAMPLE_COUNT={MAX_SAMPLE_COUNT}"
        )));
    }

    let mut instrument_by_id = HashMap::new();
    for inst in &score.instruments {
        instrument_by_id.insert(inst.id.as_str(), inst);
    }
    let mut track_by_id = HashMap::new();
    for tr in &score.tracks {
        track_by_id.insert(tr.id.as_str(), tr);
    }

    let tempo_map = TempoMap::new(&score.tempo_events, score.ppq)?;
    let rate = score.sample_rate_hz as f64;

    let mut last_tick: u64 = 0;
    for e in &score.events {
        let end_tick = e.start_tick.checked_add(e.duration_tick).ok_or_else(|| {
            MusicMachineError::BudgetExceeded(format!(
                "event start_tick {} + duration_tick {} overflows u64",
                e.start_tick, e.duration_tick
            ))
        })?;
        if end_tick > last_tick {
            last_tick = end_tick;
        }
    }
    let total_dur = tempo_map.seconds_at_tick(last_tick);
    if total_dur > MAX_DURATION_SECONDS {
        return Err(MusicMachineError::BudgetExceeded(format!(
            "score duration {total_dur}s exceeds MAX_DURATION_SECONDS={MAX_DURATION_SECONDS}"
        )));
    }

    let pre_samples = (total_dur * rate).round() as usize;
    if pre_samples > sample_limit {
        return Err(MusicMachineError::BudgetExceeded(format!(
            "score render would require {pre_samples} samples, above max_sample_count={sample_limit}"
        )));
    }

    // Validate track/instrument kind compatibility
    for ev in &score.events {
        let tr = track_by_id.get(ev.track_id.as_str()).ok_or_else(|| {
            MusicMachineError::InvalidArgument(format!("event references unknown track '{}'", ev.track_id))
        })?;
        let inst = instrument_by_id.get(tr.instrument_id.as_str()).ok_or_else(|| {
            MusicMachineError::InvalidArgument(format!("track '{}' references unknown instrument '{}'", tr.id, tr.instrument_id))
        })?;

        if ev.kind == ScoreEventKind::Note && inst.kind == "drumkit" {
            return Err(MusicMachineError::InvalidArgument(format!(
                "track '{}' uses drumkit instrument '{}' and cannot play note events",
                tr.id, inst.id
            )));
        }
        if ev.kind == ScoreEventKind::Hit && inst.kind != "drumkit" {
            return Err(MusicMachineError::InvalidArgument(format!(
                "track '{}' uses pitched instrument '{}' and cannot play hit events",
                tr.id, inst.id
            )));
        }
    }

    let mut pcm_samples: Vec<i16> = Vec::new();
    let mut rendered_notes = Vec::new();
    let mut rendered_hits = Vec::new();
    let mut clipped_sample_count = 0;

    // 1. Render Pitched Notes
    for ev in &score.events {
        if ev.kind != ScoreEventKind::Note {
            continue;
        }

        let end_tick = ev.start_tick.checked_add(ev.duration_tick).ok_or_else(|| {
            MusicMachineError::BudgetExceeded(format!(
                "note event start_tick {} + duration_tick {} overflows u64",
                ev.start_tick, ev.duration_tick
            ))
        })?;
        let start_seconds = tempo_map.seconds_at_tick(ev.start_tick);
        let end_seconds = tempo_map.seconds_at_tick(end_tick);
        let duration_seconds = end_seconds - start_seconds;
        let start_sample = sample_count_for_duration(start_seconds, rate)?;

        let tr = track_by_id[ev.track_id.as_str()];
        let inst = instrument_by_id[tr.instrument_id.as_str()];
        let divisor = if ev.notes.is_empty() { 1.0 } else { ev.notes.len() as f64 };
        let note_amplitude = ev.velocity * inst.gain / divisor;

        for note in &ev.notes {
            let mut opts = RenderOptions::new();
            opts = opts.sample_rate_hz(rate);
            opts = opts.amplitude(note_amplitude);
            opts = opts.start_time_seconds(0.0);
            opts = opts.max_sample_count(sample_limit);

            if let Some(ref prof) = inst.custom_profile {
                opts = opts.instrument(prof.clone());
            } else {
                opts = opts.instrument_id(&inst.profile_id);
            }

            let rendered_note = render_instrument_note(note.as_str(), duration_seconds, &opts)?;
            clipped_sample_count += rendered_note.pcm_buffer.clipped_sample_count();

            let required_count = start_sample.saturating_add(rendered_note.pcm_buffer.sample_count());
            if required_count > sample_limit {
                return Err(MusicMachineError::BudgetExceeded(format!(
                    "render would create {required_count} samples, above max_sample_count={sample_limit}"
                )));
            }

            clipped_sample_count += mix_pcm_samples(
                &mut pcm_samples,
                start_sample,
                rendered_note.pcm_buffer.samples(),
            );
            if pcm_samples.len() > sample_limit {
                return Err(MusicMachineError::BudgetExceeded(format!(
                    "render would create {} samples, above max_sample_count={sample_limit}",
                    pcm_samples.len()
                )));
            }
            rendered_notes.push(rendered_note);
        }
    }

    // 2. Render Drum Hits with Hi-Hat Choking (MUS07 §7 & §9)
    let mut drum_kits_present = Vec::new();
    for inst in &score.instruments {
        if inst.kind == "drumkit" {
            let kit_name = inst.kit.as_deref().unwrap_or("standard_kit").to_string();
            if !drum_kits_present.contains(&kit_name) {
                drum_kits_present.push(kit_name);
            }
        }
    }

    for kit_name in drum_kits_present {
        let mut kit_events = Vec::new();
        for ev in &score.events {
            if ev.kind != ScoreEventKind::Hit {
                continue;
            }
            let tr = track_by_id[ev.track_id.as_str()];
            let inst = instrument_by_id[tr.instrument_id.as_str()];
            let inst_kit = inst.kit.as_deref().unwrap_or("standard_kit");
            if inst_kit != kit_name {
                continue;
            }

            let end_tick = ev.start_tick.checked_add(ev.duration_tick).ok_or_else(|| {
                MusicMachineError::BudgetExceeded(format!(
                    "hit event start_tick {} + duration_tick {} overflows u64",
                    ev.start_tick, ev.duration_tick
                ))
            })?;
            let start_seconds = tempo_map.seconds_at_tick(ev.start_tick);
            let end_seconds = tempo_map.seconds_at_tick(end_tick);
            let duration_seconds = end_seconds - start_seconds;
            let hit_id = ev.hit_id.as_deref().unwrap_or("kick");

            kit_events.push(PercussionHitEvent::new(
                hit_id,
                start_seconds,
                duration_seconds,
                ev.velocity * inst.gain,
                ev.track_id.clone(),
                None,
            )?);
        }

        if !kit_events.is_empty() {
            let kit_prof = get_drum_kit(&kit_name)?;
            let seq_opts = SequenceRenderOptions {
                kit: Some(&kit_prof),
                sample_rate_hz: rate,
                max_sample_count: sample_limit,
                ..Default::default()
            };

            let seq_render = render_percussion_sequence(&kit_events, &seq_opts)?;
            clipped_sample_count += seq_render.pcm_buffer.clipped_sample_count();
            clipped_sample_count += mix_pcm_samples(
                &mut pcm_samples,
                0,
                seq_render.pcm_buffer.samples(),
            );
            rendered_hits.extend(seq_render.events);
        }
    }

    // 3. Handle Rest Bounds
    for ev in &score.events {
        if ev.kind == ScoreEventKind::Rest {
            let end_tick = ev.start_tick.checked_add(ev.duration_tick).ok_or_else(|| {
                MusicMachineError::BudgetExceeded(format!(
                    "rest event start_tick {} + duration_tick {} overflows u64",
                    ev.start_tick, ev.duration_tick
                ))
            })?;
            let end_seconds = tempo_map.seconds_at_tick(end_tick);
            let rest_end = sample_count_for_duration(end_seconds, rate)?;
            if rest_end > sample_limit {
                return Err(MusicMachineError::BudgetExceeded(format!(
                    "render would create {rest_end} samples, above max_sample_count={sample_limit}"
                )));
            }
            if pcm_samples.len() < rest_end {
                pcm_samples.resize(rest_end, 0);
            }
        }
    }

    if pcm_samples.len() > sample_limit {
        return Err(MusicMachineError::BudgetExceeded(format!(
            "render would create {} samples, above max_sample_count={sample_limit}",
            pcm_samples.len()
        )));
    }

    let format = PcmFormat::mono_16bit(rate)?;
    let pcm_buffer = PcmAudio::from_i16(pcm_samples, format, 0.0)?;

    Ok(RenderedPortableScore {
        score: score.clone(),
        pcm_buffer,
        rendered_notes,
        rendered_hits,
        clipped_sample_count,
    })
}
