# Más idiomas en JuTeX — diseño

Fecha: 2026-11-14. Pedido del usuario. Este diseño se acordó en conversación; lo marcado como
**(decisión de Claude)** lo propuso Claude y el usuario puede corregirlo al leer.

## Intención

JuTeX nació para escribir en español: la interfaz está en español e inglés, pero las plantillas, el
proyecto en blanco, la ortografía por defecto y el TeX Live que instala la app suponen español. El
usuario quiere que JuTeX **llegue a más gente** y que el idioma sea _seamless_: elegir un idioma
debe bastar para que la interfaz **y los documentos nuevos** queden en ese idioma, sin configurar
nada más.

## Objetivos

1. Seis idiomas, en la interfaz y en los documentos: **español, inglés, portugués de Brasil,
   francés, alemán e italiano** (elegidos por el usuario, «por el momento»).
2. **Idioma de la interfaz por defecto = el del sistema**; si el sistema tiene uno no soportado,
   **inglés**. Lo que el usuario elija **se guarda y sobrevive a las actualizaciones** (pedido
   explícito: «si lo tengo en español, que se quede en español al actualizar»).
3. Los documentos nuevos (plantillas y proyecto en blanco) salen en el idioma elegido: babel, nombres
   de teoremas y entornos propios, comentarios y texto de ejemplo; y su ortografía revisa en ese
   idioma.
4. Al abrir, importar o clonar un proyecto, JuTeX reconoce su idioma (babel/polyglossia) y ajusta la
   ortografía a él.
5. Un documento que pide un idioma que el TeX no tiene: JuTeX lo reconoce en el log y ofrece
   instalarlo, como hoy con un `.sty` faltante.

## No objetivos

- Idiomas de otra escritura o de derecha a izquierda (ruso, chino, árabe…): más adelante; el diseño no
  debe impedirlos (códigos de idioma abiertos, no un `enum` cerrado en todas partes).
- Traducir `CLAUDE.md`, los planes, el `CHANGELOG.md` ni las notas de los releases.
- Traducir los mensajes técnicos que Rust adjunta a los errores (el detalle bajo el texto traducido).
- Portugués de Portugal como variante aparte.

## Anti-objetivos (fallaría aunque funcione)

- Que un usuario actual vea JuTeX en otro idioma tras actualizar. **Ojo:** el `settings.json` solo
  guarda las claves que el usuario cambió; el de la máquina Linux del usuario no tiene `locale` y
  su sistema está en `en_US.UTF-8`: sin una migración, al actualizar pasaría a inglés.
- Que cambiar el idioma de la interfaz toque un proyecto que ya existe (su `project.json`, su
  preámbulo o su ortografía).
- Descargar diccionarios u otra cosa de la red para cambiar de idioma (principio «local primero»).
- Una plantilla, en cualquier idioma, que no compile.
- Que la ortografía marque en rojo todas las palabras de un documento en un idioma que no tiene
  diccionario.

## Restricciones

- Las de `CLAUDE.md`: secciones 4 (local primero, seguridad), 7 (git, PR con CI verde), 8
  (definición de terminado), Linux/macOS/Windows sin romper ninguno.
- Diccionarios de Hunspell desde npm (como `dictionary-es`/`dictionary-en`), empaquetados con el
  alias de Vite que ya existe. Versiones y licencias consultadas el 2026-11-14 (todas compatibles con
  GPL-3.0-or-later):

  | Idioma      | Paquete               | Licencia            | Tamaño sin comprimir |
  | ----------- | --------------------- | ------------------- | -------------------- |
  | pt (Brasil) | `dictionary-pt` 4.0.0 | LGPL-3.0 OR MPL-2.0 | 5,5 MB               |
  | fr          | `dictionary-fr` 3.0.0 | MPL-2.0             | 1,4 MB               |
  | de          | `dictionary-de` 3.0.0 | GPL-2.0 OR GPL-3.0  | 1,1 MB               |
  | it          | `dictionary-it` 2.0.0 | GPL-3.0             | 1,4 MB               |

  (`dictionary-pt-pt` es el de Portugal; `dictionary-pt` es el de Brasil.) La app crece ~9,4 MB.

- Una dependencia nueva fuera de la tabla del stack se registra en **Decisiones**.

## Enfoques considerados (plantillas)

- **A. Una copia completa por idioma** (`templates/<plantilla>/<idioma>/…`): cada una es un `.tex`
  que compila tal cual y que alguien que hable el idioma revisa leyendo el archivo. Contra: un
  cambio de estructura se hace en 6 copias; un test vigila que no se desalineen.
- **B. Una plantilla con marcas** y un archivo de textos por idioma, reemplazadas al crear: la
  estructura se cambia una vez, pero la plantilla no compila sola y los párrafos de ejemplo quedan
  como bloques sueltos, difíciles de traducir y revisar.
- **C. Plantillas neutras** (babel pone sus nombres, sin texto de ejemplo): pierden lo que las hace
  útiles. Descartada.

**Elegida por el usuario: A.**

## Diseño

### 1. Códigos de idioma

