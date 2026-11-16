import { ensureSyntaxTree } from "@codemirror/language";
import { EditorState } from "@codemirror/state";
import { latex } from "codemirror-lang-latex";
import { describe, expect, it, vi } from "vitest";

// The scanner, spied on: how much text each question about math makes it read.
const scanned = vi.hoisted(() => [] as number[]);
vi.mock("../lib/latexMath/regions", async (importOriginal) => {
  const original = await importOriginal<typeof import("../lib/latexMath/regions")>();
  return {
    ...original,
    mathRegionAt: (text: string, pos: number) => {
      scanned.push(text.length);
      return original.mathRegionAt(text, pos);
    },
  };
});

const { mathRegionInState } = await import("./mathRegion");

describe("mathRegionInState en un párrafo muy largo", () => {
  // As in the user's documents: thousands of lines without a blank one between them.
  const lines = Array.from({ length: 3000 }, (_, i) => `Línea ${i} con texto y $x_${i}$ al final.`);
  const doc = lines.join("\n");
  const created = EditorState.create({ doc, extensions: latex() });
  // Fully parsed, as in the editor (which parses in the background); a transaction takes the
  // parsed tree into the state.
  ensureSyntaxTree(created, created.doc.length, 10_000);
  const state = created.update({}).state;
  const lineStart = (n: number) => state.doc.line(n).from;

  it("no lee el párrafo entero para saber si el cursor está en una fórmula", () => {
    scanned.length = 0;
    // In the text of line 1500, outside its formula.
    mathRegionInState(state, lineStart(1500) + 3);
    expect(Math.max(0, ...scanned)).toBeLessThan(10_000);
  });

  it("sigue encontrando la fórmula del cursor", () => {
    const at = doc.indexOf("$x_1500$") + 2;
    const region = mathRegionInState(state, at);
    expect(region && state.sliceDoc(region.bodyFrom, region.bodyTo)).toBe("x_1500");
  });
});
