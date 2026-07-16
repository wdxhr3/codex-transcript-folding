# Codex CLI Transcript Folding

[中文](README.zh-CN.md) | **English** | [Project entry](README.md)

This repository is a source-complete fork of OpenAI Codex CLI with a small,
UI-only transcript organization feature. It is based on upstream Codex commit
`1bbdb32789e1f79932df44941236ea3658f6e965`.

## Scope

The fork adds the following behavior without changing agent semantics:

- fold or expand any completed user message;
- fold or expand any completed assistant message;
- treat a streamed assistant response as one message and one placeholder;
- fold all user and assistant messages in the current task;
- expand all messages in the current task;
- persist fold state separately for each Codex task/thread;
- restore that state after task switching or `codex resume`;
- rebuild the normal inline transcript when the `Ctrl+T` overlay closes, so both
  views show the same folded messages;
- never auto-fold messages.

Folding changes only terminal rendering. It does **not** alter the rollout,
model context, stored messages, tool calls, file changes, Git state, or external
side effects.

## Interaction

Press `Ctrl+T` in the normal Codex CLI interface to open the managed transcript.

| Key             | Action                                                   |
| --------------- | -------------------------------------------------------- |
| `Tab`           | Select the next completed user or assistant message      |
| `Shift+Tab`     | Select the previous completed user or assistant message  |
| `Space`         | Fold or expand the selected message                      |
| `f`             | Fold all user and assistant messages in the current task |
| `Shift+F`       | Expand all messages in the current task                  |
| `q` or `Ctrl+T` | Close the transcript and rebuild the normal inline view  |

Folded messages are replaced by one of these reversible placeholders:

```text
▶ User message collapsed
▶ Assistant message collapsed
```

The normal interface uses native terminal scrollback, which does not retain an
interactive component for every old message. Selection therefore happens in
the `Ctrl+T` transcript. Closing it immediately synchronizes the result back to
the normal interface.

## Requirements

The repository pins the Rust toolchain in
[`codex-rs/rust-toolchain.toml`](codex-rs/rust-toolchain.toml):

- Rust and Cargo: exactly `1.95.0`;
- rustup: required to install the pinned toolchain and the `rustfmt`, `clippy`,
  and `rust-src` components;
- Git: `2.23+` recommended;
- RAM: 4 GB minimum, 8 GB recommended;
- helper tools for the documented developer checks: `just`, `cargo-nextest`,
  and `dotslash`. The verified versions were `just 1.56.0`,
  `cargo-nextest 0.9.140`, and `dotslash 0.5.9`;
- macOS: Xcode Command Line Tools (Clang and the system SDK);
- Ubuntu/Debian: a C/C++ build toolchain plus `pkg-config` and `libcap-dev`;
- Windows: use Windows 11 with WSL2, matching the upstream support policy.

The supported upstream baseline is macOS 12+, Ubuntu 20.04+/Debian 10+, and
Windows 11 through WSL2. This fork was built and tested on macOS 15.7.7,
Apple Silicon (`arm64`), Apple Clang 17, Rust/Cargo 1.95.0. Use a terminal that
passes `Ctrl+T`, `Tab`, `Shift+Tab`, and `Shift+F` to the TUI.

`codex-rs/Cargo.lock` is committed. Build with `--locked` if you want Cargo to
reject dependency resolution changes. Cargo also fetches locked Git
dependencies and their submodules; networks that block Google Source may need
an administrator-approved mirror for `libyuv`, without changing its locked
commit.

## Clone, build, and run

```shell
git clone https://github.com/wdxhr3/codex-transcript-folding.git
cd codex-transcript-folding/codex-rs

rustup show active-toolchain
cargo build --locked --bin codex
cargo run --locked --bin codex -- --no-alt-screen
```

On first launch, sign in using the normal Codex CLI authentication flow. A
compiled binary can be run directly:

```shell
./target/debug/codex --no-alt-screen
./target/debug/codex resume --last --no-alt-screen
./target/debug/codex resume --no-alt-screen
```

