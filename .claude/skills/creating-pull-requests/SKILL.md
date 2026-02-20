---
name: creating-pull-requests
description: Use when creating a pull request or preparing changes for review. Ensures PR titles and descriptions accurately reflect ALL changes in the branch AND link to related GitHub issues.
---

# Creating Pull Requests

## Mandatory Workflow

### Step 1: Gather Complete Change Information

```bash
git diff --name-status $(git merge-base HEAD master)..HEAD
git diff $(git merge-base HEAD master)..HEAD
git log --oneline $(git merge-base HEAD master)..HEAD
git status
```

### Step 2: Search for Related GitHub Issues

```bash
gh issue list --search "keyword1 keyword2" --state all
```

### Step 3: Write PR with `gh pr create`

Include: Summary, Files Changed, Test Plan, and `Closes #XXX` for related issues.
