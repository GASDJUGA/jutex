# Fusión de cambios y cambio de repositorio — plan de implementación

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** «Traer cambios» une lo que Git une solo y deja resolver en el editor los
conflictos reales; un proyecto duplicado queda sin repositorio conectado y el panel deja
cambiar o quitar el repositorio.

**Architecture:** La fusión la hace el `git` del sistema (`merge --no-ff --no-commit`), con
las marcas en los archivos; Rust lee el estado (`MERGE_HEAD`, conflictos del índice, marcas
que quedan) y lo agrega a `BackupStatus`. La interfaz dibuja cada bloque de marcas como un
recuadro con tres botones (extensión de CodeMirror) y el panel lleva la lista de conflictos,
«Terminar fusión» y «Cancelar fusión».

**Tech Stack:** Rust (`git2` 0.21, `git` del sistema vía `remote::run`), React + Zustand,
CodeMirror 6, Vitest.

**Spec:** `docs/superpowers/specs/2026-11-17-fusion-y-repositorio-design.md`

## Global Constraints

- Programas sin shell, argumentos fijos, `--` antes de rutas; rutas del frontend validadas
  con `util/paths.rs` (`resolve_existing`/`to_relative`).
- La fusión siempre con `-c merge.conflictStyle=merge` (sin sección del ancestro).
- Los mensajes de los commits que crea JuTeX llegan de la interfaz, traducidos (como `before`
  en `backup_restore`): «Antes de traer cambios» y «Fusión con GitHub».
- Textos de la interfaz en los seis idiomas (`src/i18n/{es,en,pt,fr,de,it}.json`, mismas
  claves; el test de paridad lo comprueba).
- Nada específico de Linux; CRLF tolerado al contar marcas (las líneas pueden terminar en `\r`).
- Commits, PR y `CHANGELOG.md` en inglés; `CLAUDE.md` y este plan en español.
- Cada tarea: `pnpm typecheck`, `pnpm lint`, `pnpm test`, `cargo fmt --check`,
  `cargo clippy -- -D warnings`, `cargo test` pasan antes del commit.

## Review Focus

1. **Cancelar después de editar a mano un archivo en conflicto:** `git merge --abort` descarta
   esas ediciones; el editor debe mostrar el archivo como quedó en disco (recargado), no el
   búfer con marcas que el autoguardado volvería a escribir. → test en la Tarea 5
   (`cancelar recarga los archivos y el editor deja de mostrar marcas`).
2. **Archivo con CRLF en conflicto (Windows):** las marcas terminan en `\r`; deben contarse
   y dibujarse igual. → tests en las Tareas 1 y 4 (`cuenta bloques con CRLF`,
   `reconoce marcas con CRLF`).
3. **JuTeX se cierra a mitad de la fusión:** al volver, `backup_status` trae `merge` con los
   conflictos y el panel lo muestra. → test en la Tarea 2
   (`el_estado_de_fusion_sobrevive_a_reabrir`).
4. **Un `.tex` no listado en conflicto que tiene líneas `<<<<<<< ` (un ejemplo en el
   documento):** ni se cuenta ni se dibuja. → tests en las Tareas 2 y 4.
5. **Ruta en conflicto que empieza con `-` o tiene espacios** al elegir un lado de un binario:
   `git checkout --ours -- <ruta>` con `--`. → test en la Tarea 2
   (`elegir_un_lado_de_un_binario`, con `-figura 1.png`).

---

### Task 1: Traer cambios une los dos lados (Rust)

**Files:**

- Modify: `src-tauri/src/backup/remote.rs` (`fast_forward` → `bring_in`; `Pulled`)
- Modify: `src-tauri/src/commands/backup.rs` (`backup_pull`)
- Modify: `src-tauri/src/error.rs` (variante `MergeInProgress`, código `mergeInProgress`)
- Test: `src-tauri/src/backup/mod.rs` (módulo `tests`, junto a `si_ambos_lados_cambiaron_no_mezcla_nada`, que se reemplaza)

**Interfaces:**

