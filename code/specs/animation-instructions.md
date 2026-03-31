# Animation Instructions

## Overview

This spec defines a backend-neutral animation package for the
coding-adventures monorepo.

The package exists to separate:

- domain logic that decides WHAT should change over time
- backend logic that decides HOW to execute the change

This is the temporal counterpart to draw-instructions. Draw-instructions
describe a single frame (spatial model). Animation-instructions describe
how elements transition between states over time (temporal model).

```
DrawInstruction   = what to paint     (one frame, static)
AnimateInstruction = how things move   (between frames, dynamic)
```

Together they form a complete declarative scene description that any
backend can consume — from SVG with CSS animations to Core Animation
layers to Windows Composition visuals to Canvas with requestAnimationFrame.

## Goals

- represent common animation patterns with a small set of primitives
- avoid coupling animation logic to any specific platform
- cover the intersection of what every major animation system supports
- keep the model simple enough to inspect in tests and serialize to JSON
- enable composition (parallel, sequential, staggered) without callbacks

## Relationship to Draw Instructions

Animation instructions reference draw instruction elements by their
metadata ID. A draw scene provides the spatial layout; an animate scene
provides the temporal behavior.

```
DrawScene {
  instructions: [
    DrawRect { metadata: { id: "header-bg" }, x: 0, y: 0, ... }
    DrawText { metadata: { id: "title" }, x: 50, y: 20, ... }
  ]
}

AnimateScene {
  animations: [
    AnimateBasic { target: "header-bg", property: "opacity", from: 0, to: 1, ... }
    AnimateSpring { target: "title", property: "position.y", from: 100, to: 20, ... }
  ]
}
```

The `target` field is a string that matches a `metadata.id` value in the
draw scene. This keeps the two systems decoupled — animation instructions
know nothing about rectangles or text, only about target IDs and
animatable properties.

### How backends combine them

An HTML+React backend receives both a DrawScene and an AnimateScene.
It renders the draw instructions as DOM elements (or canvas calls) and
applies the animation instructions as CSS transitions, Web Animations API
calls, or requestAnimationFrame loops. The metadata IDs serve as the
bridge between the spatial and temporal models.

A native backend (Core Animation, Windows Composition) creates platform
layers from draw instructions and attaches platform animations from
animate instructions, again linked by ID.

## Primitives

The animation model has six instruction types plus a scene container:

- `Basic` — single property tween from A to B
- `Keyframe` — multi-value interpolation with per-segment easing
- `Spring` — physics-based animation (duration emerges from parameters)
- `Group` — parallel composition (all children start simultaneously)
- `Sequence` — sequential composition (each child starts after previous)
- `Stagger` — same animation applied to multiple targets with delay
- `AnimateScene` — top-level container

### Why these six

Every major animation system provides these patterns, just with different
APIs:

| Pattern | Core Animation | Windows Composition | Web Animations | CSS | Canvas/GSAP |
|---------|---------------|-------------------|----------------|-----|-------------|
| Basic | CABasicAnimation | ScalarKeyFrameAnimation (2 frames) | element.animate() | transition | gsap.to() |
| Keyframe | CAKeyframeAnimation | KeyFrameAnimation | element.animate() | @keyframes | gsap.to() with keyframes |
| Spring | CASpringAnimation | SpringNaturalMotionAnimation | linear() approx | linear() approx | popmotion/react-spring |
| Group | CAAnimationGroup | multiple StartAnimation | multiple animate() | multiple rules | timeline (same position) |
| Sequence | beginTime offsets | storyboard ordering | promise chain | animation-delay | timeline (sequential) |
| Stagger | manual beginTime | manual offsets | delay offsets | nth-child delay | stagger parameter |

## Public API Shape

### Animatable Properties

The universal property vocabulary. Every platform can animate these.

```typescript
type AnimatableProperty =
  | "opacity"                 // 0.0 to 1.0
  | "position.x"             // scene x coordinate
  | "position.y"             // scene y coordinate
  | "scale"                  // uniform scale multiplier
  | "scale.x"               // horizontal scale
  | "scale.y"               // vertical scale
  | "rotation"               // degrees (clockwise from 12 o'clock)
  | "width"                  // element width
  | "height"                 // element height
  | "fill"                   // fill color (string)
  | "stroke"                 // stroke color (string)
  | "strokeWidth"            // stroke thickness
  ;
```

### Easing Functions

The timing curve applied to interpolation. Cubic bezier with two control
points is the universal primitive — every platform supports it.

```typescript
type EasingFunction =
  | { kind: "linear" }
  | { kind: "cubic-bezier"; x1: number; y1: number; x2: number; y2: number }
  | { kind: "preset"; name: "ease" | "ease-in" | "ease-out" | "ease-in-out" }
  | { kind: "steps"; count: number; position: "start" | "end" }
  | { kind: "spring"; stiffness: number; damping: number; mass?: number }
  ;
```

