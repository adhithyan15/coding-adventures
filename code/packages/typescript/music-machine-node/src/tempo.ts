/**
 * Pre-computed cumulative tempo map with binary search (MUS04 §6).
 * Hardened for CWE-20, CWE-190, and CWE-400.
 */

import { DEFAULT_TEMPO_BPM } from "./constants.js";
import { TempoEvent } from "./types.js";
import {
  validateNonNegative,
  validateNonNegativeInt,
  validatePositiveInt,
} from "./validation.js";

/**
 * Pre-computed cumulative seconds table with O(log T) binary search queries.
 */
export class TempoMap {
  readonly ppq: number;
  private readonly startTicks: number[];
  private readonly bpms: number[];
  private readonly cumulativeSeconds: number[];

  constructor(tempoEvents: readonly TempoEvent[], ppq: number) {
    this.ppq = validatePositiveInt("ppq", ppq);

    const tempos = [...tempoEvents].sort((a, b) => a.startTick - b.startTick);
    if (tempos.length === 0 || tempos[0].startTick !== 0) {
      tempos.unshift(new TempoEvent(0, DEFAULT_TEMPO_BPM));
    }

    const n = tempos.length;
    const startTicks = new Array<number>(n);
    const bpms = new Array<number>(n);
    const cumulativeSeconds = new Array<number>(n);

    let curSec = 0.0;
    for (let i = 0; i < n; i += 1) {
      const t = tempos[i];
      if (i > 0) {
        const prevTick = startTicks[i - 1];
        const prevBpm = bpms[i - 1];
        if (t.startTick < prevTick) {
          throw new RangeError("tempo start_ticks out of order");
        }
        const deltaTicks = t.startTick - prevTick;
        curSec += (deltaTicks * 60.0) / (prevBpm * this.ppq);
      }
      startTicks[i] = t.startTick;
      bpms[i] = t.bpm;
      cumulativeSeconds[i] = curSec;
    }

    this.startTicks = startTicks;
    this.bpms = bpms;
    this.cumulativeSeconds = cumulativeSeconds;
  }

  /**
   * Query the timeline position in seconds corresponding to an absolute integer tick.
   */
  secondsAtTick(tick: number): number {
    validateNonNegativeInt("tick", tick);

    let low = 0;
    let high = this.startTicks.length;
    while (low < high) {
      const mid = Math.floor((low + high) / 2);
      if (tick < this.startTicks[mid]) {
        high = mid;
      } else {
        low = mid + 1;
      }
    }

    const idx = low > 0 ? low - 1 : 0;
    const baseTick = this.startTicks[idx];
    const baseBpm = this.bpms[idx];
    const baseSec = this.cumulativeSeconds[idx];

    return baseSec + ((tick - baseTick) * 60.0) / (baseBpm * this.ppq);
  }

  /**
   * Approximate tick position for given seconds elapsed.
   */
  tickAtSeconds(seconds: number): number {
    validateNonNegative("seconds", seconds);

    let low = 0;
    let high = this.cumulativeSeconds.length;
    while (low < high) {
      const mid = Math.floor((low + high) / 2);
      if (seconds < this.cumulativeSeconds[mid]) {
        high = mid;
      } else {
        low = mid + 1;
      }
    }

    const idx = low > 0 ? low - 1 : 0;
    const baseTick = this.startTicks[idx];
    const baseBpm = this.bpms[idx];
    const baseSec = this.cumulativeSeconds[idx];

    const deltaSec = seconds - baseSec;
    const deltaTicks = Math.round((deltaSec * baseBpm * this.ppq) / 60.0);
    return baseTick + Math.max(0, deltaTicks);
  }
}
