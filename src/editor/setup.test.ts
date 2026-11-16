import { insertBracket } from "@codemirror/autocomplete";
import { EditorState } from "@codemirror/state";
import { describe, expect, it } from "vitest";
import { editorExtensions } from "./setup";

const noop = { onChange: () => {}, onCursorLine: () => {}, onSave: () => {} };

/** Text after typing `char` at `cursor` (null = not auto-closed). */
function typed(char: string, doc = "", cursor = 0) {
  const state = EditorState.create({
    doc,
    selection: { anchor: cursor },
    extensions: editorExtensions(noop),
  });
  const tr = insertBracket(state, char);
  return tr ? tr.state.doc.toString() : null;
}

describe("cierre automático", () => {
  it("cierra llaves, corchetes, paréntesis y $", () => {
    expect(typed("{")).toBe("{}");
    expect(typed("[")).toBe("[]");
    expect(typed("(")).toBe("()");
    expect(typed("$")).toBe("$$");
  });

  it("cierra llaves dentro de $…$", () => {
    expect(typed("{", "$$", 1)).toBe("${}$");
  });

  it("no cierra comillas ni apóstrofos", () => {
    expect(typed("'")).toBeNull();
    expect(typed('"')).toBeNull();
  });
});

describe("rendimiento al escribir", () => {
  it("el área de desplazamiento del editor contiene su maquetación", async () => {
    const { EditorView } = await import("@codemirror/view");
    const view = new EditorView({
      state: EditorState.create({ doc: "x", extensions: editorExtensions(noop) }),
      parent: document.body,
    });
    // Without it, each key made WebKit lay out the whole window again (WebKitGTK: the delay
    // when several keys were pressed at once, in a long document).
    expect(getComputedStyle(view.scrollDOM).contain).toBe("strict");
    view.destroy();
  });
});
