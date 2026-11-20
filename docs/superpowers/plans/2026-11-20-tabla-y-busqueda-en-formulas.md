# Tabla completa y búsqueda en fórmulas — plan de implementación

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** El autocompletado de `\begin{table}` trae la tabla completa con campos, y «Buscar en el
proyecto» puede limitarse a lo que está en fórmulas o fuera de ellas (también al reemplazar).

**Architecture:** El detector de fórmulas (`src/lib/latexMath/regions.ts`) se porta a Rust
(`files/math_regions.rs`). Las dos copias comparten un archivo de casos, `cases.json`. La
búsqueda de Rust y su copia de la vista previa (`lib/projectSearch.ts`) filtran cada
coincidencia según dónde empieza. La tabla es un snippet de CodeMirror en `editor/table.ts`,
con tres puntos de entrada.

**Tech Stack:** Rust (`regex`, `serde`, `serde_json`), React 19 + TypeScript, CodeMirror 6
(`snippet` de `@codemirror/autocomplete`), Vitest, `cargo test`.

**Spec:** `docs/superpowers/specs/2026-11-20-tabla-y-busqueda-en-formulas-design.md`

## Global Constraints

- Rama `tabla-y-busqueda-en-formulas`. Cada tarea es un commit y un push, con mensaje en
  inglés (Conventional Commits) y terminado en `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Los nombres de los tests van en español, como los demás del repositorio. Los comentarios
  del código van en inglés.
- Textos de la interfaz en los seis idiomas (`src/i18n/{es,en,pt,fr,de,it}.json`), con las
  mismas claves.
- Una coincidencia está «en fórmula» si `from ≤ inicio < to` para alguna región
  (delimitadores incluidos). Comentarios y verbatim cuentan como fuera.
- El detector solo se aplica a las extensiones `.tex`, `.ltx`, `.sty`, `.cls` y `.bib`, sin
  distinguir mayúsculas. En otros archivos no hay fórmulas: con «math» no hay resultados y
  con «text» se toman todos.
- Con alcance `all` la búsqueda no calcula regiones: sus resultados y su costo no cambian.
- El esqueleto de la tabla es exactamente el de la sección 5.1 del diseño: 2 × 2,
  `{|c|c|}`, `\hline`, `[htbp]`, `\caption{}` y `\label{tab:}`.
- Antes de cada commit: `pnpm typecheck`, `pnpm lint` y los tests que toque la tarea. Al
  final, `pnpm test` y `cargo test`/`clippy`/`fmt` completos.

## Review Focus

1. Una coincidencia que **empieza en la fórmula y termina fuera** (expresión regular entre
   `$x$ y`): se clasifica por su inicio y, al reemplazar, se reemplaza entera. Test en la tarea 2.
2. Texto con **caracteres de varios bytes antes de la fórmula** (`é $x$`): en Rust los
   offsets son bytes y en TypeScript unidades UTF-16; cada lado tiene que clasificar igual.
   Tests en las tareas 2 y 3.
3. **Archivo con CRLF** y reemplazo filtrado: debe seguir con CRLF y reemplazar solo lo que
   corresponde. Test en la tarea 2.
4. El **límite de 5000 resultados** cuenta solo las coincidencias filtradas: muchas
   coincidencias fuera de las fórmulas no deben esconder las de adentro. Test en la tarea 2.
5. **«Reemplazar en este archivo»** desde el panel también respeta el alcance. Test en la
   tarea 4.

---

### Task 1: Detector de fórmulas en Rust con los casos compartidos

**Files:**
- Create: `src/lib/latexMath/cases.json`
- Modify: `src/lib/latexMath/cases.ts`
- Create: `src-tauri/src/files/math_regions.rs`
- Modify: `src-tauri/src/files/mod.rs` (agregar `pub mod math_regions;`)

**Interfaces:**
- Produces (Rust, `crate::files::math_regions`):
  - `pub struct MathRegion { pub from: usize, pub to: usize, pub body_from: usize, pub body_to: usize, pub display: bool, pub env: Option<String>, pub unclosed: bool }` (offsets en bytes del texto).
  - `pub fn math_regions(text: &str) -> Vec<MathRegion>`: regiones de primer nivel, en orden.
  - `pub fn in_math(regions: &[MathRegion], at: usize) -> bool`: `from <= at < to` en alguna.
- Produces (TS): `cases.json` es un arreglo con la forma de `MathCase`. `cases.ts` sigue
  exportando `MathCase`, `MATH_CASES` y `withCursor`, con los mismos nombres.

- [ ] **Step 1: Pasar los casos a JSON.** Mover el contenido de `MATH_CASES` sin cambios a
  `cases.json`. `cases.ts` lo importa (`import cases from "./cases.json"`, tipado como
  `MathCase[]`, como hace `lib/languages.ts` con `languages.json`).

- [ ] **Step 2: Comprobar que nada cambió en TypeScript**

Run: `pnpm vitest run src/lib/latexMath src/editor/mathRegion.test.ts`
Expected: PASS, con la misma cantidad de tests que antes del cambio.

- [ ] **Step 3: Escribir los tests de Rust que fallan** (`#[cfg(test)] mod tests` en `math_regions.rs`)

