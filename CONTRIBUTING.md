# Contributing to glitu

Thank you for your interest! Bug reports, ideas and pull requests are welcome.

## Before you start

- For a bug, open an issue with the smallest presentation that shows it, the command you ran and
  what you expected (`glitu --version`, your system and browser).
- For a new feature or a large change, open an issue first, so that we can agree on the approach
  before you spend time on it.
- Everything is written in English: code, comments, messages and documentation.

## Building

glitu is written in Rust, in the browser too (WebAssembly); there is no Node and no npm.

```sh
rustup target add wasm32-unknown-unknown
cargo build --release
target/release/glitu server tests/features/slides.md
```

The oldest supported compiler is the `rust-version` of `Cargo.toml`.

## Checks

`tools/ci.sh` runs the same checks as the CI: format, lints, tests, documentation, dependencies
and coverage. Run it before you open a pull request:

```sh
tools/ci.sh --quick   # format, lints and tests
tools/ci.sh           # everything (cargo-deny and cargo-llvm-cov are used when installed)
```

In short: `cargo fmt --all`, `cargo clippy --workspace --all-targets --all-features` without
warnings, and `cargo test --workspace --all-features`.

### Visual oracles

A change to the engine (`crates/glitu-web`) or to the themes must keep the two rendering oracles at
100 %. They need a Chromium-based browser, and the development command `oracle`:

```sh
cargo build --release --features oracle
target/release/glitu server tests/features/slides.md -p 8123 &
target/release/glitu oracle check http://localhost:8123/ tests/oracle
target/release/glitu server tests/compat/demo.md -p 8124 &
target/release/glitu oracle check http://localhost:8124/ tests/compat/reference
```

If a change is meant to alter the rendering, look at the differences (`diff_*.png`), then
re-capture the reference and say so in the pull request (see `tests/oracle/README.md`).

No script of the page may call `Math.random` before the engine: the animated backdrop replays it.

## Pull requests

- One topic per pull request, with tests for what changes.
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`,
  `fix:`, `perf:`, `docs:`, `ci:`, `chore:`…): the releases are computed from them
  (`tools/next-version.sh`).
- Add a line to the "Unreleased" section of `CHANGELOG.md` for a change users will notice.

## Licence

By contributing, you agree that your contributions are licensed under the MIT license and the
Apache License 2.0, like the project, without any additional terms or conditions.