`--no-alt-screen` is recommended while manually checking normal inline
scrollback. The feature also works with the standard alternate-screen mode.

## Automated checks

From the repository root:

```shell
just fmt-check
just clippy -p codex-tui -- -D warnings
just test -p codex-tui
```

Equivalent direct commands from `codex-rs/` are:

```shell
cargo fmt --all -- --check
cargo clippy --tests -p codex-tui -- -D warnings
RUST_MIN_STACK=8388608 NEXTEST_PROFILE=local \
  cargo nextest run --no-fail-fast -p codex-tui
```

The release candidate passed formatting, warning-free Clippy, a debug binary
build, targeted folding tests, and the complete `codex-tui` nextest suite:
3,087 passed and 4 skipped by upstream test configuration.

## Manual verification from zero

1. Clone and build using the commands above.
2. Start `./target/debug/codex --no-alt-screen` and create a new task.
3. Send two distinct prompts and wait until both assistant replies finish.
4. Press `Ctrl+T`, then use `Tab`/`Shift+Tab`. Only completed user and assistant
   messages should be selected; tool and status cells should not be selected.
5. Select a user message and press `Space`. Expect
   `▶ User message collapsed`.
6. Select an assistant response and press `Space`. Expect exactly one
   `▶ Assistant message collapsed`, even for a streamed multi-cell response.
7. Press `q`. The normal inline interface should be rebuilt with the same two
   placeholders while all other content remains visible.
8. Open `Ctrl+T`, press `f`, and expect every user/assistant message to fold.
   Close the overlay and confirm the normal view matches.
9. Reopen it, press `Shift+F`, and expect all original text to return in both
   views.
10. Fold at least one user and one assistant message again, close the overlay,
    and exit Codex normally.
11. Run `./target/debug/codex resume --last --no-alt-screen`. After replay,
    expect the same messages to remain folded in the normal view and in
    `Ctrl+T`.
12. Confirm a JSON file exists at
    `~/.codex/ui-state/transcript-folds/<thread-id>.json`.
13. Ask a follow-up that relies on a folded message and inspect `git diff`.
    Codex should retain the context, and folding itself should not change project
    files.
14. Run `/clear`, send a new message, and confirm that it does not inherit the
    old task ordinal's fold state.

## Typical use and limitations

This is intended for long implementation sessions where old prompts or verbose
assistant explanations make the terminal hard to scan, while the full context
must remain intact.

- Only completed user and assistant history can be selected. Active streaming
  output is not foldable.
- Tool calls, command output, status cards, and reasoning cells are not in the
  fold-all set.
- Rebuilding terminal-managed history can cause a small flicker and may reset a
  terminal text selection or scroll position.
- Fold state is stored separately from rollout JSONL. Copying only a rollout
  file does not copy its UI state.
- `/clear` clears the current task's ordinal map; a fork has a new thread ID and
  starts with no inherited fold state.
- This does not add mouse selection or direct selection inside native terminal
  scrollback. Doing that reliably requires replacing the inline-scrollback
  architecture with a fully managed transcript.
- This fork tracks one upstream snapshot and is not an official OpenAI release.

## Implementation and attribution

The main changes are in `codex-rs/tui/src/transcript_folding.rs`,
`pager_overlay.rs`, `app_backtrack.rs`, and `app/resize_reflow.rs`. Fold files are
written atomically under `~/.codex/ui-state/transcript-folds/` and keyed by
thread ID plus stable-in-rollout user/assistant ordinals.

For the architecture, source map, state format, safety boundary, and a more
detailed verification matrix, read
[`codex-rs/tui/TRANSCRIPT_FOLDING.en.md`](codex-rs/tui/TRANSCRIPT_FOLDING.en.md).

This project is derived from [OpenAI Codex](https://github.com/openai/codex) and
retains the upstream [Apache-2.0 license](LICENSE) and [NOTICE](NOTICE). Codex and
OpenAI are trademarks of their respective owner; this repository is an
independent experimental fork.
