import { describe, expect, it } from "vitest";
import type { SearchQuery } from "../store/types";
import {
  findInText,
  isLatexSource,
  MAX_MATCHES,
  replaceText,
  scopeFilter,
  searchFiles,
  searchRegExp,
} from "./projectSearch";

const query = (text: string, options: Partial<SearchQuery> = {}): SearchQuery => ({
  text,
  regex: false,
  caseSensitive: false,
  wholeWord: false,
  scope: "all",
  ...options,
});

const found = (q: SearchQuery, text: string) =>
  findInText(searchRegExp(q), text, 100).map((m) => [m.line, m.column, m.endLine, m.endColumn]);

// The same cases as src-tauri/src/files/search.rs: the preview behaves like the app.
describe("búsqueda en el proyecto (vista previa)", () => {
  it("texto literal, mayúsculas y caracteres especiales", () => {
    const text = "Campo eléctrico\nEL CAMPO y el campo.";
    expect(found(query("campo"), text)).toEqual([
      [1, 0, 1, 5],
      [2, 3, 2, 8],
      [2, 14, 2, 19],
    ]);
    expect(found(query("campo", { caseSensitive: true }), text)).toEqual([[2, 14, 2, 19]]);
    expect(found(query("$x^2$"), "a $x^2$ b")).toEqual([[1, 2, 1, 7]]);
  });

  it("columnas en unidades UTF-16 y CRLF", () => {
    const text = "é𝐱 tensión\r\ntensión";
    expect(found(query("tensión"), text)).toEqual([
      [1, 4, 1, 11],
      [2, 0, 2, 7],
    ]);
    const [m] = findInText(searchRegExp(query("tensión")), text, 10);
    expect([m.before, m.text, m.after]).toEqual(["é𝐱 ", "tensión", ""]);
  });

  it("palabra completa, también comandos y letras con tilde", () => {
    const whole = (t: string) => query(t, { wholeWord: true });
    expect(found(whole("vec"), "\\vec{v} vector v_vec vec")).toEqual([
      [1, 1, 1, 4],
      [1, 21, 1, 24],
    ]);
    expect(found(whole("\\frac"), "\\frac{1}{2} \\fracs")).toEqual([[1, 0, 1, 5]]);
    expect(found(whole("tensi"), "tensión tensi")).toEqual([[1, 8, 1, 13]]);
  });

  it("expresiones regulares: por línea, entre líneas, vacías y no válidas", () => {
    expect(found(query("^%", { regex: true }), "a % b\n% c\n%d")).toEqual([
      [2, 0, 2, 1],
      [3, 0, 3, 1],
    ]);
    expect(found(query("a\\s+b", { regex: true }), "xa\n  b")).toEqual([[1, 1, 2, 3]]);
    expect(found(query("x*", { regex: true }), "ab x")).toEqual([[1, 3, 1, 4]]);
    expect(() => searchRegExp(query("(sin cerrar", { regex: true }))).toThrow();
    expect(() => searchRegExp(query(""))).toThrow();
  });

  it("solo archivos del usuario, ordenados, con límite", () => {
    const results = searchFiles(
      [
        ["main.tex", "campo"],
        ["capitulos/a.tex", "Campo y campo"],
        ["main.log", "campo"],
        [".jutex/x.tex", "campo"],
      ],
      query("campo"),
    );
    expect(results.files.map((f) => [f.path, f.matches.length])).toEqual([
      ["capitulos/a.tex", 2],
      ["main.tex", 1],
    ]);
    const many = searchFiles(
      [
        ["a.tex", "x\n".repeat(MAX_MATCHES)],
        ["b.tex", "x x"],
      ],
      query("x"),
    );
    expect(many.truncated).toBe(true);
    expect(many.files.reduce((n, f) => n + f.matches.length, 0)).toBe(MAX_MATCHES);
  });

  it("reemplazo literal y con grupos", () => {
    expect(replaceText("Campo, campo y $campo$.", query("campo"), "$E$", "a.tex")).toEqual({
      text: "$E$, $E$ y $$E$$.",
      count: 3,
    });
    expect(
      replaceText(
        "\\textbf{a} y \\textbf{b c}",
        query("\\\\textbf\\{([^}]*)\\}", { regex: true, caseSensitive: true }),
        "\\emph{$1}",
        "a.tex",
      ).text,
    ).toBe("\\emph{a} y \\emph{b c}");
  });
  // The same document and positions as the scope tests in search.rs.
  const DOC =
    "Sea $x$ y \\[ x^2 \\] con x.\n% $x$ comentado\n\\begin{align*} x \\end{align*}\nCuesta \\$x";
  const cols = (q: SearchQuery, path: string, text: string) =>
    findInText(searchRegExp(q), text, 100, scopeFilter(q.scope, path, text)).map((m) => [
      m.line,
      m.column,
    ]);

  it("en fórmulas o fuera de ellas, como en Rust", () => {
    const x = (scope: SearchQuery["scope"]) => query("x", { caseSensitive: true, scope });
    expect(cols(x("math"), "a.tex", DOC)).toEqual([
      [1, 5],
      [1, 13],
      [3, 15],
    ]);
    expect(cols(x("text"), "a.tex", DOC)).toEqual([
      [1, 24],
      [2, 3],
      [4, 9],
    ]);
    expect(cols(x("all"), "a.tex", DOC)).toHaveLength(6);
    expect(cols(x("math"), "a.tex", "é ñ $x$ x")).toEqual([[1, 5]]);
    expect(cols(x("text"), "a.tex", "é ñ $x$ x")).toEqual([[1, 8]]);
    expect(cols(x("math"), "notas.txt", "sea $x$")).toEqual([]);
    expect(cols(x("text"), "notas.txt", "sea $x$")).toEqual([[1, 5]]);
    expect(cols(x("math"), "refs.BIB", "sea $x$")).toEqual([[1, 5]]);
    expect(isLatexSource("cap/uno.tex") && isLatexSource("estilo.STY")).toBe(true);
    expect(isLatexSource("datos.csv") || isLatexSource("tex")).toBe(false);
    // searchFiles applies it too.
    const results = searchFiles([["a.tex", DOC]], x("math"));
    expect(results.files[0].matches).toHaveLength(3);
  });

  it("el reemplazo respeta el alcance", () => {
    const x = (scope: SearchQuery["scope"], options: Partial<SearchQuery> = {}) =>
      query("x", { caseSensitive: true, scope, ...options });
    expect(replaceText("x $x$\nx $x$", x("math"), "y", "a.tex")).toEqual({
      text: "x $y$\nx $y$",
      count: 2,
    });
    expect(replaceText("x $x$", x("text", { text: "(x)", regex: true }), "[$1]", "a.tex")).toEqual({
      text: "[x] $x$",
      count: 1,
    });
    expect(
      replaceText("a $x$ y", x("math", { text: "x\\$ y", regex: true }), "Z", "a.tex"),
    ).toEqual({ text: "a $Z", count: 1 });
    expect(replaceText("$x$", x("math"), "y", "notas.txt")).toEqual({ text: "$x$", count: 0 });
  });
  it("el filtro es rápido con muchas fórmulas", () => {
    // 2000 formulas (~50 KB): asking about every offset must not walk every region.
    const text = "texto $e^x$ y algo más ".repeat(2000);
    const keep = scopeFilter("math", "a.tex", text);
    const start = performance.now();
    let kept = 0;
    for (let at = 0; at < text.length; at++) if (keep(at)) kept++;
    expect(kept).toBe(2000 * "$e^x$".length);
    expect(performance.now() - start).toBeLessThan(100);
  });
});
