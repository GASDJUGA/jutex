"""Screenshots for docs/screenshots: the browser preview (sample data, no personal files)
rendered by WebKitGTK, the engine of the app, at 1440×900.

Usage (with the preview running, `pnpm dev --port 1430`):

    Xvfb :97 -screen 0 1440x900x24 &
    DISPLAY=:97 WEBKIT_DISABLE_DMABUF_RENDERER=1 python3 scripts/screenshots.py docs/screenshots
    DISPLAY=:97 WEBKIT_DISABLE_DMABUF_RENDERER=1 python3 scripts/screenshots.py docs/screenshots/en en

The second argument is the interface language (`es` by default): the page gets it as the
browser's language, and the buttons are found by their text in src/i18n/<language>.json.
Needs python-gobject and webkit2gtk-4.1. Writes inicio, proyecto, paquetes, registros,
historial and respaldo .png (the README uses all but registros).
"""
import json
import os
import sys

import gi

gi.require_version("Gtk", "3.0")
gi.require_version("WebKit2", "4.1")
from gi.repository import GLib, Gtk, WebKit2  # noqa: E402

OUT = sys.argv[1]
LANG = sys.argv[2] if len(sys.argv) > 2 else "es"
URL = "http://localhost:1430/"

with open(os.path.join(os.path.dirname(__file__), "..", "src", "i18n", f"{LANG}.json")) as f:
    MESSAGES = json.load(f)


def t(key, **params):
    """The interface text `key` in LANG, as a JavaScript string."""
    text = MESSAGES
    for part in key.split("."):
        text = text[part]
    for name, value in params.items():
        text = text.replace("{" + name + "}", value)
    return json.dumps(text, ensure_ascii=False)

W, H = 1440, 900

win = Gtk.Window()
win.set_default_size(W, H)
WebKit2.WebContext.get_default().set_preferred_languages([LANG])
view = WebKit2.WebView()
win.add(view)
win.show_all()

steps = []


def js(code, then):
    def done(v, res):
        try:
            r = v.evaluate_javascript_finish(res)
            val = r.to_json(0) if r is not None else None
        except Exception as e:  # noqa: BLE001
            val = f"ERR {e}"
        print("js:", (val or "")[:200], flush=True)
        then()

    view.evaluate_javascript(code, -1, None, None, None, done)


def shot(name, then):
    def done(v, res):
        surface = v.get_snapshot_finish(res)
        surface.write_to_png(f"{OUT}/{name}.png")
        print("shot", name, flush=True)
        then()

    view.get_snapshot(
        WebKit2.SnapshotRegion.VISIBLE, WebKit2.SnapshotOptions.NONE, None, done
    )


def run(i=0):
    if i >= len(steps):
        Gtk.main_quit()
        return
    kind, arg, wait = steps[i]
    nxt = lambda: GLib.timeout_add(wait, lambda: (run(i + 1), False)[1])  # noqa: E731
    if kind == "js":
        js(arg, nxt)
    else:
        shot(arg, nxt)


def step(kind, arg, wait=600):
    steps.append((kind, arg, wait))


# Helpers available in the page.
HELP = r"""
window.$click = (pred) => { const b = [...document.querySelectorAll('button,[role=tab],[role=menuitem]')].find(pred); if (b) b.click(); return !!b; };
window.$label = (t) => $click(b => (b.getAttribute('aria-label') || b.textContent.trim()) === t);
window.$view = () => { const c = document.querySelector('.cm-content'); return c && (c.cmView?.view ?? c.cmTile?.view); };
'ok'
"""

