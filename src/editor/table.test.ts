import { undo } from "@codemirror/commands";
import { EditorState } from "@codemirror/state";
import { EditorView, runScopeHandlers } from "@codemirror/view";
import { afterEach, describe, expect, it } from "vitest";
import { editorExtensions } from "./setup";

let view: EditorView | null = null;
afterEach(() => {
  view?.destroy();
  view = null;
});

/** An editor like the app's, with `doc` and the cursor at `|`. */
function editor(doc: string) {
  const cursor = doc.indexOf("|");
  view = new EditorView({
    state: EditorState.create({
      doc: doc.replace("|", ""),
      selection: { anchor: cursor },
      extensions: editorExtensions({ onChange() {}, onCursorLine() {}, onSave() {} }, [], []),
    }),
    parent: document.body,
  });
  return view;
}

const key = (v: EditorView, k: string) =>
  runScopeHandlers(v, new KeyboardEvent("keydown", { key: k }), "editor");
const type = (v: EditorView, text: string) =>
  v.dispatch({ ...v.state.replaceSelection(text), userEvent: "input.type" });
const text = (v: EditorView) => v.state.doc.toString();

const DOC = "\\begin{document}\n  \\begin{table|}\n\\end{document}";

describe("tabla completa al autocompletar \\begin{table}", () => {
  it("cerrar \\begin{table con } trae la tabla completa", () => {
    const v = editor(DOC);
    expect(key(v, "}")).toBe(true);
    expect(text(v)).toBe(
      [
        "\\begin{document}",
        "  \\begin{table}[htbp]",
        "    \\centering",
        "    \\begin{tabular}{|c|c|}",
        "      \\hline",
        "       &  \\\\ \\hline",
        "       &  \\\\ \\hline",
        "    \\end{tabular}",
        "    \\caption{}",
        "    \\label{tab:}",
        "  \\end{table}",
        "\\end{document}",
      ].join("\n"),
    );
    // The cursor in the first cell.
    const head = v.state.selection.main.head;
    expect(v.state.doc.lineAt(head).number).toBe(6);
    expect(head - v.state.doc.line(6).from).toBe(6);
  });

  it("Tab recorre las celdas, el caption y el label", () => {
    const v = editor(DOC);
    key(v, "}");
    for (const [i, part] of ["a", "b", "c", "d", "Cap", "t1"].entries()) {
      if (i > 0) expect(key(v, "Tab")).toBe(true);
      type(v, part);
    }
    const out = text(v);
    expect(out).toContain("      a & b \\\\ \\hline\n      c & d \\\\ \\hline");
    expect(out).toContain("\\caption{Cap}\n    \\label{tab:t1}");
  });

  it("table* cierra con table*", () => {
    const v = editor(DOC.replace("table|", "table*|"));
    key(v, "}");
    expect(text(v)).toContain("\\begin{table*}[htbp]");
    expect(text(v)).toContain("  \\end{table*}\n\\end{document}");
    expect(text(v).match(/\\end\{table\*\}/g)).toHaveLength(1);
  });

  it("un solo deshacer quita la tabla", () => {
    const v = editor(DOC);
    const before = text(v);
    key(v, "}");
    undo(v);
    expect(text(v)).toBe(before);
  });
});
