import {ClockOverflowError} from "./errors.js";

/** Portable clock ceiling so TypeScript and unbounded-integer ports agree. */
export const MAX_SAFE_TICK = Number.MAX_SAFE_INTEGER;

export interface IoTiming {
  readonly startTick: number;
  readonly seekTicks: number;
  readonly rotationTicks: number;
  readonly transferTicks: number;
  readonly totalTicks: number;
  readonly endTick: number;
}

/** Internal exact timing calculator, also used to probe the portable ceiling. */
export function calculateIoTiming(
  startTick: number,
  headTrack: number,
  targetTrack: number,
  targetSector: number,
  seekTicksPerTrack: number,
  sectorsPerTrack: number,
  transferTicksPerSector: number,
): IoTiming {
  const seekTicks = Math.abs(targetTrack - headTrack) * seekTicksPerTrack;
  const phaseAfterSeek = (startTick % sectorsPerTrack + seekTicks) % sectorsPerTrack;
  const rotationTicks = (targetSector - phaseAfterSeek + sectorsPerTrack) % sectorsPerTrack;
  const transferTicks = transferTicksPerSector;
  const totalTicks = seekTicks + rotationTicks + transferTicks;
  if (totalTicks > MAX_SAFE_TICK - startTick) {
    throw new ClockOverflowError("operation would exceed the maximum exact tick");
  }
  const endTick = startTick + totalTicks;
  return Object.freeze({
    startTick,
    seekTicks,
    rotationTicks,
    transferTicks,
    totalTicks,
    endTick,
  });
}
