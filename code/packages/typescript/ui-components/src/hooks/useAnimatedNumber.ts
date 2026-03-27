import { useEffect, useRef, useState } from "react";

function easeInOutCubic(t: number): number {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
}

export function useAnimatedNumber(target: number, durationMs: number = 280): number {
  const [animatedValue, setAnimatedValue] = useState(target);
  const valueRef = useRef(target);

  useEffect(() => {
    const fromValue = valueRef.current;
    const toValue = target;
    const start = performance.now();

    let frameId = 0;

    const tick = (now: number) => {
      const rawT = Math.min((now - start) / durationMs, 1);
      const easedT = easeInOutCubic(rawT);
      const nextValue = fromValue + (toValue - fromValue) * easedT;

      setAnimatedValue(nextValue);

      if (rawT < 1) {
        frameId = requestAnimationFrame(tick);
      } else {
        valueRef.current = toValue;
      }
    };

    frameId = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frameId);
  }, [target, durationMs]);

  return animatedValue;
}
