# Fusión de cambios y cambio de repositorio en el respaldo — diseño

Fecha: 2026-11-17. Pedido del usuario. Parte del respaldo con Git y GitHub (CLAUDE.md, 5.9).

## Por qué

1. **Traer cambios se detiene ante cualquier divergencia.** Si hay respaldos nuevos aquí y
   también en GitHub, JuTeX dice «únelos con git en una terminal» (`AppError::Diverged`,
   `remote::fast_forward`). El usuario quiere resolverlo dentro de JuTeX, sin perder nada.
   Hoy se rechaza incluso lo que Git une solo (archivos distintos o partes distintas de un
   archivo), que es el caso más común.
2. **Un proyecto duplicado sigue conectado al repositorio del original.** `copy_project`
   copia el `.git` entero, con `origin`, así que la copia sube al repositorio del otro. Y una
   vez conectado, el panel no deja cambiar de repositorio.

## Objetivos

- «Traer cambios» une solo lo que Git puede unir y, para los conflictos reales (las mismas
  líneas cambiadas en los dos lados), deja resolverlos en el editor de JuTeX.
- Nada se pierde: antes de unir se respalda lo local; «Cancelar fusión» deja todo como antes.
- Un proyecto duplicado no queda conectado a ningún repositorio.
- En el panel se puede cambiar el repositorio conectado: desconectar, conectar otro por
  dirección o crear uno privado nuevo en GitHub.

## No objetivos

- Rebase, elegir estrategias de fusión, fusionar ramas distintas de la actual o con otros
  remotos que `origin`.
- Resolver conflictos de archivos binarios con marcas (ver «Archivos binarios»).
- Subir automáticamente el resultado de la fusión: se sube con «Subir», como siempre (o
  con «Subir después de cada respaldo» si está activado).

## Anti-objetivos

- Perder trabajo: ni un cambio sin respaldar pisado, ni un respaldo de GitHub descartado
  sin que el usuario lo elija.
- Dejar el proyecto en un estado que solo se arregla con una terminal.
- Registrar como respaldo un archivo que todavía tiene marcas de conflicto.

## Enfoque (elegido por el usuario)

**Considerados:**

- **A. La fusión de Git, con las marcas en los archivos (elegido).** `git merge` sin
  commit: Git une lo que puede y escribe `<<<<<<<` / `=======` / `>>>>>>>` en los archivos
  con conflictos. JuTeX reconoce el estado de fusión (`MERGE_HEAD`) y dibuja sobre las marcas
  recuadros con botones. Terminar = commit; cancelar = `git merge --abort`. A favor: es el
  estado estándar de Git (sobrevive a cerrar JuTeX y lo entiende cualquier otra
  herramienta), encaja con el editor actual y es poco código. En contra: mientras hay
  conflictos los archivos tienen las marcas en el disco y compilar falla, lo esperable.
- **B. Fusión en memoria con `git2`.** Los archivos no tienen marcas hasta terminar, pero si
  JuTeX se cierra a mitad se pierde la fusión, hay mucho código propio (unir, guardar estado,
  vista aparte) y no se puede editar y compilar normalmente mientras se resuelve.

**Para resolver cada conflicto (elegido por el usuario, opción C):** botones en cada lugar
marcado («Quedarme con lo mío», «Quedarme con lo de GitHub», «Ambos») y, además, edición a
mano del archivo.

## Comportamiento

### Traer cambios

1. El usuario pulsa «Traer cambios». JuTeX guarda los búferes del proyecto.
2. Si hay cambios sin respaldar, JuTeX crea un respaldo automático con el mensaje «Antes de
   traer cambios» (se ve en la lista de respaldos como cualquier otro).
3. `git fetch` (como hoy, con progreso y cancelable).
4. Según la relación entre la rama local y la de GitHub:
   - **Nada nuevo en GitHub:** como hoy («ya está al día»).
   - **Solo hay cosas nuevas en GitHub:** avance rápido, como hoy.
   - **Los dos lados tienen respaldos nuevos:** `git merge --no-ff --no-commit
refs/remotes/origin/<rama>`.
     - **Sin conflictos:** JuTeX registra la fusión como respaldo con el mensaje «Fusión con
       GitHub» (commit con los dos padres) y recarga los archivos cambiados. Aviso: «Unidos
       N cambios de GitHub».
     - **Con conflictos:** el proyecto queda en **fusión en curso** (abajo).

### Fusión en curso

- El panel de respaldo lo dice arriba de todo («Fusión con GitHub en curso»), con la lista de
  archivos con conflictos (clic abre el archivo en el lugar del primer conflicto) y cuántos
  quedan por resolver en cada uno.
- Los demás archivos que trajo la fusión ya están en su lugar (recargados en el editor).
- En el editor, cada conflicto se ve como un recuadro con dos partes, «Lo mío» y «Lo de
  GitHub», y tres botones:
  - **Quedarme con lo mío:** reemplaza el bloque entero (marcas incluidas) por la parte local.
  - **Quedarme con lo de GitHub:** por la parte de GitHub.
  - **Ambos:** por la parte local seguida de la de GitHub.
    Cada botón es un solo paso de deshacer. El texto dentro del recuadro se puede editar a mano,
    y borrar las marcas a mano también resuelve ese conflicto.
