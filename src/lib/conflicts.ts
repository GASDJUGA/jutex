/**
 * Git's conflict marks in a file of a merge in progress (5.9). The rule is the same as
 * `count_blocks` in `src-tauri/src/backup/merge.rs`: a line starting with `<<<<<<< `, then one
 * that is exactly `=======`, then one starting with `>>>>>>> ` (lines may end in `\r`).
 */

export type Range = { from: number; to: number };

export type ConflictBlock = Range & {
  /** What is here (between `<<<<<<<` and `=======`). */
  ours: Range;
  /** What came from GitHub (between `=======` and `>>>>>>>`). */
  theirs: Range;
};

export type Resolution = "ours" | "theirs" | "both";

const bare = (line: string) => (line.endsWith("\r") ? line.slice(0, -1) : line);

export function findConflicts(text: string): ConflictBlock[] {
  const blocks: ConflictBlock[] = [];
  let start = -1;
  let oursFrom = 0;
  let oursTo = 0;
  let theirsFrom = -1;
  let pos = 0;
  while (pos <= text.length) {
    const newline = text.indexOf("\n", pos);
    const end = newline === -1 ? text.length : newline;
    const next = newline === -1 ? text.length : newline + 1;
    const line = text.slice(pos, end);
    if (line.startsWith("<<<<<<< ")) {
      start = pos;
      oursFrom = next;
      theirsFrom = -1;
    } else if (start >= 0 && theirsFrom < 0 && bare(line) === "=======") {
      oursTo = pos;
      theirsFrom = next;
    } else if (start >= 0 && theirsFrom >= 0 && line.startsWith(">>>>>>> ")) {
      blocks.push({
        from: start,
        to: next,
        ours: { from: oursFrom, to: oursTo },
        theirs: { from: theirsFrom, to: pos },
      });
      start = -1;
    }
    if (newline === -1) break;
    pos = next;
  }
  return blocks;
}

/** The change that replaces `block` (marks included) by the part(s) kept. */
export function resolve(
  text: string,
  block: ConflictBlock,
  how: Resolution,
): { from: number; to: number; insert: string } {
  const part = (r: Range) => {
    const body = text.slice(r.from, r.to);
    return body === "" || body.endsWith("\n") ? body : `${body}\n`;
  };
  const insert =
    how === "ours"
      ? part(block.ours)
      : how === "theirs"
        ? part(block.theirs)
        : part(block.ours) + part(block.theirs);
  return { from: block.from, to: block.to, insert };
}
