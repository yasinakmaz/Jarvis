//! `cargo metadata --no-deps` çıktısı (yalnızca ihtiyaç duyulan alanlar).
//!
//! `--no-deps` bağımlılık çözümlemesi yapmaz; çevrimdışı çalışır ve yalnızca
//! çalışma alanı paketlerinin bildirdiği bağımlılıkları verir.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use serde::Deserialize;

use crate::cmd;

#[derive(Debug, Deserialize)]
pub struct Metadata {
    pub packages: Vec<Package>,
}

#[derive(Debug, Deserialize)]
pub struct Package {
    pub name: String,
    pub manifest_path: PathBuf,
    pub dependencies: Vec<Dependency>,
    pub targets: Vec<Target>,
}

#[derive(Debug, Deserialize)]
pub struct Dependency {
    pub name: String,
    /// `None` normal bağımlılık; `Some("dev")` veya `Some("build")`.
    pub kind: Option<String>,
}

impl Dependency {
    pub fn is_dev(&self) -> bool {
        self.kind.as_deref() == Some("dev")
    }
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub name: String,
    pub kind: Vec<String>,
}

impl Metadata {
    /// Çalışma alanı metaverisini yükler.
    pub fn load(root: &Path) -> anyhow::Result<Self> {
        let json = cmd::output(cmd::cargo(root).args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--offline",
        ]))?;
        serde_json::from_str(&json).context("cargo metadata çıktısı ayrıştırılamadı")
    }

    /// Çalışma alanındaki paket adları.
    pub fn member_names(&self) -> BTreeSet<&str> {
        self.packages.iter().map(|p| p.name.as_str()).collect()
    }

    /// Çalışma alanı dışından gelen (üçüncü taraf) bağımlılık adları.
    pub fn external_dependencies(&self) -> BTreeSet<&str> {
        let members = self.member_names();
        self.packages
            .iter()
            .flat_map(|p| p.dependencies.iter())
            .map(|d| d.name.as_str())
            .filter(|name| !members.contains(name))
            .collect()
    }
}
