/**
 * Drum kit profiles, 15 standard starter voices, and canonical alias resolution (MUS07 §5.3, §6, §11).
 */

import { PercussionEnvelope } from "./envelope.js";
import { PercussionMode } from "./mode.js";
import { PercussionNoiseProfile } from "./noise.js";
import { PercussionVoiceProfile } from "./profile.js";
import { validateNonEmptyString } from "./validation.js";

export function canonicalHitId(input: string): string | null {
  if (typeof input !== "string") {
    return null;
  }
  const normalized = input.trim().toLowerCase().replaceAll("-", "_");

  switch (normalized) {
    case "kick":
    case "kick_naive":
    case "bd":
    case "bass_drum":
    case "bass":
    case "kick_drum":
      return "kick";
    case "snare":
    case "snare_naive":
    case "sd":
    case "snare_drum":
    case "snare_acoustic":
      return "snare";
    case "closed_hihat":
    case "closed_hihat_naive":
    case "closed_hat":
    case "hat_closed":
    case "ch":
      return "closed_hihat";
    case "open_hihat":
    case "open_hihat_naive":
    case "open_hat":
    case "hat_open":
    case "oh":
      return "open_hihat";
    case "pedal_hihat":
    case "pedal_hihat_naive":
    case "pedal_hat":
    case "hat_pedal":
    case "foot_hihat":
    case "foot_hat":
    case "ph":
      return "pedal_hihat";
    case "low_tom":
    case "low_tom_naive":
    case "floor_tom":
    case "tom_low":
    case "lt":
      return "low_tom";
    case "mid_tom":
    case "mid_tom_naive":
    case "tom_mid":
    case "mt":
      return "mid_tom";
    case "high_tom":
    case "high_tom_naive":
    case "tom_high":
    case "ht":
      return "high_tom";
    case "crash":
    case "crash_naive":
    case "crash_cymbal":
    case "cc":
      return "crash";
    case "ride":
    case "ride_naive":
    case "ride_cymbal":
    case "rc":
      return "ride";
    case "rimshot":
    case "rimshot_naive":
    case "rim":
    case "side_stick":
    case "sidestick":
    case "rs":
      return "rimshot";
    case "clap":
    case "clap_naive":
    case "hand_clap":
    case "handclap":
    case "cp":
      return "clap";
    case "cowbell":
    case "cowbell_naive":
    case "cb":
      return "cowbell";
    case "tambourine":
    case "tambourine_naive":
    case "tamb":
    case "tb":
      return "tambourine";
    case "shaker":
    case "shaker_naive":
    case "sh":
      return "shaker";
    case "woodblock":
    case "woodblock_naive":
    case "wood_block":
    case "wb":
      return "woodblock";
    default:
      return null;
  }
}

function mode(freq: number, amp: number, dec: number, phase = 0.0): PercussionMode {
  return new PercussionMode(freq, amp, dec, phase);
}

function env(att: number, hld: number, dec: number): PercussionEnvelope {
  return new PercussionEnvelope(att, hld, dec);
}

function noise(amp: number, dec: number, seed: number | bigint): PercussionNoiseProfile {
  return new PercussionNoiseProfile(amp, dec, seed);
}

// 1. Kick
export function kickNaive(): PercussionVoiceProfile {
  return new PercussionVoiceProfile(
    "kick_naive",
    "Naive Kick",
    "kick",
    0.9,
    [mode(58.0, 1.0, 0.2), mode(94.0, 0.35, 0.12), mode(142.0, 0.1, 0.06)],
    noise(0.08, 0.015, 11),
    env(0.0, 0.015, 0.22),
    0.55,
  );
}

// 2. Snare
export function snareNaive(): PercussionVoiceProfile {
  return new PercussionVoiceProfile(
    "snare_naive",
    "Naive Snare",
    "snare",
    0.75,
    [mode(180.0, 0.3, 0.16), mode(330.0, 0.15, 0.09)],
    noise(0.95, 0.11, 23),
    env(0.0, 0.008, 0.18),
    0.28,
  );
}

// 3. Closed Hi-Hat
export function closedHihatNaive(): PercussionVoiceProfile {
  return new PercussionVoiceProfile(
    "closed_hihat_naive",
    "Naive Closed Hi-Hat",
    "hat",
    0.45,
    [mode(6200.0, 0.24, 0.05), mode(9100.0, 0.18, 0.04)],
    noise(0.92, 0.05, 31),
    env(0.0, 0.003, 0.06),
    0.1,
    "hihat",
  );
}

// 4. Open Hi-Hat
export function openHihatNaive(): PercussionVoiceProfile {
  return new PercussionVoiceProfile(
    "open_hihat_naive",
    "Naive Open Hi-Hat",
    "hat",
    0.42,
    [mode(5900.0, 0.24, 0.2), mode(8700.0, 0.16, 0.24)],
    noise(0.95, 0.26, 37),
    env(0.0, 0.004, 0.32),
    0.08,
    "hihat",
  );
}

