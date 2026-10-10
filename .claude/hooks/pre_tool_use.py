"""PreToolUse hook'u: kalite kapısı yapılandırmasına yazmayı engeller (Mimari §9 Katman 5).

Çıkış kodu 2, Claude Code'da aracı engeller ve stderr'i modele gösterir. Herhangi bir
belirsizlik ya da hata durumunda engellenir (başarısızlıkta kapalı).

Bash denetimi en iyi çabadır: asıl zorlama CODEOWNERS, branch protection ve CI'dadır.
Bu dosya da korunur; yalnızca insan değiştirir.
"""

import json
import os
import re
import sys

PROTECTED_FILES = {
    "clippy.toml",
    "rustfmt.toml",
    "rust-toolchain.toml",
    "architecture.toml",
    "deny.toml",
}
PROTECTED_DIRS = ("xtask/", ".github/", ".claude/", ".cargo/", "supply-chain/")
FILE_TOOLS = {"Write": "file_path", "Edit": "file_path", "MultiEdit": "file_path",
              "NotebookEdit": "notebook_path"}

_NAMES = "|".join(re.escape(n) for n in sorted(PROTECTED_FILES)) + \
    "|" + "|".join(re.escape(d.rstrip("/")) for d in PROTECTED_DIRS)
MENTION = re.compile(r"(?<![\w.-])(?:\./)?(?:" + _NAMES + r")(?![\w-])")
WRITE_HINT = re.compile(
    r">|\btee\b|\bsed\b[^|;&]*\s-i|\bperl\b[^|;&]*\s-i|\b(mv|cp|rm|rmdir|truncate|dd|install"
    r"|ln|chmod|chown|touch|unlink|patch|python3?|awk|ed|ex|vi|vim|nano)\b"
    r"|\bgit\s+(checkout|restore|apply|rm|mv|reset|stash|clean)\b|\bcurl\b|\bwget\b")
SAFE_FRAGMENTS = re.compile(r"\bcargo\s+xtask\b|(?:-p|--package)\s+xtask\b|\d*>&\d+|\d*>\s*/dev/null")

MESSAGE = ("ENGELLENDİ: `{target}` kalite kapısı yapılandırmasıdır (Mimari §9 Katman 5). "
           "Yapay zeka kapı sistemini değiştiremez veya gevşetemez. Sorunu kodda çözün; "
           "kapı değişikliği gerekiyorsa gerekçesini kullanıcıya bildirin.")


def relative(path, root):
    """Yolu proje köküne göre normalleştirir; kök dışındaysa None döner."""
    full = os.path.realpath(os.path.join(root, path))
    root = os.path.realpath(root)
    if full != root and not full.startswith(root + os.sep):
        return None
    return os.path.relpath(full, root).replace(os.sep, "/")


def is_protected(rel):
    return rel in PROTECTED_FILES or any(rel.startswith(d) for d in PROTECTED_DIRS)


def check_bash(command):
    """Korunan bir yolu anan ve yazma belirtisi taşıyan komutları engeller."""
    cleaned = SAFE_FRAGMENTS.sub(" ", command)
    mention = MENTION.search(cleaned)
    if mention and WRITE_HINT.search(cleaned):
        return mention.group(0)
    return None


def decide(event, root):
    tool = event.get("tool_name", "")
    tool_input = event.get("tool_input") or {}
    if tool in FILE_TOOLS:
        path = tool_input.get(FILE_TOOLS[tool])
        if not isinstance(path, str) or not path:
            return "<yol yok>"
        rel = relative(path, root)
        return rel if rel is not None and is_protected(rel) else None
    if tool == "Bash":
        command = tool_input.get("command")
        if not isinstance(command, str):
            return "<komut yok>"
        return check_bash(command)
    return None


def main():
    try:
        event = json.load(sys.stdin)
        root = os.environ.get("CLAUDE_PROJECT_DIR") or event.get("cwd") or os.getcwd()
        target = decide(event, root)
    except Exception as err:  # başarısızlıkta kapalı
        print(f"ENGELLENDİ: pre-tool-use hook'u girdiyi değerlendiremedi: {err}", file=sys.stderr)
        return 2
    if target is not None:
        print(MESSAGE.format(target=target), file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
