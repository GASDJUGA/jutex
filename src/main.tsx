// Before anything else: what LibreTeX left in localStorage under its name (see the module).
import "./lib/migrateStorageAtStart";
import { isTauri } from "@tauri-apps/api/core";
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { IS_WINDOWS } from "./lib/platform";
import { windowsFileDrops } from "./lib/windowsDrop";
import { useSettings } from "./store/settings";
import "./styles/index.css";

/** How long the first drawing waits for the settings (their language) at most. */
const SETTINGS_WAIT_MS = 1500;

// On Windows the page takes the files dropped from the Explorer from the start, so none is
// ever opened by WebView2 (see lib/windowsDrop.ts).
if (isTauri() && IS_WINDOWS) windowsFileDrops();

// The interface is drawn in its language from the first frame: the settings say which (the
// system's, or the one chosen). If they fail or are slow, it draws anyway and App retries.
void Promise.race([
  useSettings
    .getState()
    .load()
    .catch(() => {}),
  new Promise((resolve) => setTimeout(resolve, SETTINGS_WAIT_MS)),
]).then(() => {
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
});
