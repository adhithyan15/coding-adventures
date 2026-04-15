export enum NibType {
  U4 = "u4",
  U8 = "u8",
  BCD = "bcd",
  BOOL = "bool",
}

export function parseTypeName(name: string): NibType | null {
  switch (name) {
    case "u4":
      return NibType.U4;
    case "u8":
      return NibType.U8;
    case "bcd":
      return NibType.BCD;
    case "bool":
      return NibType.BOOL;
    default:
      return null;
  }
}

export function typesAreCompatible(lhs: NibType, rhs: NibType): boolean {
  return lhs === rhs;
}

export function isBcdOpAllowed(operatorValue: string): boolean {
  return operatorValue === "+%" || operatorValue === "-";
}

export function isNumeric(type: NibType): boolean {
  return type === NibType.U4 || type === NibType.U8 || type === NibType.BCD;
}
