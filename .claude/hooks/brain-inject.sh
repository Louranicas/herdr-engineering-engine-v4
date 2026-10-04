#!/usr/bin/env bash
# brain-inject — SessionStart hook: print the brain index so the agent knows what build memory exists.
#
# Ported from brainmaxxing (poteto/brainmaxxing, .claude/hooks/inject-brain.sh, MIT). Advisory: prints
# plain text (Claude Code adds SessionStart stdout to context), never blocks, exits 0 on any error,
# no network, no Jev. Bounded: one `cat`.
#
# Seams:
#   HEE4_BRAIN   the brain directory   (default: $CLAUDE_PROJECT_DIR/brain, else <this file>/../../brain)
set -u
trap 'exit 0' ERR
cat > /dev/null 2>&1 || true   # drain the hook JSON on stdin; the banner does not depend on it

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd) || exit 0
repo=${CLAUDE_PROJECT_DIR:-$(cd "$here/../.." 2>/dev/null && pwd)}
brain=${HEE4_BRAIN:-$repo/brain}
index="$brain/index.md"

[ -f "$index" ] || exit 0

echo "Brain (v4 build memory, $brain) index. Read the relevant files before acting; /brain-reflect to add to it:"
echo ""
head -c 16384 "$index"
exit 0
