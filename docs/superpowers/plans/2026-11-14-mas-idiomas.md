# Más idiomas en JuTeX — plan de implementación

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** JuTeX en seis idiomas (es, en, pt, fr, de, it): la interfaz sigue al sistema (o a lo que el
usuario eligió, que se conserva al actualizar) y los documentos nuevos, la ortografía y el TeX quedan
en el idioma elegido sin configurar nada más.

**Architecture:** Una tabla única de idiomas (`src/lib/languages.json`) que leen Rust
(`include_str!`) y el frontend (import de JSON). Rust resuelve el idioma del sistema
(`platform/locale.rs`), fija los ajustes en el primer arranque (`settings::settle_first_run`) y
devuelve los idiomas efectivos en `get_settings`. Plantillas con una carpeta por idioma
(`templates/<id>/<código>/`), detección del idioma de proyectos abiertos en `projects/detect.rs`,
y el error de babel de idioma faltante convertido en un `missingFile` del parser de logs.

**Tech Stack:** Tauri 2 (Rust 2024), React + TypeScript, Vitest, nspell + diccionarios Hunspell de npm.

**Spec:** `docs/superpowers/specs/2026-11-14-mas-idiomas-design.md`

## Global Constraints

- Códigos: `es`, `en`, `pt` (portugués de **Brasil**), `fr`, `de`, `it`. Si el sistema no tiene uno
  soportado: `en`.
- Ajuste `locale`: `"system"` o un código; por defecto `"system"`. Ajuste `documentLanguage`:
  `"interface"` o un código; por defecto `"interface"`.
- Un usuario actual (hay `settings.json` o `projects.json` y no hay `locale`) queda en `"es"`.
- Nada cambia un proyecto existente salvo el usuario; nada se descarga para cambiar de idioma.
- Diccionarios: `dictionary-pt@^4.0.0`, `dictionary-fr@^3.0.0`, `dictionary-de@^3.0.0`,
  `dictionary-it@^2.0.0` (licencias en el diseño; registrarlas en **Decisiones**).
- Opciones de babel: `spanish`, `english`, `brazilian`, `french`, `ngerman`, `italian`.
- `CLAUDE.md`: secciones 4, 7 y 8; nunca `sudo`; Linux, macOS y Windows sin romper (CI verde);
  vista previa en el puerto 1430, detenerla al terminar; Xvfb se detiene con `kill $(pgrep -x Xvfb)`.
- Rama `mas-idiomas` desde `main` al día; el diseño y este plan van en el primer commit. Un commit
  - push por tarea; PR a `main` con CI verde al final; release 2.1.0 solo con confirmación.
- Mensajes de commit en español (Conventional Commits) terminados en
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Comandos de verificación: `pnpm typecheck`, `pnpm lint`, `pnpm test`,
  `cargo test --manifest-path src-tauri/Cargo.toml`,
  `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`.

## Review Focus

1. Segundo arranque de una instalación nueva: `settings.json` ya existe pero el usuario nunca eligió
   idioma → debe seguir en `"system"`, no fijarse en `"es"` (test en Task 2).
2. Idioma del sistema con variantes: `LANGUAGE=fr:en`, `pt_PT.UTF-8`, `de_AT`, `C.UTF-8`, `POSIX`,
   `en_US.UTF-8` con `LC_MESSAGES=es_CL.UTF-8` (test en Task 1).
3. Preámbulos reales: `\usepackage[english,spanish]{babel}` (manda el último),
   `\usepackage[main=english,spanish]{babel}`, una línea de babel comentada con `%`, polyglossia
   `\setdefaultlanguage[variant=brazilian]{portuguese}` (test en Task 8).
4. Un proyecto con `project.json` propio no cambia de `spellLang` al abrirlo de nuevo ni al cambiar
   el idioma de la interfaz (test en Task 8).
5. Textos largos en alemán y francés en barras, menús, diálogos y Ajustes a 1024 px de ancho
   (revisión visual en Task 11).

---

### Task 1: Tabla de idiomas e idioma del sistema

**Files:**

- Create: `src/lib/languages.json`, `src/lib/languages.ts`, `src/lib/languages.test.ts`
- Create: `src-tauri/src/lang.rs`, `src-tauri/src/platform/locale.rs`
- Modify: `src-tauri/src/lib.rs` (módulo `lang`), `src-tauri/src/platform/mod.rs`

**Interfaces:**

