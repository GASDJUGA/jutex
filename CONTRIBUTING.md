# Contributing to JuTeX

Thanks for your interest! JuTeX is a 100 % local desktop LaTeX editor, built with Tauri 2
(Rust) and React + TypeScript.

## Before you start

- For bugs and ideas, open an [issue](https://github.com/GASDJUGA/jutex/issues) before a
  big change, so we can talk it over.
- Vulnerabilities don't go in issues: see [SECURITY.md](SECURITY.md).

## Setup

Requirements for each system are in the [README](README.md#development): Rust (with
`rustup`), Node 26 and pnpm (the version pinned in `packageManager`), and the Tauri 2
dependencies for your system. A TeX distribution is optional for development, but the
real-compilation tests need one (they are skipped without TeX).

```sh
pnpm install
pnpm tauri dev
```

## Before sending a PR

All of this must pass (CI runs it on Linux, macOS and Windows):

```sh
pnpm lint
pnpm typecheck
pnpm test
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

- One branch per change and a PR to `main` (which is protected: changes only go in through a
  PR with green CI).
- Commit messages and PRs in English, with
  [Conventional Commits](https://www.conventionalcommits.org/): `feat(editor): …`,
  `fix(compile): …`, `docs: …`.
- Non-trivial logic comes with tests.
- Anything specific to one system goes behind `#[cfg(...)]` or in `src-tauri/src/platform/`
  (Rust) and in `src/lib/platform.ts` (interface), without breaking the others.
- The interface comes in six languages (`src/i18n/<language>.json`, all with the same keys;
  the list is in `src/lib/languages.json`). Corrections to the translations are welcome.
- No personal data in commits, test logs or screenshots.

## License

By contributing, you agree that your contribution is published under the project's license,
[GPL-3.0-or-later](LICENSE).
