# Project: Mossambi Refinement
# Scope: All Milestones

## Architecture
- Module/package boundaries: `src/gui`, `src/tui`, `src/core`, `src/codec`, `src/platform`.
- Data flow: Core state management via `src/core/state_db.rs`, GUI/TUI interface with core logic.

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 1 | UI/UX Polish | Frameless window, logo, icons, themes, zoom | M1 | ORIGINAL_REQUEST |
| 2 | Algorithmic Perf | Algorithmic optimization | M2 | ORIGINAL_REQUEST |
| 3 | Memory Leak Fixes | Detect and fix memory leaks | M3 | ORIGINAL_REQUEST |
| 4 | Security Audit | Kali Linux-based security audit & hardening | M4 | ORIGINAL_REQUEST |
| 5 | Persistence | Settings persistence | M5 | ORIGINAL_REQUEST |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | UI/UX Overhaul | Frameless, Icons, Themes, Zoom | none | PLANNED |
| 2 | Performance Optimization | Algorithmic perf | none | PLANNED |
| 3 | Memory Management | Memory leak fixes | M2 | PLANNED |
| 4 | Security Hardening | Vulnerability audit & Rust hardening | none | PLANNED |
| 5 | Persistence | Settings persistence | none | PLANNED |

## Interface Contracts
### `gui` ↔ `core`
- Interaction between UI components and core state management.