```rust
#[test]
fn los_casos_compartidos_con_la_interfaz() {
    // include_str!("../../../src/lib/latexMath/cases.json"), leído con serde_json::Value.
    // Por cada caso: quitar el «|» (su byte es la posición), buscar la región con
    // body_from <= pos <= body_to; si `expected` es null, ninguna. Si no:
    // &text[body_from..body_to] == expected.body, display, env (null → None), unclosed
    // (ausente → false). Si falla, el mensaje dice el `name` del caso.
}

#[test]
fn todos_los_entornos_matematicos_con_y_sin_asterisco() {
    // equation, align, gather, multline, flalign, alignat, eqnarray, math, displaymath y la
    // versión con «*» de cada uno: "\\begin{X} a \\end{X}" → env == Some("X").
    // "\\begin{itemize} a \\end{itemize}" → sin regiones.
}

#[test]
fn in_math_incluye_los_delimitadores() {
    let text = "x $ab$ y";
    let r = math_regions(text);
    assert!(!in_math(&r, 1));
    assert!(in_math(&r, 2)); // el «$» que abre
    assert!(in_math(&r, 5)); // el «$» que cierra
    assert!(!in_math(&r, 6));
}
```

- [ ] **Step 4: Ver que fallan**

Run: `cargo test --manifest-path src-tauri/Cargo.toml math_regions`
Expected: no compila (`math_regions` no existe).

- [ ] **Step 5: Portar `regions.ts` línea por línea** a `math_regions.rs`, sobre `text.as_bytes()`:
  - Todos los delimitadores son ASCII, así que los índices de bytes caen siempre en el límite
    de un carácter.
  - `isLetter` incluye `@`.
  - `paragraphEnd` es el primer `\n[ \t]*\n` (a mano o con `regex`).
  - El mismo `MATH_ENV`, el mismo `VERBATIM_ENVS` y el mismo `skipInlineVerbatim`.

- [ ] **Step 6: Ver que pasan**

Run: `cargo test --manifest-path src-tauri/Cargo.toml math_regions`
Expected: PASS (3 tests).

- [ ] **Step 7: Commit**

```bash
git add src/lib/latexMath/cases.json src/lib/latexMath/cases.ts src-tauri/src/files/math_regions.rs src-tauri/src/files/mod.rs
git commit -m "feat(search): math region scanner in Rust, sharing its test cases with the frontend"
```

---

### Task 2: Alcance de la búsqueda y del reemplazo en Rust

**Files:**
- Modify: `src-tauri/src/files/search.rs`
- Modify: `src-tauri/src/commands/files.rs`, solo si hace falta para compilar (`SearchQuery` ya
  se deserializa ahí).

**Interfaces:**
- Consumes: `math_regions`, `in_math` (tarea 1).
- Produces:
  - `#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)] #[serde(rename_all = "lowercase")] pub enum SearchScope { #[default] All, Math, Text }`.
  - `SearchQuery` gana `#[serde(default)] pub scope: SearchScope`. En JSON se escribe `"all" | "math" | "text"`.
  - `pub fn is_latex_source(path: &str) -> bool`: extensiones tex, ltx, sty, cls y bib, sin
    distinguir mayúsculas.
  - `pub struct ScopeFilter`, con `pub fn new(scope: SearchScope, path: &str, text: &str) -> Self`
    (con `All` no calcula regiones) y `pub fn keeps(&self, at: usize) -> bool`.
  - `find_in_text(re: &Regex, text: &str, limit: usize, filter: &ScopeFilter) -> Vec<SearchMatch>`:
    las coincidencias descartadas no cuentan para `limit`.
  - `search` y `replace` mantienen su firma pero aplican el alcance de `query.scope`.

