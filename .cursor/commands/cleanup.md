# Review current commit changes

Identify the plan / phase file currently being worked on, if unclear, ask the user.

Ensure the changes match the plan, and are complete. If not, stop and inform the user.

Review each changed .rs file:

- ensure compliance with docs/rules.md
- ensure the implementation matches the C code, and that any deviations are well docmented and justified
- ensure all references to symbols are imported at the top of the file, not referenced inline like crate::modulue

Review each changed test file:

- ensure the tests match the C reference
- ensure no reductions in test functionality

Look for TODOs, FIXME, XXX, dbg!, println!, and any other temporary code and remove them.

Fix any violations of the rules.

Then run `just fci` to fix any warnings and run the tests.

Fix any warnings or errors.