- Produces (JSON): un arreglo, en este orden, de
  `{ code, name, babel, aliases: string[], texlive: string[], pacman, windowsLang, wikibook, blankComment }`:

  | code | name               | babel     | texlive                           | pacman                 | windowsLang | wikibook                                       |
  | ---- | ------------------ | --------- | --------------------------------- | ---------------------- | ----------- | ---------------------------------------------- |
  | es   | Español            | spanish   | babel-spanish, hyphen-spanish     | texlive-langspanish    | 10          | https://es.wikibooks.org/wiki/Manual_de_LaTeX  |
  | en   | English            | english   | babel-english, hyphen-english     | texlive-langenglish    | 9           | https://en.wikibooks.org/wiki/LaTeX            |
  | pt   | Português (Brasil) | brazilian | babel-portuges, hyphen-portuguese | texlive-langportuguese | 22          | https://pt.wikibooks.org/wiki/Latex            |
  | fr   | Français           | french    | babel-french, hyphen-french       | texlive-langfrench     | 12          | https://fr.wikibooks.org/wiki/LaTeX            |
  | de   | Deutsch            | ngerman   | babel-german, hyphen-german       | texlive-langgerman     | 7           | https://de.wikibooks.org/wiki/LaTeX-Kompendium |
  | it   | Italiano           | italian   | babel-italian, hyphen-italian     | texlive-langitalian    | 16          | https://en.wikibooks.org/wiki/LaTeX            |

  (El wikibook italiano no existe: va el inglés. Las URL respondieron 200 el 2026-11-14.)
  `aliases` (nombres de babel/polyglossia que cuentan como ese idioma, sin distinguir mayúsculas):
  es `spanish`, `espanol`; en `english`, `american`, `USenglish`, `british`, `UKenglish`, `canadian`,
  `australian`, `newzealand`; pt `brazil`, `brazilian`, `portuguese`, `portuges`; fr `french`,
  `francais`, `frenchb`, `acadian`, `canadien`; de `german`, `ngerman`, `germanb`, `ngermanb`,
  `austrian`, `naustrian`, `swissgerman`, `nswissgerman`; it `italian`.
  `blankComment`: el comentario del proyecto en blanco, «% Idioma y fuentes (compilar con XeLaTeX)»
  traducido (en: «% Language and fonts (compile with XeLaTeX)»…).

- Produces (Rust, `lang.rs`): `pub struct Language { code, name, babel, aliases, texlive, pacman,
windows_lang: u16, wikibook, blank_comment }` (todos `String`/`Vec<String>` salvo indicado);
  `pub fn all() -> &'static [Language]`; `pub fn get(code: &str) -> Option<&'static Language>`;
  `pub fn from_locale_tag(tag: &str) -> Option<&'static str>`;
  `pub fn resolve(candidates: &[String]) -> &'static str`;
  `pub fn from_tex_name(name: &str) -> Option<&'static str>`.
- Produces (Rust, `platform/locale.rs`): `pub fn linux_candidates(var: impl Fn(&str) -> Option<String>) -> Vec<String>`
  (puro, se compila en todos los sistemas); `pub fn system_languages() -> Vec<String>`;
  `pub fn system_locale() -> &'static str` (= `lang::resolve(&system_languages())`).
- Produces (TS, `languages.ts`): `export type Locale = "es" | "en" | "pt" | "fr" | "de" | "it"`;
  `export const LANGUAGES: readonly Language[]`; `export function languageName(code: Locale): string`;
  `export function resolveLocale(tags: readonly string[]): Locale` (mismas reglas que Rust).

- [ ] **Step 1: Tests de Rust que fallan** (en `lang.rs` y `platform/locale.rs`)

```rust
#[test] fn la_tabla_tiene_los_seis_en_orden() {
    let codes: Vec<_> = all().iter().map(|l| l.code.as_str()).collect();
    assert_eq!(codes, ["es", "en", "pt", "fr", "de", "it"]);
}
#[test] fn etiquetas_del_sistema() {
    assert_eq!(from_locale_tag("pt_PT.UTF-8"), Some("pt"));
    assert_eq!(from_locale_tag("de-AT"), Some("de"));
    assert_eq!(from_locale_tag("es_CL.UTF-8@euro"), Some("es"));
    assert_eq!(from_locale_tag("C.UTF-8"), None);
    assert_eq!(from_locale_tag("POSIX"), None);
    assert_eq!(from_locale_tag("ru_RU.UTF-8"), None);
}
#[test] fn sin_idioma_soportado_es_ingles() {
    assert_eq!(resolve(&["ru".into(), "C".into()]), "en");
    assert_eq!(resolve(&["ja".into(), "fr_CA".into()]), "fr");
}
#[test] fn nombres_de_tex() {
    assert_eq!(from_tex_name("NGerman"), Some("de"));
    assert_eq!(from_tex_name("brazil"), Some("pt"));
    assert_eq!(from_tex_name("russian"), None);
}
// platform/locale.rs
#[test] fn orden_de_variables_en_linux() {
    let env = |vars: &'static [(&'static str, &'static str)]| move |k: &str|
        vars.iter().find(|(n, _)| *n == k).map(|(_, v)| v.to_string());
    // LANGUAGE (lista con «:») primero, luego LC_ALL, LC_MESSAGES, LANG.
    assert_eq!(resolve(&linux_candidates(env(&[("LANGUAGE", "fr:en"), ("LANG", "es_CL.UTF-8")]))), "fr");
    assert_eq!(resolve(&linux_candidates(env(&[("LANG", "en_US.UTF-8"), ("LC_MESSAGES", "es_CL.UTF-8")]))), "es");
    assert_eq!(resolve(&linux_candidates(env(&[("LANG", "C.UTF-8")]))), "en");
}
```

- [ ] **Step 2: Correr y ver que fallan**

Run: `cargo test --manifest-path src-tauri/Cargo.toml lang`
Expected: FAIL (no compila: `lang` no existe).

- [ ] **Step 3: Implementar** `languages.json` (tabla de arriba), `lang.rs` (parseado una vez con
      `std::sync::LazyLock` desde `include_str!("../../src/lib/languages.json")`) y `platform/locale.rs`:
      Linux `linux_candidates(|k| std::env::var(k).ok())`; macOS `NSLocale::preferredLanguages`
      (`objc2-foundation`, ya en el árbol); Windows `GetUserDefaultUILanguage` → el código cuyo
      `windows_lang` coincide con `langid & 0x3ff` (la función de `old_app.rs`).