// 5. Pedal Hi-Hat
export function pedalHihatNaive(): PercussionVoiceProfile {
  return new PercussionVoiceProfile(
    "pedal_hihat_naive",
    "Naive Pedal Hi-Hat",
    "hat",
    0.4,
    [mode(5600.0, 0.18, 0.06), mode(8200.0, 0.12, 0.05)],
    noise(0.78, 0.07, 41),
    env(0.0, 0.002, 0.08),
    0.12,
    "hihat",
  );
}

// 6. Low Tom
export function lowTomNaive(): PercussionVoiceProfile {
  return new PercussionVoiceProfile(
    "low_tom_naive",
    "Naive Low Tom",
    "tom",
    0.72,
    [mode(110.0, 1.0, 0.22), mode(170.0, 0.28, 0.16), mode(250.0, 0.1, 0.1)],
    noise(0.1, 0.03, 43),
    env(0.0, 0.01, 0.2),
    0.2,
  );
}

// 7. Middle Tom
export function midTomNaive(): PercussionVoiceProfile {
  return new PercussionVoiceProfile(
    "mid_tom_naive",
    "Naive Mid Tom",
    "tom",
    0.7,
    [mode(150.0, 1.0, 0.2), mode(225.0, 0.25, 0.14), mode(320.0, 0.08, 0.08)],
    noise(0.08, 0.025, 47),
    env(0.0, 0.008, 0.18),
    0.18,
  );
}

// 8. High Tom
export function highTomNaive(): PercussionVoiceProfile {
  return new PercussionVoiceProfile(
    "high_tom_naive",
    "Naive High Tom",
    "tom",
    0.68,
    [mode(205.0, 1.0, 0.17), mode(295.0, 0.22, 0.12), mode(405.0, 0.07, 0.07)],
    noise(0.06, 0.02, 53),
    env(0.0, 0.007, 0.15),
    0.16,
  );
}

// 9. Crash
export function crashNaive(): PercussionVoiceProfile {
  return new PercussionVoiceProfile(
    "crash_naive",
    "Naive Crash",
    "cymbal",
    0.38,
    [mode(3200.0, 0.18, 0.35), mode(5100.0, 0.12, 0.48), mode(7900.0, 0.08, 0.55)],
    noise(1.0, 0.6, 59),
    env(0.0, 0.004, 0.65),
    0.12,
  );
}

// 10. Ride
export function rideNaive(): PercussionVoiceProfile {
  return new PercussionVoiceProfile(
    "ride_naive",
    "Naive Ride",
    "cymbal",
    0.36,
    [mode(2600.0, 0.14, 0.32), mode(4100.0, 0.1, 0.44), mode(6500.0, 0.08, 0.5)],
    noise(0.82, 0.42, 61),
    env(0.0, 0.004, 0.45),
    0.1,
  );
}

// 11. Clap
export function clapNaive(): PercussionVoiceProfile {
  return new PercussionVoiceProfile(
    "clap_naive",
    "Naive Clap",
    "hand_percussion",
    0.7,
    [mode(800.0, 0.4, 0.06), mode(1250.0, 0.35, 0.05), mode(2500.0, 0.2, 0.03)],
    noise(0.88, 0.08, 67),
    env(0.0, 0.008, 0.14),
    0.35,
  );
}

// 12. Cowbell
export function cowbellNaive(): PercussionVoiceProfile {
  return new PercussionVoiceProfile(
    "cowbell_naive",
    "Naive Cowbell",
    "hand_percussion",
    0.65,
    [mode(587.0, 1.0, 0.25), mode(845.0, 0.7, 0.2), mode(1450.0, 0.15, 0.1)],
    noise(0.08, 0.015, 71),
    env(0.0, 0.004, 0.28),
    0.45,
  );
}

// 13. Tambourine
export function tambourineNaive(): PercussionVoiceProfile {
  return new PercussionVoiceProfile(
    "tambourine_naive",
    "Naive Tambourine",
    "hand_percussion",
    0.45,
    [mode(4800.0, 0.25, 0.12), mode(7200.0, 0.2, 0.15), mode(10500.0, 0.15, 0.18)],
    noise(0.9, 0.16, 73),
    env(0.0, 0.003, 0.22),
    0.22,
  );
}

// 14. Shaker
export function shakerNaive(): PercussionVoiceProfile {
  return new PercussionVoiceProfile(
    "shaker_naive",
    "Naive Shaker",
    "hand_percussion",
    0.4,
    [mode(3200.0, 0.08, 0.08), mode(6400.0, 0.06, 0.06)],
    noise(0.95, 0.1, 79),
    env(0.01, 0.005, 0.12),
    0.04,
  );
}

// 15. Woodblock
export function woodblockNaive(): PercussionVoiceProfile {
  return new PercussionVoiceProfile(
    "woodblock_naive",
    "Naive Woodblock",
    "woodblock",
    0.72,
    [mode(780.0, 1.0, 0.04), mode(1650.0, 0.3, 0.025), mode(2800.0, 0.1, 0.015)],
    noise(0.05, 0.01, 83),
    env(0.0, 0.002, 0.055),
    0.6,
  );
}

