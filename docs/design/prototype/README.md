# ClassiVAR Screen Prototype Kit

Use this folder to prototype new ClassiVAR screens or layout ideas in plain HTML/CSS in seconds—before porting the chosen design into tested Rust/GPUI panels.

## Files

- `tokens.css` — ClassiVAR's exact light/dark theme tokens (`crates/classivar/assets/classivar-theme.json`), semantic WCAG colour pairs (`crates/classivar/src/design.rs`), typography (`JetBrains Mono`), and calm-instrument primitives (`crates/classivar/src/look.rs`).
- `starter.html` — Interactive workbench shell built with real published RYR1 data (`CA024770` / `ClinVar 65979`). Includes toggles for Light/Dark, `1240×900` vs `960×600` (rail folded), and Pathogenic / Withheld / Empty states.

## Workflow

1. Copy the starter to a new file in this folder:

   ```powershell
   Copy-Item docs/design/prototype/starter.html docs/design/prototype/my-idea.html
   ```

2. Start the zero-dependency local live-reload server (auto-reloads on edits to `docs/design/prototype/*` and pulls live colours from `crates/classivar/assets/classivar-theme.json`):

   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts/prototype.ps1 -File my-idea.html
   ```

   Or open `starter.html` directly in any browser (`file://`)—live polling only runs over `http://`.

## Checklist before porting a prototype to Rust/GPUI

- Checked in both **Light** and **Dark** themes.
- Checked at both **1240×900** (rail visible) and **960×600** (rail folded).
- Checked in **Empty** ("No class yet", grey dial) and **Withheld** (no class invented, conflict notice shown) states.
- `UNVERIFIED rules vX` status pill stays visible beside every class.
- New colour pairs added to `design.rs` pass WCAG AA contrast (`4.5:1` text, `3:1` emphasis edges).
- Every button gets a stable id (and `accessibility_label` when abbreviated) and every text box gets an `aria_label`.
