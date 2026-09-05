const mosaic$tableCapacities = new WeakMap<HTMLTableElement, { rows?: number; problem?: string }>();

// A callback-ref factory supports both React 18's null cleanup and React 19.
// Last capacity belongs to the DOM table, not the render's callback identity.
export function mosaic$tableCapacityRef(onRows: (rows: number) => void, reveal?: (table: HTMLTableElement) => void): (table: HTMLTableElement | null) => void {
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
      const capacity = Math.max(1, Math.floor(available / pitch));
      if (capacity === state.rows) return;
      state.rows = capacity;
      onRows(capacity);
    };
    const schedule = () => { if (live && !pending) pending = requestAnimationFrame(measure); };
    const observer = new ResizeObserver(schedule);
    observer.observe(scrollFrame);
    observer.observe(table);
    if (table.tHead) observer.observe(table.tHead);
    if (table.tFoot) observer.observe(table.tFoot);
    for (const row of Array.from(table.tBodies[0]?.rows ?? [])) observer.observe(row);
    schedule();
    dispose = () => { live = false; observer.disconnect(); if (pending) cancelAnimationFrame(pending); };
  };
}