- [ ] **Step 4: Test de TS que falla** (`src/lib/languages.test.ts`)

```ts
it("resuelve como Rust", () => {
  expect(resolveLocale(["pt-PT", "en"])).toBe("pt");
  expect(resolveLocale(["ru-RU"])).toBe("en");
  expect(resolveLocale([])).toBe("en");
});
it("el tipo Locale coincide con la tabla", () => {
  const codes: Locale[] = ["es", "en", "pt", "fr", "de", "it"];
  expect(LANGUAGES.map((l) => l.code)).toEqual(codes);
});
```

Run: `pnpm vitest run src/lib/languages.test.ts` → Expected: FAIL (módulo no existe).

- [ ] **Step 5: Implementar `languages.ts`**, correr los dos tests

Run: `cargo test --manifest-path src-tauri/Cargo.toml lang && pnpm vitest run src/lib/languages.test.ts`
Expected: PASS.

- [ ] **Step 6: Commit** (con el diseño y este plan)

```bash
git add docs/superpowers/specs/2026-11-14-mas-idiomas-design.md docs/superpowers/plans/2026-11-14-mas-idiomas.md src/lib/languages.* src-tauri/src/lang.rs src-tauri/src/platform/locale.rs src-tauri/src/platform/mod.rs src-tauri/src/lib.rs
git commit -m "feat(idiomas): tabla de idiomas e idioma del sistema"
```

### Task 2: Ajustes: idioma de la interfaz y de los documentos, migración

**Files:**

- Modify: `src-tauri/src/settings.rs`, `src-tauri/src/state.rs`, `src-tauri/src/commands/settings.rs`,
  `src-tauri/src/lib.rs` (menú de macOS con el idioma efectivo)

**Interfaces:**

- Consumes: `lang::get`, `platform::locale::system_locale()`.
- Produces: `Settings.locale` (`"system"` | código, por defecto `"system"`),
  `Settings.document_language` (`"interface"` | código, por defecto `"interface"`),
  `Settings.new_project_spellcheck: bool` (por defecto `true`); se quita `default_spell_lang` (si
  queda en el archivo, se conserva como clave desconocida).
  `impl Settings { pub fn effective_locale(&self, system: &str) -> String; pub fn effective_document_language(&self, system: &str) -> String }`.
  `pub fn settle_first_run(settings_file: &Path, projects_file: &Path) -> AppResult<()>`.
  `AppState.system_locale: &'static str`. `NewProjectDefaults { compiler, spell_lang, language }`
  (`language` = documento efectivo; `spell_lang` = ese idioma, o `"off"` si
  `new_project_spellcheck` es falso).
  `SettingsInfo` suma `system_locale`, `effective_locale`, `effective_document_language`
  (JSON: `systemLocale`, `effectiveLocale`, `effectiveDocumentLanguage`).

- [ ] **Step 1: Tests que fallan** (`settings.rs`)

```rust
#[test] fn instalacion_nueva_sigue_al_sistema_tambien_al_segundo_arranque() {
    // ningún archivo → escribe "system"; al volver a arrancar (el archivo ya existe) no cambia
    settle_first_run(&settings, &projects).unwrap();
    assert_eq!(load(&settings).unwrap().locale, "system");
    std::fs::write(&projects, "[]").unwrap();
    settle_first_run(&settings, &projects).unwrap();
    assert_eq!(load(&settings).unwrap().locale, "system");
}
#[test] fn usuario_actual_sin_locale_queda_en_espanol() {
    // el settings.json real del usuario: sin locale ni defaultSpellLang
    write(&settings, json!({"editorFontFamily": "", "keymap": "default", "texDistribution": "system"}));
    settle_first_run(&settings, &projects).unwrap();
    let s = load(&settings).unwrap();
    assert_eq!((s.locale.as_str(), s.effective_locale("en")), ("es", "es".to_owned()));
    assert_eq!(s.effective_document_language("en"), "es");
}
#[test] fn solo_projects_json_tambien_es_instalacion_anterior() { /* → "es" */ }
#[test] fn default_spell_lang_viejo_pasa_a_idioma_de_documentos() {
    // {"defaultSpellLang": "en"} → documentLanguage "en"; {"defaultSpellLang": "off"} → newProjectSpellcheck false
}
#[test] fn un_locale_guardado_no_se_toca() { /* {"locale": "de"} → sigue "de" */ }
#[test] fn valores_validos() {
    // update locale "system"/"pt" ok; "klingon" → Forbidden; documentLanguage "interface"/"it" ok; "xx" → Forbidden
}
#[test] fn efectivos() {
    // locale "system" + sistema "fr" → "fr"; documentLanguage "interface" → el de la interfaz
}
```

- [ ] **Step 2: Correr y ver que fallan**

Run: `cargo test --manifest-path src-tauri/Cargo.toml settings`
Expected: FAIL (no existen `settle_first_run`, `document_language`…).

- [ ] **Step 3: Implementar.** `settle_first_run` escribe solo si el archivo no tiene `locale`:
      instalación anterior (`settings_file` o `projects_file` existían) → `"es"` y, si había
      `defaultSpellLang` `es`/`en`, `documentLanguage` = ese; `off` → `newProjectSpellcheck: false`;
      instalación nueva → `"system"`. Se llama en `AppState::new` antes de leer los ajustes (los dos
      archivos están en la carpeta de configuración; el de proyectos es el de `registry`). `EDITABLE`
      pierde `defaultSpellLang` y suma `documentLanguage` y `newProjectSpellcheck`. El menú de macOS
      (`lib.rs` y `commands/settings.rs`) usa `effective_locale(state.system_locale)`.