- Produces:
  - En `backup/merge.rs` (se crea aquí; la Tarea 2 lo completa):
    `pub enum ConflictKind { Text, Binary, Deleted }` y
    `pub struct Conflict { pub path: String, pub kind: ConflictKind, pub remaining: usize }`
    (serde `camelCase`);
    `pub fn count_blocks(text: &str) -> usize` (bloques completos `<<<<<<< ` … `=======` …
    `>>>>>>> `, cada marca al inicio de su línea, la de en medio sola en la suya, con o sin
    `\r`); `pub(crate) fn start(repo: &Repository, root: &Path) -> AppResult<Vec<Conflict>>`:
    lee los conflictos del índice (rutas con etapa 1, 2 o 3; `Deleted` si falta la 2 o la 3,
    `Binary` si alguna versión no es UTF-8, si no `Text` con `remaining = count_blocks` del
    archivo en disco) y los guarda en `.git/JUTEX_MERGE` (una línea `kind\truta` por archivo),
    para recordar cuáles eran aunque ya estén resueltos.
  - `Pulled { commits: usize, changed: Vec<String>, merged: bool, conflicts: Vec<Conflict> }`
    (`merged`: se hizo un commit de fusión; `conflicts` no vacío: quedó una fusión en curso).
  - `pub fn bring_in(root: &Path, merged_message: &str, author: Option<&Identity>) -> AppResult<Pulled>`
  - Comando `backup_pull(id: String, before: String, merged: String) -> AppResult<Pulled>`:
    si hay cambios sin respaldar, `backup::backup(root, &before, author)` (sin subir) antes del
    `fetch`.
  - `AppError::MergeInProgress` (mensaje libre), código `"mergeInProgress"`.

- [ ] **Step 1: tests que fallan.** En `merge.rs`, `cuenta_bloques`:
      `count_blocks("a\n<<<<<<< HEAD\nx\n=======\ny\n>>>>>>> origin/main\nb\n") == 1`; `== 2` con
      dos bloques; `== 0` si falta `>>>>>>> ` o si la línea del medio es `"======= x"`; con `\r\n`
      en todas las líneas `== 1` (**cuenta bloques con CRLF**, Review Focus 2). En `mod.rs` (con
      `with_origin()` y `git_in`, saltan sin `git`):
  - `traer_une_cambios_en_partes_distintas`: remoto cambia `main.tex` línea 1 de
    `"a\nb\nc\n"`, local cambia la línea 3 y respalda; `bring_in(root, "Fusión con GitHub", None)`
    → `merged == true`, `conflicts.is_empty()`, archivo == `"A\nb\nC\n"` (sin `\r`),
    `list(root)[0].message == "Fusión con GitHub"` y ese commit tiene 2 padres;
    `status(...).ahead == 2`.
  - `traer_con_conflicto_deja_la_fusion_en_curso`: los dos lados cambian la misma línea →
    `conflicts == [Conflict { path: "main.tex", kind: Text, remaining: 1 }]`, `merged == false`,
    el archivo contiene `"<<<<<<< "`, `"======="`, `">>>>>>> "` y `root/.git/MERGE_HEAD` existe.
  - `traer_respalda_antes_lo_sin_respaldar` (comando a nivel de función: llamar a la función
    que usa `backup_pull`, ver Step 3): cambio sin respaldar + remoto con cambios en otro
    archivo → hay un respaldo `"Antes de traer cambios"` debajo de la fusión y el cambio local
    sigue en el archivo.
  - El caso «solo GitHub tiene cosas nuevas» sigue cubierto por
    `subir_y_traer_cambios_de_otro_clon` (cambiar `fast_forward` por `bring_in` y comprobar
    `merged == false`).
- [ ] **Step 2:** `cargo test --manifest-path src-tauri/Cargo.toml backup::` → fallan (no
      existe `bring_in`).
