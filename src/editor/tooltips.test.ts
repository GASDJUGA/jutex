import { startCompletion } from "@codemirror/autocomplete";
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { editorExtensions } from "./setup";
import { tooltipSpace } from "./tooltips";

let view: EditorView | null = null;
afterEach(() => {
  view?.destroy();
  view = null;
});

function editor(doc: string) {
  view = new EditorView({
    state: EditorState.create({
      doc,
      selection: { anchor: doc.length },
      extensions: editorExtensions({ onChange() {}, onCursorLine() {}, onSave() {} }),
    }),
    parent: document.body,
  });
  return view;
}

describe("la lista de sugerencias y la vista previa", () => {
  it("se colocan con posición absoluta dentro del editor, no fija", async () => {
    const v = editor("$\\al");
    startCompletion(v);
    await waitFor(() => expect(v.dom.querySelector(".cm-tooltip-autocomplete")).not.toBeNull());
    const list = v.dom.querySelector<HTMLElement>(".cm-tooltip-autocomplete")!;
    expect(list.style.position).toBe("absolute");
  });

  it("tienen como espacio el del editor (no se salen al panel de al lado)", () => {
    const v = editor("");
    const rect = { left: 300, top: 80, right: 900, bottom: 700 } as DOMRect;
    v.dom.getBoundingClientRect = () => rect;
    expect(tooltipSpace(v)).toBe(rect);
  });
});
