/**
 * requestAnimationFrame hooks for smooth animations.
 *
 * Two hooks:
 *   - useAnimationFrame: low-level rAF loop with delta timing
 *   - useAutoStep: rate-limited stepping (N iterations/second)
 */

import { useRef, useEffect, useCallback } from "react";

/**
 * Hook that calls a callback on each animation frame.
 *
 * Uses requestAnimationFrame for smooth, display-synced updates.
 * Automatically pauses when the tab is hidden.
 *
 * @param callback - Called each frame with elapsed time in ms since last frame.
 * @param active - Whether the animation loop is running.
 */
export function useAnimationFrame(
  callback: (deltaMs: number) => void,
  active: boolean,
): void {
  const callbackRef = useRef(callback);
  const lastTimeRef = useRef<number>(0);

  useEffect(() => {
    callbackRef.current = callback;
  }, [callback]);

  useEffect(() => {
    if (!active) return;

    let frameId: number;

    const tick = (timestamp: number) => {
      const delta = lastTimeRef.current ? timestamp - lastTimeRef.current : 0;
      lastTimeRef.current = timestamp;
      callbackRef.current(delta);
      frameId = requestAnimationFrame(tick);
    };

    lastTimeRef.current = 0;
    frameId = requestAnimationFrame(tick);

    return () => {
      cancelAnimationFrame(frameId);
    };
  }, [active]);
}

/**
 * Hook for auto-stepping at a configurable rate.
 *
 * Accumulates fractional steps across frames for smooth timing.
 * At 60fps with 100 steps/sec, executes ~1.67 steps per frame.
 *
 * @param stepFn - Function to execute each step.
 * @param stepsPerSecond - Target execution rate.
 * @param active - Whether auto-stepping is enabled.
 */
export function useAutoStep(
  stepFn: () => void,
  stepsPerSecond: number,
  active: boolean,
): void {
  const accumulatorRef = useRef(0);

  const onFrame = useCallback(
    (deltaMs: number) => {
      const stepsThisFrame =
        (deltaMs / 1000) * stepsPerSecond + accumulatorRef.current;
      const wholeSteps = Math.floor(stepsThisFrame);
      accumulatorRef.current = stepsThisFrame - wholeSteps;

      for (let i = 0; i < wholeSteps; i++) {
        stepFn();
      }
    },
    [stepFn, stepsPerSecond],
  );

  useAnimationFrame(onFrame, active);
}