- [ ] **Step 3: implementar.** `bring_in`: igual que `fast_forward` hasta `ahead > 0`; ahí, en
      vez de `Diverged`, `git -c merge.conflictStyle=merge merge --no-ff --no-commit --no-edit
<remote_ref>` con `run`. Si el índice (`git2`) no tiene conflictos: commit con padres
      `[HEAD, MERGE_HEAD]`, autor como `backup` (`author_of`), mensaje `merged_message`, y
      `repo.cleanup_state()`. Si tiene: `conflicts = merge::start(&repo, root)?`.
      Lo de antes del `fetch` va en una función `pub fn prepare_pull(root, before, author) -> AppResult<()>`
      en `backup/mod.rs`, que es la que prueba `traer_respalda_antes_lo_sin_respaldar`.
      Si `MERGE_HEAD` ya existe, `bring_in` y `prepare_pull` devuelven `MergeInProgress`.
- [ ] **Step 4:** los tests de Step 1 y los de `backup::` pasan.
- [ ] **Step 5: commit** `feat(backup): merge both sides when pulling`.

### Task 2: Estado de la fusión, terminar, cancelar y elegir lado (Rust)

**Files:**

- Modify: `src-tauri/src/backup/merge.rs` (lógica), `src-tauri/src/backup/mod.rs`
  (`BackupStatus.merge`, bloqueo de `backup`/`restore_*` durante la fusión)
- Modify: `src-tauri/src/backup/remote.rs` (`push` bloqueado durante la fusión)
- Modify: `src-tauri/src/commands/backup.rs`, `src-tauri/src/lib.rs` (registrar comandos)
- Modify: `src-tauri/src/history/mod.rs` (`snapshot` no toma versiones `Auto`/`Compile`
  con fusión en curso: devuelve `Ok(None)`)
- Test: `src-tauri/src/backup/merge.rs` (`mod tests`)

**Interfaces:**

- Consumes: `Conflict`, `ConflictKind`, `bring_in` (Tarea 1).
- Produces:
  - `pub struct MergeState { pub conflicts: Vec<Conflict> }` (serde `camelCase`) y
    `pub fn state(root: &Path) -> AppResult<Option<MergeState>>`: `None` sin `MERGE_HEAD`; si
    no, los archivos de `.git/JUTEX_MERGE` con `remaining` recalculado del disco (`Text`:
    `count_blocks`; `Binary`/`Deleted`: 1 mientras el índice tenga el conflicto, 0 después).
    Un archivo resuelto sigue listado con `remaining: 0` hasta terminar. `finish` y `abort`
    borran `JUTEX_MERGE`.
  - `pub fn finish(root: &Path, message: &str, author: Option<&Identity>) -> AppResult<()>`:
    `Forbidden` si algún `remaining > 0`; si no, registra todo como `backup` y hace el commit
    con padres `[HEAD, MERGE_HEAD]`; `cleanup_state`.
  - `pub fn abort(root: &Path) -> AppResult<Vec<String>>`: `git merge --abort`; devuelve las
    rutas que cambiaron en disco (las de `JUTEX_MERGE` más las que trajo la fusión).
  - `pub fn choose(root: &Path, path: &str, side: Side) -> AppResult<()>` con
    `pub enum Side { Ours, Theirs }` (serde `camelCase`): solo para `Binary`/`Deleted`;
    `git checkout --ours|--theirs -- <ruta>` y `git add -- <ruta>`, o `git rm -q -- <ruta>` si
    ese lado lo borró. Ruta validada (relativa, dentro del proyecto).
  - `BackupStatus.merge: Option<MergeState>`.
  - Comandos: `backup_finish_merge(id, message) -> BackupStatus`,
    `backup_abort_merge(id) -> Vec<String>`, `backup_choose(id, path, side) -> BackupStatus`.
  - `backup()`, `restore_file`, `restore_project`, `remote::push` y `bring_in` devuelven
    `MergeInProgress` con fusión en curso.

