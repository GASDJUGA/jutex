# JuTeX changelog

What's new in each version compared with the previous one, from newest to oldest. The notes
of each release (with how to install it) are in `docs/releases/<version>.md`.
Up to 1.2.2 it was called LibreTeX.

## Unreleased

### Fixes

- Linux: typing several keys at once in a long document no longer lags. Each key cost the
  editor up to ~14 ms (a scan of the whole paragraph, the formula preview redrawn, a forced
  layout of the window), and keys pressed together queued up in WebKitGTK for up to a fifth
  of a second. In the user's 1055-line document, bursts of keys now show in 14 ms (median)
  in text and 21 ms in a formula, from 59 and 95 ms.

## 2.1.0 — 2026-11-15

### New

- Six interface languages: Spanish, English, Portuguese (Brazil), French, German and
  Italian. By default, the system's (English if it's not one of those); anyone who already
  used JuTeX stays in Spanish.
- Templates in all six languages and a language for new documents (Settings and the «New
  project» dialog); the article template is called «Article».
- The compiler is chosen when creating a project (pdfLaTeX, XeLaTeX or LuaLaTeX; by default,
  the one in Settings, templates included); with pdfLaTeX the preamble uses `fontenc` and
  `lmodern` instead of `fontspec`.
- Spell checking in all six languages with `spellbook` (Rust), much faster and lighter than
  nspell.
- When opening, importing or cloning a project, its language (babel or polyglossia) decides
  the spell checking.
- A missing babel language can be installed from Logs and errors, like a missing `.sty`;
  JuTeX's own TeX Live comes with all six languages.
- macOS menu, Windows notice and the link to the LaTeX documentation in the chosen language.
- Main README in English (`README.es.md` in Spanish).

### Fixes

- Settings: selectors no longer cut off long texts.

## 2.0.0 — 2026-11-14

### New

- LibreTeX is now called **JuTeX** (_Just a TeX editor_): another editor called LibreTeX
  already exists. New logo with a J, and the app icon with a margin, the same size as other
  apps' icons in the taskbar or the Dock. Repository `GASDJUGA/jutex`.
- The first time it opens, JuTeX brings over everything from LibreTeX by itself: settings,
  project list, personal dictionary, its TeX Live, the cache, what the window remembered
  (tabs, cursor, PDF page, panels) and, in each project, `.libretex/` → `.jutex/` with its
  history. A `~/LibreTeX` projects folder keeps being used. On Windows, if LibreTeX is open,
  JuTeX asks you to close it first.
- Arch: the `jutex` package replaces `libretex`. On macOS and Windows, LibreTeX is deleted or
  uninstalled by hand (see the release notes).

### Fixes

- Linux: the heading-level selector in the editor toolbar no longer shows up white (with
  almost invisible text) when the desktop uses a light theme.

## 1.2.2 — 2026-11-13

### New

- LibreTeX opens again the way it was closed: with the same size, maximized or in full
  screen (also if it was closed while minimized), and in full screen on the same monitor (on
  Wayland, a normal or maximized window goes where the desktop puts it). And it goes back to
  the project that was in view, instead of Home.