- [ ] **Step 4: Correr Rust completo**

Run: `cargo test --manifest-path src-tauri/Cargo.toml && cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`
Expected: PASS (actualizar los tests viejos que usaban `defaultSpellLang`/`"es"` por defecto).

- [ ] **Step 5: Commit** — `feat(ajustes): idioma del sistema por defecto e idioma de los documentos`

### Task 3: Interfaz: seis idiomas, Ajustes y arranque sin parpadeo

**Files:**

- Modify: `src/i18n/index.ts`, `src/i18n/i18n.test.ts`, `src/i18n/es.json`, `src/i18n/en.json`,
  `src/store/types.ts`, `src/store/settings.ts`, `src/lib/settings.ts`, `src/lib/mockBackend.ts`,
  `src/components/home/SettingsTab.tsx`, `src/main.tsx`, `src/App.tsx`, `src/editor/phrases.ts`
- Create: `src/i18n/pt.json`, `src/i18n/fr.json`, `src/i18n/de.json`, `src/i18n/it.json` como copia
  de `en.json` (se traducen en Task 4)

**Interfaces:**

- Consumes: `Locale`, `LANGUAGES`, `resolveLocale` (Task 1); `SettingsInfo` nuevo (Task 2).
- Produces: `Settings.locale: Locale | "system"`, `documentLanguage: Locale | "interface"`,
  `newProjectSpellcheck: boolean`; `SpellLang = Locale | "off"`; `SettingsInfo.systemLocale`,
  `effectiveLocale`, `effectiveDocumentLanguage` (todos `Locale`); `useSettings` guarda
  `effectiveDocumentLanguage` en su estado. `translate` cae en **inglés**.

- [ ] **Step 1: Tests que fallan**
  - `i18n.test.ts`: la paridad de claves y marcadores recorre `locales` (los seis);
    `translate("fr", <clave que solo está en en>)` devuelve el texto inglés.
  - `store/settings.test.ts`: con un backend que da `locale: "system"`, `effectiveLocale: "de"`,
    `useI18n.getState().locale` queda `"de"`; elegir «es» en Ajustes llama a `updateSettings({ locale: "es" })`.
  - `SettingsTab.test.tsx`: el selector de idioma muestra «Igual que el sistema (Deutsch)» y los seis
    nombres propios; «Idioma de los documentos nuevos» muestra «Igual que la interfaz» y los seis;
    la casilla «Revisar la ortografía de los proyectos nuevos».
  - `mockBackend`: `systemLocale` = `resolveLocale(navigator.languages)`.

Run: `pnpm vitest run src/i18n src/store/settings.test.ts src/components/home/SettingsTab.test.tsx`
Expected: FAIL.

- [ ] **Step 2: Implementar.** Claves nuevas en `es.json`/`en.json` (`settings.localeSystem`
      con `{name}`, `settings.documentLanguage`, `settings.documentLanguageInterface`,
      `settings.newProjectSpellcheck`; se quitan las de `defaultSpellLang`). `phrases.ts`: un objeto por
      idioma (los de pt/fr/de/it se llenan en Task 4; mientras, inglés de CodeMirror). Arranque:
      `main.tsx` espera `useSettings.getState().load()` (máx. 1500 ms; si falla o tarda, dibuja igual)
      antes de `render`, y `App.tsx` deja de pedir la primera carga.

- [ ] **Step 3: Verificar**

Run: `pnpm typecheck && pnpm lint && pnpm test`
Expected: PASS.

- [ ] **Step 4: Ver en la vista previa** (puerto 1430): Ajustes con los selectores; cambiar a «English»
      y volver; recargar la página: no se ve un instante otro idioma. Detener la vista previa.

- [ ] **Step 5: Commit** — `feat(i18n): seis idiomas, idioma del sistema y ajustes de idioma`

### Task 4: Traducciones de la interfaz (pt, fr, de, it)

**Files:**

- Modify: `src/i18n/pt.json`, `fr.json`, `de.json`, `it.json`, `src/editor/phrases.ts`

**Interfaces:**

- Consumes: claves de `es.json`/`en.json` (Task 3). Produces: nada nuevo.

- [ ] **Step 1: Test que falla** (`i18n.test.ts`): ningún texto de pt/fr/de/it es idéntico al inglés,
      salvo una lista explícita de excepciones (nombres propios, «PDF», «ZIP», «LaTeX», atajos…).

Run: `pnpm vitest run src/i18n` → Expected: FAIL (son copias del inglés).

- [ ] **Step 2: Traducir** desde `es.json` y `en.json` (el español es el original). Mantener
      `{marcadores}` y formas `one`/`other`. Trato al usuario: de «Sie», fr «vous», it «tu»
      (imperativo), pt-BR «você». Términos: compilar, plantilla, respaldo, historial con el término que usa Overleaf/TeXstudio en
      ese idioma.

- [ ] **Step 3: Verificar** — Run: `pnpm test && pnpm lint` → Expected: PASS.

- [ ] **Step 4: Commit** — `feat(i18n): interfaz en portugués, francés, alemán e italiano`

