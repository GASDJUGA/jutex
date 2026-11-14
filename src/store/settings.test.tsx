import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { SettingsTab } from "../components/home/SettingsTab";
import { useI18n } from "../i18n";
import { applyEditorAppearance } from "../lib/appearance";
import { createMockBackend, type MockBackend } from "../lib/mockBackend";
import { setBackend } from "../lib/tauri";
import { resetApp } from "../test/reset";
import { initialSettingsState, LEGACY_STORAGE_KEY, useSettings } from "./settings";
import { useToasts } from "./toasts";

let backend: MockBackend;

beforeEach(() => {
  resetApp();
  backend = createMockBackend();
  setBackend(backend);
});

/** As after restarting the app: fresh store, same settings file. */
async function restart() {
  useSettings.setState(initialSettingsState);
  await act(() => useSettings.getState().load());
}

describe("ajustes", () => {
  it("se guardan y siguen ahí al reiniciar", async () => {
    await useSettings.getState().load();
    await useSettings.getState().update({ keymap: "vim", editorFontSize: 17, liveCheck: false });
    await restart();
    const s = useSettings.getState();
    expect([s.keymap, s.editorFontSize, s.liveCheck, s.loaded]).toEqual(["vim", 17, false, true]);
    expect((await backend.getSettings()).settings.keymap).toBe("vim");
  });

  it("un valor no válido se rechaza, se avisa y se vuelve al guardado", async () => {
    await useSettings.getState().load();
    await useSettings.getState().update({ editorFontSize: 99 });
    expect(useSettings.getState().editorFontSize).toBe(14);
    expect(useToasts.getState().toasts.at(-1)?.kind).toBe("error");
  });

  it("lo que versiones anteriores guardaban en el navegador pasa a settings.json una vez", async () => {
    localStorage.setItem(
      LEGACY_STORAGE_KEY,
      JSON.stringify({ state: { indentMode: "full", pdfDarkMode: true, raro: 1 }, version: 0 }),
    );
    await useSettings.getState().load();
    expect((await backend.getSettings()).settings).toMatchObject({
      indentMode: "full",
      pdfDarkMode: true,
    });
    expect(localStorage.getItem(LEGACY_STORAGE_KEY)).toBeNull();
  });

  it("«igual que el sistema» usa el idioma del sistema", async () => {
    setBackend(createMockBackend(undefined, { systemLocale: "de" }));
    await useSettings.getState().load();
    expect(useSettings.getState().locale).toBe("system");
    expect(useI18n.getState().locale).toBe("de");
    expect(useSettings.getState().effectiveDocumentLanguage).toBe("de");
    await useSettings.getState().update({ locale: "es" });
    expect(useI18n.getState().locale).toBe("es");
    await useSettings.getState().update({ locale: "system" });
    expect(useI18n.getState().locale).toBe("de");
  });

  it("los selectores de idioma muestran los seis con su nombre propio", async () => {
    setBackend(createMockBackend(undefined, { systemLocale: "de" }));
    await act(() => useSettings.getState().update({ locale: "es" }));
    render(<SettingsTab />);
    const language = screen.getByRole("combobox", { name: "Idioma" });
    const options = [...language.querySelectorAll("option")].map((o) => o.textContent);
    expect(options).toEqual([
      "Igual que el sistema (Deutsch)",
      "Español",
      "English",
      "Português (Brasil)",
      "Français",
      "Deutsch",
      "Italiano",
    ]);
    const documents = screen.getByRole("combobox", { name: /^Idioma de los documentos nuevos/ });
    expect([...documents.querySelectorAll("option")].map((o) => o.textContent)).toEqual([
      "Igual que la interfaz",
      "Español",
      "English",
      "Português (Brasil)",
      "Français",
      "Deutsch",
      "Italiano",
    ]);
    fireEvent.change(documents, { target: { value: "it" } });
    await waitFor(() => expect(useSettings.getState().effectiveDocumentLanguage).toBe("it"));
    const spell = screen.getByRole("checkbox", {
      name: /^Revisar la ortografía de los proyectos nuevos/,
    });
    fireEvent.click(spell);
    await waitFor(() => expect(useSettings.getState().newProjectSpellcheck).toBe(false));
  });

  it("el idioma de la interfaz se aplica al cargar y al cambiarlo", async () => {
    await backend.updateSettings({ locale: "en" });
    await useSettings.getState().load();
    expect(useI18n.getState().locale).toBe("en");
  });

  it("tema y fuente del editor como CSS", () => {
    const root = document.createElement("div");
    applyEditorAppearance(
      { editorTheme: "dark", editorFontFamily: "Fira Code", editorFontSize: 16 },
      root,
    );
    expect(root.dataset.editorTheme).toBe("dark");
    expect(root.style.getPropertyValue("--jt-editor-font")).toBe('"Fira Code", var(--font-mono)');
    expect(root.style.getPropertyValue("--jt-editor-font-size")).toBe("16px");
    applyEditorAppearance({ editorTheme: "light", editorFontFamily: "", editorFontSize: 14 }, root);
    expect(root.style.getPropertyValue("--jt-editor-font")).toBe("");
  });
});

describe("pantalla de Ajustes", () => {
  it("cada opción se guarda al cambiarla", async () => {
    render(<SettingsTab />);
    await act(() => useSettings.getState().load());
    fireEvent.change(screen.getByRole("combobox", { name: "Tema del editor" }), {
      target: { value: "dark" },
    });
    fireEvent.change(screen.getByRole("combobox", { name: /Atajos de teclado/ }), {
      target: { value: "emacs" },
    });
    fireEvent.change(await screen.findByRole("combobox", { name: "Fuente" }), {
      target: { value: "Fira Code" },
    });
    fireEvent.change(screen.getByRole("spinbutton", { name: "Tamaño de fuente" }), {
      target: { value: "18" },
    });
    fireEvent.change(screen.getByRole("combobox", { name: /Compilador por defecto/ }), {
      target: { value: "lualatex" },
    });
    fireEvent.change(screen.getByRole("combobox", { name: /Idioma de los documentos nuevos/ }), {
      target: { value: "en" },
    });
    fireEvent.click(screen.getByRole("checkbox", { name: /Páginas en modo oscuro/ }));
    await waitFor(async () =>
      expect((await backend.getSettings()).settings).toMatchObject({
        editorTheme: "dark",
        keymap: "emacs",
        editorFontFamily: "Fira Code",
        editorFontSize: 18,
        defaultCompiler: "lualatex",
        documentLanguage: "en",
        pdfDarkMode: true,
      }),
    );
    // A new blank project gets the defaults.
    const project = await backend.createProject("Nuevo", null);
    expect([project.compiler, project.spellLang]).toEqual(["lualatex", "en"]);
  });

  it("cambiar la carpeta de proyectos usa el diálogo del sistema", async () => {
    render(<SettingsTab />);
    await act(() => useSettings.getState().load());
    expect(screen.getByText("~/JuTeX")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Cambiar…" }));
    expect(await screen.findByText("~/Documentos/Proyectos LaTeX")).toBeInTheDocument();
  });

  it("el idioma de la interfaz cambia en vivo", async () => {
    render(<SettingsTab />);
    await act(() => useSettings.getState().load());
    fireEvent.change(screen.getByRole("combobox", { name: "Idioma" }), {
      target: { value: "en" },
    });
    expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
  });
});