- [ ] **Step 1: tests que fallan:**
  - `el_estado_de_fusion_sobrevive_a_reabrir`: tras `bring_in` con conflicto, `state(root)`
    (leyendo del disco, sin nada en memoria) da el conflicto con `remaining: 1`; tras
    escribir el archivo sin marcas, `remaining: 0` y el `Conflict` sigue en la lista.
  - `terminar_exige_sin_marcas_y_hace_el_commit`: con marcas → `Err(Forbidden)`; sin
    marcas → `Ok`, el último respaldo tiene 2 padres y mensaje `"Fusión con GitHub"`,
    `state == None`, `.git/JUTEX_MERGE` no existe, `pending` vacío.
  - `cancelar_vuelve_a_como_estaba`: tras `bring_in` con conflicto y una edición a mano,
    `abort` → archivo == contenido local de antes, `state == None`, y la ruta está en lo
    devuelto.
  - `elegir_un_lado_de_un_binario`: conflicto en `-figura 1.png` (bytes distintos en cada
    lado) → `kind == Binary`; `choose(.., Theirs)` deja los bytes de GitHub y
    `remaining == 0`.
  - `borrado_contra_cambio`: GitHub borra `cap.tex`, local lo cambia → `kind == Deleted`;
    `choose(.., Theirs)` lo borra del disco.
  - `nada_se_respalda_ni_sube_durante_la_fusion`: `backup(..)`, `remote::push(..)` y
    `restore_project(..)` → `Err(MergeInProgress)`.
  - En `history`: `sin_versiones_automaticas_durante_la_fusion` (crear `.git/MERGE_HEAD` a
    mano en un proyecto con historial → `snapshot(.., Auto, None) == Ok(None)`; `Manual` sí
    crea versión).
  - Un `.tex` **no** listado con un bloque de marcas no aparece en `state` (**Review Focus 4**).
- [ ] **Step 2:** `cargo test ... backup::merge history::` → fallan.
- [ ] **Step 3:** implementar según las interfaces.
- [ ] **Step 4:** pasan los de Step 1 y toda la suite de Rust.
- [ ] **Step 5: commit** `feat(backup): finish, cancel and resolve a merge in progress`.

### Task 3: Duplicar sin repositorio, desconectar y cambiar de repositorio (Rust)

**Files:**

- Modify: `src-tauri/src/projects/ops.rs` (`duplicate`)
- Modify: `src-tauri/src/backup/remote.rs` (`disconnect`, `create_github_repo` con `origin`
  existente)
- Modify: `src-tauri/src/commands/backup.rs`, `src-tauri/src/lib.rs`
- Test: `src-tauri/src/projects/ops.rs`, `src-tauri/src/backup/mod.rs`

**Interfaces:**

- Produces:
  - `pub fn disconnect(root: &Path) -> AppResult<()>`: borra `origin` (`git2`
    `remote_delete`, que quita también `refs/remotes/origin/*`) y la configuración
    `branch.<rama>.remote`/`merge` de la rama actual.
  - `create_github_repo`: si `origin` existe, lo quita antes de `gh repo create` y, si `gh`
    falla, lo vuelve a poner con su dirección anterior.
  - Comando `backup_disconnect(id) -> BackupStatus`.
- [ ] **Step 1: tests que fallan:**
  - `duplicar_deja_la_copia_sin_repositorio` (en `ops.rs`, con un proyecto con `.git` y
    `origin` = `https://github.com/ana/tesis.git` y un respaldo): la copia tiene `.git`, el
    mismo número de respaldos (`backup::list`) y `status(..).remote == None`; el original sigue
    con `origin`.
  - `desconectar_quita_el_remoto_y_su_seguimiento`: con `with_origin()` y un `push` →
    `disconnect` → `status.remote == None`, `tracking == false`, `ahead == 1`; `connect_url`
    con otra dirección y `push` vuelve a fijar el seguimiento.
- [ ] **Step 2:** fallan.
- [ ] **Step 3:** en `duplicate`, tras `copy_project`, si la copia tiene `.git`:
      `remote::disconnect(&dest)` (un error aquí borra la copia, como hoy si falla copiar).
- [ ] **Step 4:** pasan.
- [ ] **Step 5: commit** `feat(backup): duplicates start unlinked; disconnect or replace the repository`.

### Task 4: Marcas de conflicto en el editor (frontend)

**Files:**

