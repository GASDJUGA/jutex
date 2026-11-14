import { describe, expect, it } from "vitest";
import { LANGUAGES, languageName, resolveLocale, type Locale } from "./languages";

describe("idiomas", () => {
  it("resuelve como Rust", () => {
    expect(resolveLocale(["pt-PT", "en"])).toBe("pt");
    expect(resolveLocale(["ru-RU"])).toBe("en");
    expect(resolveLocale(["ja", "fr-CA"])).toBe("fr");
    expect(resolveLocale([])).toBe("en");
  });

  it("el tipo Locale coincide con la tabla", () => {
    const codes: Locale[] = ["es", "en", "pt", "fr", "de", "it"];
    expect(LANGUAGES.map((l) => l.code)).toEqual(codes);
  });

  it("cada idioma con su nombre propio", () => {
    expect(languageName("de")).toBe("Deutsch");
    expect(languageName("pt")).toBe("Português (Brasil)");
  });
});
