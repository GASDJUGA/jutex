/**
 * `\begin{table}` comes with everything a table needs, like the toolbar's table button: a
 * centered 2×2 `tabular` with rules, caption and label. Its cells, the caption and the label
 * are snippet fields: the cursor starts in the first cell and Tab moves through them
 * (pedido del usuario). Reached by closing `\begin{table` by hand, by picking «table» from
 * the list of environments, or by the language package's `\begin{table}` shortcut.
 */
import { snippet } from "@codemirror/autocomplete";
import type { EditorState } from "@codemirror/state";
import type { EditorView } from "@codemirror/view";

export const TABLE = /^table\*?$/;

/**
 * The snippet: fields `${1}`…`${6}` in order (cells, caption, label), relative indentation
 * (`snippet()` adds the line's own indentation to the following lines).
 */
export function tableTemplate(name: string): string {
  return [
    `\\begin{${name}}[htbp]`,
    "  \\centering",
    "  \\begin{tabular}{|c|c|}",
    "    \\hline",
    "    ${1} & ${2} \\\\ \\hline",
    "    ${3} & ${4} \\\\ \\hline",
    "  \\end{tabular}",
    "  \\caption{${5}}",
    "  \\label{tab:${6}}",
    `\\end{${name}}`,
  ].join("\n");
}

/** Replaces `from`–`to` (from the `\begin` on) with the table, in one undo step. */
export function insertTable(view: EditorView, from: number, to: number, name: string): void {
  snippet(tableTemplate(name))(view, null, from, to);
}

/**
 * Where `\begin{table` (or `table*`) starts, when the line's text before `pos` plus `typed`
 * ends with it; null otherwise.
 */
export function tableRangeAt(
  state: EditorState,
  pos: number,
  typed: string,
): { from: number; name: string } | null {
  const line = state.doc.lineAt(pos);
  const before = line.text.slice(0, pos - line.from) + typed;
  const begin = /\\begin\{(table\*?)$/.exec(before);
  if (!begin) return null;
  return { from: line.from + begin.index, name: begin[1] };
}