The `spring` easing enables backends without native spring support
(CSS, SVG) to approximate the curve using the `linear()` CSS function
or pre-computed keyframes.

Presets map to well-known cubic bezier values:

```
ease          → cubic-bezier(0.25, 0.1, 0.25, 1.0)
ease-in       → cubic-bezier(0.42, 0.0, 1.0,  1.0)
ease-out      → cubic-bezier(0.0,  0.0, 0.58, 1.0)
ease-in-out   → cubic-bezier(0.42, 0.0, 0.58, 1.0)
```

### Fill Mode

Controls what value the property holds before the animation starts
(during delay) and after it finishes.

```typescript
type FillMode = "none" | "forwards" | "backwards" | "both";
```

- `none` — snap back to the element's base value
- `forwards` — hold the final animated value after completion
- `backwards` — apply the `from` value during the delay period
- `both` — forwards + backwards

### Animation Instructions

```typescript
type AnimateInstruction =
  | AnimateBasic
  | AnimateKeyframe
  | AnimateSpring
  | AnimateGroup
  | AnimateSequence
  | AnimateStagger
  ;

interface AnimateBasic {
  kind: "basic";
  target: string;                // metadata.id of the draw element
  property: AnimatableProperty;
  from?: number | string;        // omit to animate from current value
  to: number | string;
  duration: number;              // milliseconds
  delay?: number;                // milliseconds, default 0
  easing?: EasingFunction;       // default: linear
  fill?: FillMode;               // default: "none"
  metadata?: Record<string, string | number | boolean>;
}

interface AnimateKeyframe {
  kind: "keyframe";
  target: string;
  property: AnimatableProperty;
  keyframes: Array<{
    offset: number;              // 0.0 to 1.0 (progress)
    value: number | string;
    easing?: EasingFunction;     // easing for this segment → next
  }>;
  duration: number;              // milliseconds
  delay?: number;
  iterations?: number;           // default 1; Infinity for loop
  direction?: "normal" | "reverse" | "alternate" | "alternate-reverse";
  fill?: FillMode;
  metadata?: Record<string, string | number | boolean>;
}

interface AnimateSpring {
  kind: "spring";
  target: string;
  property: AnimatableProperty;
  from?: number | string;
  to: number | string;
  stiffness: number;             // spring constant (k), e.g. 300
  damping: number;               // friction coefficient (c), e.g. 20
  mass?: number;                 // default 1.0
  initialVelocity?: number;      // default 0.0
  metadata?: Record<string, string | number | boolean>;
}

interface AnimateGroup {
  kind: "group";
  animations: AnimateInstruction[];
  metadata?: Record<string, string | number | boolean>;
}

interface AnimateSequence {
  kind: "sequence";
  animations: AnimateInstruction[];
  metadata?: Record<string, string | number | boolean>;
}

interface AnimateStagger {
  kind: "stagger";
  targets: string[];             // ordered list of element IDs
  template: Omit<AnimateBasic | AnimateKeyframe | AnimateSpring, "target">;
  staggerDelay: number;          // milliseconds between each target's start
  metadata?: Record<string, string | number | boolean>;
}

interface AnimateScene {
  animations: AnimateInstruction[];
  metadata?: Record<string, string | number | boolean>;
}
```

### AnimateRenderer

```typescript
interface AnimateRenderer<Output> {
  render(scene: AnimateScene): Output;
}
```

Possible outputs:
- `AnimateRenderer<string>` — CSS @keyframes + animation rules
- `AnimateRenderer<void>` — imperative calls to Web Animations API
- `AnimateRenderer<CAAnimation[]>` — Core Animation objects (native)
- `AnimateRenderer<string>` — GSAP timeline code generation

## Design Notes

### Pure data, no callbacks

Animation instructions are plain data objects — no closures, no DOM
references, no platform pointers. They can be serialized to JSON,
transmitted over the wire, stored in a database, or inspected in tests.

This is the same principle as draw-instructions: the instruction set is
the intermediate representation, and backends consume it however they
need to.

### No explicit duration on spring animations

Spring animations derive their duration from the physics parameters
(stiffness, damping, mass). This matches how Core Animation's
CASpringAnimation and Windows' SpringNaturalMotionAnimation work.

Backends that require an explicit duration (CSS, Web Animations API)
should compute the settling time from the spring parameters:

```
settlingTime ≈ -ln(threshold) / (damping / (2 * mass))
```

where `threshold` is a small epsilon (e.g., 0.001) below which the
spring is considered at rest.

### Target resolution

