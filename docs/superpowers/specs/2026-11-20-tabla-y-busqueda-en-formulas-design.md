# Tabla completa al autocompletar y búsqueda en fórmulas — diseño

Fecha: 2026-11-20. Pedido del usuario (cambios de calidad de vida, para la 2.3.0). Rama
`tabla-y-busqueda-en-formulas`.

## 1. Intención

Dos mejoras independientes, en el mismo PR:

1. **Tabla completa.** Al autocompletar `\begin{table}`, hoy solo aparece `\begin{table}` …
   `\end{table}` (o, con el atajo del paquete de LaTeX, una tabla con textos de relleno en
   inglés: `header1 & header2…`). El usuario quiere que venga todo lo que una tabla necesita
   (`\centering`, `tabular`, `\hline`, `\caption`, `\label`), como ya pasa con `figure`.
2. **Buscar dentro o fuera de fórmulas.** En «Buscar en el proyecto» (Ctrl+Shift+F), poder
   restringir los resultados a lo que está **en fórmulas**, a lo que está **fuera de fórmulas**,
   o ver **todo** (como hoy). «Reemplazar todo» respeta la elección. El usuario lo pidió
   «solo si no es muy complicado»; lo es poco porque el detector de fórmulas ya existe.

## 2. Objetivos, no objetivos y antiobjetivos

**Objetivos**

- Las tres formas de llegar a una tabla dan el mismo esqueleto: el atajo `\begin{table}` de
  la lista, `\begin{` + «table» de la lista de entornos, y escribir a mano la `}` de
  `\begin{table`. También `table*`.
- El esqueleto tiene campos: el cursor queda en la primera celda y **Tab** recorre las
  celdas, luego `\caption{}` y luego `\label{tab:}`.
- La búsqueda en el proyecto ofrece tres opciones: todo / en fórmulas / fuera de fórmulas.
  El conteo, la lista y «Reemplazar todo» (también «reemplazar en este archivo») usan solo
  las coincidencias que pasan el filtro.
- «En fórmula» significa lo mismo que en la vista previa de matemáticas: las mismas reglas
  de `lib/latexMath/regions.ts` (delimitadores, entornos con y sin `*`, `\$`, comentarios,
  verbatim, `$` sin cerrar hasta el fin del párrafo).

**No objetivos**

- Elegir filas × columnas al autocompletar (eso ya lo hace el botón de tabla de la barra).
- Paquetes de tablas (`booktabs`, `tabularx`): el esqueleto usa solo LaTeX base.
- Filtro de fórmulas en la búsqueda del editor (Ctrl+F) o del PDF.
- Detectar como fórmula lo que no lo es para el detector actual (`\ensuremath`, nodos de
  TikZ en modo matemático, etc.).

**Antiobjetivos** (sería un fracaso aunque funcione)

- Que el filtro y la vista previa no estén de acuerdo sobre qué es una fórmula (dos
  detectores que se separan con el tiempo).
- Que «Reemplazar todo» con «en fórmulas» cambie algo fuera de las fórmulas.
- Que la búsqueda sin filtro («todo») se vuelva más lenta o cambie sus resultados.

## 3. Restricciones

- La búsqueda y el reemplazo corren en Rust (`src-tauri/src/files/search.rs`); la vista
  previa del navegador usa su copia en `src/lib/projectSearch.ts` con el backend simulado.
- Textos de la interfaz en los seis idiomas (`src/i18n/*.json`, mismas claves).
- Tests: lo no trivial con tests (sección 8 de `CLAUDE.md`); los de Rust en español, como los
  demás del archivo.

## 4. Enfoques considerados

### Búsqueda

- **A. Detector de fórmulas también en Rust (elegido).** Se porta `mathRegions` a Rust; cada
  coincidencia se clasifica por la posición donde **empieza** y se filtra antes de contarla.
  El reemplazo usa el mismo filtro. Costo: dos copias del detector (TypeScript y Rust). Se
  controla con un único archivo de casos (`src/lib/latexMath/cases.json`) que leen los tests
  de las dos copias: si una cambia y la otra no, un test falla.
- **B. Filtrar en la interfaz.** Rust devuelve todo y la interfaz lee cada archivo con
  resultados para descartar los que no pasan; el reemplazo tendría que hacerse por
  posiciones sueltas. Más código, más lectura de archivos y un reemplazo frágil. Descartado.

### Tabla

- **A. Esqueleto con campos de snippet (elegido).** Como `\frac{}{}` hoy: Tab salta de celda
  en celda y después al caption y al label.
- **B. Esqueleto sin campos, como la figura.** Más simple, pero hay que moverse a mano entre
  celdas. Descartado por el usuario.

## 5. Diseño

### 5.1 Tabla

Esqueleto (2 × 2, mismo formato que el botón de tabla de la barra, `editor/commands.ts`
`table`), con la sangría de la línea donde está `\begin` y dos espacios por nivel:

```latex
\begin{table}[htbp]
  \centering
  \begin{tabular}{|c|c|}
    \hline
    ① & ② \\ \hline
    ③ & ④ \\ \hline
  \end{tabular}
  \caption{⑤}
  \label{tab:⑥}
\end{table}
```

①–⑥ son los campos, en ese orden; el cursor empieza en ①. Con `table*`, el `\end` dice
`table*`.

- Nuevo módulo `src/editor/table.ts`, hermano de `figure.ts`: arma la plantilla (snippet de
  CodeMirror, `snippet()` de `@codemirror/autocomplete`) a partir de la sangría y el nombre
  del entorno, y la aplica reemplazando un rango.
