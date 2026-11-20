# ClassiVAR design system

A shared application title bar surrounds the opening screen and case workspace. The opening screen shows a fictional example scored with the loaded rules.

The current workbench puts the explanation on the left and the evidence editor on the right. Expand explanation gives the chart the full workspace. Compare reports starts with the selected report and the current evidence. See [the evidence workbench design](evidence-workbench.md).

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

Two faces. The system interface font for words; **JetBrains Mono** (built into the program, `assets/fonts/`) for numbers, criterion codes and small labels, so values line up and read as measurements. The roles live in `look.rs`:

| Role | Function | Used for |
|---|---|---|
| Display | `display()`, 34 px semibold | the class name |
| Figure | `figure(size)`, mono semibold | the total (28 px), codes on chips and blocks (11 to 12 px) |
| Eyebrow | `eyebrow(text)`, mono 10.5 px capitals, muted | the small label above a value or section ("TODAY'S ASSESSMENT", "2015 TABLE") |
| Body | `text_sm` | everything else |
| Caption | `caption(muted)` from `design.rs` | notes, sources, hints |

## Space and shape

- A 4 px grid: 4, 8, 12, 16, 24, 32. Page margin 16 px; panels 16 px apart; stage padding 32 px.
- Three surfaces: the **canvas** (theme `background`), **panels** floating on it (`look::panel`: theme `popover`, hairline border, a two-layer soft shadow from `look::elevation`), and **cards** inside a panel (`look::card`: theme `muted`, no shadow).
- Corners: 16 for panels, 10 for cards and controls, 7 for segments and code chips, full round for pills and chips.
- A one-of-several choice is a **segmented control** (`look::segmented` with `look::segment`): the chosen segment is raised (panel colour and a small shadow in light; the lighter `accent` in dark). It is still a `Button`, so it keeps its id, focus ring and pressed state for assistive technology.
- Shadows carry no meaning: plain black at low opacity, a little stronger in dark.

## Layout

- **Frame**: the title bar on the canvas colour with no line under it: the logo mark (a small ink dial), the name, and the case with whose evidence is open. No controls, so it all drags the window.
- **Dropdowns**: a text box with suggestions shows them in a floating panel under it (`suggest.rs`); a ▾ menu beside it lists every choice for the keyboard. A one-of-many value with a long list (strength, year, VUS tier) is a menu button with the current value and ▾; the current choice is ticked. Short lists stay segmented controls.
- **Top bar** (no border, on the canvas): the mode switch (New variant | Reported variant), "Research and learning only", then the status pill ("UNVERIFIED rules vX" with a yellow attention dot while unverified), Open, Save (the one primary button), theme and Learning.
- **Rail** (left, 236 px, only at 1180 px and wider): the case name, then the reports over time as a vertical timeline. Each stop: name and year, the class the lab stated, its total under the loaded rules, and a thin bar of the class colours with a tick at the total. A report whose class the engine withholds gets no tick. Today is the last stop. Clicking a stop opens that evidence.
- **Stage** (middle, a floating panel): Evidence | Compare reports, the picture switch (Dial | Column | Track) and Expand. Under them the picture with its readout, the keyboard chips, the inspector, three insight cards, and the older detail sections.
- **Sheet** (right, 30 % of the window, 340 to 420 px, a floating panel): "Evidence" with what is being edited and Clear; then the case form, the helpers bar and the criteria. Expand hides it.
- **Pictures.** The dial (default), the column and the track draw the same engine numbers. The class shown is in full colour and, on the dial, raised with an ink rim; the other bands are quieter. Every picture selects a criterion on click and previews the class without it on a second click (dashed needle, line or marker). With no evidence the dial is grey with no needle.
- **How the points add up** (under the scale, shown only where the scale is, so it never suggests a class the result withholds): one row per counted criterion, strongest pathogenic first, then benign. Each row is a bar from the running total before the criterion to the total after it, on the scale's own axis, in the direction colours, with the class limits as thin lines and the new total at the right. The axis widens when the evidence goes past the scale (a total of 19 is past 12), and the first and last class zones stretch with it; the point value at each limit is printed under the zones, so a widened axis still reads in points. A line of words under the picture gives the same numbers ("PS1 +4 → 4, PM2 +1 → 5 …"), which is its text alternative. The layout rules are plain functions in `view/panels/evidence_stack.rs` with their own tests; the axis is drawn once in `view/panels/point_axis.rs` and shared with "What would change it".
- **What the 2015 table says** (after the lanes, same condition as the stack): one sentence naming the rows that decide the class, then every row of the table grouped under a class chip (the class colours from `design.rs`). A row the evidence meets shows a "✓ met" mark in the direction colour and bold text; the other rows are muted. The one row that is not from the 2015 paper says "added by ClinGen in 2020". Meaning is in the words and the mark, never in colour alone. With no row met, the sentence says the table gives VUS; while a class is withheld the sentence names no class and says it needs expert judgment. A row the evidence is part of the way to (started, not met, at most two criteria short) adds a muted "needs 1 more Strong" after its text; it names strengths, never criteria, and it never appears on a row whose class the evidence already has, because adding to that row would change nothing.
- **What would change it** (the text lines, then a picture): from the current total, how many points up reach the next class and how many down reach the one below, as outlined bars on the same axis. Outlined means a move that is possible, never evidence that is applied, and each bar says its own numbers at the right. A class with nowhere to go (the top one, or stand-alone benign) draws only the move it does have; a withheld class draws none. The numbers come from the engine, so the picture and the lines above it cannot disagree.
- Minimum window 960 × 600. The scale fits the result pane at that width; the stack scrolls sideways inside its own box when it is wider.

## States

- Selected or pressed: a filled button with a visible edge (theme `accent`). It is also announced as pressed to assistive technology (`.toggled`).
- Focus: the theme `ring`, 3:1 against the page.
- Hover: the theme's hover fill. Hover and selected look alike on some buttons; the pressed state is the one that is announced.
- Empty: "No class yet" and "none" in the lanes. A failed lookup is never shown as "nothing found".

## Every new control

A stable id, a visible label, and an `accessibility_label` when the label is a glyph or an abbreviation (see `AGENTS.md`). The smoke script finds controls by accessible name, so renaming one means updating `scripts/gui-smoke.ps1`.

## Not decided yet

- Spacing and type scale: the 2026-11-08 decision to keep GPUI's own scale was replaced on 2026-11-20 by the roles in `look.rs` (see Type, and Space and shape).
- Frame times: not measured. A timer around `render` would only measure building the element tree, so a real number needs a way to see layout and drawing too.
