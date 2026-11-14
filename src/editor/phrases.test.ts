import { describe, expect, it } from "vitest";
import { LOCALES } from "../lib/languages";
import { PHRASES } from "./phrases";

describe("frases de CodeMirror", () => {
  it.each(LOCALES.filter((l) => l !== "en"))("%s tiene las mismas que el español", (locale) => {
    expect(Object.keys(PHRASES[locale] ?? {}).sort()).toEqual(Object.keys(PHRASES.es ?? {}).sort());
  });
});
