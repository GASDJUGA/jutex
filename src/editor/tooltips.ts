import { EditorView, tooltips } from "@codemirror/view";

/**
 * Where the tooltips (the completion list, the math preview) may go: inside the editor. They
 * are positioned in it (see below), and the panels around it cut what sticks out, so CodeMirror
 * flips or shifts them to fit here instead.
 */
export const tooltipSpace = (view: EditorView): DOMRect => view.dom.getBoundingClientRect();

/**
 * Tooltips positioned `absolute` inside the editor instead of CodeMirror's `fixed`. With a
 * fixed tooltip, WebKitGTK (Wayland, NVIDIA) drew the editor's text shifted onto the sidebar
 * for a couple of frames when the completion list appeared (pedido del usuario: parpadeo al
 * empezar un comando). Absolute, tried at length in the user's session, it never did.
 */
export const editorTooltips = tooltips({ position: "absolute", tooltipSpace });
