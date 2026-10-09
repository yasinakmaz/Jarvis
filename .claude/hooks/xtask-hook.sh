#!/bin/sh
# PostToolUse / Stop: `cargo xtask hook <olay>`. Sıfır dışı her sonuç 2'ye çevrilir:
# Claude Code yalnızca 2'de engeller ve stderr'i modele gösterir.
cd "${CLAUDE_PROJECT_DIR:-.}" || exit 2
cargo xtask hook "$1" 1>&2
[ "$?" -eq 0 ] && exit 0
exit 2
