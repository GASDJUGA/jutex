import { CheckCircle2, GitMerge, Loader2 } from "lucide-react";
import { useState } from "react";
import { useT } from "../../i18n";
import { findConflicts } from "../../lib/conflicts";
import { notifyError, notifyInfo } from "../../lib/errors";
import { useBackup } from "../../store/backup";
import { useBuffers } from "../../store/buffers";
import { useProjectView } from "../../store/project";
import type { Conflict } from "../../store/types";
import { Dialog } from "../common/Dialog";

const smallButton =
  "rounded border border-line px-2 py-0.5 text-xs hover:bg-hover disabled:opacity-50";

/** Opens a text file at its first conflict (or at its start). */
function openConflict(path: string) {
  const text = useBuffers.getState().buffers[path]?.content;
  const first = text ? findConflicts(text)[0] : undefined;
  const line = first && text ? text.slice(0, first.from).split("\n").length : 1;
  useProjectView.getState().goToLine(path, line);
}

function ConflictRow({ conflict, busy }: { conflict: Conflict; busy: boolean }) {
  const t = useT();
  const done = conflict.remaining === 0;
  const choose = (side: "ours" | "theirs") =>
    void useBackup.getState().choose(conflict.path, side).catch(notifyError);
  return (
    <li className="flex flex-col gap-1 py-1">
      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={() => openConflict(conflict.path)}
          disabled={conflict.kind !== "text"}
          className="min-w-0 flex-1 truncate text-left font-mono text-xs hover:underline disabled:no-underline"
        >
          {conflict.path}
        </button>
        <span className={`shrink-0 text-xs ${done ? "text-success" : "text-fg-muted"}`}>
          {done ? (
            <span className="flex items-center gap-1">
              <CheckCircle2 size={12} aria-hidden /> {t("backup.resolved")}
            </span>
          ) : (
            t("backup.remainingCount", { count: conflict.remaining })
          )}
        </span>
      </div>
      {conflict.kind !== "text" && !done && (
        <>
          <p className="text-xs text-fg-muted">
            {conflict.kind === "deleted" ? t("backup.kindDeleted") : t("backup.kindBinary")}
          </p>
          <div className="flex gap-1">
            <button
              type="button"
              disabled={busy}
              onClick={() => choose("ours")}
              className={smallButton}
            >
              {t("backup.keepOurs")}
            </button>
            <button
              type="button"
              disabled={busy}
              onClick={() => choose("theirs")}
              className={smallButton}
            >
              {t("backup.keepTheirs")}
            </button>
          </div>
        </>
      )}
    </li>
  );
}

/**
 * The merge with GitHub in progress (after «Traer cambios» found conflicts): the files with
 * conflicts, «Terminar fusión» once none is left and «Cancelar fusión».
 */
export function MergeSection() {
  const t = useT();
  const merge = useBackup((s) => s.status?.merge ?? null);
  const working = useBackup((s) => s.working);
  const [confirming, setConfirming] = useState(false);
  if (!merge) return null;
  const left = merge.conflicts.some((c) => c.remaining > 0);
  const busy = working !== null;

  const finish = () =>
    void useBackup
      .getState()
      .finishMerge()
      .then(() => notifyInfo(t("backup.mergeFinished")))
      .catch(notifyError);
  const abort = () => {
    setConfirming(false);
    void useBackup.getState().abortMerge().catch(notifyError);
  };

  return (
    <div className="flex flex-col gap-2 border-b border-line bg-accent/10 px-3 py-3 text-sm">
      <h3 className="flex items-center gap-1.5 font-semibold">
        <GitMerge size={14} aria-hidden /> {t("backup.mergeTitle")}
      </h3>
      <p className="text-xs text-fg-muted">{t("backup.mergeIntro")}</p>
      <ul aria-label={t("backup.mergeTitle")}>
        {merge.conflicts.map((c) => (
          <ConflictRow key={c.path} conflict={c} busy={busy} />
        ))}
      </ul>
      <div className="flex gap-2">
        <button
          type="button"
          onClick={finish}
          disabled={busy || left}
          className="flex flex-1 items-center justify-center gap-1.5 rounded-md bg-accent px-2 py-1.5 text-sm font-medium text-accent-fg hover:bg-accent-hover disabled:opacity-50"
        >
          {working === "merge" && <Loader2 size={14} className="animate-spin" aria-hidden />}
          {t("backup.finishMerge")}
        </button>
        <button
          type="button"
          onClick={() => setConfirming(true)}
          disabled={busy}
          className="flex flex-1 items-center justify-center rounded-md border border-line px-2 py-1.5 text-sm hover:bg-hover disabled:opacity-50"
        >
          {t("backup.abortMerge")}
        </button>
      </div>
      {confirming && (
        <Dialog
          title={t("backup.abortTitle")}
          submitLabel={t("backup.abortMerge")}
          danger
          onSubmit={abort}
          onClose={() => setConfirming(false)}
        >
          <p className="text-sm">{t("backup.abortConfirm")}</p>
        </Dialog>
      )}
    </div>
  );
}
