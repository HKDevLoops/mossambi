## Description

Please provide a clear and concise summary of the changes made in this pull request and the problem they solve.

Fixes / Closes #(issue_number)

---

## Type of Change

- [ ] Bug fix (non-breaking change fixing an issue or regression)
- [ ] New feature (non-breaking addition of functionality)
- [ ] Performance optimization (algorithmic or memory improvement)
- [ ] Documentation update
- [ ] CI / CD or contributor tooling improvement

---

## Contributor Quality Checklist

Please confirm all requirements are satisfied before submitting:

- [ ] **Tests added**: A regression test is included in `tests/test_bug_regressions.rs` or relevant test suite for any bugfix.
- [ ] **All tests pass**: `cargo test --workspace` passes with zero failures.
- [ ] **Formatting**: `cargo fmt --check` succeeds.
- [ ] **Clippy**: `cargo clippy --workspace --all-targets -- -D warnings` compiles with zero warnings.
- [ ] **ASCII Discipline**: All new code and comments are pure ASCII (range `\x00` - `\x7F`).
- [ ] **No UI Emoji**: No emoji introduced into user-facing GUI strings.
