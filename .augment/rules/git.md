---
type: always_apply
description: Version control and git operation rules
---

# Version Control Rules

## Git Operations
- Whenever you run `git commit`, commit **ONLY** the staged changes.
- Whenever you run `git commit`, never add untracked files or changes.
- Whenever you run `git commit`, if there is an important untracked file, tell me, but commit without adding it.
- Whenever you run `git commit`, determine a suitable commit message which summarizes the output of `git diff --cached`
- Whenever my prompt is just "commit", run `git commit`
- After you run `git commit`, show me the commit message that you used.
