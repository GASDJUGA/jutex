import { useCallback } from "react";
import { create } from "zustand";
import { LOCALES, resolveLocale, type Locale } from "../lib/languages";
import de from "./de.json";
import en from "./en.json";
import es from "./es.json";
import fr from "./fr.json";
import it from "./it.json";
import pt from "./pt.json";

export type { Locale };

type PluralForms = { one: string; other: string };
export type Messages = typeof es;

/** Dotted paths to every message (string or plural forms) in the Spanish dictionary. */
type MessageKey<T, Prefix extends string = ""> = {
  [K in keyof T & string]: T[K] extends string | PluralForms
    ? `${Prefix}${K}`
    : MessageKey<T[K], `${Prefix}${K}.`>;
}[keyof T & string];

export type TranslationKey = MessageKey<Messages>;
export type TranslationParams = Record<string, string | number>;

export const dictionaries: Record<Locale, Messages> = { es, en, pt, fr, de, it };
export const locales: readonly Locale[] = LOCALES;
/** What a text missing in a language falls back to. */
export const fallbackLocale: Locale = "en";

/** The browser's language until the settings arrive (the app waits for them to draw). */
function startLocale(): Locale {
  return typeof navigator === "undefined" ? fallbackLocale : resolveLocale(navigator.languages);
}

function lookup(messages: unknown, key: string): unknown {
  return key
    .split(".")
    .reduce<unknown>(
      (node, part) =>
        node && typeof node === "object" ? (node as Record<string, unknown>)[part] : undefined,
      messages,
    );
}

function isPlural(value: unknown): value is PluralForms {
  return typeof value === "object" && value !== null && "one" in value && "other" in value;
}

/**
 * Translates `key` to `locale`, falling back to English and then to the key itself.
 * `{name}` placeholders are replaced from `params`; plural forms are chosen with
 * `Intl.PluralRules` from `params.count`.
 */
export function translate(locale: Locale, key: TranslationKey, params?: TranslationParams): string {
  let value = lookup(dictionaries[locale], key) ?? lookup(dictionaries[fallbackLocale], key);
  if (isPlural(value)) {
    const count = Number(params?.count ?? 0);
    const category = new Intl.PluralRules(locale).select(count);
    value = category === "one" ? value.one : value.other;
  }
  if (typeof value !== "string") return key;
  return params
    ? value.replace(/\{(\w+)\}/g, (match, name: string) =>
        name in params ? String(params[name]) : match,
      )
    : value;
}

type I18nState = {
  locale: Locale;
  setLocale: (locale: Locale) => void;
};

const initialLocale = startLocale();
// The page's language (screen readers, hyphenation) from the start, not only on a change.
if (typeof document !== "undefined") document.documentElement.lang = initialLocale;

export const useI18n = create<I18nState>()((set) => ({
  locale: initialLocale,
  setLocale: (locale) => {
    document.documentElement.lang = locale;
    set({ locale });
  },
}));

/** Returns a `t` function bound to the current locale; components re-render on change. */
export function useT() {
  const locale = useI18n((s) => s.locale);
  return useCallback(
    (key: TranslationKey, params?: TranslationParams) => translate(locale, key, params),
    [locale],
  );
}
