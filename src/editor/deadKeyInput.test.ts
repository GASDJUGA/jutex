import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { latex } from "codemirror-lang-latex";
import { afterEach, describe, expect, it, vi } from "vitest";

// Whether the cursor is in math, spied on: it is the expensive question.
const asked = vi.hoisted(() => ({ count: 0 }));
vi.mock("./mathRegion", async (importOriginal) => {
  const original = await importOriginal<typeof import("./mathRegion")>();
  return {
    ...original,
    mathRegionInState: (...args: Parameters<typeof original.mathRegionInState>) => {
      asked.count++;
      return original.mathRegionInState(...args);
    },
  };
});

const { deadKeyInput } = await import("./brackets");

let view: EditorView | null = null;
afterEach(() => {
  view?.destroy();
  view = null;
});

/** Types `text` at the `|` of `doc` through `deadKeyInput` only. */
function type(doc: string, text: string) {
  const at = doc.indexOf("|");
  view = new EditorView({
    state: EditorState.create({
      doc: doc.replace("|", ""),
      selection: { anchor: at },
      extensions: [latex(), deadKeyInput],
    }),
    parent: document.body,
  });
  const v = view;
  const handled = v.state
    .facet(EditorView.inputHandler)
    .some((handler) =>
      handler(v, at, at, text, () => v.state.update({ changes: { from: at, insert: text } })),
    );
  if (!handled) v.dispatch({ changes: { from: at, insert: text }, selection: { anchor: at + 1 } });
  return v.state.doc.toString();
}

describe("deadKeyInput", () => {
  it("una letra común no pregunta si el cursor está en una fórmula", () => {
    asked.count = 0;
    expect(type("$x|$", "a")).toBe("$xa$");
    expect(asked.count).toBe(0);
  });

  it("lo compuesto con la tecla muerta sigue siendo ^x en matemáticas y no fuera", () => {
    expect(type("$x|$", "²")).toBe("$x^2$");
    expect(type("texto |", "²")).toBe("texto ²");
    expect(type("$x|$", "â")).toBe("$x^a$");
  });
});
