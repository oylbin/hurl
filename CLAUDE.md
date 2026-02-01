# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Hurl is a command-line tool written in Rust that runs HTTP requests defined in a plain text format. It wraps curl/libcurl and supports testing HTTP sessions with assertions, captures, and various query languages (JSONPath, XPath).

## Build Commands

```bash
cargo build                    # Debug build
cargo build --release          # Optimized release build
cargo fmt --all                # Format all Rust code
cargo clippy --all-targets     # Run Clippy linter
```

## Testing

### Unit Tests
```bash
cargo test --lib               # Run all Rust unit tests
cargo test -p hurl_core        # Test specific crate
```

### Integration Tests
Integration tests require Python 3.9+ and running test servers.

```bash
# Setup (one-time)
python3 -m venv .venv
source .venv/bin/activate      # On Windows: .venv\Scripts\activate
pip install --requirement bin/requirements-frozen.txt

# Start test servers (Flask on ports 8000-8003, proxy on 8888)
bin/test/test_prerequisites.sh

# Run all integration tests
cd integration/hurl
python3 integration.py

# Run a single integration test
python3 ../test_script.py tests_ok/hello.sh
```

Test directories:
- `tests_ok/` - Tests that must succeed (exit 0)
- `tests_failed/` - Tests that must fail (runtime errors)
- `tests_error_parser/` - Parser error tests

## Architecture

Three-crate Rust workspace:

### hurl_core (packages/hurl_core)
Core parsing library with no runtime dependencies.
- `ast/` - Abstract syntax tree for Hurl format
- `parser/` - Combinators parsing .hurl files into AST
- `format/` - Output formatting (HTML export)

### hurl (packages/hurl)
Main CLI application and HTTP runner.
- `cli/` - Argument parsing, config file handling
- `http/` - curl-based HTTP client
- `runner/` - Core execution engine (entry, assert, capture, filter, predicate, query, template)
- `jsonpath/` - JSONPath evaluator
- `report/` - Test reports (JSON, JUnit, TAP, HTML)
- `parallel/` - Parallel execution support

### hurlfmt (packages/hurlfmt)
Formatter and linter for .hurl files.
- `command/` - check, export, format commands
- `linter/` - Linting rules
- `curl/` - Convert to/from curl commands

## Code Style

- Clippy strict rules enforced (no wildcard imports, manual string new, etc.)
- All commits must be GPG/SSH signed
- Commit messages: simple phrases, not conventional commits
  - Good: "Fix missing space in variable option HTML export"
  - Bad: "fix: missing space in variable option HTML export"

## Key Dependencies

- **curl/libcurl** - HTTP client
- **clap** - CLI argument parsing
- **serde** - Serialization
- **Flask** - Integration test servers (Python)