- [ ] **Step 1: Escribir los tests que fallan** (en `search.rs`; `query()` del test agrega `scope: SearchScope::All`)

```rust
const DOC: &str = "Sea $x$ y \\[ x^2 \\] con x.\n% $x$ comentado\n\\begin{align*} x \\end{align*}\nCuesta \\$x";

fn scoped(text: &str, scope: SearchScope) -> SearchQuery { SearchQuery { scope, case_sensitive: true, ..query(text) } }
fn cols(q: &SearchQuery, path: &str, text: &str) -> Vec<(usize, usize)>
// find_in_text con ScopeFilter::new(q.scope, path, text) → (línea, columna)

#[test]
fn en_formulas_solo_lo_de_adentro() {
    assert_eq!(cols(&scoped("x", SearchScope::Math), "a.tex", DOC), [(1, 5), (1, 13), (3, 15)]);
}
#[test]
fn fuera_de_formulas_incluye_comentarios_y_dolar_escapado() {
    assert_eq!(cols(&scoped("x", SearchScope::Text), "a.tex", DOC), [(1, 24), (2, 3), (4, 9)]);
}
#[test]
fn todo_como_antes() { /* All: las 6, iguales a las de antes del cambio */ }
#[test]
fn archivos_que_no_son_latex_no_tienen_formulas() {
    // "notas.txt" con "$x$": Math → [], Text → 1 resultado. "refs.BIB" sí se analiza.
}
#[test]
fn letras_de_varios_bytes_antes_de_la_formula() {
    // "é ñ $x$ x", Math → solo la x de adentro (columna 5).
}
#[test]
fn se_clasifica_por_donde_empieza() {
    // regex r"x\$ y" en "a $x$ y": Math → 1 resultado (empieza en la x de adentro).
    // replace con Math y "Z" → "a $Z", la coincidencia entera reemplazada.
}
#[test]
fn el_limite_cuenta_solo_lo_filtrado() {
    // Un .tex con MAX_MATCHES + 10 «x» fuera y después "$x$": search con Math → 1 resultado,
    // truncated == false.
}
#[test]
fn reemplazo_filtrado_y_con_grupos() {
    // Proyecto con a.tex = "x $x$\r\nx $x$\r\n":
    // - replace Math, "y" → "x $y$\r\nx $y$\r\n" (conserva CRLF), count 2.
    // - replace Text con regex "(x)" y "[$1]" → "[x] $y$…", count 2.
    // - con Math, un archivo sin coincidencias en fórmulas no aparece en el resultado.
}
```

Las columnas de los tests de arriba deben comprobarse contra `DOC` al escribirlos. Si alguna
difiere, se corrige el número del test y no el código, siempre que la clasificación sea la
del diseño.

- [ ] **Step 2: Ver que fallan**

Run: `cargo test --manifest-path src-tauri/Cargo.toml files::search`
Expected: no compila (`SearchScope` no existe).

- [ ] **Step 3: Implementar.**
  - `search` arma un `ScopeFilter` por archivo con su ruta relativa y su texto.
  - `replace`, con `All`, queda como hoy. Si no, recorre `re.captures_iter` y copia el texto
    entre coincidencias. Las que `keeps(m.start())` se reemplazan con `caps.expand` (si
    `query.regex`) o con el texto literal; las demás quedan tal cual. `count` cuenta solo las
    reemplazadas y, si es 0, el archivo no se escribe.
  - Las coincidencias vacías se siguen saltando, como hoy.

- [ ] **Step 4: Ver que pasan, junto con los de antes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml files::search && cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`
Expected: PASS; clippy sin advertencias.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/files/search.rs src-tauri/src/commands/files.rs
git commit -m "feat(search): limit project search and replace to formulas or outside them"
```

---

### Task 3: El mismo alcance en la vista previa del navegador

**Files:**
- Modify: `src/store/types.ts` (`SearchQuery`)
- Modify: `src/lib/projectSearch.ts`
- Modify: `src/lib/mockBackend.ts` (`replaceInProject` pasa `path`)
- Modify: `src/store/search.ts` (`initialSearchState.query.scope = "all"`)
- Test: `src/lib/projectSearch.test.ts`

