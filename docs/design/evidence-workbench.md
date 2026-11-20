# The evidence workbench

Design for the native GPUI app, November 2026.

## Purpose and approaches

Explain how a curator's chosen evidence produces an assessment, and how that assessment relates to an earlier report. A comparison-first window gives history the most space but assumes there is a previous report. The chosen layout opens on interactive evidence for every case and puts report comparison one action away for reported variants.

## Evidence

Keep the variant name and the assessment being viewed at the top. Give the class, rule version and review status the strongest visual emphasis. For empty evidence show the action to choose the first criterion. When the engine withholds a class, give its reasons that same space.

Use the existing running-total chart as the main figure. Each selectable row shows the criterion, strength and signed contribution on a shared points axis. Keep text values visible in both themes. Use the existing direction colours and a separate focus outline. Selecting a row by mouse or keyboard opens its explanation immediately below the chart, with the rule source and any recorded reason. Inspection must never apply evidence. Keep the evidence editor available alongside the explanation.

For today's assessment, offer **Preview without this criterion** from the selected row. Reassess a temporary copy using the engine. Label the preview and show both outcomes, including conflicts and empty evidence. Dismiss the preview when the selection, evidence, case or view changes. Saving always uses the actual evidence.

## Compare reports

In reported mode, place **Evidence** and **Compare reports** beside the assessment heading. Start comparison with the latest complete record using the existing record ordering. Allow choosing another by lab and year. If no record is complete, identify the missing details and link to the record editor. Viewing a record for comparison never switches the evidence editor to that record.

Keep the lab's stated class, the score reconstructed from its recorded evidence under the loaded rules, and today's assessment explicitly labelled. Show changes first in aligned columns, with criterion codes and written descriptions such as “Strong to Supporting”. Preserve access to matching evidence, the full history and the existing timeline. Where a report omits a criterion, describe it as unlisted. A missing entry alone cannot establish why a lab made its decision.

## Proposed layout and checks

Use the existing GPUI Kit controls and colour roles. At the minimum window size of 960 by 600, keep the view switch and assessment heading above the scrolling details and allow the chart to scroll within its pane. Keep existing case files, scoring rules, gene checks and learning material compatible. The builder chooses precise spacing and component boundaries.

Verify both themes, keyboard selection, visible focus, switching between reports and returning to evidence. Test that inspection and preview preserve saved evidence, including earlier strengths. Cover an empty case, a withheld class, an incomplete record and a lab whose stated class differs from its reconstructed result. Run the merge gate and, when the machine is available, exercise the controls in the native window.
