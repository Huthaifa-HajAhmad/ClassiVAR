# ClassiVAR

A desktop app for variant curation that shows how a classification is built: evidence, points, class, and what would change it. Built with Rust and GPUI Kit.

**Status:** early. The rules are a draft and marked UNVERIFIED. For research and learning only. Not a validated clinical reporting system.

Prerequisites, first run, the merge gate, recipes and debugging: **`docs/DEVELOPING.md`**.

```
cargo run -p classivar          # run the app
scripts/check.ps1               # merge gate (scripts/check.sh on POSIX)
scripts/gui-smoke.ps1           # Windows: drive the real window, save screenshots
```

Read next: `AGENTS.md` (how to work here, especially for AI agents), `docs/DEVELOPING.md` (contributor guide), `docs/PRD.md` (what and why), `docs/MVP.md` (scope and milestones), `docs/ARCHITECTURE.md` (framework, folders, data flow), `docs/design/design-brief.md` (the design; still a draft), `docs/ROADMAP.md` (what is built, what is next), `docs/AUDIT-2026-11-04.md` (the audit as a snapshot of the baseline commit; `ROADMAP.md` tracks how each finding was handled).
Rules live in `rules/acmg-amp-points.toml`. They are built into the program; to try a corrected rule file without rebuilding, run `cargo run -p classivar -- --rules path/to/file.toml`.

CI (`.github/workflows/`) runs the same checks plus `cargo audit` on Windows and macOS (both blocking); the first fully green run was [42086149175](https://github.com/Huthaifa-HajAhmad/ClassiVAR/actions/runs/42086149175).