### Task 5: Textos que escribe Rust

**Files:**

- Modify: `src-tauri/src/platform/menu.rs`, `src-tauri/src/platform/old_app.rs`,
  `src-tauri/src/commands/help.rs`, `src-tauri/src/history/mod.rs`, `src-tauri/src/backup/mod.rs`,
  sus comandos (`commands/history.rs`, `commands/backup.rs`), `src/lib/tauri.ts`, `src/store/history.ts`,
  `src/lib/mockBackend.ts`, `src/i18n/*.json`

**Interfaces:**

- Consumes: `lang::get`, `lang::resolve`, `platform::locale::system_languages` (Task 1).
- Produces: `menu::texts(locale: &str) -> [&'static str; 6]` con los seis idiomas;
  `old_app` elige su texto con `lang::resolve(&system_languages())`;
  `help::latex_docs(locale) -> &'static str` = `lang::get(locale).wikibook` (inglés si no existe);
  restaurar en historial y respaldo reciben `before: String` (el nombre de la versión «Antes de
  restaurar …», ya traducido por el frontend con `history.beforeRestoreFile` /
  `history.beforeRestoreProject`); Rust cambia saltos de línea y controles por espacios y corta a
  200 caracteres (no puede romper la línea `JuTeX-Kind`).

- [ ] **Step 1: Tests que fallan**
  - `menu.rs`: `texts("de")[0] == "Bearbeiten"`, `texts("xx") == texts("en")`.
  - `help.rs`: `latex_docs("it")` es la URL inglesa; `latex_docs("pt") == "https://pt.wikibooks.org/wiki/Latex"`.
  - `history/mod.rs`: restaurar con `before = "Before restoring «main.tex»\nJuTeX-Kind: manual"`
    deja una versión con nombre de una sola línea y con el mismo tipo que hoy tiene la versión
    «Antes de restaurar» (no `manual`).
  - `old_app.rs`: el texto para `fr` empieza con «LibreTeX est ouvert».

Run: `cargo test --manifest-path src-tauri/Cargo.toml` → Expected: FAIL.

- [ ] **Step 2: Implementar** y llevar el texto desde el frontend (`store/history.ts` y el panel de
      respaldo arman `before` con `t(...)`).

- [ ] **Step 3: Verificar** — Run: `cargo test … && cargo clippy … -- -D warnings && pnpm test`
      Expected: PASS. (`old_app.rs` y `menu.rs` solo compilan en su sistema: el CI de Windows y macOS los
      prueba; en Linux, los tests puros de texto van fuera del `cfg`.)

- [ ] **Step 4: Commit** — `feat(idiomas): menú de macOS, aviso de Windows, ayuda y versiones en el idioma`

### Task 6: Ortografía en los seis idiomas

**Files:**

- Modify: `package.json`, `pnpm-lock.yaml`, `vite.config.ts` (alias `@dictionaries/(es|en|pt|fr|de|it)`),
  `src/lib/spell/checker.ts`, `src/store/types.ts`, `src-tauri/src/projects/meta.rs`
  (`SPELL_LANGS` desde `lang::all()` + `"off"`), el menú Ver (idioma de ortografía),
  `src/lib/mockBackend.ts`
- Create: `scripts/measure-dictionaries.mjs`

**Interfaces:**

- Consumes: `Locale` (Task 1). Produces: `SpellLang = Locale | "off"` aceptado por
  `update_project_meta`; `spell_lang_or_default(lang, fallback: &str) -> String` (cae en el idioma
  de documentos efectivo, no en `"es"`).

- [ ] **Step 1: Medir antes de integrar.** `pnpm add dictionary-pt@^4.0.0 dictionary-fr@^3.0.0
dictionary-de@^3.0.0 dictionary-it@^2.0.0`; `scripts/measure-dictionaries.mjs` carga cada
      diccionario con `nspell` en Node y escribe tiempo de carga, `heapUsed` y el tiempo de revisar 1000
      palabras y de 20 sugerencias.

Run: `node scripts/measure-dictionaries.mjs`
Expected: una línea por idioma. **Si alguno pasa de ~3 s de carga o ~300 MB, detenerse y avisar al
usuario** (diseño §5) antes de seguir con esta tarea.

- [ ] **Step 2: Tests que fallan**
  - `meta.rs`: `update_project_meta` acepta `spellLang: "de"` y rechaza `"ru"`;
    `spell_lang_or_default("klingon", "fr") == "fr"`.
  - `spellcheck.test.ts`: con `fr`, «maison» no se marca y «maisonn» sí; con `de`, «Haus» no se
    marca; con `pt`, «ação» no se marca.
  - el menú Ver lista los seis idiomas y «Sin revisión».

Run: `pnpm vitest run src/editor/spellcheck.test.ts && cargo test --manifest-path src-tauri/Cargo.toml meta`
Expected: FAIL.

- [ ] **Step 3: Implementar**; correr `pnpm test && cargo test …` → Expected: PASS.

- [ ] **Step 4: En WebKitGTK** (`pnpm tauri dev` o el binario en Xvfb): con un proyecto en `de` y otro
      en `pt`, tiempo hasta las primeras marcas (hoy ~0,7 s en español); anotarlo en el commit.

- [ ] **Step 5: Commit** — `feat(ortografía): diccionarios de portugués, francés, alemán e italiano`
      (y en **Decisiones**: dependencias y licencias).

