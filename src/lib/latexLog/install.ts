import type { LogEntry } from "./parse";

/**
 * Whether a missing file can be installed from here: a package's `.sty` or a babel
 * language's `.ldf` (`ngerman.ldf` → babel-german).
 */
export function offersInstall(entry: LogEntry): boolean {
  return (
    entry.kind === "missingFile" &&
    (entry.missingFile?.endsWith(".sty") === true || entry.missingFile?.endsWith(".ldf") === true)
  );
}
