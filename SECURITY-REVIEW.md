# Security Review Findings

Date: 2026-06-25

Scope: Staged and unstaged changes across the whole app.

## Findings

| # | Severity | File | Lines | Vulnerability | Confidence |
|---|----------|------|-------|---------------|------------|
| 1 | HIGH | .github/workflows/release.yml | 71-72 | GitHub Action pinned to mutable tag (`ilammy/setup-nasm@v1`), creating a supply-chain risk if tag is retargeted | 9/10 |

## Recommended Remediation

Pin third-party GitHub Actions to full commit SHAs instead of mutable tags, especially in release workflows that publish artifacts or use write-scoped tokens.
