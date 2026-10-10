#!/bin/sh
# PreToolUse: korunan dosyalara yazmayı engeller. python3 yoksa da engeller (fail-closed).
if ! command -v python3 >/dev/null 2>&1; then
    echo "ENGELLENDİ: pre-tool-use hook'u için python3 gerekli (başarısızlıkta kapalı)." >&2
    exit 2
fi
python3 -I "$(dirname "$0")/pre_tool_use.py"
code=$?
[ "$code" -eq 0 ] && exit 0
exit 2
