import cases from "./cases.json";

/**
 * Shared cases for both math detectors (text scanner and syntax tree). `|` marks the cursor;
 * `expected` is the body to render (or null: not in math), with the region's kind.
 */
export type MathCase = {
  name: string;
  text: string;
  expected: { body: string; display: boolean; env?: string | null; unclosed?: boolean } | null;
};

/** In `cases.json`, also read by the Rust scanner's tests (`files/math_regions.rs`). */
export const MATH_CASES: MathCase[] = cases;

/** Text without the cursor mark and the cursor position. */
export function withCursor(text: string): { text: string; pos: number } {
  const pos = text.indexOf("|");
  return { text: text.slice(0, pos) + text.slice(pos + 1), pos };
}