- Folding and unfolding a folder of the file tree or a section of the outline shows: its
  contents shrink or grow and fade, instead of vanishing at once (no animation with the
  system's «reduce motion»).

### Fixes

- Typing with a long PDF open no longer lags behind the keyboard: pdf.js rearranged every
  text layer of the PDF on each key, and in WebKit every replaced viewer (each compilation,
  each zoom) stayed alive with its listeners.
- With two windows, selecting text in one while the other had something selected no longer
  makes the selection flicker.
- A copy of a project shows first in «Recently opened» (it used to be last, because it had
  never been opened).
- When a menu (File, Edit…) is opened with the mouse, its first option is no longer
  highlighted as if the pointer were over it; opened with the keyboard it is, to go through
  it with the arrow keys.
- The formula preview no longer fails with a prime right before the parenthesis of a physics
  function (`\cos(\omega t')`, `\div(J(r',t'))`): MathJax didn't close that parenthesis.
- Linux: with WebKitGTK 2.54 and NVIDIA, the screen no longer goes back to an earlier frame
  for a while (a line just typed that came and went, the outline flickering under the
  pointer).

### Known issues

- Linux (WebKitGTK 2.54, NVIDIA): when coming back to the window, the active line sometimes
  flickers for a moment (~0.3 s) between the old one and the new one. The page doesn't
  change: it's the screen.
- When typing an accent with a dead key (´ + o), WebKitGTK shows a frame without the accent
  or the letter; if the line is right at the edge, it looks like a short jump.

## 1.2.1 — 2026-11-09

### New

- More animations, in the style of the dialogs (short, smooth and turned off with the
  system's «reduce motion»): when switching tabs the view comes in from its tab's side; the
  home sections (Projects, Trash, Packages, Settings), the side-rail panels and the history
  move up or down according to their place in the list; menus and drop-downs open and close
  with a transition, and notices appear and leave with one. When switching between open
  projects, the current view leaves as soon as you click and the new one comes in whole even
  if it takes a moment to load.
- «Loading project»: if a project takes a while to open, you see the logo with a bar that
  fills as it loads (the project, its files, the editor and the PDF), like a Mac starting up;
  the project comes in when it's ready.

### Fixes

- macOS (applies to all platforms): the tab no longer jumps when going from home to a
  project. While the project came in from the side, the page was 18 px wider than the window
  and a trackpad scroll moved everything, tabs included, until the animation ended.
- Typing `\left(` inside a `\left( … \right)` pair, right before its `\right`, now also adds
  its `\right)` (it used to be taken as that pair's closer).
- The brace closes by itself right after `^` or `_` even if something is already written
  after it (`x^{|}y`, to add a missing superscript or subscript).
- Math preview with commands defined with `\DeclareRobustCommand` and with your own fontspec
  fonts (`\newfontfamily`): a script font, like Griffiths' r, shows as `\mathscr`;
  `\normalfont`, `\bfseries`, `\itshape`… inside `\text{}` no longer break it.
- Typing in the editor is faster: each letter no longer redraws the tabs, the toolbar or the
  outline, nor rereads the preamble; the outline and the macros catch up when you stop
  typing (¼ s). Moving the cursor to another line no longer redraws the whole project view.
- Linux: the text flicker while typing with WebKitGTK 2.54 and NVIDIA (a known issue in
  1.2.0) goes away with NVIDIA driver 620; no app changes needed. If the app is slow after
  updating the driver, restart the computer (with the old module loaded, WebKit draws
  without the GPU).
- Security: importing a ZIP uses `zip` 9 (previously 8.6), which closes several
  vulnerabilities with crafted ZIPs (contradictory headers, fake sizes, entries that exhaust
  memory). LibreTeX already validated each path on its own; now the library also stops
  reading an entry that goes past its declared size.
- Downloads (LibreTeX's TeX Live, the CTAN index) with `rustls` 0.23.46, which closes four
  TLS alerts.
- Development dependencies and those of the MathJax menu (`glob`, `minimatch`, `undici`) in
  versions without known vulnerabilities.
- Security (Windows): any user of the PC can create folders at the root of `C:\`, so another
  user could prepare a `C:\texlive\…` or `C:\Strawberry\…` with programs that LibreTeX ran
  when compiling, before MiKTeX. Now those folders are only used if they belong to you or to
  the system (what an installer run as administrator leaves).

## 1.2.0 — 2026-11-07

First version for **macOS** and **Windows**, besides Linux. (1.1.0 stayed as a tag of the
macOS line, without its own release: its changes are here.)

### macOS

- Shortcuts with ⌘ and a proper macOS menu; «Quit» saves before closing.
- MacTeX is found also when the app is opened from the Dock; the package manager gives the
  `sudo tlmgr install …` command to copy. LibreTeX's own TeX Live works on macOS too.
- Paste images (also those copied from Preview), trackpad pinch in the PDF viewer, the app's
  Trash (send and restore) and «Show in Finder».

### Windows

- Per-user installer, without administrator rights.
- MiKTeX as package manager (install, uninstall and update without administrator rights),
  besides the official TeX Live and LibreTeX's own.
- No console windows when compiling, and cancelling ends every process of the compilation.
- Paste screenshots, drag and drop from Explorer, «Show in Explorer», files with CRLF kept as
  they are when saving.

### On all platforms

- The file outline folds: chapters and sections collapse and expand (with the mouse or with
  ← →), remembered per project.
- Search is case-sensitive by default, in the editor and in the project («Aa» switches it).
- No autocompletion after `\\` (line break).
- Smooth transition when opening a project or going back home.
- Linux: dragging a tab out of the window opens a new window on Wayland too.

### Known issue (Linux)

With **WebKitGTK 2.54** and an **NVIDIA** card on Wayland, the editor text may flicker while
typing (it comes from WebKitGTK; 2.52 doesn't do it). Until it's fixed:
`LIBRETEX_SAFE_RENDERER=1 libretex` removes it, but draws without the GPU and is slower.

## 1.0.0 — 2026-10-26

First stable version, for Linux (Arch and AppImage).

- Projects: templates, open a folder, import a ZIP (Overleaf's too) or from GitHub; trash.
- Editor: CodeMirror with autocompletion (the document's commands and those of the loaded
  packages, `\ref`, `\cite`, paths), live checking, Spanish/English spell checking and a
  math preview.
- Compilation with latexmk (pdfLaTeX, XeLaTeX, LuaLaTeX), errors linked to the code, PDF
  viewer with SyncTeX.
- Package manager (LibreTeX's own TeX Live, the system's, or pacman's).
- Local history, backup with Git and GitHub, export to PDF and ZIP.
- Several projects in tabs and in several windows; interface themes.
- Paste images from the clipboard (screenshots) as figures.