- Create: `src/lib/conflicts.ts`, `src/lib/conflicts.test.ts`
- Create: `src/editor/conflictMarkers.ts`, `src/editor/conflictMarkers.test.ts`
- Modify: `src/editor/theme.ts` (colores del recuadro con tokens del editor)

**Interfaces:**

- Produces:
  - `export type ConflictBlock = { from: number; to: number; ours: { from: number; to: number }; theirs: { from: number; to: number } }`
    (posiciones en el texto; `to` del bloque incluye el salto de línea final si lo hay).
  - `export function findConflicts(text: string): ConflictBlock[]` — la misma regla que
    `count_blocks` (Tarea 2).
  - `export type Resolution = "ours" | "theirs" | "both"` y
    `export function resolve(text: string, block: ConflictBlock, how: Resolution): ChangeSpec`
    (`both` = la parte local y después la de GitHub).
  - `export const setConflictsEnabled: StateEffectType<boolean>` y
    `export function conflictMarkers(labels: { ours: string; theirs: string; keepOurs: string; keepTheirs: string; keepBoth: string }): Extension`:
    con el efecto en `true`, cada bloque se dibuja como recuadro (encabezado «Lo mío» con los
    tres botones, separador «Lo de GitHub»; las líneas de marcas reemplazadas por esos
    widgets); cada botón despacha `resolve(...)` en una sola transacción
    (`userEvent: "input.resolve"`). Con `false`, nada.
- [ ] **Step 1: tests que fallan** (`conflicts.test.ts`, mismos casos que `cuenta_bloques`,
      incluido CRLF y `"======= x"`; `resolve` con los tres modos sobre
      `"a\n<<<<<<< HEAD\nmío\n=======\nsuyo\n>>>>>>> origin/main\nb\n"` → `"a\nmío\nb\n"`,
      `"a\nsuyo\nb\n"`, `"a\nmío\nsuyo\nb\n"`). `conflictMarkers.test.ts` con una `EditorView`
      real: con el efecto en `true` hay un `.jt-conflict` y tres botones con los textos dados;
      clic en «Quedarme con lo de GitHub» → doc `"a\nsuyo\nb\n"` y un solo `undo` vuelve al
      texto con marcas; con el efecto en `false` (**no listado**, Review Focus 4) no hay
      `.jt-conflict`.
- [ ] **Step 2:** `pnpm vitest run src/lib/conflicts.test.ts src/editor/conflictMarkers.test.ts` → fallan.
- [ ] **Step 3:** implementar.
- [ ] **Step 4:** pasan; `pnpm lint`, `pnpm typecheck`.
- [ ] **Step 5: commit** `feat(editor): show merge conflicts with buttons to resolve them`.

### Task 5: Backend, store e interfaz de la fusión y del repositorio (frontend)

**Files:**

- Modify: `src/lib/tauri.ts` (tipos y métodos), `src/lib/mockBackup.ts` (simulación),
  `src/store/backup.ts`
- Create: `src/components/backup/MergeSection.tsx`
- Modify: `src/components/backup/BackupPanel.tsx`, `src/components/backup/GitHubSection.tsx`,
  `src/components/backup/ConnectDialog.tsx` (reemplazar un remoto existente),
  `src/components/editor/CodeEditor.tsx` (`Compartment` con `conflictMarkers`, activo si la
  ruta está en `status.merge.conflicts`), `src/i18n/*.json`
- Test: `src/components/backup/MergeSection.test.tsx`, `src/store/backup.test.ts` (o el
  archivo de tests del panel que ya exista)

**Interfaces:**

- Consumes: comandos de las Tareas 1–3; `conflictMarkers`, `setConflictsEnabled` (Tarea 4).
- Produces:
  - `Backend`: `backupPull(id, before, merged): Promise<Pulled>`,
    `backupFinishMerge(id, message): Promise<BackupStatus>`,
    `backupAbortMerge(id): Promise<string[]>`,
    `backupChoose(id, path, side: "ours" | "theirs"): Promise<BackupStatus>`,
    `backupDisconnect(id): Promise<BackupStatus>`. Tipos TS `Conflict`, `MergeState`,
    `BackupStatus.merge: MergeState | null`, `Pulled.merged`, `Pulled.conflicts`.
  - Store `useBackup`: `pull()` guarda los búferes y pasa los mensajes traducidos
    (`backup.beforePull`, `backup.mergeMessage`); `finishMerge()`, `abortMerge()` (recarga
    con `reloadChangedFiles` lo que devuelve), `choose(path, side)`, `disconnect()`.
