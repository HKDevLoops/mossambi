# Contributing to Mossambi

Thank you for your interest in contributing to Mossambi! We welcome contributions from developers across Windows, Linux, and macOS.

This guide outlines our development workflow, quality gates, and testing requirements to ensure smooth, pleasant, and high-quality contributions.

---

## Architecture Overview

Mossambi is organized into modular crates and subsystems:

- `src/core`: Platform-agnostic business logic, filesystem abstractions, config management, action journal/undo, and LRU caches.
- `src/gui`: macOS Finder-inspired and Windows Fluent GUI built on `egui` and `eframe`.
- `src/tui`: Terminal UI file explorer built with `ratatui` and `crossterm`.
- `src/codec`: Zero-loss media codecs, converters, and exam tuners.
- `src/platform`: OS-specific integrations (Windows OLE drag & drop, shell context menus, default handler, macOS/Linux desktop integration).
- `tests/`: Integration tests, headless GUI test harnesses, and regression test suites.

---

## Getting Started

### Prerequisites

1. **Rust Toolchain**: Install Rust 1.75+ via [rustup](https://rustup.rs):
   ```bash
   rustup default stable
   rustup component add clippy rustfmt
   ```

2. **Platform Build Dependencies**:
   - **Windows**: Visual Studio C++ Build Tools or MSVC toolchain.
   - **Linux (Ubuntu/Debian)**:
     ```bash
     sudo apt-get update && sudo apt-get install -y \
       libasound2-dev libgl1-mesa-dev libgtk-3-dev \
       libxkbcommon-dev libwayland-dev pkg-config ffmpeg
     ```
   - **macOS**:
     ```bash
     brew install ffmpeg
     ```

### Building & Running

```bash
# Build dev binary
cargo build

# Run the GUI
cargo run -- --app

# Run the TUI
cargo run -- --tui
```

---

## Testing & Quality Gates

Mossambi enforces strict continuous integration gates. Every PR must pass all quality gates locally before submission:

### 1. Code Formatting
```bash
cargo fmt --check
```
Auto-format before committing using:
```bash
cargo fmt
```

### 2. Clippy Linter
All targets must compile with zero warnings:
```bash
cargo clippy --workspace --all-targets -- -D warnings
```

### 3. Test Suites
Run the entire workspace test suite:
```bash
cargo test --workspace
```

Run targeted test suites during development:
```bash
# Core unit tests
cargo test --lib

# Bug regressions suite
cargo test --test test_bug_regressions

# Headless GUI smoke tests
cargo test --test test_ui_smoke

# E2E features suite
cargo test --test e2e_tier1_features
```

### 4. Regression Test Requirement
**Every bug fix MUST be accompanied by a regression test.**
- Add regression tests to `tests/test_bug_regressions.rs` or the relevant feature test suite under `tests/`.
- Ensure tests verify both the fixed state and prevent accidental regressions.

### 5. ASCII Discipline & UI Guidelines
- All new source code, comments, and strings must be **pure ASCII** (range `\x00` - `\x7F`).
- **No emoji in user-facing GUI strings.** All UI text must be clean, professional, and accessible.

---

## Submitting a Pull Request

1. **Create a topic branch**:
   ```bash
   git checkout -b fix/context-menu-viewport
   ```
2. **Follow Conventional Commits**:
   - `feat(gui): ...`
   - `fix(explorer): ...`
   - `perf(core): ...`
   - `test(regressions): ...`
   - `docs: ...`
3. **Verify all gates pass locally**:
   - `cargo fmt --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`
4. **Push and open a Pull Request**:
   - Fill out the PR checklist in `.github/pull_request_template.md`.
   - CI will automatically run format checks, clippy, cross-platform matrix tests (Windows, Linux, macOS), and release compilation.
