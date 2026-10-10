//! Taban ref'e göre değişiklikler: diff kapsaması ve diff mutasyonu bunlara dayanır.
//!
//! Çalışma ağacı (izlenmeyen ama yok sayılmayan dosyalar dahil) geçici bir git dizinine
//! eklenip ağaç nesnesi olarak yazılır; asıl index'e dokunulmaz.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context as _, bail};

use crate::{cmd, workspace};

/// Git'in boş ağaç nesnesi: `--base root` ilk commit için tüm depoyu "değişmiş" sayar.
pub const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";
const DEFAULT_BASE: &str = "origin/main";

/// Taban ref'i çözer: `--base` > `JARVIS_BASE_REF` > `origin/main`. Sessiz geri dönüş yok.
pub fn resolve_base(root: &Path, explicit: Option<&str>) -> anyhow::Result<String> {
    let env = std::env::var("JARVIS_BASE_REF")
        .ok()
        .filter(|v| !v.is_empty());
    let wanted = explicit
        .map(str::to_owned)
        .or(env)
        .unwrap_or_else(|| DEFAULT_BASE.into());
    if wanted == "root" {
        return Ok(EMPTY_TREE.into());
    }
    let commit = format!("{wanted}^{{commit}}");
    if cmd::output(cmd::git(root).args(["rev-parse", "--verify", "--quiet", &commit])).is_err() {
        bail!(
            "taban ref `{wanted}` bulunamadı. `--base <ref>` veya JARVIS_BASE_REF verin \
             (taban dalı olmayan ilk commit için `--base root`)."
        );
    }
    let merge_base = cmd::output(cmd::git(root).args(["merge-base", "HEAD", &wanted]))
        .with_context(|| format!("HEAD ile `{wanted}` arasında ortak ata yok"))?;
    Ok(merge_base.trim().to_owned())
}

/// Çalışma ağacının anlık görüntüsünü ağaç nesnesi olarak yazar ve kimliğini döner.
pub fn snapshot(root: &Path) -> anyhow::Result<String> {
    let scratch = workspace::scratch(root);
    std::fs::create_dir_all(&scratch)?;
    let index = scratch.join("snapshot.index");
    let _ = std::fs::remove_file(&index);
    cmd::run(
        cmd::git(root)
            .env("GIT_INDEX_FILE", &index)
            .args(["add", "--all", "--", "."]),
    )?;
    let tree = cmd::output(
        cmd::git(root)
            .env("GIT_INDEX_FILE", &index)
            .arg("write-tree"),
    )?;
    Ok(tree.trim().to_owned())
}

/// `base` ile `tree` arasındaki birleşik diff; `paths` ile sınırlanır.
pub fn diff(root: &Path, base: &str, tree: &str, paths: &[&str]) -> anyhow::Result<String> {
    let mut command = cmd::git(root);
    command.args([
        "diff",
        "--no-color",
        "--no-ext-diff",
        "--unified=3",
        base,
        tree,
        "--",
    ]);
    command.args(paths);
    cmd::output(&mut command)
}

/// Diff'te eklenen/değişen satırlar: dosya → satır numaraları (yeni dosyaya göre).
pub fn added_lines(diff: &str) -> BTreeMap<String, BTreeSet<u32>> {
    let mut result: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
    let mut file: Option<String> = None;
    let mut line_no: u32 = 0;
    let mut previous = "";
    for line in diff.lines() {
        // `+++` yalnızca `---` başlığından hemen sonra dosya başlığıdır; aksi hâlde
        // `++` ile başlayan eklenmiş bir satırdır.
        let header = previous.starts_with("--- ");
        previous = line;
        if let (true, Some(path)) = (header, line.strip_prefix("+++ ")) {
            file = path.strip_prefix("b/").map(str::to_owned);
        } else if let Some(hunk) = line.strip_prefix("@@ ") {
            line_no = hunk_start(hunk).unwrap_or(0);
        } else if line.starts_with("diff ") {
            file = None;
        } else if let Some(name) = &file {
            if line.starts_with('+') {
                result.entry(name.clone()).or_default().insert(line_no);
                line_no = line_no.saturating_add(1);
            } else if line.starts_with(' ') {
                line_no = line_no.saturating_add(1);
            }
        }
    }
    result
}

/// `-a,b +c,d @@` başlığından yeni dosyadaki başlangıç satırını (`c`) okur.
fn hunk_start(hunk: &str) -> Option<u32> {
    let new_range = hunk
        .split_whitespace()
        .find_map(|part| part.strip_prefix('+'))?;
    new_range.split(',').next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIFF: &str = "\
diff --git a/crates/a/src/lib.rs b/crates/a/src/lib.rs
index 1..2 100644
--- a/crates/a/src/lib.rs
+++ b/crates/a/src/lib.rs
@@ -1,3 +1,5 @@
 //! doc
-fn old() {}
+fn new() {}
+fn added() {}
+++x;
 fn same() {}
@@ -10 +12,0 @@
-fn removed() {}
diff --git a/crates/b/src/new.rs b/crates/b/src/new.rs
new file mode 100644
--- /dev/null
+++ b/crates/b/src/new.rs
@@ -0,0 +1,2 @@
+fn one() {}
+fn two() {}
diff --git a/crates/c/src/gone.rs b/crates/c/src/gone.rs
deleted file mode 100644
--- a/crates/c/src/gone.rs
+++ /dev/null
@@ -1 +0,0 @@
-fn gone() {}
";

    #[test]
    fn collects_added_lines_per_file() {
        let added = added_lines(DIFF);
        assert_eq!(added.len(), 2, "{added:?}");
        assert_eq!(added["crates/a/src/lib.rs"], BTreeSet::from([2, 3, 4]));
        assert_eq!(added["crates/b/src/new.rs"], BTreeSet::from([1, 2]));
    }

    #[test]
    fn parses_hunk_headers() {
        assert_eq!(hunk_start("-1,3 +7,4 @@ fn x"), Some(7));
        assert_eq!(hunk_start("-0,0 +1 @@"), Some(1));
        assert_eq!(hunk_start("garbage"), None);
    }
}
