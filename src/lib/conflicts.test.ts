import { describe, expect, it } from "vitest";
import { findConflicts, resolve } from "./conflicts";

const ONE = "a\n<<<<<<< HEAD\nmío\n=======\nsuyo\n>>>>>>> origin/main\nb\n";

/** The text after applying `resolve` to its first block. */
function resolved(text: string, how: "ours" | "theirs" | "both") {
  const [block] = findConflicts(text);
  const change = resolve(text, block, how);
  return text.slice(0, change.from) + change.insert + text.slice(change.to);
}

describe("findConflicts", () => {
  // The same cases as `count_blocks` in Rust (src-tauri/src/backup/merge.rs).
  it("encuentra los bloques completos de marcas", () => {
    expect(findConflicts(ONE)).toHaveLength(1);
    expect(findConflicts(ONE + ONE)).toHaveLength(2);
    expect(findConflicts("a\n<<<<<<< HEAD\nx\n=======\ny\nb\n")).toHaveLength(0);
    expect(findConflicts("<<<<<<< HEAD\nx\n======= x\ny\n>>>>>>> origin/main\n")).toHaveLength(0);
    expect(findConflicts("")).toHaveLength(0);
  });

  it("reconoce marcas con CRLF", () => {
    expect(findConflicts(ONE.replaceAll("\n", "\r\n"))).toHaveLength(1);
  });

  it("da dónde está cada parte", () => {
    const [block] = findConflicts(ONE);
    expect(ONE.slice(block.from, block.to)).toBe(
      "<<<<<<< HEAD\nmío\n=======\nsuyo\n>>>>>>> origin/main\n",
    );
    expect(ONE.slice(block.ours.from, block.ours.to)).toBe("mío\n");
    expect(ONE.slice(block.theirs.from, block.theirs.to)).toBe("suyo\n");
  });
});

describe("resolve", () => {
  it("se queda con lo mío, con lo de GitHub o con ambos", () => {
    expect(resolved(ONE, "ours")).toBe("a\nmío\nb\n");
    expect(resolved(ONE, "theirs")).toBe("a\nsuyo\nb\n");
    expect(resolved(ONE, "both")).toBe("a\nmío\nsuyo\nb\n");
  });

  it("un bloque al final del archivo, sin salto de línea", () => {
    const text = "<<<<<<< HEAD\nmío\n=======\nsuyo\n>>>>>>> origin/main";
    expect(resolved(text, "theirs")).toBe("suyo\n");
  });
});