### Task 7: Plantillas por idioma: mecanismo, español e inglés

**Files:**

- Move: `templates/articulo-xelatex-es/` → `templates/articulo/`; en cada plantilla, sus archivos
  (menos `template.json`) → `templates/<id>/es/`
- Create: `templates/<id>/en/…` (traducción al inglés de cada plantilla)
- Modify: `templates/*/template.json`, `src-tauri/src/projects/templates.rs`,
  `src-tauri/src/projects/ops.rs` (`create`, `blank_main`), `src-tauri/src/commands/projects.rs`,
  `src/lib/tauri.ts`, `src/store/projects.ts`, `src/components/home/ProjectDialogs.tsx`,
  `src/mocks/templates.ts`, `src/lib/mockBackend.ts`, tests que nombran `articulo-xelatex-es`

**Interfaces:**

- Consumes: `lang::get` (Task 1), `NewProjectDefaults.language` (Task 2), `effectiveDocumentLanguage`
  (Task 3).
- Produces: `template.json` con `name`/`description` como objeto de los seis códigos (Rust
  `Localized = BTreeMap<String, String>`; TS `Record<Locale, string>`); `languages: Vec<String>`
  en `TemplateInfo` (las carpetas de idioma que tiene);
  `templates::copy_into(root: &Path, id: &str, language: &str, dest: &Path) -> AppResult<TemplateInfo>`;
  `blank_main(title: &str, language: &Language) -> String`;
  `Ctx::create(&self, name: &str, template: Option<&str>, language: Option<&str>) -> AppResult<ProjectSummary>`
  (`None` = idioma de documentos efectivo; idioma no soportado → `AppError::Forbidden`); comando
  `create_project(name, template, language: Option<String>)`; TS
  `createProject(name: string, template: string | null, language: Locale): Promise<ProjectSummary>`.
  El `spellLang` del proyecto nuevo = `language` (o `"off"` si `newProjectSpellcheck` es falso).

- [ ] **Step 1: Tests que fallan** (`templates.rs`, `ops.rs`, `ProjectsTab.test.tsx`)

```rust
#[test] fn cada_plantilla_tiene_la_misma_estructura_en_cada_idioma() {
    // para cada plantilla y cada idioma de `languages`: mismas carpetas (por profundidad), misma
    // cantidad de archivos y mismas extensiones que en "es" (los nombres pueden traducirse)
}
#[test] fn cada_main_carga_babel_con_su_idioma() {
    // templates/<id>/<code>/main.tex contiene format!("[{}]{{babel}}", lang::get(code).babel)
}
#[test] fn proyecto_en_blanco_en_ingles() {
    let p = f.ctx.create("Thesis", None, Some("en")).unwrap();
    // main.tex contiene "\\usepackage[english]{babel}" y "% Language and fonts"; spell_lang "en"
}
#[test] fn plantilla_en_un_idioma_que_no_tiene_es_un_error_claro() { /* UnknownTemplate */ }
```

En la interfaz: el diálogo «Nuevo proyecto» tiene «Idioma del documento» prellenado con
`effectiveDocumentLanguage` y crea con ese idioma; el id por defecto es `articulo`.
Run: `cargo test --manifest-path src-tauri/Cargo.toml templates ops && pnpm vitest run src/components/home`
Expected: FAIL.

- [ ] **Step 2: Implementar y traducir al inglés.** En cada copia se traducen: opción de babel,
      comentarios, nombres de teoremas, comandos y entornos propios (`\pregunta`/`respuesta` →
      `\question`/`answer`; `\curso`, `\tarea`, `\estudiante`, `\profesor` → `\course`, `\assignment`,
      `\student`, `\instructor`), datos de ejemplo y texto de ejemplo, y el nombre del archivo del
      capítulo (`capitulos/01-introduccion.tex` → `chapters/01-introduction.tex`, con su `\input`). No
      cambian `\ihat`, `\jhat`, `\khat`. Nombre del artículo: «Artículo XeLaTeX» / «XeLaTeX article».
      Mientras falten los otros cuatro idiomas (Task 9), el selector solo ofrece los idiomas que la
      plantilla elegida tiene (`languages`).

- [ ] **Step 3: Test de compilación** (como los de compilación real de hoy: se salta sin TeX):
      `todas_las_plantillas_compilan` crea cada plantilla × idioma disponible y el proyecto en blanco en
      cada idioma, y compila con `latexmk -xelatex`; sin errores.
      Run: `cargo test --manifest-path src-tauri/Cargo.toml todas_las_plantillas_compilan -- --include-ignored`
      Expected: PASS con `es` y `en`.

- [ ] **Step 4: Verificar** — Run: `pnpm typecheck && pnpm lint && pnpm test && cargo test …`
      Expected: PASS.

- [ ] **Step 5: Commit** — `feat(plantillas): una carpeta por idioma; plantillas en inglés`

### Task 8: Reconocer el idioma de un proyecto abierto

**Files:**

- Modify: `src-tauri/src/projects/detect.rs`, `src-tauri/src/projects/ops.rs` (`open_folder`)

**Interfaces:**