**Interfaces:**
- Consumes: `mathRegions` de `src/lib/latexMath/regions.ts` (ya existe).
- Produces:
  - `export type SearchScope = "all" | "math" | "text"`; `SearchQuery` gana `scope: SearchScope`.
  - `export function isLatexSource(path: string): boolean`.
  - `export function scopeFilter(scope: SearchScope, path: string, text: string): (offset: number) => boolean`.
  - `findInText(re, text, limit, keep: (offset: number) => boolean = () => true)`.
  - `searchFiles(files, query)` filtra con `scopeFilter`.
  - `replaceText(text, query, replacement, path: string)` gana `path`. Usa un `keep` y
    devuelve `{ text, count }` con solo las reemplazadas.

- [ ] **Step 1: Escribir los tests que fallan.** Usan el mismo `DOC` y los mismos resultados
  esperados que la tarea 2: líneas y columnas de `en_formulas_solo_lo_de_adentro`,
  `fuera_de_formulas_incluye_comentarios_y_dolar_escapado`, «é ñ $x$ x» y `.txt`/`.BIB`. Se
  agrega `replaceText` con `math` y `text`, incluida la expresión regular con `$1`.
  - `it("en fórmulas o fuera de ellas, como en Rust", …)`
  - `it("el reemplazo respeta el alcance", …)`

- [ ] **Step 2: Ver que fallan**

Run: `pnpm vitest run src/lib/projectSearch.test.ts`
Expected: FAIL (`scope` no se aplica / `scopeFilter` no existe).

- [ ] **Step 3: Implementar** según las interfaces. En TypeScript los offsets son índices de
  cadena (UTF-16), los mismos que usa `mathRegions`.

- [ ] **Step 4: Ver que pasan**

Run: `pnpm vitest run src/lib/projectSearch.test.ts && pnpm typecheck`
Expected: PASS, sin errores de tipos (los demás usos de `SearchQuery` reciben `scope`).

- [ ] **Step 5: Commit**

```bash
git add src/store/types.ts src/lib/projectSearch.ts src/lib/projectSearch.test.ts src/lib/mockBackend.ts src/store/search.ts
git commit -m "feat(search): same formula scope in the browser preview's search"
```

---

### Task 4: Selector «Todo / En fórmulas / Fuera» en el panel

**Files:**
- Modify: `src/components/search/SearchPanel.tsx`
- Modify: `src/i18n/{es,en,pt,fr,de,it}.json` (bajo `projectSearch`)
- Test: `src/components/search/SearchPanel.test.tsx`

**Interfaces:**
- Consumes: `SearchScope` (tarea 3), `setQuery({ scope })` del store, que ya vuelve a buscar.
- Produces: claves `projectSearch.scope`, `scopeAll`, `scopeMath`, `scopeText` y `scopeTextShort`.

Textos exactos:

| clave | es | en | pt | fr | de | it |
|---|---|---|---|---|---|---|
| scope | Dónde buscar | Where to search | Onde buscar | Où chercher | Wo suchen | Dove cercare |
| scopeAll | Todo | All | Tudo | Tout | Alles | Tutto |
| scopeMath | En fórmulas | In formulas | Em fórmulas | Dans les formules | In Formeln | Nelle formule |
| scopeText | Fuera de fórmulas | Outside formulas | Fora das fórmulas | Hors des formules | Außerhalb von Formeln | Fuori dalle formule |
| scopeTextShort | Fuera | Outside | Fora | Hors | Außerhalb | Fuori |

- [ ] **Step 1: Escribir los tests que fallan** (en `SearchPanel.test.tsx`, con el proyecto
  «tesis» del backend simulado. Si su texto no tiene una palabra que aparezca a la vez dentro
  y fuera de una fórmula, se le agrega al proyecto del test con `backend` antes de buscar.)
  - `it("buscar solo en fórmulas o fuera de ellas", …)`:
    - `getByRole("radiogroup", { name: "Dónde buscar" })` existe, con «Todo» marcado
      (`aria-checked="true"`).
    - Clic en `getByRole("radio", { name: "En fórmulas" })` hace que el conteo baje a lo que
      está en fórmulas.
    - Clic en `getByRole("radio", { name: "Fuera de fórmulas" })` (el nombre accesible es el
      completo; el botón muestra «Fuera») hace que el conteo cambie al complemento.
  - `it("reemplazar en un archivo respeta el alcance", …)`: con «En fórmulas», «Reemplazar en
    este archivo» → confirmar; el archivo cambia solo dentro de las fórmulas.