Un solo juego de códigos en todo JuTeX: `es`, `en`, `pt`, `fr`, `de`, `it` (`pt` = portugués de
Brasil). Para cada código, en un solo lugar de Rust y uno del frontend (o uno compartido generado):
nombre propio («Español», «English», «Português (Brasil)», «Français», «Deutsch», «Italiano»), opción
de babel (`spanish`, `english`, `brazilian`, `french`, `ngerman`, `italian`), diccionario y paquetes de
TeX Live (`babel-*`, `hyphen-*`; los nombres exactos se verifican en el `texlive.tlpdb`, por ejemplo
`babel-portuges`). **(decisión de Claude)**: `ngerman` (ortografía alemana actual) y `english`
(inglés de EE. UU., el de la separación por defecto de TeX).

### 2. Idioma de la interfaz

- Ajustes → Interfaz → Idioma: **«Igual que el sistema (<idioma resuelto>)»** y los seis idiomas,
  cada uno con su nombre propio. Valor guardado en `locale`: `"system"` o un código.
  **(decisión de Claude: guardar `"system"` y no el idioma detectado en el primer arranque, así una
  instalación nueva sigue al sistema si este cambia, y una elección explícita queda fija.)**
- Idioma del sistema, leído en Rust (`platform/locale.rs`), porque también lo necesitan el menú de
  macOS y el aviso de Windows antes de que exista la ventana:
  - Linux: `LC_ALL`, `LC_MESSAGES`, `LANG` (en ese orden; `C`/`POSIX` = no soportado).
  - macOS: idiomas preferidos de `NSLocale` (`objc2-foundation`, ya en el árbol).
  - Windows: `GetUserDefaultUILanguage` (ya se usa en `platform/old_app.rs`).
  - Se toma el primer idioma soportado por su parte principal (`pt_PT` cuenta como `pt`, `de_AT` como
    `de`); si ninguno, `en`.
- **Migración para quien ya usa JuTeX:** si no hay `locale` guardado pero sí había una instalación
  anterior (existe `settings.json` o `projects.json` en la carpeta de configuración, también tras
  migrar desde LibreTeX), se escribe `"locale": "es"` (lo que veía) antes de leer los ajustes. Una
  instalación nueva queda en `"system"`. Test con los dos casos y con el `settings.json` real del
  usuario (sin `locale`, sistema en inglés → sigue en español).
- `get_settings` devuelve también el idioma efectivo (`effectiveLocale`) y el del sistema, para el
  rótulo «Igual que el sistema (…)»; el frontend usa el efectivo.
- Al arrancar no debe verse un instante otro idioma: la interfaz se dibuja ya con el idioma
  efectivo (hoy arranca en español hasta que llegan los ajustes). En la vista previa del navegador
  (backend simulado), el idioma del sistema es `navigator.languages`.
- `i18n`: `Locale` con los seis; `translate` cae en **inglés** (no en español) si falta una clave;
  el test de paridad de claves y marcadores recorre los seis diccionarios. `editor/phrases.ts`
  (textos de CodeMirror) en los seis.
- Traducciones de `pt`, `fr`, `de`, `it` hechas por Claude a partir de `es.json`/`en.json`; se
  recomienda que alguien que hable cada idioma las revise antes de publicar (anotarlo en las notas
  del release como «traducción nueva, se agradecen correcciones»).
- Textos visibles que hoy escribe Rust: el menú de macOS (`platform/menu.rs`, hoy es/en), el aviso de
  Windows (`platform/old_app.rs`, hoy es/en según el sistema), el nombre de la versión «Antes de
  restaurar …» del historial y del respaldo (hoy siempre en español: que lo escriba el frontend o
  que Rust reciba el idioma), y el enlace de «Documentación de LaTeX» (`commands/help.rs`: un
  wikibook por idioma si existe; si no, el inglés; verificar cada URL).

### 3. Idioma de los documentos nuevos

- Ajustes → nuevo ajuste **«Idioma de los documentos nuevos»**: «Igual que la interfaz» (por
  defecto) o uno de los seis. Reemplaza a `defaultSpellLang`. Migración: un `defaultSpellLang`
  guardado `es`/`en` pasa a ser ese idioma de documentos; si era `off`, los proyectos nuevos siguen
  saliendo sin ortografía (la forma de guardarlo queda al constructor; nadie pierde lo que eligió).
- El diálogo **Nuevo proyecto** trae un selector «Idioma del documento», prellenado con ese ajuste
  **(decisión de Claude: para quien escribe en dos idiomas sin cambiar Ajustes)**.
- Al crear: se copia `templates/<plantilla>/<idioma>/` y `project.json` lleva `spellLang` = ese
  idioma. El proyecto en blanco (`blank_main`) recibe el idioma: babel, comentario y nada más cambia.
- `spellLang` de `project.json` acepta los seis y `off`; un valor desconocido sigue cayendo al
  defecto (hoy `es`; ahora el idioma de documentos efectivo).

### 4. Plantillas (enfoque A)