- [ ] **Step 1: tests que fallan** (con el backend simulado, que simula: divergencia sin
      conflicto → fusión; misma línea → fusión en curso con marcas en el archivo):
  - `traer con conflicto muestra la fusión y la lista`: panel con «Fusión con GitHub en
    curso», `main.tex` con «1 por resolver», «Terminar fusión» deshabilitado, «Respaldar»,
    «Subir» y «Traer cambios» deshabilitados.
  - `resolver en el editor habilita terminar`: tras quitar las marcas del búfer (y guardar),
    «Terminar fusión» se habilita; al pulsarlo desaparece la sección y hay un respaldo
    «Fusión con GitHub».
  - `cancelar recarga los archivos y el editor deja de mostrar marcas` (**Review Focus 1**):
    con una edición a mano dentro del conflicto, «Cancelar fusión» + confirmar → el búfer
    abierto tiene el contenido de antes y no hay `.jt-conflict`.
  - `cambiar repositorio`: con remoto conectado, menú «Cambiar repositorio» → «Desconectar»
    - confirmar → vuelve «Conectar con GitHub»; «Conectar otro…» abre el diálogo y la
      dirección nueva reemplaza la anterior.
  - Paridad de claves i18n (test existente) con las claves nuevas en los seis idiomas.
- [ ] **Step 2:** fallan.
- [ ] **Step 3:** implementar; textos (es): «Fusión con GitHub en curso», «N por resolver»,
      «Terminar fusión», «Cancelar fusión», «¿Cancelar la fusión? Los archivos vuelven a como
      estaban antes de traer los cambios.», «Quedarme con lo mío», «Quedarme con lo de GitHub»,
      «Ambos», «Lo mío», «Lo de GitHub», «Cambiar repositorio», «Conectar otro…», «Desconectar»,
      «¿Desconectar este proyecto de su repositorio? No se borra nada en GitHub.», «Antes de traer
      cambios», «Fusión con GitHub», «Unidos {n} cambios de GitHub», «Fusión terminada. Súbela
      con Subir», «Hay una fusión en curso: termínala o cancélala primero» (para
      `mergeInProgress`). Reescribir `errors.diverged` para el caso que queda (subir con cambios
      nuevos en GitHub): «GitHub tiene respaldos que no están aquí: trae los cambios antes de
      subir.»
- [ ] **Step 4:** pasan; `pnpm lint`, `pnpm typecheck`, `pnpm test`.
- [ ] **Step 5: commit** `feat(backup): resolve merges and change the repository from the panel`.

### Task 6: Prueba de punta a punta y documentación

**Files:**

- Modify: `CHANGELOG.md` («Unreleased»: New), `CLAUDE.md` (Estado actual, Decisiones,
  5.9), `README.md` y `README.es.md` (sección del respaldo, si describe «Traer cambios»)

- [ ] **Step 1:** con la app compilada (`pnpm tauri build --no-bundle`) en Xvfb y un HOME
      temporal, un proyecto con `origin` en un repositorio desnudo local y otro clon que cambia la
      misma línea: «Traer cambios» → recuadro con botones en el editor; «Quedarme con lo de
      GitHub» → «Terminar fusión» → `git log --graph` muestra la fusión; repetir con «Cancelar
      fusión»; duplicar el proyecto → la copia sin remoto; «Desconectar». Capturas para el PR.
- [ ] **Step 2:** documentación.
- [ ] **Step 3:** suite completa (Rust y frontend) en verde.
- [ ] **Step 4: commit** `docs: merge resolution and repository changes in the backup` y push;
      PR a `main` (se fusiona con confirmación del usuario).
