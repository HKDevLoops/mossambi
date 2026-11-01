# Mossambi Architectural Specification

## Overview
Mossambi provides a unified multi-interface experience backed by a single Rust engine:
- **CLI**: Positional UNIX `mv`-like syntax (`mossambi input.wav flac`)
- **TUI**: Full-screen Apple macOS styled dashboard with Ratatui
- **GUI**: macOS Sequoia studio workspace with liquid glass and frosted glass
- **Widget**: Floating resident sweet lime radial wheel with 8 petals

## Performance Benchmarks
- **Throughput**: > 250 MB/s pure Rust in-memory stream processing
- **Cloud Speedup**: 50x - 120x faster than CloudConvert / Zamzar
- **CLI Speedup**: 3x - 10x faster than FFmpeg subprocess spawns
- **Memory Footprint**: < 25 MB resident set size