- `templates/<plantilla>/template.json` (nombre y descripción en los seis idiomas; `Localized` deja
  de ser `{es, en}` fijo) y una carpeta por idioma con todos sus archivos (`main.tex`,
  `capitulos/…`, `referencias.bib`).
- **(decisión de Claude)** El id `articulo-xelatex-es` pasa a `articulo` (solo se usa al crear; no
  queda guardado en los proyectos) y su nombre a «Artículo XeLaTeX».
- Cada copia traduce: la opción de babel, los comentarios, los nombres de teoremas
  (`Teorema`→`Theorem`…), los nombres de los comandos y entornos propios que el usuario escribe
  (`\pregunta`/`respuesta`→`\question`/`answer`…, en cada idioma), los datos de ejemplo («Nombre
  del curso») y el texto de ejemplo. Las macros matemáticas (`\ihat`, `\jhat`, `\khat`) no cambian.
- Tests: cada plantilla tiene los seis idiomas, con la misma lista de archivos; cada `main.tex`
  carga babel con la opción de su idioma; las 24 copias y el proyecto en blanco en los seis idiomas
  **compilan** con XeLaTeX (test con TeX real, como los de compilación de hoy: se salta si no hay TeX;
  corrido de verdad en Linux antes de cerrar).

### 5. Ortografía

- Los seis diccionarios empaquetados; `SpellLang` y `SPELL_LANGS` con los seis; menú Ver con los
  seis y «Sin revisión»; diccionario personal compartido, como hoy.
- **Riesgo a medir antes de seguir:** `nspell` con el alemán (compuestos) y el portugués (5,5 MB) puede
  tardar mucho en cargar o usar mucha memoria (el español tarda ~0,5 s y ~80 MB). Medir carga y
  memoria de cada uno en el worker; si alguno pasa de ~3 s o ~300 MB, detenerse y avisar al
  usuario antes de buscar alternativas.

### 6. Reconocer el idioma de un proyecto

- Al registrar un proyecto que no tiene `spellLang` propio (abrir carpeta, importar ZIP, clonar de
  GitHub), Rust lee el preámbulo del archivo principal y busca el idioma principal:
  `\usepackage[…]{babel}` (la última opción de idioma, o `main=`), `\babelprovide[main]{…}` y
  polyglossia (`\setdefaultlanguage`, `\setmainlanguage`). Sinónimos: `spanish`; `english`,
  `american`, `USenglish`, `british`, `UKenglish`…; `brazil`, `brazilian`, `portuguese`,
  `portuges`; `french`, `francais`; `german`, `ngerman`, `austrian`, `naustrian`; `italian`.
- Idioma reconocido y soportado → ese `spellLang`. Reconocido pero sin diccionario (por ejemplo
  `russian`) → `off` (anti-objetivo: todo en rojo). Sin babel ni polyglossia → el idioma de
  documentos efectivo.
- Un proyecto con `spellLang` guardado nunca se cambia solo.

### 7. TeX

- TeX Live de JuTeX: `BASE_PACKAGES` suma babel y separación de los seis idiomas (paquetes chicos;
  medir cuánto crece la instalación, hoy 207 MB).
- Idioma que falta: babel lo informa como `Package babel Error: Unknown option 'french'. Either you
misspelled it or the language definition file french.ldf was not found` (verificar el texto con un
  log real y agregarlo como fixture). El parser de logs lo convierte en `missingFile: french.ldf`, y
  desde ahí sirve el flujo que ya existe (`package_for_file` → instalar y recompilar; con pacman, el
  grupo `texlive-langfrench`…; MiKTeX lo instala al vuelo). Lo mismo para polyglossia
  (`gloss-<idioma>.ldf`) si su error no es ya un «File not found».

### 8. Documentación y versión

- `CLAUDE.md`: secciones 1, 5.1, 5.6 y 5.8 (idiomas, ajustes, plantillas), **Estado actual** y
  **Decisiones** (dependencias nuevas, `"system"`, migración).
- README **en inglés como principal** (decisión del usuario, para llegar a más gente): `README.md`
  en inglés y `README.es.md` con el texto actual en español, cada uno con un enlace al otro al
  comienzo; los dos dicen qué idiomas trae JuTeX y cómo se elige. `CONTRIBUTING.md` y
  `SECURITY.md` siguen como están. La descripción del repositorio en GitHub, en inglés, solo con
  confirmación del usuario (cambia algo público).
- Versión **2.1.0** con notas y `CHANGELOG.md`, como cada release (publicar solo con confirmación).

## Lo que queda al constructor

- Representación exacta de los ajustes nuevos y de la migración de `defaultSpellLang = off`.
- Cómo comparten Rust y el frontend la tabla de idiomas (duplicada con un test que las compare, o
  generada).
- Los nombres exactos de los paquetes de TeX Live y de los grupos de pacman (verificar en el
  `tlpdb` y con `pacman -Ss`).
- Que los textos más largos (alemán, francés) no rompan la interfaz: revisar barras, menús,
  diálogos y Ajustes en la vista previa con cada idioma y en la ventana más angosta.
