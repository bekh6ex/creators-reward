# Repo-wide recipes. Each project also has its own justfile; run `just --list` inside
# a project directory, or `just <project>::<recipe>` from the root.

set shell := ["bash", "-cu"]

mod backend
mod frontend
mod e2e-tests
mod dev

[private]
default:
    @just --list --list-submodules

# Run ast-grep convention rule tests.
[group('root')]
ast-grep-test:
    ast-grep test --skip-snapshot-tests

# Run ast-grep convention scan across the repo.
[group('root')]
ast-grep-scan:
    ast-grep scan --report-style short

# Scan for secrets in the full git history.
[group('root')]
gitleaks:
    gitleaks git --no-banner --redact .

# Verify every file has a known licence and copyright.
[group('root')]
reuse:
    reuse lint

# Run the pre-commit checks on all files.
[group('root')]
prek:
    prek run --all-files

# Run the repo-wide checks plus every project gate that needs no running stack
# (e2e `qa` loads the Playwright config, which resolves the dev stack's URL).
# Lists all failures at the end.
check:
    #!/usr/bin/env bash
    set -uo pipefail
    failed_steps=()
    run_step() {
        local name="$1"
        shift
        echo "==> $name"
        if "$@"; then
            echo "    OK: $name"
        else
            echo "    FAIL: $name"
            failed_steps+=("$name")
        fi
        echo
    }
    run_step "ast-grep test" just ast-grep-test
    run_step "ast-grep scan" just ast-grep-scan
    run_step "gitleaks" just gitleaks
    run_step "reuse lint" just reuse
    run_step "backend check" just backend::check
    run_step "viewer check" just frontend::viewer::check
    run_step "creator check" just frontend::creator::check
    echo "========================================"
    if [ ${#failed_steps[@]} -eq 0 ]; then
        echo "All checks passed."
        exit 0
    else
        echo "Failed steps:"
        printf '  - %s\n' "${failed_steps[@]}"
        exit 1
    fi
