# Taste

## Environment & Tooling

- Develops Rust projects on Windows (paths like `F:\Dev\Rust\patchkit`). When debugging input/event handling, consider Windows-specific behavior — e.g., crossterm on Windows fires both `Press` and `Release` key events, which can silently double-handle keystrokes if the event loop doesn't filter on `key.kind`. Confidence: 0.8
- Works on terminal-UI applications using crossterm (event-loop-driven, tab-based TUI in `patchkit`). Confidence: 0.6

## Workflow & Communication

- Reports issues tersely without pasting error output; expects the agent to reproduce and diagnose by running the tooling itself (e.g., `cargo check`, escalating to `cargo build 2>&1`, and explicitly requested `cargo run`) instead of asking for details. Wants fixes verified at runtime — actually execute the app and inspect its output/panics, not just confirm it compiles. Confidence: 0.7
- Makes their own edits in the IDE between agent turns (e.g., added a `pages` module mid-session, causing file drift from earlier reads); re-read files before editing rather than trusting previously-read state. Confidence: 0.6
