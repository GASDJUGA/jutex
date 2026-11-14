//! Spell checking (see `crate::spellcheck`) and the personal dictionary (see `crate::spell`).

use crate::error::{AppError, AppResult};
use crate::spell;
use crate::spellcheck;
use crate::state::AppState;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

/// Runs a dictionary operation off the main thread, one at a time.
async fn with_dictionary<F>(state: &State<'_, AppState>, f: F) -> AppResult<Vec<String>>
where
    F: FnOnce(&std::path::Path) -> AppResult<Vec<String>> + Send + 'static,
{
    let path: PathBuf = state.dictionary_file.clone();
    let lock = state.dictionary_lock.clone();
    let words = tauri::async_runtime::spawn_blocking(move || {
        let _guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        f(&path)
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))??;
    // The checks see the new list at once.
    *state
        .personal_words
        .lock()
        .unwrap_or_else(|p| p.into_inner()) = Some(Arc::new(words.iter().cloned().collect()));
    Ok(words)
}

/// Most words checked in one call (the editor sends what is visible, in pieces).
const MAX_WORDS: usize = 20_000;

/// The personal dictionary's words, read from its file the first time.
async fn personal_words(state: &State<'_, AppState>) -> AppResult<Arc<HashSet<String>>> {
    if let Some(words) = state
        .personal_words
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
    {
        return Ok(words);
    }
    with_dictionary(state, spell::read_words).await?;
    Ok(state
        .personal_words
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
        .unwrap_or_default())
}

/// The words of `words` that are not correct in `lang` (one of JuTeX's languages).
#[tauri::command]
pub async fn spell_wrong(
    state: State<'_, AppState>,
    lang: String,
    words: Vec<String>,
) -> AppResult<Vec<String>> {
    if words.len() > MAX_WORDS {
        return Err(AppError::Forbidden("demasiadas palabras".into()));
    }
    let personal = personal_words(&state).await?;
    let checkers = state.spell_checkers.clone();
    tauri::async_runtime::spawn_blocking(move || {
        Ok(spellcheck::wrong(&*checkers.get(&lang)?, &personal, &words))
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))?
}

/// Corrections for `word` in `lang` (the interface orders them).
#[tauri::command]
pub async fn spell_suggest(
    state: State<'_, AppState>,
    lang: String,
    word: String,
) -> AppResult<Vec<String>> {
    let checkers = state.spell_checkers.clone();
    tauri::async_runtime::spawn_blocking(move || {
        Ok(spellcheck::suggest(&*checkers.get(&lang)?, &word))
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))?
}

#[tauri::command]
pub async fn personal_dictionary(state: State<'_, AppState>) -> AppResult<Vec<String>> {
    with_dictionary(&state, spell::read_words).await
}

#[tauri::command]
pub async fn add_to_dictionary(state: State<'_, AppState>, word: String) -> AppResult<Vec<String>> {
    with_dictionary(&state, move |path| spell::add_word(path, &word)).await
}

#[tauri::command]
pub async fn remove_from_dictionary(
    state: State<'_, AppState>,
    word: String,
) -> AppResult<Vec<String>> {
    with_dictionary(&state, move |path| spell::remove_word(path, &word)).await
}
