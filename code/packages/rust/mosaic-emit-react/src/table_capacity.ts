const mosaic$tableCapacities = new WeakMap<HTMLTableElement, { rows?: number; pitch?: number; wheelRows?: number; problem?: string }>();

// A callback-ref factory supports both React 18's null cleanup and React 19.
// Last capacity belongs to the DOM table, not the render's callback identity.
export function mosaic$tableCapacityRef(onRows: (rows: number) => void, reveal?: (table: HTMLTableElement) => void, window?: { offset: number; total: number; shift: (rows: number) => void }): (table: HTMLTableElement | null) => void {
  let dispose: (() => void) | undefined;
  return table => {
    dispose?.();
    dispose = undefined;
    if (!table) return;
    reveal?.(table);
    const state = mosaic$tableCapacities.get(table) ?? {};
    mosaic$tableCapacities.set(table, state);
    const warn = (problem: string) => {
      if (state.problem !== problem) console.warn(`Mosaic HostTable: ${problem}`);
      state.problem = problem;
    };
    if (typeof ResizeObserver === 'undefined') { warn('viewport capacity observation is unavailable'); return; }
    let frame = table.parentElement;
    while (frame && frame !== document.body && !/(auto|scroll)/.test(`${getComputedStyle(frame).overflowX} ${getComputedStyle(frame).overflowY}`)) frame = frame.parentElement;
    if (!frame || frame === document.body) { warn('viewport capacity requires a bounded scroll frame'); return; }
    const scrollFrame = frame;
    let live = true;
    let pending = 0;
    const measure = () => {
      pending = 0;
      state.pitch = undefined;
      if (!live || !table.isConnected || !scrollFrame.clientHeight) return;
      const rows = Array.from(table.tBodies[0]?.rows ?? []);
      if (!rows.length) return;
      const rects = rows.map(row => row.getBoundingClientRect());
      if (rects.some(rect => rect.height <= 0)) return;
      const spacing = getComputedStyle(table).borderCollapse === 'collapse' ? 0 : parseFloat(getComputedStyle(table).borderSpacing.split(' ').pop() ?? '0') || 0;
      const pitch = rects.length > 1 ? rects[1].top - rects[0].top : rects[0].height + spacing;
      if (pitch <= 0) return;
      if (rects.some((rect, i) => Math.abs(rect.height - rects[0].height) > 0.5 || (i > 0 && Math.abs(rect.top - rects[i - 1].top - pitch) > 0.5))) {
        warn('variable-height rows are not supported by uniform viewport capacity'); return;
      }
      const pinned = [table.tHead, table.tFoot].reduce((height, section) => height + (section && getComputedStyle(section).position === 'sticky' ? section.getBoundingClientRect().height : 0), 0);
      const available = scrollFrame.clientHeight - pinned;
      if (available <= 0) return;
      state.problem = undefined;
      state.pitch = pitch;
      const capacity = Math.max(1, Math.floor(available / pitch));
      if (capacity === state.rows) return;
      state.rows = capacity;
      onRows(capacity);
    };
    const schedule = () => { if (live && !pending) pending = requestAnimationFrame(measure); };
    const observer = new ResizeObserver(schedule);
    const wheel = (event: WheelEvent) => {
      if (!live || !window || !state.pitch || event.defaultPrevented || !event.cancelable || event.ctrlKey || event.metaKey || event.shiftKey || Math.abs(event.deltaX) >= Math.abs(event.deltaY)) return;
      if (!(event.target instanceof Element) || event.target.closest('table') !== table) return;
      if (event.target instanceof Element && event.target.closest('input, textarea, select, [contenteditable="true"]')) return;
      const maximum = Math.max(0, window.total - (table.tBodies[0]?.rows.length ?? 0));
      if ((event.deltaY < 0 && window.offset <= 0) || (event.deltaY > 0 && window.offset >= maximum)) { state.wheelRows = 0; return; }
      const amount = event.deltaY * (event.deltaMode === 1 ? 1 : event.deltaMode === 2 ? (state.rows ?? 1) : 1 / state.pitch);
      if (!Number.isFinite(amount) || amount === 0) return;
      event.preventDefault();
      const previous = state.wheelRows ?? 0;
      const accumulated = (Math.sign(previous) === Math.sign(amount) ? previous : 0) + amount;
      const rows = Math.trunc(accumulated);
      state.wheelRows = accumulated - rows;
      if (rows) window.shift(rows);
    };
    if (window) scrollFrame.addEventListener('wheel', wheel, { passive: false });
    observer.observe(scrollFrame);
    observer.observe(table);
    if (table.tHead) observer.observe(table.tHead);
    if (table.tFoot) observer.observe(table.tFoot);
    for (const row of Array.from(table.tBodies[0]?.rows ?? [])) observer.observe(row);
    schedule();
    dispose = () => { live = false; observer.disconnect(); scrollFrame.removeEventListener('wheel', wheel); if (pending) cancelAnimationFrame(pending); };
  };
}
