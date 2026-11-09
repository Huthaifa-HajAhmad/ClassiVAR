# ClassiVAR design system

Written 2026-11-08 from the screens as built, not from a wish list. If this file and the code disagree, the code is right: fix this file. The colours and their tests are in `crates/classivar/src/design.rs`; the theme is `crates/classivar/assets/classivar-theme.json`.

Status: partly done. Done: the colour jobs, the theme, the helpers bar, message kinds, a pass at 1240 px and 960 px in both themes. Open: a macOS look-through.

## The one idea

The learner must read the class, and what moves it, at a glance. So the chrome is quiet neutral ink on paper, and **colour is kept for meaning**. A person who sees red, blue or yellow knows it means something.

## Colour: one job each

| Job | Colours | Used for | Never used for |
|---|---|---|---|
| A class | `class_fill` / `class_ink` (five classes, light and dark) | the class badge, the points scale | anything else |
| A direction | `direction_colour` (red for disease, blue against) with `direction_ink` | the dots on the two criteria groups, evidence blocks, chips in the history table | selection or links |
| A blocking state | `warning_fill` / `warning_ink` (pale yellow, near-black text) | a withheld class, a failed lookup, the confirm bar, a missing record class, file messages | good news |
| Something done | `confirm_fill` / `confirm_ink` (pale green, near-black text) | "Saved to …", and "Opened." when nothing needed fixing | a warning |
| Chrome | the theme tokens (`background`, `foreground`, `muted_foreground`, `border`, `accent`, `primary`, `ring`) | everything else, including a selected or pressed button | meaning |

Rules:
- Colour is never the only clue. Every class has its words, every direction a sign (+ or −), every warning its text.
- A pair that carries meaning is checked at 4.5:1 for text in both themes (`design.rs` tests). The theme's own main pairs are checked in `theme.rs` tests (text 4.5:1 or 7:1, control edge and focus ring 3:1).
- Do not write a raw `rgb(...)` in a view file. Add the colour to `design.rs` with its ink and its test. The confirmation window's dim layer is `scrim()` (45 % black); it carries no meaning, so it has no pair and no test.
- The kit's own warning pair is about 1.8:1 in the light theme. Do not use it.
- Chrome stays neutral: no blue, red or yellow tint in `accent`, `secondary` or `muted`, because those hues already mean benign, pathogenic and warning. (A blue-grey accent was tried and replaced on 2026-11-08 for this reason.)
- File and case messages carry a kind (`status.rs`): done (green) or needs attention (yellow). A case opened under other rules, or with a strength lowered, asks for attention; a failed save or open always does.

## Type

One system font (the kit's default). Sizes used today, with how often they appear:

| Size | Count | Job |
|---|---|---|
| `text_2xl` | 3 | the class name, the points, "No class yet" |
| `text_xl` | 3 | the variant name |
| `text_lg` | 2 | a named VUS tier |
| `text_sm` | 42 | body text, row titles, headings |
| `text_xs` | 89 | captions, rule summaries, notes, source lines |

Body text is `text_sm`. Captions are `text_xs` in `muted_foreground`; use `caption(muted)` from `design.rs` (64 uses) instead of repeating the two calls. There is no `text_base`. Bold is for the class and headings; semibold for panel and group titles.

## Space and shape

- Gaps between siblings are almost always `gap_2` (46 uses) or `gap_1`. Panels use `p_3` or `p_4`; the page panes use `px_5`/`px_6`.
- Corners: `rounded_sm` for chips and blocks, `rounded_md` for cards and helper panels, `rounded_lg` for the class badge and the verdict card.
- Borders are one pixel in the theme `border` colour. A control's edge uses `input.border`, which is darker so it reaches 3:1.

## Layout

- Top bar (one row down to the 960 px minimum window): name with "Research and learning only" beneath it, mode switch, status chip ("UNVERIFIED rules vX"), Save, Open, theme, Learning. Save, Open and the theme button have short visible labels; their accessible names stay "Save case…", "Open case…" and "Theme: …".
- Case panel: variant, gene, gene decision, one-line gene source chips; earlier records in Reported mode.
- Left, 480 px: **Criteria**. A header, then the **Evidence helpers** bar (one row of buttons; an open helper's panel shows under it with a Close button), then the criteria in two groups (disease, then against disease), by category, one line each. A criterion is applied by its code button or by clicking its name and note (a pointer shortcut with a pointer cursor); the code button is the keyboard path and the only announced control.
- Gene source cards (under "Details"): all four share one row from 960 to 1240 px; each shows two detail lines and a "Show all N" button. Nothing is cut off silently: a card either shows a line in full or has the button. The strip scrolls (up to 360 px) only when a card is expanded or the window is narrow.
- Right: **Result**. The class badge with its review status and rule version, the points, the scale, one line on the 2015 table, then "What it is made of" (lanes), "What would change it", "Notes", and "About these rules" last. In Reported mode the verdict and history come first. The history table freezes its first column (the criterion code, 48 px) and its last ("Today (you)", 120 px); only the reported records between them scroll sideways, so a long history never hides what today's evidence says. Each header is two one-line texts (record name and reported class, "Today (you)" and "your evidence", "Criterion" and "code"), truncated rather than wrapped, so the columns keep their rows aligned.
- Minimum window 960 × 600. The scale fits the result pane at that width.

## States

- Selected or pressed: a filled button with a visible edge (theme `accent`). It is also announced as pressed to assistive technology (`.toggled`).
- Focus: the theme `ring`, 3:1 against the page.
- Hover: the theme's hover fill. Hover and selected look alike on some buttons; the pressed state is the one that is announced.
- Empty: "No class yet" and "none" in the lanes. A failed lookup is never shown as "nothing found".

## Every new control

A stable id, a visible label, and an `accessibility_label` when the label is a glyph or an abbreviation (see `AGENTS.md`). The smoke script finds controls by accessible name, so renaming one means updating `scripts/gui-smoke.ps1`.

## Not decided yet

- Spacing and type scale: decided on 2026-11-08 to keep GPUI's own scale (it is already named: `text_xs`, `gap_2`, `p_3`) and to name only roles that repeat. Today that is the caption. Other combinations repeat too few times (the semibold panel title 5 times, the section heading once) to deserve a helper yet.
- Frame times: not measured. A timer around `render` would only measure building the element tree, so a real number needs a way to see layout and drawing too.
