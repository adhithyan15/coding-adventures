/**
 * requestAnimationFrame hook for auto-stepping the CPU.
 *
 * Used by the "Free Run" mode in the CPU view: executes N instructions
 * per animation frame at an adjustable speed.
 *
 * === Why requestAnimationFrame? ===
 *
 * We want the CPU to execute instructions at a visible speed so users
 * can watch the registers change. setInterval would work but doesn't
 * sync with the display refresh rate. requestAnimationFrame ensures
 * smooth visual updates and automatically pauses when the tab is hidden.
 */

import { useRef, useEffect, useCallback } from "react";

/**
 * Hook that calls a callback on each animation frame.
 *
 * @param callback - Function to call each frame. Receives elapsed time in ms.
 * @param active - Whether the animation loop is running.
 */
export function useAnimationFrame(
  callback: (deltaMs: number) => void,
  active: boolean,
): void {
  const callbackRef = useRef(callback);
  const lastTimeRef = useRef<number>(0);

  // Keep callback ref current without re-subscribing
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
 * Hook for auto-stepping the CPU at a configurable rate.
 *
 * @param stepFn - Function that executes one CPU instruction.
 * @param instructionsPerSecond - Target execution rate.
 * @param active - Whether auto-stepping is enabled.
 */
export function useAutoStep(
  stepFn: () => void,
  instructionsPerSecond: number,
  active: boolean,
): void {
  const accumulatorRef = useRef(0);

  const onFrame = useCallback(
    (deltaMs: number) => {
      // Calculate how many instructions to execute this frame
      const instructionsThisFrame =
        (deltaMs / 1000) * instructionsPerSecond + accumulatorRef.current;
      const wholeInstructions = Math.floor(instructionsThisFrame);
      accumulatorRef.current = instructionsThisFrame - wholeInstructions;

      for (let i = 0; i < wholeInstructions; i++) {
        stepFn();
      }
    },
    [stepFn, instructionsPerSecond],
  );

  useAnimationFrame(onFrame, active);
}
