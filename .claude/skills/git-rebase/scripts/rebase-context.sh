#!/bin/bash
# rebase-context.sh - Gather full context before rebasing
# Usage: ./rebase-context.sh [base-branch]

set -e

BASE_BRANCH="${1:-origin/main}"
CURRENT_BRANCH=$(git branch --show-current)

echo "=============================================="
echo "REBASE CONTEXT ANALYSIS"
echo "=============================================="
echo "Current branch: $CURRENT_BRANCH"
echo "Target base:    $BASE_BRANCH"
echo ""

git fetch origin --quiet

echo "COMMITS IN $BASE_BRANCH (not in $CURRENT_BRANCH):"
git log --oneline HEAD..$BASE_BRANCH

echo ""
echo "YOUR COMMITS (will be replayed):"
git log --oneline $BASE_BRANCH..HEAD

echo ""
echo "FILES CHANGED IN BOTH (potential conflicts):"
BASE_FILES=$(git diff --name-only HEAD...$BASE_BRANCH 2>/dev/null || echo "")
OUR_FILES=$(git diff --name-only $BASE_BRANCH...HEAD 2>/dev/null || echo "")

if [ -n "$BASE_FILES" ] && [ -n "$OUR_FILES" ]; then
    OVERLAP=$(comm -12 <(echo "$BASE_FILES" | sort) <(echo "$OUR_FILES" | sort))
    if [ -n "$OVERLAP" ]; then
        echo "$OVERLAP"
    else
        echo "(No overlapping files - conflicts unlikely)"
    fi
fi
