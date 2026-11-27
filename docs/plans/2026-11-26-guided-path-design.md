# Guided assessment

The curator can choose **Guided assessment** from the welcome screen to review the evidence one question at a time in the existing workbench. A documented assessment can have a withheld class. The existing scoring engine owns every class and reason for withholding it. Every displayed class carries the review status and rule version.

## First release

1. Prototype the most difficult interactions with RYR1 CA024770. Observe a curator resolve a disagreement, save and resume an incomplete review, and explain a withheld result. Check keyboard access, both themes and both window sizes.
2. Add an optional ordered path to the rule file. Validate unique step IDs, complete single coverage of criteria, helper ownership and valid hint references at load time. Include questions and hints in the sign-off file and review packet. A rule file without a path continues to support the free workbench.
3. Keep one shared helper-to-criteria table. Use the existing guides and strength limits in both modes. Optional consequence and splicing questions support unknown answers. Separate predicted and observed effects when the wording depends on that distinction. Facts may add hints. All steps remain available.
4. Derive progress from individual decisions. Preserve applied evidence, explicit exclusions, deferrals and untouched criteria. Give each guide and gene-level decision a stable ID. PP5 and BP6 are visible references and require no answer for a new assessment.
5. Save review decisions and facts as optional case fields. Validate once on read. Preserve unknown review entries through save and reopen with different rules, identify them to the curator and exclude them from current progress. Keep historical records intact. Test a full round trip under the original rules.
6. Add navigation to the existing editor. Allow jumping, switching mode and saving at any point. Show pending proposals separately from confirmed evidence. Include progress and the engine's current result in the summary. Match free and guided assessments for identical confirmed inputs.
7. Run headless window, keyboard and case tests, the merge gate and benchmark. Update the project map, roadmap, scope and review packet. Run the Windows mouse and keyboard smoke test when the owner confirms the machine is free.

## Population step acceptance examples

| Action | Result |
| --- | --- |
| View a BS1 proposal with PM2 applied | Only PM2 counts. BA1 and BS1 retain their prior review states. |
| Keep PM2 | Dismiss the proposal. BS1 remains unanswered unless explicitly reviewed. |
| Confirm the labeled replacement | Remove PM2 and apply BS1. PM2 returns to unreviewed. BA1 is untouched. |
| Defer BS1 | Keep PM2 and record BS1 as deferred. |
| Edit an answer | Recompute progress and assessment from the current choices. |
| Save and reopen | Restore decisions, distinguish deferred from untouched, and show any remaining work. |
| Encounter contradictory applied evidence | Show the engine's reason for withholding. A proposal alone cannot cause withholding. |

## Later release decision

Class stability needs an independent decision after guided review ships. Define which existing choices are held fixed, include deferred evidence and pairs of open criteria, use the actual gene decision and evaluate possible completions against the existing engine. Check exhaustive small cases and measure how much review time early stopping saves. The first release displays progress without making a stability claim.
