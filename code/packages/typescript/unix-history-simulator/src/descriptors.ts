// A descriptor is a process-local slot. Two slots can refer to the same
// inode without sharing a position. Each transition returns a fresh table.
export interface Descriptor {
  readonly inode: number;
  readonly offset: number;
}

export type DescriptorTable = readonly (Descriptor | null)[];

export function openDescriptor(table: DescriptorTable, inode: number) {
  const fd = table.indexOf(null);
  if (fd === -1) return undefined;
  return { fd, table: table.with(fd, { inode, offset: 0 }) };
}

export function advanceDescriptor(table: DescriptorTable, fd: number, bytes: number) {
  const slot = table[fd];
  if (!slot || !Number.isSafeInteger(bytes) || bytes < 0) return undefined;
  const offset = slot.offset + bytes;
  if (!Number.isSafeInteger(offset)) return undefined;
  return table.with(fd, { inode: slot.inode, offset });
}

export function closeDescriptor(table: DescriptorTable, fd: number) {
  return table[fd] ? table.with(fd, null) : undefined;
}