- Consumes: `lang::from_tex_name` (Task 1), `NewProjectDefaults` (Task 2).
- Produces: `pub enum DetectedLanguage { Supported(&'static str), Unsupported, Unknown }`;
  `pub fn detect_language(source: &str) -> DetectedLanguage` (sobre el texto sin comentarios `%`;
  hasta `\begin{document}`). En `open_folder`, solo si no había `project.json`:
  `Supported(c)` → `spellLang = c`; `Unsupported` → `"off"`; `Unknown` → el `spell_lang` de
  `NewProjectDefaults`. Importar ZIP y clonar pasan por `open_folder`.

- [ ] **Step 1: Tests que fallan** (`detect.rs`, `ops.rs`)

```rust
#[test] fn idioma_del_preambulo() {
    use DetectedLanguage::*;
    assert_eq!(detect_language("\\usepackage[english,spanish]{babel}"), Supported("es"));
    assert_eq!(detect_language("\\usepackage[main=english,spanish]{babel}"), Supported("en"));
    assert_eq!(detect_language("%\\usepackage[french]{babel}\n\\usepackage[ngerman]{babel}"), Supported("de"));
    assert_eq!(detect_language("\\usepackage{polyglossia}\n\\setdefaultlanguage[variant=brazilian]{portuguese}"), Supported("pt"));
    assert_eq!(detect_language("\\setmainlanguage{italian}"), Supported("it"));
    assert_eq!(detect_language("\\babelprovide[main,import]{french}"), Supported("fr"));
    assert_eq!(detect_language("\\usepackage[russian]{babel}"), Unsupported);
    assert_eq!(detect_language("\\usepackage[T1]{fontenc}\n\\usepackage{babel}"), Unknown);
    assert_eq!(detect_language("\\documentclass[french,a4paper]{article}\n\\usepackage{babel}"), Supported("fr"));
}
#[test] fn abrir_carpeta_toma_el_idioma_del_documento() { /* main.tex con [french]{babel} → spell_lang "fr" */ }
#[test] fn idioma_sin_diccionario_desactiva_la_ortografia() { /* [russian] → "off" */ }
#[test] fn un_proyecto_con_project_json_no_cambia() {
    // project.json con spellLang "es" y main.tex con [english]{babel}: abrir de nuevo → sigue "es"
}
```

(Las opciones de idioma pueden ir en `\documentclass[…]`, que babel toma como globales: se miran
ahí también, y manda la última de babel.)

Run: `cargo test --manifest-path src-tauri/Cargo.toml detect ops` → Expected: FAIL.

- [ ] **Step 2: Implementar**; Run: `cargo test … && cargo clippy … -- -D warnings` → Expected: PASS.

- [ ] **Step 3: Commit** — `feat(proyectos): reconocer el idioma del documento al abrirlo`

### Task 9: Plantillas en portugués, francés, alemán e italiano

**Files:**

- Create: `templates/<id>/{pt,fr,de,it}/…` para `articulo`, `apuntes`, `tarea`, `beamer`
- Modify: `templates/*/template.json` (nombres y descripciones en los seis)

**Interfaces:**

- Consumes: estructura y tests de Task 7. Produces: nada nuevo.

- [ ] **Step 1: Hacer que falten los idiomas:** el test de Task 7 exige ahora los seis idiomas en cada
      plantilla y los seis nombres en `template.json`.
      Run: `cargo test --manifest-path src-tauri/Cargo.toml templates` → Expected: FAIL.

- [ ] **Step 2: Traducir** desde la copia en español (la original) con las reglas de Task 7; los
      nombres de comandos propios en cada idioma (de: `\frage`/`antwort`; fr: `\question`/`reponse`;
      it: `\domanda`/`risposta`; pt: `\questao`/`resposta`, sin tildes en nombres de comandos).

- [ ] **Step 3: Compilar las 24 y el blanco en los seis.** Este Linux no tiene `french.ldf`,
      `ngerman.ldf`, `italian.ldf` ni `brazilian.ldf` en el TeX del sistema: pedir al usuario
      `sudo pacman -S texlive-langfrench texlive-langgerman texlive-langitalian texlive-langportuguese`,
      o instalar `babel-*`/`hyphen-*` en el TeX Live de JuTeX con su `tlmgr` (sin sudo) y elegirlo.
      Run: `cargo test --manifest-path src-tauri/Cargo.toml todas_las_plantillas_compilan -- --include-ignored`
      Expected: PASS con los seis idiomas.

- [ ] **Step 4: Commit** — `feat(plantillas): plantillas en portugués, francés, alemán e italiano`

### Task 10: TeX: idiomas en el TeX Live de JuTeX e idioma faltante

**Files:**

- Modify: `src-tauri/src/tex/install_tl.rs` (`BASE_PACKAGES`), `src/lib/latexLog/parse.ts`,
  `src/lib/latexLog/parse.test.ts`
- Create: `tests/fixtures/logs/babel-missing-language.log` (y su fuente en `tests/fixtures/logs/sources/`)

**Interfaces:**

- Consumes: `lang::all()` (`texlive` de cada idioma). Produces: el diagnóstico
  `{ kind: "missingFile", missingFile: "<idioma>.ldf" }` para el error de babel, que sigue el camino
  existente (`package_for_file` → «Instalar «babel-french» y recompilar»; pacman: el grupo).

- [ ] **Step 1: Fixture real.** Compilar con el TeX del sistema (que no tiene `french.ldf`) un
      documento con `\usepackage[french]{babel}` y guardar el `.log` (usuario y rutas de ejemplo: «ana»,
      `~/JuTeX`). Verificar el texto exacto del error (babel lo parte en varias líneas).