// Rimshot
export function rimshotNaive(): PercussionVoiceProfile {
  return new PercussionVoiceProfile(
    "rimshot_naive",
    "Naive Rimshot",
    "snare",
    0.78,
    [mode(450.0, 0.6, 0.08), mode(920.0, 0.3, 0.04)],
    noise(0.4, 0.03, 89),
    env(0.0, 0.003, 0.07),
    0.5,
  );
}

export class DrumKitProfile {
  readonly id: string;
  readonly displayName: string;
  readonly voicesByHitId: ReadonlyMap<string, PercussionVoiceProfile>;

  constructor(
    id: string,
    displayName: string,
    voicesByHitId: ReadonlyMap<string, PercussionVoiceProfile> | Record<string, PercussionVoiceProfile>,
  ) {
    this.id = validateNonEmptyString("kit id", id);
    this.displayName = validateNonEmptyString("kit displayName", displayName);

    const map = new Map<string, PercussionVoiceProfile>();
    if (voicesByHitId instanceof Map) {
      for (const [k, v] of voicesByHitId.entries()) {
        const key = validateNonEmptyString("kit voice key", k);
        if (!(v instanceof PercussionVoiceProfile)) {
          throw new TypeError(`kit voice "${key}" must be a PercussionVoiceProfile`);
        }
        map.set(key, v);
      }
    } else if (typeof voicesByHitId === "object" && voicesByHitId !== null) {
      for (const [k, v] of Object.entries(voicesByHitId)) {
        const key = validateNonEmptyString("kit voice key", k);
        if (!(v instanceof PercussionVoiceProfile)) {
          throw new TypeError(`kit voice "${key}" must be a PercussionVoiceProfile`);
        }
        map.set(key, v);
      }
    } else {
      throw new TypeError("voicesByHitId must be a Map or object");
    }

    if (map.size === 0) {
      throw new RangeError("DrumKitProfile must contain at least one voice profile");
    }

    this.voicesByHitId = map;
  }

  get voiceCount(): number {
    return this.voicesByHitId.size;
  }

  resolveHitId(hitId: string): string | null {
    if (typeof hitId !== "string") {
      return null;
    }
    if (this.voicesByHitId.has(hitId)) {
      return hitId;
    }
    const canon = canonicalHitId(hitId);
    if (canon && this.voicesByHitId.has(canon)) {
      return canon;
    }
    return null;
  }

  hasVoice(hitId: string): boolean {
    return this.resolveHitId(hitId) !== null;
  }

  getVoice(hitId: string): PercussionVoiceProfile {
    const resolved = this.resolveHitId(hitId);
    if (!resolved) {
      throw new RangeError(`Unknown percussion voice hitId: "${hitId}" in kit "${this.id}"`);
    }
    return this.voicesByHitId.get(resolved)!;
  }
}

export function standardKit(): DrumKitProfile {
  const map = new Map<string, PercussionVoiceProfile>([
    ["kick", kickNaive()],
    ["snare", snareNaive()],
    ["closed_hihat", closedHihatNaive()],
    ["open_hihat", openHihatNaive()],
    ["pedal_hihat", pedalHihatNaive()],
    ["low_tom", lowTomNaive()],
    ["mid_tom", midTomNaive()],
    ["high_tom", highTomNaive()],
    ["crash", crashNaive()],
    ["ride", rideNaive()],
    ["rimshot", rimshotNaive()],
    ["clap", clapNaive()],
    ["cowbell", cowbellNaive()],
    ["tambourine", tambourineNaive()],
    ["shaker", shakerNaive()],
    ["woodblock", woodblockNaive()],
  ]);
  return new DrumKitProfile("standard_kit", "Standard Kit", map);
}

const KITS: ReadonlyMap<string, () => DrumKitProfile> = new Map([
  ["standard_kit", standardKit],
]);

export function getDrumKit(id: string): DrumKitProfile {
  validateNonEmptyString("kit id", id);
  const factory = KITS.get(id);
  if (!factory) {
    throw new RangeError(`Unknown drum kit id: "${id}"`);
  }
  return factory();
}

export function allDrumKits(): readonly DrumKitProfile[] {
  return [standardKit()];
}

export function getPercussionVoice(
  hitId: string,
  kit?: DrumKitProfile,
): PercussionVoiceProfile {
  validateNonEmptyString("hitId", hitId);
  const activeKit = kit ?? standardKit();

  const direct = activeKit.voicesByHitId.get(hitId);
  if (direct) {
    return direct;
  }

  const canon = canonicalHitId(hitId);
  if (canon) {
    const fromCanon = activeKit.voicesByHitId.get(canon);
    if (fromCanon) {
      return fromCanon;
    }
  }

  throw new RangeError(`Unknown percussion voice hitId: "${hitId}" in kit "${activeKit.id}"`);
}

export function allStandardHitIds(): readonly string[] {
  return [
    "kick",
    "snare",
    "closed_hihat",
    "open_hihat",
    "pedal_hihat",
    "low_tom",
    "mid_tom",
    "high_tom",
    "crash",
    "ride",
    "clap",
    "cowbell",
    "tambourine",
    "shaker",
    "woodblock",
  ];
}
