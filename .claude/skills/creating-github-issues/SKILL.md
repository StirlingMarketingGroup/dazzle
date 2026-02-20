---
name: creating-github-issues
description: Use when creating GitHub issues, making new issues, filing bugs, requesting features, or before running `gh issue create`. Triggers on "new issue", "create issue", "file issue", "gh issue", "open issue". Ensures proper research, labels, and comprehensive issue documentation.
---

# Creating GitHub Issues

## Overview

Every GitHub issue should be well-researched and clearly documented. This skill ensures issues contain enough context for anyone to understand and implement them.

## Mandatory Workflow

### Step 1: Research the Codebase

Before writing the issue, use the Explore agent or search tools to understand:

- Where relevant code lives (file paths, functions, components)
- How similar functionality is currently implemented
- Database tables, API endpoints, or UI components involved
- Existing patterns that should be followed

**Include in the issue:**
- Specific file paths and line numbers
- Code snippets showing current implementation
- References to similar existing code as examples

### Step 2: Create the Issue

Use `gh issue create` with a well-structured body:

```bash
gh issue create --title "Brief descriptive title" --body "$(cat <<'EOF'
## Summary
<1-3 sentences describing what needs to be done>

## Current State
<What exists now, with file paths and code references>

## Proposed Changes
<What should change, with specific details>

## Technical Notes
<File paths, database tables, API endpoints, code snippets>

## Acceptance Criteria
- [ ] Specific testable requirement
- [ ] Another requirement
EOF
)"
```

### Step 3: Add Labels

Discover current labels and apply appropriate ones:

```bash
gh label list --limit 50
gh issue edit <number> --add-label "enhancement"
```
