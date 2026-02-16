#!/usr/bin/env bash
set -euo pipefail

pattern='\p{Emoji_Presentation}|\p{Emoji}\x{FE0F}'

status=0
rg -nUP --glob '*.md' "$pattern" docs || status=$?

if [ "$status" -eq 0 ]; then
  echo "docs check failed: emoji detected in docs markdown"
  exit 1
fi

if [ "$status" -eq 1 ]; then
  echo "docs check passed: no emojis detected"
  exit 0
fi

echo "docs check failed: ripgrep returned unexpected status $status"
exit "$status"
