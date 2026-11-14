/**
 * The languages JuTeX speaks (interface, new documents, spelling, TeX): one table,
 * `languages.json`, also read by Rust (`src-tauri/src/lang.rs`).
 */
import table from "./languages.json";

export type Locale = "es" | "en" | "pt" | "fr" | "de" | "it";

export type Language = {
  code: Locale;
  /** Its own name («Deutsch»). */
  name: string;
  /** Option of babel for new documents. */
  babel: string;
  /** Names of babel and polyglossia that mean this language (lowercase). */
  aliases: string[];
  texlive: string[];
  pacman: string;
  windowsLang: number;
  wikibook: string;
  blankComment: string;
};

export const LANGUAGES = table as readonly Language[];

export const LOCALES: readonly Locale[] = LANGUAGES.map((l) => l.code);

export function isLocale(value: unknown): value is Locale {
  return LOCALES.includes(value as Locale);
}

export function languageName(code: Locale): string {
  return LANGUAGES.find((l) => l.code === code)?.name ?? code;
}

/**
 * The first supported language of `tags` (`pt-BR`, `de_AT.UTF-8`…, most preferred first),
 * by their primary part; English if none (the same rule as Rust's `lang::resolve`).
 */
export function resolveLocale(tags: readonly string[]): Locale {
  for (const tag of tags) {
    const primary = tag.split(/[_\-.@]/)[0].toLowerCase();
    if (isLocale(primary)) return primary;
  }
  return "en";
}