- [ ] **Step 2: Tests que fallan**
  - `parse.test.ts`: el fixture da un error con `kind: "missingFile"` y `missingFile: "french.ldf"`.
  - `install_tl.rs`: `BASE_PACKAGES` incluye `babel-french`, `hyphen-german`, `babel-portuges` y
    sigue incluyendo `xetex`, `latexmk` y `synctex`.
  - catálogo: `package_for_file("french.ldf")` → `babel-french` (con el tlpdb del sistema; se salta si no está).

Run: `pnpm vitest run src/lib/latexLog && cargo test --manifest-path src-tauri/Cargo.toml install_tl catalog`
Expected: FAIL.

- [ ] **Step 3: Implementar.** `BASE_PACKAGES` deja de ser un arreglo fijo: una función
      `base_packages() -> Vec<String>` (los de hoy sin los de español + los `texlive` de `lang::all()`).
      Si polyglossia ya informa `File 'gloss-x.ldf' not found`, no hace falta nada más (comprobarlo).

- [ ] **Step 4: Verificar** — Run: `pnpm test && cargo test …` → Expected: PASS. Instalación real del
      TeX Live de JuTeX en una carpeta temporal (test ignorado `instalacion_real`): anotar cuánto creció
      (hoy 207 MB) y compilar la plantilla `articulo/fr` con él.

- [ ] **Step 5: Commit** — `feat(tex): idiomas en el TeX Live de JuTeX e instalar el idioma que falta`

### Task 11: Revisión visual de la interfaz en cada idioma

**Files:**

- Modify: componentes donde un texto largo desborde (según lo que se vea)

**Interfaces:** ninguna nueva.

- [ ] **Step 1: Recorrer en la vista previa** (puerto 1430, ventana de 1024×700) en `de`, `fr`, `pt` e
      `it`: inicio (las cuatro pestañas), «Nuevo proyecto», vista de proyecto (barra superior, menús, riel,
      barra del editor, panel PDF, Registros, Paquetes, Historial, Respaldo), Ajustes completo, atajos.
      Expected: ningún texto cortado sin elipsis, ningún botón partido en dos líneas ni desplazamiento
      horizontal. Cada arreglo, con su test si es lógica; si es CSS, captura antes y después.

- [ ] **Step 2: En WebKitGTK** (Xvfb, `GDK_BACKEND=x11`, HOME vacío): arrancar con `LANG=de_DE.UTF-8`
      → la interfaz sale en alemán desde el primer cuadro; crear un proyecto → plantilla en alemán,
      ortografía en alemán; compilar.

- [ ] **Step 3: Verificar** — Run: `pnpm typecheck && pnpm lint && pnpm test` → Expected: PASS.

- [ ] **Step 4: Commit** — `fix(interfaz): textos largos en alemán y francés`

### Task 12: Documentación y README en inglés

**Files:**

- Create: `README.es.md` (el README actual, actualizado)
- Modify: `README.md` (en inglés), `scripts/screenshots.py` (acepta el idioma), `docs/screenshots/`,
  `CLAUDE.md` (secciones 1, 5.1, 5.6, 5.8, **Estado actual**, **Decisiones**), `CHANGELOG.md`
  («2.1.0 (sin publicar)»), `docs/releases/2.1.0.md`

**Interfaces:** ninguna.

- [ ] **Step 1: README.** `README.md` en inglés y `README.es.md` en español, cada uno con un enlace al
      otro en su primera línea («Leer en español» / «Read in English»); los dos dicen los seis idiomas y
      que la interfaz sigue al sistema (Ajustes → Interfaz para cambiarla). Capturas en inglés para
      `README.md` (`docs/screenshots/en/`) y las de hoy para `README.es.md`.

- [ ] **Step 2: Notas.** `docs/releases/2.1.0.md` y `CHANGELOG.md`: novedades (seis idiomas, el
      sistema decide la primera vez, plantillas por idioma, ortografía, idioma reconocido al abrir,
      instalar el idioma que falta) y que las traducciones nuevas son de Claude y se agradecen
      correcciones. **Decisiones**: tabla de idiomas única, `"system"`, migración, dependencias.

- [ ] **Step 3: Verificar** — Run: `pnpm lint` (Prettier revisa los `.md`) → Expected: PASS.

- [ ] **Step 4: Commit** — `docs: README en inglés y notas de la 2.1.0`

### Task 13: PR, otras plataformas y release (con confirmación)

**Files:**

- Modify: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`,
  `packaging/arch/PKGBUILD` (`pkgver`) → **2.1.0**

- [ ] **Step 1:** versión 2.1.0 en los cuatro archivos; Run: `pnpm test && cargo test …` → PASS; commit
      `chore(release): versión 2.1.0`; push y PR a `main`. Fusionar solo cuando el usuario lo diga con el
      CI verde (Linux, macOS y Windows).
- [ ] **Step 2:** macOS y Windows (en sus propios chats) revisan: idioma del sistema
      (`NSLocale`, `GetUserDefaultUILanguage`), menú de macOS y aviso de Windows en el idioma, y que un
      usuario actual siga en español; suben sus instaladores al borrador.
- [ ] **Step 3:** Linux crea el borrador `v2.1.0` con el AppImage y el paquete de Arch, fija el commit y
      publica **solo con confirmación del usuario**. Cambiar la descripción del repositorio de GitHub al
      inglés también solo con su confirmación.