- «Terminar fusión» (en el panel) solo se activa cuando ningún archivo del proyecto tiene
  marcas de conflicto. Guarda los búferes, registra los archivos (`git add`) y hace el commit
  de la fusión con el mensaje «Fusión con GitHub». Aviso: «Fusión terminada. Súbela con
  Subir».
- «Cancelar fusión» (con confirmación) ejecuta `git merge --abort`: los archivos vuelven a
  como estaban tras el respaldo «Antes de traer cambios». JuTeX recarga los archivos
  afectados.
- Si JuTeX se cierra o se cambia de proyecto con una fusión en curso, al volver el panel la
  muestra igual (el estado vive en el `.git` del proyecto).
- Mientras dura la fusión: «Respaldar», «Subir» y «Traer cambios» están deshabilitados (con
  la explicación al pasar el mouse); restaurar un respaldo también. Compilar sigue permitido.

### Duplicar un proyecto

- La copia conserva su historial de respaldos (el `.git` se copia), pero sin `origin`: queda
  desconectada. Su panel ofrece «Conectar con GitHub», como un proyecto nuevo.
- Si el original estaba en una fusión en curso, la copia se hace igual (copia lo que hay en
  disco); no hace falta un caso aparte.

### Cambiar de repositorio

- Con un repositorio conectado, junto a su dirección, un menú «Cambiar repositorio» con:
  - **Conectar otro…:** el diálogo de conexión actual (dirección existente o repositorio
    privado nuevo en GitHub), que reemplaza `origin`.
  - **Desconectar:** quita `origin` (con confirmación). No borra nada en GitHub.
- Tras cambiar o desconectar, «N respaldos sin subir» se recalcula; la rama remota anterior
  se olvida (`refs/remotes/origin/*` se borra con el remoto).

## Detalles que el diseño fija

- **Marcas de conflicto:** el estilo normal de Git (`<<<<<<< HEAD`, `=======`,
  `>>>>>>> origin/<rama>`), sin la sección del ancestro común (`diff3`): JuTeX pasa
  `-c merge.conflictStyle=merge` para no depender de la configuración del usuario.
- **Reconocer un conflicto en el editor:** una línea que empieza exactamente con
  `<<<<<<< `, luego una `=======` sola y luego una `>>>>>>> `, en ese orden. Solo en los
  archivos que Git lista como en conflicto (no en cualquier `.tex` que tenga esas líneas por
  otra razón).
- **Qué cuenta como resuelto:** el archivo ya no tiene ningún bloque completo de marcas.
- **Archivos binarios** (imágenes, PDF) con conflicto: no llevan marcas. En la lista del panel
  tienen dos botones, «Quedarme con lo mío» / «Quedarme con lo de GitHub»
  (`git checkout --ours|--theirs -- <archivo>`).
- **Borrado contra cambio** (un lado borró el archivo, el otro lo cambió): igual que los
  binarios, con «Quedarme con lo mío» (que puede ser «borrado») o «con lo de GitHub».
- **Identidad del autor** del commit de fusión: la misma del respaldo (5.9).
- **Historial interno** (`.jutex/history.git`): las versiones automáticas no se toman mientras
  hay una fusión en curso (tendrían marcas).
- **«Subir después de cada respaldo»** (decisión de Claude): no se aplica al respaldo
  automático «Antes de traer cambios» (se sube junto con la fusión); sí al commit de la
  fusión, sin conflictos o al terminarla, como a cualquier respaldo.
- **Desconectar** (decisión de Claude): además de quitar `origin`, se borra la rama remota de
  seguimiento de la rama actual (`branch.<rama>.remote`/`merge`), para que el estado no
  apunte a un remoto que ya no existe; el próximo «Subir» la vuelve a fijar (`push -u`).
- **Seguridad:** igual que el resto del respaldo: programas sin shell, argumentos fijos, `--`
  antes de rutas, rutas validadas con `util/paths.rs`; nada de red salvo el `fetch`.

## Restricciones

- Lo local con `git2` donde ya se usa; lo que Git hace mejor (fusionar, abortar) con el `git`
  del sistema, como `merge --ff-only` hoy (Decisiones 2026-10-18).
- Las tres plataformas: nada específico de Linux; en Windows, los archivos con CRLF se leen y
  escriben como ya hace el editor (las marcas son líneas normales).
- Textos en los seis idiomas (`src/i18n/*.json`).
- El backend simulado (`lib/mockBackup.ts`) también simula la fusión, para los tests y la
  vista previa.
- Todo lo que se ve en GitHub, en inglés (CLAUDE.md, sección 7).

## Lo que queda al implementador

- Cómo se dibuja el recuadro en CodeMirror (decoraciones de línea y widgets), mientras se vea
  como el boceto: dos partes rotuladas y tres botones.
- Los nombres de los comandos Tauri nuevos y cómo se reparte el estado de la fusión en
  `BackupStatus`.
- Si los avisos y textos de error existentes (`diverged`, `localChanges`) se reescriben o se
  quitan (ya no deberían aparecer al traer cambios, salvo casos raros).