- Puntos de entrada:
  - `editor/latexCompletion.ts` → `environmentBraces`: la opción «table»/«table*» de la lista
    de entornos (como `withFigureBody`) y la opción `\begin{table}` del paquete (su `apply`
    se reemplaza por el nuestro).
  - `editor/figure.ts` → `beginOnCloseBrace`: escribir `}` tras `\begin{table` (o `table*`)
    aplica la tabla antes de probar `environmentEnd`. Si hay una `}` cerrada automáticamente
    después del cursor, se reemplaza (como en la figura).
- Un solo paso de deshacer. No se agrega ningún paquete al preámbulo.

### 5.2 Búsqueda en fórmulas

**Interfaz.** Debajo de la fila de reemplazo, un selector de tres botones contiguos que ocupa
el ancho del panel (radio group accesible):

```
┌───────────────────────────────────┐
│ \vec                     Aa ab .* │
│ reemplazar por…               ⇄  │
│ [ Todo ][ En fórmulas ][ Fuera ]  │
└───────────────────────────────────┘
```

Textos: «Todo», «En fórmulas», «Fuera de fórmulas» (si no cabe, el botón dice «Fuera» y el
nombre completo va en `title`/`aria-label`), traducidos a los seis idiomas. Por defecto
«Todo». Cambiarlo vuelve a buscar como cualquier otra opción (`setQuery`). Al cambiar de
proyecto vuelve a «Todo» (como las demás opciones hoy).

**Datos.** `SearchQuery` gana `scope: "all" | "math" | "text"` (Rust:
`#[serde(default)]`, enum `SearchScope` con `All` por defecto; TypeScript: `store/types.ts`).

**Regla.** Una coincidencia está «en fórmula» si su primer carácter está dentro de una región
matemática, delimitadores incluidos (`from ≤ inicio < to`). Lo que está en `\text{…}` dentro
de una fórmula cuenta como fórmula. Lo que está en comentarios o en verbatim cuenta como
«fuera de fórmulas».

**Rust.**

- Nuevo módulo `src-tauri/src/files/math_regions.rs` con `math_regions(text: &str) ->
  Vec<MathRegion>` (rangos en bytes; todos los delimitadores son ASCII) y las mismas reglas
  que `regions.ts`. Solo hace falta `from`/`to` para filtrar, pero se portan también
  `body_from`/`body_to`, `display`, `env` y `unclosed` para poder comprobar los casos
  compartidos completos.
- `search.rs`: `find_in_text` recibe el filtro (o las regiones) y salta las coincidencias que
  no pasan **antes** de contarlas contra el límite. Las regiones se calculan solo si el
  alcance no es «todo», una vez por archivo.
- `replace`: con alcance «todo», igual que hoy; si no, recorre las coincidencias, reemplaza
  solo las que pasan (con `Captures::expand` si es expresión regular, literal si no) y cuenta
  esas.
- A qué archivos se aplica el detector: `.tex`, `.ltx`, `.sty`, `.cls`, `.bib` (decisión mía:
  son fuentes LaTeX; un `.bib` tiene fórmulas en sus títulos). Los demás archivos no tienen
  fórmulas: con «en fórmulas» no dan resultados; con «fuera de fórmulas», todos.

**Casos compartidos.** `MATH_CASES` pasa de `cases.ts` a `src/lib/latexMath/cases.json`
(mismo contenido); `cases.ts` lo importa y conserva `withCursor` y el tipo. Rust lo lee en
sus tests con `include_str!` (como `lang.rs` lee `languages.json`) y comprueba cada caso con
la región que contiene al cursor. Los tests actuales del escáner y del árbol
(`regions.test.ts`, `editor/mathRegion.test.ts`) siguen usando los mismos casos.

**Vista previa del navegador.** `lib/projectSearch.ts` aplica el mismo filtro con
`mathRegions` de TypeScript (en `searchFiles` y `replaceText`), con la misma lista de
extensiones.

## 6. Tests (lo que prueba cada parte)

- Tabla: con una vista real de CodeMirror, por cada entrada (opción del paquete, nombre de
  entorno, `}` a mano): el texto resultante, el cursor en ①, Tab lleva a ②…⑥ en orden, la
  sangría sigue a la línea, `table*`, un solo deshacer lo quita todo.
- Rust: los casos compartidos; búsqueda con cada alcance sobre un texto con `$…$`,
  `\[…\]`, `align*`, un comentario con `$` y `\$`; el límite de resultados cuenta solo los
  filtrados; reemplazo con «en fórmulas» deja intacto lo de fuera (y al revés), también con
  expresión regular y grupos; un `.txt` con «en fórmulas» no da resultados.
- Interfaz: el selector cambia `scope` y vuelve a buscar; el backend simulado filtra igual
  que Rust (mismos ejemplos).

## 7. Lo que queda al implementador

- Nombres internos, cómo pasar el filtro a `find_in_text` (regiones o un cierre), y el
  diseño visual fino del selector dentro de los tokens existentes.
- Escapar en la plantilla del snippet lo que CodeMirror interpretaría (`\{`, `\}`, `${`).

## 8. Decisiones tomadas por mí (para revisar)

1. Esqueleto 2 × 2 con `|c|c|` y `\hline`, igual que el botón de la barra (no las líneas
   `booktabs` ni los textos `header1…` del paquete).
2. La opción `\begin{table}` del paquete de LaTeX también da nuestro esqueleto.
3. Una coincidencia se clasifica por dónde empieza; los delimitadores cuentan como fórmula.
4. Comentarios y verbatim cuentan como «fuera de fórmulas».
5. El detector solo se aplica a `.tex`, `.ltx`, `.sty`, `.cls` y `.bib`.
6. El alcance no se recuerda entre proyectos (vuelve a «Todo», como las demás opciones).
