---
name: git-rebase
description: Performs git rebases with intelligent conflict resolution through commit message analysis. Use when rebasing branches, updating feature branches, resolving rebase conflicts, or cleaning up commit history.
---

# Git Rebase Assistant

Performs safe, effective rebases with intelligent conflict detection and resolution.

## Core Principle: Intent Before Resolution

Never resolve a conflict mechanically. Before touching any conflict:
1. Understand what master/main changed and WHY
2. Understand what the feature branch changed and WHY
3. Only then decide how to combine the changes

## Workflow

1. Validate prerequisites (`git status` must be clean)
2. Create safety backup branch
3. Gather context with `git log` on both sides
4. Execute rebase
5. Handle conflicts with intent-first approach
6. Verify and push

See `references/` for detailed conflict analysis and ours-vs-theirs guidance.
