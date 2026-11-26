---
category: TypeScript / JavaScript
---

# A JavaScript constant-time helper must copy an ArrayBuffer input, because converting the other argument can run caller code that detaches or rewrites it

typescript `ct-compare` validated its inputs carefully: genuine typed arrays
were copied, and plain arrays were read once per element. But a genuine
`ArrayBuffer` was only *wrapped* (`new Uint8Array(buffer)`). `ctEq(left, right)`
converts `left` first. Converting a plain-array or `Proxy` `right` runs caller
code (a `length` trap, an element getter), and that code can detach
(`structuredClone(buf, {transfer: [buf]})`), shrink (resizable
`ArrayBuffer.resize`) or rewrite `left`'s buffer after it was "converted". Two
different inputs then compared equal. Found by the LV-007b pre-release security
review (SEC-012).

**Fix:** copy at conversion time with `new Uint8Array(new Uint8Array(buffer))`;
the inner view still throws for a buffer detached before the call.
`tests/snapshot.test.ts` covers detach, shrink, rewrite and select.

**Next time:** in JavaScript, any argument conversion that can call user code
(getters, Proxy traps, `valueOf`) runs *between* the conversions of the other
arguments. Snapshot (copy) every argument at conversion; never keep a live
view of caller-owned memory.
