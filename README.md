# ClassiVAR

A desktop app for variant curation that shows how a classification is built: evidence, points, class, and what would change it. Built with Rust and GPUI Kit.

**Status:** early. The rules are a draft and marked UNVERIFIED. For research and learning only. Not a validated clinical reporting system.

```
cargo run -p classivar
cargo test --workspace
```

Read next: `docs/PRD.md` (what and why), `docs/MVP.md` (scope and milestones), `docs/ARCHITECTURE.md` (framework, folders, plan), `docs/design/design-brief.md` (the agreed design).
Rules live in `rules/acmg-amp-points.toml`.
