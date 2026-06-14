import { sym } from "@coding-adventures/symbolic-ir";

export const CANONICAL = sym("Canonical");

export const ASSUME = sym("Assume");
export const FORGET = sym("Forget");
export const IS = sym("Is");
export const SIGN = sym("Sign");

export const RADCAN = sym("Radcan");
export const LOGCONTRACT = sym("LogContract");
export const LOGEXPAND = sym("LogExpand");
export const EXPONENTIALIZE = sym("Exponentialize");
export const DEMOIVRE = sym("DeMoivre");

const COMMUTATIVE_FLAT = new Set(["Add", "Mul"]);

export function isCommutativeFlat(headName: string): boolean {
  return COMMUTATIVE_FLAT.has(headName);
}