- [ ] **Step 2: Ver que fallan**

Run: `pnpm vitest run src/components/search/SearchPanel.test.tsx`
Expected: FAIL (no hay `radiogroup`).

- [ ] **Step 3: Implementar el selector** debajo de la fila de reemplazo:
  - Un `div role="radiogroup"` con `aria-label={t("projectSearch.scope")}` y tres `button
    role="radio"` de igual ancho que ocupan la fila, con `aria-checked`.
  - Los botones muestran `scopeAll`, `scopeMath` y `scopeTextShort`. `title` y `aria-label`
    llevan el texto completo.
  - Estilo como los `Toggle` actuales (`aria-checked:bg-accent/30`).
  - Flechas ←/→ mueven la elección (patrón ARIA de radio group).
  - Agregar las claves en los seis JSON.

- [ ] **Step 4: Ver que pasan, junto con la paridad de idiomas y la accesibilidad**

Run: `pnpm vitest run src/components/search src/i18n src/test/a11y.test.tsx`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/components/search src/i18n
git commit -m "feat(search): All / In formulas / Outside selector in the project search panel"
```

---

### Task 5: Tabla completa al autocompletar

**Files:**
- Create: `src/editor/table.ts`
- Modify: `src/editor/latexCompletion.ts` (`environmentBraces`)
- Modify: `src/editor/figure.ts` (`beginOnCloseBrace`)
- Test: `src/editor/table.test.ts`, `src/editor/latexCompletion.test.ts`

**Interfaces:**
- Produces (`src/editor/table.ts`):
  - `export const TABLE = /^table\*?$/`.
  - `export function tableTemplate(name: string): string`: plantilla de snippet de
    CodeMirror, con campos `${1}`…`${6}` en el orden ① a ⑥ del diseño y sangría relativa
    (dos espacios por nivel; `snippet()` antepone la sangría de la línea a las líneas
    siguientes).
  - `export function insertTable(view: EditorView, from: number, to: number, name: string): void`:
    `snippet(tableTemplate(name))(view, null, from, to)`. `from`–`to` es lo que va desde
    `\begin` hasta el final de lo que reemplaza, incluida una `}` cerrada automáticamente.
  - `export function tableRangeAt(state: EditorState, pos: number, typed: string): { from: number; name: string } | null`:
    si el texto de la línea antes de `pos` más `typed` termina en `\begin{table` o
    `\begin{table*`, devuelve dónde empieza ese `\begin` y el nombre; si no, `null`.

Plantilla (sin la sangría base), con `①`…`⑥` como `${1}`…`${6}`:

```
\begin{NAME}[htbp]
  \centering
  \begin{tabular}{|c|c|}
    \hline
    ① & ② \\ \hline
    ③ & ④ \\ \hline
  \end{tabular}
  \caption{⑤}
  \label{tab:⑥}
\end{NAME}
```

Antes de dar la plantilla por buena, comprobar en la fuente de `@codemirror/autocomplete`
qué secuencias interpreta `snippet()` (`${`, `#{`, `\{`, `\}`) y escaparlas si aparecen.

- [ ] **Step 1: Escribir los tests que fallan.** Usan una vista con `editorExtensions`, como
  `latexCompletion.test.ts` (el helper `editor()` y `tab` de ese archivo).
  - `table.test.ts`:
    - `it("cerrar \\begin{table con } trae la tabla completa", …)`: doc `"\\begin{document}\n  \\begin{table|}\n\\end{document}"`, tecla `}` con `runScopeHandlers` → `true`. El texto contiene el esqueleto con sangría de 2 espacios base (`"  \\begin{table}[htbp]\n    \\centering\n    \\begin{tabular}{|c|c|}\n      \\hline\n       & "`…`"  \\end{table}\n\\end{document}"`) y una sola `\end{table}`.
    - `it("Tab recorre las celdas, el caption y el label", …)`: tras el paso anterior se escribe `a`, Tab, `b`, Tab, `c`, Tab, `d`, Tab, `Cap`, Tab, `t1`. Resultado: `"a & b \\\\ \\hline"`, `"c & d \\\\ \\hline"`, `"\\caption{Cap}"` y `"\\label{tab:t1}"`.
    - `it("table* cierra con table*", …)`.
    - `it("un solo deshacer quita la tabla", …)`: `undo(view)` deja `\begin{table` como estaba antes de la `}`.
  - `latexCompletion.test.ts`:
    - `it("elegir table en la lista de entornos trae la tabla", …)`: igual que el de `figure`, con `"\\begin{tab|}"`.
    - `it("el atajo \\begin{table} del paquete trae la misma tabla", …)`: en `"\\begin{document}\n\\begin{tab|\n\\end{document}"`, con `startCompletion`, buscar la opción `\begin{table}` y aplicarla. El resultado no contiene `header1` y sí `\begin{tabular}{|c|c|}`.

- [ ] **Step 2: Ver que fallan**

Run: `pnpm vitest run src/editor/table.test.ts src/editor/latexCompletion.test.ts`
Expected: FAIL (`table.ts` no existe; la lista sigue dando `header1`).

- [ ] **Step 3: Implementar `table.ts` y conectarlo:**
  - En `environmentBraces`, la opción con `TABLE.test(o.label)` y la opción
    `o.label === "\\begin{table}"` reciben un `apply` que llama a `insertTable`:
    - Para el nombre del entorno, el rango va desde el `\begin` de la línea hasta `to` más
      una `}` siguiente, si la hay.
    - Para el atajo del paquete, es `from`–`to` de la opción, con nombre `table`.
  - En `beginOnCloseBrace`, antes de `figureSpec`: si `tableRangeAt(state, head, "")`, llamar
    a `insertTable(view, range.from, head + closing, name)` y devolver `true`.

- [ ] **Step 4: Ver que pasan**

Run: `pnpm vitest run src/editor && pnpm typecheck && pnpm lint`
Expected: PASS (los tests de `figure` siguen iguales).

- [ ] **Step 5: Commit**

```bash
git add src/editor/table.ts src/editor/table.test.ts src/editor/latexCompletion.ts src/editor/latexCompletion.test.ts src/editor/figure.ts
git commit -m "feat(editor): \\begin{table} completes the whole table, with Tab through its cells"
```

---

### Task 6: Verificación en la app, documentación y PR

**Files:**
- Modify: `CHANGELOG.md` (sección `## Unreleased` arriba, en inglés: las dos funciones en «New»)
- Modify: `CLAUDE.md`:
  - **Estado actual**: la entrada «Linux: release 2.2.0 listo para publicar» pasa a decir
    «publicado» (2026-11-18, tag `v2.2.0` en `27f904a`); nueva entrada de esta rama.
  - **5.2**: autocompletado de tablas.
  - **5.x**: búsqueda con alcance.
  - **Decisiones**: el detector en Rust con casos compartidos; el atajo del paquete reemplazado.

- [ ] **Step 1: Suites completas**

Run: `pnpm lint && pnpm typecheck && pnpm test && cargo fmt --manifest-path src-tauri/Cargo.toml --check && cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings && cargo test --manifest-path src-tauri/Cargo.toml`
Expected: todo verde.

- [ ] **Step 2: Probar en la vista previa** (servidor `vite-preview`, puerto 1430):
  - Ctrl+Shift+F con «x», cambiar el alcance y ver que cambia el conteo; reemplazar con «En
    fórmulas».
  - Escribir `\begin{table}` y recorrer con Tab.
  - Captura de cada una. Detener la vista previa al terminar.

- [ ] **Step 3: Probar en WebKitGTK** con `pnpm tauri dev` y un proyecto de prueba (no los
  del usuario), en Xvfb (`GDK_BACKEND=x11`, HOME temporal):
  - La búsqueda con alcance usa el backend de Rust.
  - La tabla con Tab.
  - Detener Xvfb con `kill $(pgrep -x Xvfb)`.

- [ ] **Step 4: Commit, push y PR** (título y cuerpo en inglés, con checklist; el cuerpo
  termina con `🤖 Generated with [Claude Code](https://claude.com/claude-code)`). No se
  fusiona sin confirmación del usuario.

```bash
git add CHANGELOG.md CLAUDE.md
git commit -m "docs: table completion and formula scope in project search"
git push
gh pr create --title "Table skeleton on completion and searching in formulas" --body-file <archivo>
```
