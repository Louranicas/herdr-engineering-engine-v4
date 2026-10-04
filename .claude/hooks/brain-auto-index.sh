#!/usr/bin/env bash
# brain-auto-index — PostToolUse (Edit|Write) hook: rebuild brain/index.md when a brain file is added or removed.
#
# Ported from brainmaxxing (poteto/brainmaxxing, .claude/hooks/auto-index-brain.sh, MIT). Changes in the
# port: the brain directory is a seam (HEE4_BRAIN); the hook reads the tool's file_path from the hook JSON
# and acts only when that path is inside the brain (Claude Code matchers match the TOOL name, so the
# original `matcher: "brain/"` never fired); `find` is called as `command find`; advisory, exits 0 on any
# error, writes nothing but the index, no network, no Jev. Emits bare wikilinks, no generated prose.
#
# Seams:
#   HEE4_BRAIN   the brain directory   (default: $CLAUDE_PROJECT_DIR/brain, else <this file>/../../brain)
set -u
trap 'exit 0' ERR

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd) || exit 0
repo=${CLAUDE_PROJECT_DIR:-$(cd "$here/../.." 2>/dev/null && pwd)}
brain=${HEE4_BRAIN:-$repo/brain}
index="$brain/index.md"

# Only act on an edit inside the brain. The hook JSON carries tool_input.file_path (Edit, Write).
input=$(head -c 65536 2>/dev/null || true)
file=""
if command -v jq >/dev/null 2>&1; then
  file=$(printf '%s' "$input" | jq -r '.tool_input.file_path // .tool_input.path // empty' 2>/dev/null || true)
else
  file=$(printf '%s' "$input" | sed -n 's/.*"file_path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)
fi
[ -n "$file" ] || exit 0
[ -d "$brain" ] || exit 0
[ -f "$index" ] || exit 0
brain_abs=$(cd "$brain" 2>/dev/null && pwd -P) || exit 0
case "$file" in
  "$brain_abs"/*|"$brain"/*) ;;
  *) exit 0 ;;
esac

# All .md files except index.md: paths relative to the brain, without the .md extension.
disk=$(command find "$brain_abs" -name '*.md' ! -name 'index.md' -type f 2>/dev/null \
    | sed "s|^${brain_abs}/||; s|\.md$||" | sort)

# Wikilinks the index already carries.
indexed=$(sed -n 's/.*\[\[\([^]|#]*\)[^]]*\]\].*/\1/p' "$index" | sort)

# Nothing added or removed: leave the index alone (it may carry hand-written grouping).
[ "$disk" = "$indexed" ] && exit 0

emit_files() {
  while IFS= read -r f; do
    [ -z "$f" ] && continue
    echo "- [[$f]]"
  done
}

dirs=$(printf '%s\n' "$disk" | grep '/' | sed 's|/.*||' | sort -u || true)

tmp=$(mktemp "${brain_abs}/.index.XXXXXX") || exit 0
{
  echo "# Brain"
  for section in $dirs; do
    files=$(printf '%s\n' "$disk" | grep "^${section}\(/\|$\)" || true)
    [ -z "$files" ] && continue
    header="$(printf '%s' "$section" | sed 's/./\U&/')"
    printf '\n## %s\n' "$header"
    printf '%s\n' "$files" | emit_files
  done
  standalone=$(printf '%s\n' "$disk" | grep -v '/' || true)
  if [ -n "$standalone" ]; then
    printf '\n## Other\n'
    printf '%s\n' "$standalone" | emit_files
  fi
  echo ""
} > "$tmp" && mv -f "$tmp" "$index"
rm -f "$tmp" 2>/dev/null || true
exit 0