step("js", HELP, 300)
step("shot", "inicio", 300)
step("js", f"$label({t('projects.open', name='Tesis de magíster')})", 2000)
step("js", HELP, 200)
step("js", f"$label({t('compile.recompile')})", 4000)
# A formula with the user's macros, the cursor inside: the live preview shows.
step(
    "js",
    r"""(() => { const v = $view(); const d = v.state.doc.toString();
      const at = d.indexOf('\\subsection{Motivación}');
      const block = '\\begin{align*}\n  \\vec{v} &= a\\,\\ihat + b\\,\\jhat \\\\\n  \\lVert \\vec{v} \\rVert &= \\sqrt{a^2 + b^2}\n\\end{align*}\n\n';
      v.dispatch({ changes: { from: at, insert: block } });
      const cur = at + block.indexOf('\\jhat') + 5;
      v.focus(); v.dispatch({ selection: { anchor: cur }, scrollIntoView: false });
      v.dispatch({ ...v.state.replaceSelection(' '), userEvent: 'input.type' });
      document.querySelector('.cm-scroller').scrollTop = 0;
      return cur; })()""",
    2000,
)
step("js", f"$label({t('rail.files')})", 700)
step("shot", "proyecto", 300)
step("js", f"$label({t('rail.packages')})", 1500)
step(
    "js",
    r"""(() => { const i = document.querySelector('input[type=search], input[placeholder*="CTAN"]');
      const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
      set.call(i, 'units'); i.dispatchEvent(new Event('input', { bubbles: true })); return !!i; })()""",
    1500,
)
step("shot", "paquetes", 300)
step("js", f"$label({t('rail.logs')})", 800)
step("shot", "registros", 300)
# A named version, then the history with its diff.
step("js", f"$label({t('menu.file')})", 400)
step("js", f"$click(b => b.textContent.includes({t('history.saveVersion')}.replace('…', '')))", 600)
step(
    "js",
    r"""(() => { const i = document.querySelector('[role=dialog] input');
      const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
      set.call(i, VERSION_NAME); i.dispatchEvent(new Event('input', { bubbles: true }));
      i.form.requestSubmit(); return true; })()""".replace(
        "VERSION_NAME", json.dumps("Entrega 1" if LANG == "es" else "Draft 1")
    ),
    1200,
)
step("js", f"$label({t('rail.logs')})", 400)
step("js", f"document.querySelectorAll('[aria-label^={t('common.dismiss')}]').forEach(b => b.click()); 'ok'", 400)
step("js", f"$label({t('history.title')})", 1500)
step(
    "js",
    f"(() => {{ const b = document.querySelector('[aria-label=' + JSON.stringify({t('history.versions')}) + '] button'); b && b.click(); return !!b; }})()",
    2000,
)
step("js", f"document.querySelectorAll('[aria-label^={t('common.dismiss')}]').forEach(b => b.click()); 'ok'", 600)
step("shot", "historial", 300)

# Backup with Git, connected to GitHub (simulated).
step("js", f"$label({t('history.title')})", 600)
step("js", f"$label({t('rail.backup')})", 1200)
step("js", f"$click(b => b.textContent.trim() === {t('backup.enable')})", 800)
step("js", f"$click(b => b.textContent.trim() === {t('backup.useGithub')})", 800)
step("js", f"$click(b => b.textContent.trim() === {t('backup.enableSubmit')})", 1200)
step("js", f"$click(b => b.textContent.trim() === {t('backup.backupNow')})", 1200)
step("js", f"$click(b => b.textContent.trim() === {t('backup.connectButton')})", 800)
step("js", f"$click(b => b.textContent.trim() === {t('backup.createRepo')})", 1200)
step("js", f"$click(b => b.textContent.trim() === {t('backup.push')})", 1500)
step(
    "js",
    r"""(() => { const v = $view(); const d = v.state.doc.toString();
      const at = d.indexOf('Los editores en línea');
      v.dispatch({ changes: { from: at, insert: 'Un cambio sin respaldar. ' } });
      document.querySelectorAll('[aria-label^=' + JSON.stringify(DISMISS) + ']').forEach(b => b.click());
      return at; })()""".replace("DISMISS", t("common.dismiss")),
    2500,
)
step("js", f"document.querySelectorAll('[aria-label^={t('common.dismiss')}]').forEach(b => b.click()); 'ok'", 600)
step("shot", "respaldo", 300)

win.connect("destroy", Gtk.main_quit)
view.connect(
    "load-changed",
    lambda v, e: e == WebKit2.LoadEvent.FINISHED and GLib.timeout_add(2500, lambda: (run(), False)[1]),
)
view.load_uri(URL)
Gtk.main()
print(json.dumps({"done": True}))
