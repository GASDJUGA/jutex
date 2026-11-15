/**
 * Spell checkers by language. The Hunspell dictionaries live in Rust (crate `spellbook`,
 * bundled with the app: «local primero»); here, what was already asked is cached per word
 * and the suggestions are ordered by typing distance.
 */
import type { Locale } from "../languages";
import { getBackend } from "../tauri";
import { rankSuggestions } from "./speller";

/** What the editor needs: asynchronous, the dictionary is in the backend. */
export type SpellChecker = {
  /** The words of `words` that are not correct. */
  wrong(words: string[]): Promise<string[]>;
  suggest(word: string): Promise<string[]>;
  /** Words of the personal dictionary (also when it changes). */
  setPersonal(words: string[]): void;
  /** Known answer for `word`, if it was already checked (`true` = correct). */
  known(word: string): boolean | undefined;
  dispose(): void;
};

/** Most words asked in one request (the backend refuses more than 20 000). */
const WORDS_PER_REQUEST = 5000;

type Backend = {
  request(
    type: "wrong" | "suggest" | "add" | "remove",
    payload: string[] | string,
  ): Promise<string[]>;
  dispose(): void;
};

/** Checker on top of a backend (a worker, or nspell directly in the tests). */
export function makeChecker(backend: Backend): SpellChecker {
  const answers = new Map<string, boolean>();
  let personal = new Set<string>();
  return {
    async wrong(words) {
      const unknown = [...new Set(words.filter((w) => !answers.has(w)))];
      // In pieces: a long file has tens of thousands of distinct words.
      for (let i = 0; i < unknown.length; i += WORDS_PER_REQUEST) {
        const piece = unknown.slice(i, i + WORDS_PER_REQUEST);
        const wrong = new Set(await backend.request("wrong", piece));
        for (const w of piece) answers.set(w, !wrong.has(w));
      }
      return words.filter((w) => answers.get(w) === false);
    },
    suggest: (word) => backend.request("suggest", word),
    setPersonal(words) {
      const next = new Set(words);
      const added = [...next].filter((w) => !personal.has(w));
      const removed = [...personal].filter((w) => !next.has(w));
      personal = next;
      if (added.length > 0) void backend.request("add", added);
      if (removed.length > 0) void backend.request("remove", removed);
      // Answers may change for these words and their forms (capitalized…). Also when the
      // list is the same: the confirmed list comes after the optimistic one, and a check in
      // between may have been answered before the backend had the new word.
      answers.clear();
    },
    known: (word) => answers.get(word),
    dispose: () => backend.dispose(),
  };
}

/**
 * A checker for `lang` asking the backend (Rust keeps the personal dictionary itself, so
 * changes to it only clear the answers kept here).
 */
export function backendChecker(lang: Locale): SpellChecker {
  return makeChecker({
    async request(type, payload) {
      switch (type) {
        case "wrong":
          return getBackend().spellWrong(lang, payload as string[]);
        case "suggest":
          return rankSuggestions(
            payload as string,
            await getBackend().spellSuggest(lang, payload as string),
          );
        default:
          return [];
      }
    },
    dispose: () => {},
  });
}
