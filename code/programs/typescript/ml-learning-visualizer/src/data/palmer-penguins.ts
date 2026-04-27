import rawPenguins from "./palmer-penguins.csv?raw";
import type { TrainingPoint } from "../training.js";

export type PenguinMeasure =
  | "bill_length_mm"
  | "bill_depth_mm"
  | "flipper_length_mm"
  | "body_mass_g"
  | "year";

interface PenguinRow {
  species: string;
  island: string;
  bill_length_mm: number;
  bill_depth_mm: number;
  flipper_length_mm: number;
  body_mass_g: number;
  sex: string;
  year: number;
}

const columns = [
  "species",
  "island",
  "bill_length_mm",
  "bill_depth_mm",
  "flipper_length_mm",
  "body_mass_g",
  "sex",
  "year",
] as const;

function parseNumber(value: string): number {
  return Number(value);
}

function parsePenguins(csv: string): PenguinRow[] {
  return csv
    .trim()
    .split("\n")
    .slice(1)
    .map((line) => {
      const values = line.split(",");
      const row = Object.fromEntries(columns.map((column, index) => [column, values[index] ?? ""]));

      return {
        species: row.species,
        island: row.island,
        bill_length_mm: parseNumber(row.bill_length_mm),
        bill_depth_mm: parseNumber(row.bill_depth_mm),
        flipper_length_mm: parseNumber(row.flipper_length_mm),
        body_mass_g: parseNumber(row.body_mass_g),
        sex: row.sex,
        year: parseNumber(row.year),
      };
    })
    .filter((row) => Number.isFinite(row.bill_length_mm) && Number.isFinite(row.body_mass_g));
}

export const PALMER_PENGUINS = parsePenguins(rawPenguins);

export function penguinRegressionPoints(
  xKey: PenguinMeasure,
  yKey: PenguinMeasure,
): TrainingPoint[] {
  return PALMER_PENGUINS.map((row) => ({
    x: row[xKey],
    y: row[yKey],
    label: `${row.species} on ${row.island}`,
    group: row.species,
  }));
}
