import { describe, expect, it } from "vitest";
import { dictionaries, locales, translate, type TranslationKey } from ".";

function leafPaths(node: unknown, prefix = ""): string[] {
  if (typeof node === "string") return [prefix];
  return Object.entries(node as Record<string, unknown>).flatMap(([k, v]) =>
    leafPaths(v, prefix ? `${prefix}.${k}` : k),
  );
}

function placeholders(text: string): string[] {
  return [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
}

describe("diccionarios", () => {
  const es = leafPaths(dictionaries.es).sort();
  const get = (d: unknown, path: string) =>
    path.split(".").reduce<unknown>((n, p) => (n as Record<string, unknown>)[p], d);

  it("hay un diccionario por cada idioma de la tabla", () => {
    expect(locales).toEqual(["es", "en", "pt", "fr", "de", "it"]);
    for (const locale of locales) expect(dictionaries[locale], locale).toBeDefined();
  });

  it.each(locales)("%s tiene exactamente las mismas claves que es", (locale) => {
    expect(leafPaths(dictionaries[locale]).sort()).toEqual(es);
  });

  it.each(locales)("%s usa los mismos marcadores {…}", (locale) => {
    for (const path of es) {
      expect(placeholders(String(get(dictionaries[locale], path))), `${locale}: ${path}`).toEqual(
        placeholders(String(get(dictionaries.es, path))),
      );
    }
  });

  it.each(locales)("%s no tiene textos vacíos", (locale) => {
    for (const path of es) {
      expect(String(get(dictionaries[locale], path)).trim(), `${locale}: ${path}`).not.toBe("");
    }
  });
});

/** Texts that are the same in English and the other languages (names, abbreviations…). */
const SAME_AS_ENGLISH = new Set<string>([
  "Link",
  "Layout",
  "Editor",
  "Visual",
  "Normal",
  "Editor: {file}",
  "Zoom",
  "v{version} · 100% local",
  "MiKTeX",
  "Interface",
  "Cache",
  "{name} <{email}>",
  "E-mail",
  "Backups",
  "PgUp",
  "PgDn",
  "Format",
  "Figure…",
  "Citation (\\cite)",
  "Section",
  "Versions",
  "Code",
  "Relations",
  "Page {page}",
  "Sections",
  "Version",
  "Documentation",
  "Compilation",
  "Indigo",
  "Graphite",
  "Image {name}",
  "Distribution",
  "Version {version}",
  "Compiler",
  "Name",
  "In {folder}",
  "in {seconds} s",
  "GitHub (optional)",
  "Details",
  "File",
  "Home",
  "in {count} file",
  "Output",
]);

describe("traducciones", () => {
  const en = leafPaths(dictionaries.en);
  const get = (d: unknown, path: string) =>
    String(path.split(".").reduce<unknown>((n, p) => (n as Record<string, unknown>)[p], d));

  it.each(["pt", "fr", "de", "it"] as const)("%s no deja textos en inglés", (locale) => {
    const untranslated = en.filter((path) => {
      const text = get(dictionaries[locale], path);
      return (
        text === get(dictionaries.en, path) && /\p{L}{2}/u.test(text) && !SAME_AS_ENGLISH.has(text)
      );
    });
    expect(untranslated).toEqual([]);
  });
});

describe("translate", () => {
  it("traduce según el idioma", () => {
    expect(translate("es", "menu.file")).toBe("Archivo");
    expect(translate("en", "menu.file")).toBe("File");
  });

  it("interpola parámetros", () => {
    expect(translate("es", "projects.open", { name: "Tesis" })).toBe("Abrir Tesis");
    expect(translate("en", "pdf.page", { page: 3 })).toBe("Page 3");
  });

  it("deja intactos los marcadores sin valor", () => {
    expect(translate("es", "projects.open", {})).toBe("Abrir {name}");
  });

  it("elige la forma plural", () => {
    expect(translate("es", "projects.count", { count: 1 })).toBe("1 proyecto");
    expect(translate("es", "projects.count", { count: 0 })).toBe("0 proyectos");
    expect(translate("en", "projects.count", { count: 2 })).toBe("2 projects");
  });

  it("lo que falta en un idioma sale en inglés", () => {
    const fr = dictionaries.fr as unknown as { menu: Record<string, string> };
    const saved = fr.menu.file;
    delete fr.menu.file;
    try {
      expect(translate("fr", "menu.file")).toBe("File");
    } finally {
      fr.menu.file = saved;
    }
  });

  it("devuelve la clave si no existe", () => {
    expect(translate("en", "no.existe" as TranslationKey)).toBe("no.existe");
  });
});
