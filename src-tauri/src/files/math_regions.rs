//! Math regions of a LaTeX source, for searching inside or outside formulas. A port of the
//! frontend's text scanner (`src/lib/latexMath/regions.ts`): same rules, and the same test
//! cases (`src/lib/latexMath/cases.json`).

/// A top-level math region. Offsets are bytes of the scanned text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MathRegion {
    /// The whole region, delimiters included.
    pub from: usize,
    pub to: usize,
    /// Between the delimiters.
    pub body_from: usize,
    pub body_to: usize,
    /// Display math (`$$`, `\[`, environments) or inline (`$`, `\(`).
    pub display: bool,
    /// The environment (`align*`…), or None for delimiters.
    pub env: Option<String>,
    /// No closing delimiter before the end of the paragraph (or of the text).
    pub unclosed: bool,
}

/// Math environments (also with `*`).
const MATH_ENVS: [&str; 9] = [
    "equation",
    "align",
    "gather",
    "multline",
    "flalign",
    "alignat",
    "eqnarray",
    "math",
    "displaymath",
];

/// Environments whose content is not LaTeX.
const VERBATIM_ENVS: [&str; 11] = [
    "verbatim",
    "verbatim*",
    "Verbatim",
    "Verbatim*",
    "BVerbatim",
    "LVerbatim",
    "lstlisting",
    "minted",
    "comment",
    "filecontents",
    "filecontents*",
];

fn is_math_env(name: &str) -> bool {
    MATH_ENVS.contains(&name.strip_suffix('*').unwrap_or(name))
}

fn is_letter(c: Option<u8>) -> bool {
    matches!(c, Some(b'a'..=b'z' | b'A'..=b'Z' | b'@'))
}

/// Index of `needle` at or after `from`.
fn find_from(b: &[u8], from: usize, needle: u8) -> Option<usize> {
    b.get(from..)?
        .iter()
        .position(|&c| c == needle)
        .map(|i| from + i)
}

/// `\begin{name}` / `\end{name}` at `at` (the backslash), on one line: the name and the
/// index after the closing brace.
fn env_tag<'a>(text: &'a str, at: usize, kind: &str) -> Option<(&'a str, usize)> {
    let b = text.as_bytes();
    let word_end = at + 1 + kind.len();
    if b.get(at) != Some(&b'\\') || b.get(at + 1..word_end)? != kind.as_bytes() {
        return None;
    }
    if is_letter(b.get(word_end).copied()) {
        return None;
    }
    let mut i = word_end;
    while matches!(b.get(i), Some(b' ' | b'\t')) {
        i += 1;
    }
    if b.get(i) != Some(&b'{') {
        return None;
    }
    let close = find_from(b, i, b'}')?;
    if find_from(b, i, b'\n').is_some_and(|n| n < close) {
        return None;
    }
    Some((text[i + 1..close].trim(), close + 1))
}

/// Index of the next blank line (paragraph end) at or after `from`, or the text length.
fn paragraph_end(b: &[u8], from: usize) -> usize {
    let mut i = from;
    while let Some(nl) = find_from(b, i, b'\n') {
        let mut j = nl + 1;
        while matches!(b.get(j), Some(b' ' | b'\t')) {
            j += 1;
        }
        if b.get(j) == Some(&b'\n') {
            return nl;
        }
        i = nl + 1;
    }
    b.len()
}

/// End of the comment starting at `at` (the `%`): the end of its line.
fn comment_end(b: &[u8], at: usize) -> usize {
    find_from(b, at, b'\n').unwrap_or(b.len())
}

/// Skips `\verb|…|`, `\lstinline` and `\mintinline` arguments; returns the new index.
fn skip_inline_verbatim(b: &[u8], name: &str, mut i: usize) -> usize {
    if name == "mintinline"
        && b.get(i) == Some(&b'{')
        && let Some(brace) = find_from(b, i, b'}')
    {
        i = brace + 1;
    }
    if b.get(i) == Some(&b'*') {
        i += 1;
    }
    if name != "verb"
        && b.get(i) == Some(&b'[')
        && let Some(close) = find_from(b, i, b']')
    {
        i = close + 1;
    }
    let Some(&delimiter) = b.get(i) else {
        return i;
    };
    if delimiter == b'\n' {
        return i;
    }
    let closing = if delimiter == b'{' { b'}' } else { delimiter };
    let end = find_from(b, i + 1, closing);
    let newline = find_from(b, i + 1, b'\n');
    match end {
        Some(end) if newline.is_none_or(|n| n >= end) => end + 1,
        _ => i + 1,
    }
}

enum Closer<'a> {
    Dollar,
    Double,
    Paren,
    Bracket,
    Env(&'a str),
}

