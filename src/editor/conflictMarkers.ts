/**
 * Merge conflicts in the editor (5.9): while a file is in a merge in progress, each block of
 * Git's marks shows as a box with «what is mine» and «what came from GitHub» and three
 * buttons that keep one, the other or both. The text inside stays editable; removing the
 * marks by hand resolves it too. Only active for the files Git lists in conflict
 * (`setConflictsEnabled`), never for a file that happens to have such lines.
 */
import {
  RangeSetBuilder,
  StateEffect,
  StateField,
  type EditorState,
  type Extension,
} from "@codemirror/state";
import { Decoration, EditorView, WidgetType, type DecorationSet } from "@codemirror/view";
import { findConflicts, resolve, type Resolution } from "../lib/conflicts";

export type ConflictLabels = {
  ours: string;
  theirs: string;
  keepOurs: string;
  keepTheirs: string;
  keepBoth: string;
};

/** Turns the boxes on (the file is in conflict) or off. */
export const setConflictsEnabled = StateEffect.define<boolean>();

const enabledField = StateField.define<boolean>({
  create: () => false,
  update(value, tr) {
    for (const effect of tr.effects) if (effect.is(setConflictsEnabled)) value = effect.value;
    return value;
  },
});

/** Resolves the block that contains `pos` (where the clicked header is). */
function resolveAt(view: EditorView, pos: number, how: Resolution) {
  const text = view.state.doc.toString();
  const block = findConflicts(text).find((b) => b.from <= pos && pos < b.to);
  if (!block) return;
  view.dispatch({ changes: resolve(text, block, how), userEvent: "input.resolve" });
  view.focus();
}

class HeaderWidget extends WidgetType {
  constructor(readonly labels: ConflictLabels) {
    super();
  }

  eq(other: HeaderWidget) {
    return other.labels === this.labels;
  }

  toDOM(view: EditorView) {
    const dom = document.createElement("div");
    dom.className = "jt-conflict jt-conflict-header";
    const label = document.createElement("span");
    label.className = "jt-conflict-label";
    label.textContent = this.labels.ours;
    dom.append(label);
    const actions: [Resolution, string][] = [
      ["ours", this.labels.keepOurs],
      ["theirs", this.labels.keepTheirs],
      ["both", this.labels.keepBoth],
    ];
    for (const [how, text] of actions) {
      const button = document.createElement("button");
      button.type = "button";
      button.textContent = text;
      // Keep the cursor where it is until the click resolves the block.
      button.addEventListener("mousedown", (e) => e.preventDefault());
      button.addEventListener("click", () => resolveAt(view, view.posAtDOM(dom), how));
      dom.append(button);
    }
    return dom;
  }
}

class LabelWidget extends WidgetType {
  constructor(
    readonly text: string,
    readonly kind: "separator" | "footer",
  ) {
    super();
  }

  eq(other: LabelWidget) {
    return other.text === this.text && other.kind === this.kind;
  }

  toDOM() {
    const dom = document.createElement("div");
    dom.className = `jt-conflict jt-conflict-${this.kind}`;
    dom.textContent = this.text;
    return dom;
  }
}

function decorations(state: EditorState, labels: ConflictLabels): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  if (!state.field(enabledField)) return builder.finish();
  const header = Decoration.replace({ widget: new HeaderWidget(labels), block: true });
  const separator = Decoration.replace({
    widget: new LabelWidget(labels.theirs, "separator"),
    block: true,
  });
  const footer = Decoration.replace({ widget: new LabelWidget("", "footer"), block: true });
  const oursLine = Decoration.line({ class: "jt-conflict-ours" });
  const theirsLine = Decoration.line({ class: "jt-conflict-theirs" });
  const { doc } = state;
  const lineRange = (pos: number) => {
    const line = doc.lineAt(pos);
    return [line.from, line.to] as const;
  };
  for (const block of findConflicts(doc.toString())) {
    builder.add(...lineRange(block.from), header);
    for (let pos = block.ours.from; pos < block.ours.to; pos = doc.lineAt(pos).to + 1) {
      builder.add(doc.lineAt(pos).from, doc.lineAt(pos).from, oursLine);
    }
    builder.add(...lineRange(block.ours.to), separator);
    for (let pos = block.theirs.from; pos < block.theirs.to; pos = doc.lineAt(pos).to + 1) {
      builder.add(doc.lineAt(pos).from, doc.lineAt(pos).from, theirsLine);
    }
    builder.add(...lineRange(block.theirs.to), footer);
  }
  return builder.finish();
}

export function conflictMarkers(labels: ConflictLabels): Extension {
  const boxes = StateField.define<DecorationSet>({
    create: (state) => decorations(state, labels),
    update(value, tr) {
      const toggled = tr.effects.some((e) => e.is(setConflictsEnabled));
      if (!tr.docChanged && !toggled) return value;
      return decorations(tr.state, labels);
    },
    provide: (field) => EditorView.decorations.from(field),
  });
  return [enabledField, boxes];
}
