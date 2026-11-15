import { beforeEach, describe, expect, it } from "vitest";
import { createMockBackend } from "../mockBackend";
import { setBackend } from "../tauri";
import { backendChecker, makeChecker } from "./checker";
import { rankSuggestions, typingDistance } from "./speller";

describe("verificador ortográfico", () => {
  let asked: { lang: string; words: string[] }[];

  beforeEach(() => {
    asked = [];
    const backend = createMockBackend();
    backend.spellWrong = async (lang, words) => {
      asked.push({ lang, words });
      return words.filter((w) => w.endsWith("nn"));
    };
    backend.spellSuggest = async (_lang, word) =>
      word === "teh" ? ["ten", "eh", "tea", "the"] : [];
    setBackend(backend);
  });

  it("pregunta al backend en su idioma y recuerda las respuestas", async () => {
    const checker = backendChecker("fr");
    expect(await checker.wrong(["maison", "maisonn"])).toEqual(["maisonn"]);
    expect(asked).toEqual([{ lang: "fr", words: ["maison", "maisonn"] }]);
    expect(checker.known("maison")).toBe(true);
    expect(checker.known("maisonn")).toBe(false);
    // Already known: not asked again.
    expect(await checker.wrong(["maison"])).toEqual([]);
    expect(asked).toHaveLength(1);
  });

  it("las sugerencias llegan ordenadas por parecido", async () => {
    expect((await backendChecker("en").suggest("teh"))[0]).toBe("the");
  });

  it("un cambio del diccionario personal hace volver a preguntar", async () => {
    const checker = backendChecker("es");
    await checker.wrong(["Kirchhoffnn"]);
    checker.setPersonal(["Kirchhoffnn"]);
    expect(checker.known("Kirchhoffnn")).toBeUndefined();
  });

  it("la lista confirmada del diccionario personal vuelve a preguntar aunque no cambie", async () => {
    // As «Agregar al diccionario»: the optimistic list, a check that the backend answers
    // before saving the word, then the same list confirmed.
    let saved = false;
    const checker = makeChecker({
      request: async (type, payload) =>
        type === "wrong" ? (payload as string[]).filter((w) => w === "Feler" && !saved) : [],
      dispose() {},
    });
    checker.setPersonal(["Feler"]);
    expect(await checker.wrong(["Feler"])).toEqual(["Feler"]);
    saved = true;
    checker.setPersonal(["Feler"]);
    expect(await checker.wrong(["Feler"])).toEqual([]);
  });

  it("un archivo con muchas palabras se pregunta en partes", async () => {
    const sizes: number[] = [];
    const checker = makeChecker({
      request: async (type, payload) => {
        const words = payload as string[];
        if (type === "wrong") sizes.push(words.length);
        return type === "wrong" ? words.filter((w) => w.endsWith("7")) : [];
      },
      dispose() {},
    });
    const words = Array.from({ length: 12_000 }, (_, i) => `w${i}`);
    const wrong = await checker.wrong(words);
    expect(Math.max(...sizes)).toBeLessThanOrEqual(5000);
    expect(sizes.reduce((a, b) => a + b, 0)).toBe(12_000);
    expect(wrong).toHaveLength(1200);
  });

  it("makeChecker con un backend propio", async () => {
    const checker = makeChecker({ request: async () => ["x"], dispose() {} });
    expect(await checker.wrong(["x", "y"])).toEqual(["x"]);
  });
});

describe("orden de las sugerencias", () => {
  it("primero lo que más se parece al escribir", () => {
    expect(typingDistance("teh", "the")).toBe(0.5);
    expect(typingDistance("ecuacion", "ecuación")).toBe(0.25);
    expect(typingDistance("ten", "the")).toBe(2);
    expect(rankSuggestions("teh", ["ten", "eh", "tea", "the"])[0]).toBe("the");
  });
});