The `target` field is a string matching `metadata.id` from a draw
instruction. This is intentionally loose coupling — animation
instructions don't import or depend on draw-instructions at all. A
backend receives both a DrawScene and an AnimateScene, resolves targets
by ID, and applies animations to the appropriate platform objects.

### Property values

Numeric properties (opacity, position, scale, rotation, dimensions)
use `number` values. Color properties (fill, stroke) use `string`
values in the same format as draw-instructions (e.g., "#ff0000",
"rgba(255,0,0,0.5)").

When `from` is omitted, the backend should animate from the element's
current value. This enables animations that "pick up from wherever
the element is now" — essential for interruptible animations and
gesture-driven transitions.

## Backend Mapping

### How each primitive maps to each platform

```
AnimateBasic →
  Core Animation:    CABasicAnimation(keyPath: property, fromValue: from, toValue: to)
  Windows:           ScalarKeyFrameAnimation with 2 keyframes (0.0 → from, 1.0 → to)
  Web Animations:    element.animate([{[property]: from}, {[property]: to}], options)
  CSS:               transition: property duration easing delay
  Canvas/GSAP:       gsap.fromTo(target, {[property]: from}, {[property]: to, duration})

AnimateKeyframe →
  Core Animation:    CAKeyframeAnimation(keyPath: property, values: [...], keyTimes: [...])
  Windows:           KeyFrameAnimation with N InsertKeyFrame() calls
  Web Animations:    element.animate(keyframes.map(kf => ({offset, [property]: kf.value})))
  CSS:               @keyframes name { 0% { ... } 50% { ... } 100% { ... } }
  Canvas/GSAP:       gsap.to() with keyframes array

AnimateSpring →
  Core Animation:    CASpringAnimation(mass, stiffness, damping, initialVelocity)
  Windows:           SpringNaturalMotionAnimation(dampingRatio, period, finalValue)
  Web Animations:    element.animate() with linear() easing approximating the spring curve
  CSS:               linear() function with ~40 sample points from spring solver
  Canvas:            Solve F = -kx - cv each frame in rAF loop

AnimateGroup →
  Core Animation:    CAAnimationGroup(animations: [...])
  Windows:           Multiple StartAnimation() calls on the same visual
  Web Animations:    Multiple element.animate() calls
  CSS:               Multiple animation rules on same element
  Canvas/GSAP:       gsap.timeline() with animations at same position

AnimateSequence →
  Core Animation:    CAAnimationGroup with incremental beginTime offsets
  Windows:           Storyboard with ordered transitions
  Web Animations:    Chain via animation.finished.then(...)
  CSS:               Incremental animation-delay values
  Canvas/GSAP:       gsap.timeline().to().to().to()

AnimateStagger →
  Core Animation:    Loop with incremental beginTime = index * staggerDelay
  Windows:           Loop with incremental delay offsets
  Web Animations:    Loop with incremental delay option
  CSS:               :nth-child(n) with calc(n * staggerDelay)
  Canvas/GSAP:       gsap.to(targets, { stagger: staggerDelay })
```

## What This Spec Intentionally Excludes

### Expression animations

Windows.UI.Composition has ExpressionAnimation — reactive, constraint-
based relationships between properties (e.g., "this.Offset.X =
tracker.Position.X * 0.5"). This is a fundamentally different paradigm
(declarative constraints, not timeline animations). Future extension
as `kind: "expression"`.

### Path-based motion

Animating an element along an arbitrary bezier curve (Core Animation's
CAKeyframeAnimation with CGPath, CSS `offset-path`). Future extension:
add a `path` property type to AnimateKeyframe.

### 3D transforms

The model uses 2D properties to match draw-instructions' 2D scene model.
Future extension: `rotation.x`, `rotation.y`, `perspective`.

### Gesture/input-driven animation

Interactive animations that respond to touch/scroll position in real-time.
This is a reactive model (continuous input → continuous output) that
doesn't fit the instruction pattern well. Future work: a separate
`InteractionInstruction` model.

### Transition effects

Content-swap effects (CATransition, CSS view-transition, page transitions).
These are higher-level orchestration patterns, not property animations.

## Future Extensions

- expression animations (reactive constraints)
- path-based motion (animate along a curve)
- 3D transforms (rotation.x, perspective)
- gesture-driven animation (drag, scroll, pinch → property)
- animation events (onStart, onComplete as metadata tags, not callbacks)
- CSS renderer (DrawScene + AnimateScene → HTML + CSS @keyframes)
- Web Animations renderer (→ element.animate() calls)
- Canvas renderer (→ rAF loop with interpolation engine)
- Core Animation renderer (→ CAAnimation objects via FFI)
- Spring solver utility (shared math for spring curve computation)