/// End of the math region whose body starts at `from`: the closer at brace depth 0 (so
/// `\text{… $x$ …}` inside display math does not end it), skipping comments. Returns the
/// body end and the region end, or None when it is not closed.
fn find_closer(text: &str, from: usize, closer: &Closer) -> Option<(usize, usize)> {
    let b = text.as_bytes();
    let limit = match closer {
        Closer::Env(_) => b.len(),
        _ => paragraph_end(b, from),
    };
    let mut depth = 0usize;
    let mut nested = 0usize;
    let mut i = from;
    while i < limit {
        match b[i] {
            b'%' => {
                i = comment_end(b, i);
                continue;
            }
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            b'$' => {
                if depth == 0 {
                    match closer {
                        Closer::Double if b.get(i + 1) == Some(&b'$') => return Some((i, i + 2)),
                        Closer::Dollar => return Some((i, i + 1)),
                        _ => {}
                    }
                }
            }
            b'\\' => {
                let next = b.get(i + 1);
                match closer {
                    Closer::Paren if next == Some(&b')') && depth == 0 => return Some((i, i + 2)),
                    Closer::Bracket if next == Some(&b']') && depth == 0 => {
                        return Some((i, i + 2));
                    }
                    Closer::Env(name) => {
                        if env_tag(text, i, "begin").is_some_and(|(n, _)| n == *name) {
                            nested += 1;
                        }
                        if let Some((n, end)) = env_tag(text, i, "end")
                            && n == *name
                        {
                            if nested == 0 {
                                return Some((i, end));
                            }
                            nested -= 1;
                        }
                    }
                    _ => {}
                }
                i += 1; // a control symbol (`\$`, `\\`, `\%`…) is never a delimiter
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Top-level math regions of `text`, in order.
pub fn math_regions(text: &str) -> Vec<MathRegion> {
    let b = text.as_bytes();
    let n = b.len();
    let mut regions = Vec::new();
    let mut add = |from: usize, body_from: usize, closer: Closer, display: bool| {
        let end = find_closer(text, body_from, &closer);
        let (env, to) = match &closer {
            Closer::Env(name) => (Some((*name).to_owned()), end.map_or(n, |e| e.1)),
            _ => (
                None,
                end.map_or_else(|| paragraph_end(b, body_from), |e| e.1),
            ),
        };
        regions.push(MathRegion {
            from,
            to,
            body_from,
            body_to: end.map_or(to, |e| e.0),
            display,
            env,
            unclosed: end.is_none(),
        });
        to
    };
    let mut i = 0;
    while i < n {
        let c = b[i];
        if c == b'%' {
            i = comment_end(b, i);
        } else if c == b'$' {
            i = if b.get(i + 1) == Some(&b'$') {
                add(i, i + 2, Closer::Double, true)
            } else {
                add(i, i + 1, Closer::Dollar, false)
            };
        } else if c != b'\\' {
            i += 1;
        } else if b.get(i + 1) == Some(&b'(') {
            i = add(i, i + 2, Closer::Paren, false);
        } else if b.get(i + 1) == Some(&b'[') {
            i = add(i, i + 2, Closer::Bracket, true);
        } else if !is_letter(b.get(i + 1).copied()) {
            i += 2; // `\$`, `\\`, `\%`…
        } else {
            let mut j = i + 1;
            while j < n && is_letter(Some(b[j])) {
                j += 1;
            }
            let name = &text[i + 1..j];
            if matches!(name, "verb" | "lstinline" | "mintinline") {
                i = skip_inline_verbatim(b, name, j);
            } else if name == "begin" {
                match env_tag(text, i, "begin") {
                    Some((env, end)) if is_math_env(env) => {
                        i = add(i, end, Closer::Env(env), true);
                    }
                    Some((env, end)) if VERBATIM_ENVS.contains(&env) => {
                        let end_tag = format!("\\end{{{env}}}");
                        i = text[end..]
                            .find(&end_tag)
                            .map_or(n, |k| end + k + end_tag.len());
                    }
                    Some((_, end)) => i = end,
                    None => i = j,
                }
            } else {
                i = j;
            }
        }
    }
    regions
}

/// Whether `at` (a byte offset) is inside a region, delimiters included. The regions are in
/// order and do not overlap (as `math_regions` returns them): a binary search.
pub fn in_math(regions: &[MathRegion], at: usize) -> bool {
    let k = regions.partition_point(|r| r.to <= at);
    regions.get(k).is_some_and(|r| r.from <= at)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn los_casos_compartidos_con_la_interfaz() {
        let cases: Vec<Value> =
            serde_json::from_str(include_str!("../../../src/lib/latexMath/cases.json")).unwrap();
        assert!(!cases.is_empty());
        for case in cases {
            let name = case["name"].as_str().unwrap();
            let marked = case["text"].as_str().unwrap();
            let pos = marked.find('|').unwrap();
            let text = marked.replacen('|', "", 1);
            let regions = math_regions(&text);
            let region = regions
                .iter()
                .find(|r| r.body_from <= pos && pos <= r.body_to);
            let expected = &case["expected"];
            if expected.is_null() {
                assert!(region.is_none(), "{name}: {region:?}");
                continue;
            }
            let region = region.unwrap_or_else(|| panic!("{name}: sin región"));
            assert_eq!(
                &text[region.body_from..region.body_to],
                expected["body"].as_str().unwrap(),
                "{name}"
            );
            assert_eq!(
                region.display,
                expected["display"].as_bool().unwrap(),
                "{name}"
            );
            assert_eq!(region.env.as_deref(), expected["env"].as_str(), "{name}");
            assert_eq!(
                region.unclosed,
                expected["unclosed"].as_bool().unwrap_or(false),
                "{name}"
            );
        }
    }

    #[test]
    fn todos_los_entornos_matematicos_con_y_sin_asterisco() {
        for env in [
            "equation",
            "align",
            "gather",
            "multline",
            "flalign",
            "alignat",
            "eqnarray",
            "math",
            "displaymath",
        ] {
            for name in [env.to_owned(), format!("{env}*")] {
                let text = format!("\\begin{{{name}}} a \\end{{{name}}}");
                let regions = math_regions(&text);
                assert_eq!(regions.len(), 1, "{name}");
                assert_eq!(regions[0].env.as_deref(), Some(name.as_str()));
                assert_eq!(regions[0].to, text.len());
            }
        }
        assert!(math_regions("\\begin{itemize} a \\end{itemize}").is_empty());
    }

    #[test]
    fn in_math_incluye_los_delimitadores() {
        let text = "x $ab$ y";
        let r = math_regions(text);
        assert!(!in_math(&r, 1));
        assert!(in_math(&r, 2));
        assert!(in_math(&r, 5));
        assert!(!in_math(&r, 6));
    }
}
