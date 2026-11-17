import { deleteCharBackward, history, undo } from "@codemirror/commands";
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { afterEach, describe, expect, it } from "vitest";
import { conflictMarkers, setConflictsEnabled } from "./conflictMarkers";

const TEXT = "a\n<<<<<<< HEAD\nmío\n=======\nsuyo\n>>>>>>> origin/main\nb\n";
const LABELS = {
  ours: "Lo mío",
  theirs: "Lo de GitHub",
  keepOurs: "Quedarme con lo mío",
  keepTheirs: "Quedarme con lo de GitHub",
  keepBoth: "Ambos",
};

let view: EditorView | null = null;
afterEach(() => {
  view?.destroy();
  view = null;
});

function create(enabled: boolean) {
  view = new EditorView({
    state: EditorState.create({ doc: TEXT, extensions: [history(), conflictMarkers(LABELS)] }),
    parent: document.body,
  });
  view.dispatch({ effects: setConflictsEnabled.of(enabled) });
  return view;
}

const buttons = (v: EditorView) =>
  [...v.dom.querySelectorAll<HTMLButtonElement>(".jt-conflict button")].map((b) => b.textContent);

describe("conflictMarkers", () => {
  it("dibuja cada conflicto con sus tres botones", () => {
    const v = create(true);
    expect(v.dom.querySelectorAll(".jt-conflict")).not.toHaveLength(0);
    expect(buttons(v)).toEqual(["Quedarme con lo mío", "Quedarme con lo de GitHub", "Ambos"]);
    expect(v.dom.textContent).toContain("Lo de GitHub");
  });

  it("un botón resuelve el bloque en un solo paso de deshacer", () => {
    const v = create(true);
    const keepTheirs = [...v.dom.querySelectorAll<HTMLButtonElement>(".jt-conflict button")].find(
      (b) => b.textContent === "Quedarme con lo de GitHub",
    )!;
    keepTheirs.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    keepTheirs.click();
    expect(v.state.doc.toString()).toBe("a\nsuyo\nb\n");
    expect(v.dom.querySelectorAll(".jt-conflict")).toHaveLength(0);
    undo(v);
    expect(v.state.doc.toString()).toBe(TEXT);
  });

  it("sin activar (un archivo que no está en conflicto) no dibuja nada", () => {
    const v = create(false);
    expect(v.dom.querySelectorAll(".jt-conflict")).toHaveLength(0);
  });

  it("Retroceso junto a un recuadro no mete texto en las marcas ocultas", () => {
    const v = create(true);
    // At the start of the line after the box: would join «b» to the hidden `>>>>>>>` line.
    const after = TEXT.indexOf("b\n", TEXT.indexOf(">>>>>>>"));
    v.dispatch({ selection: { anchor: after } });
    deleteCharBackward(v);
    expect(v.state.doc.toString()).toBe(TEXT);
    // At the start of «mío»: would join it to the hidden `<<<<<<<` line.
    v.dispatch({ selection: { anchor: TEXT.indexOf("mío") } });
    deleteCharBackward(v);
    expect(v.state.doc.toString()).toBe(TEXT);
    // Typing inside a part works as always.
    v.dispatch({
      changes: { from: TEXT.indexOf("mío"), insert: "x" },
      userEvent: "input.type",
    });
    expect(v.state.doc.toString()).toContain("xmío");
  });
});
