//! `cargo xtask verify`: kalite kapılarını Mimari §9'daki sırayla çalıştırır, ilk hatada
//! durur. `--fast` ilk dört adımı (hook'larda kullanılan hızlı alt küme) çalıştırır.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use anyhow::{Context as _, bail};

use crate::metadata::Metadata;
use crate::report::Findings;
use crate::{arch, cmd, coverage, deps, diff, expect_red, hygiene, mutants, size, workspace};

#[derive(Debug, Default)]
pub struct Options {
    pub fast: bool,
    pub base: Option<String>,
    /// Aynı çalışma ağacı daha önce yeşil doğrulandıysa tekrar çalıştırma (Stop hook'u).
    pub cache: bool,
}

/// Adımlar arasında paylaşılan durum.
struct Ctx {
    root: PathBuf,
    base: Option<String>,
    tree: Option<String>,
    lcov: Option<PathBuf>,
    diff: Option<String>,
}

type Step = (&'static str, fn(&mut Ctx) -> anyhow::Result<()>);

const FAST_STEPS: &[Step] = &[
    ("biçim (rustfmt)", fmt),
    ("dosya ve satır boyutu", size_gate),
    ("katman bağımlılığı ve çalışma alanı hijyeni", architecture),
    ("clippy", clippy),
];

const FULL_STEPS: &[Step] = &[
    (
        "bağımlılıklar (beyaz liste, cargo-deny, machete, hack)",
        dependencies,
    ),
    ("testler (nextest, kapsama ölçümüyle)", tests),
    ("değişen satır kapsaması", diff_coverage),
    ("değişen kodda mutasyon testi", mutation),
    ("sıcak yol performansı", performance),
    ("belgeler (cargo doc)", docs),
];

pub fn main(args: &[String]) -> anyhow::Result<ExitCode> {
    let mut options = Options::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--fast" => options.fast = true,
            "--base" => options.base = Some(iter.next().context("--base bir ref ister")?.clone()),
            "--expect-red" => {
                let test = iter.next().context("--expect-red bir test adı ister")?;
                return expect_red::run(&workspace::root(), test);
            }
            other => bail!("verify: bilinmeyen argüman `{other}`"),
        }
    }
    run(&workspace::root(), &options)?;
    Ok(ExitCode::SUCCESS)
}

/// Kapıları çalıştırır; ilk kırmızı adımda hata döner.
pub fn run(root: &Path, options: &Options) -> anyhow::Result<()> {
    let started = Instant::now();
    let mut ctx = Ctx {
        root: root.to_path_buf(),
        base: None,
        tree: None,
        lcov: None,
        diff: None,
    };
    let mut steps: Vec<Step> = FAST_STEPS.to_vec();
    if !options.fast {
        cmd::require_tools(root, cmd::REQUIRED_TOOLS)?;
        ctx.base = Some(diff::resolve_base(root, options.base.as_deref())?);
        ctx.tree = Some(diff::snapshot(root)?);
        steps.extend_from_slice(FULL_STEPS);
    }
    let marker = workspace::scratch(root).join("verified");
    let key = format!(
        "{} {}",
        ctx.base.as_deref().unwrap_or("-"),
        ctx.tree.as_deref().unwrap_or("-")
    );
    if options.cache && !options.fast && std::fs::read_to_string(&marker).ok() == Some(key.clone())
    {
        eprintln!("✔ çalışma ağacı değişmedi; önceki tam doğrulama geçerli ({key})");
        return Ok(());
    }
    let total = steps.len();
    for (index, (name, step)) in steps.iter().enumerate() {
        eprintln!("▶ [{}/{total}] {name}", index.saturating_add(1));
        let step_started = Instant::now();
        step(&mut ctx).with_context(|| format!("kapı kırmızı: {name}"))?;
        eprintln!("✔ {name} ({:.1?})", step_started.elapsed());
    }
    if !options.fast {
        std::fs::write(&marker, &key)?;
    }
    eprintln!(
        "✔ tüm kapılar yeşil: {total} adım, {:.1?}",
        started.elapsed()
    );
    Ok(())
}

fn fmt(ctx: &mut Ctx) -> anyhow::Result<()> {
    cmd::run(cmd::cargo(&ctx.root).args(["fmt", "--all", "--check"]))
        .context("biçim farkı var; `cargo fmt --all` çalıştırın")
}

fn size_gate(ctx: &mut Ctx) -> anyhow::Result<()> {
    size::check(&ctx.root)?.finish("boyut")
}

fn architecture(ctx: &mut Ctx) -> anyhow::Result<()> {
    let metadata = Metadata::load(&ctx.root)?;
    let mut findings = arch::check(&ctx.root, &metadata)?;
    findings.merge(hygiene::check(&ctx.root, &metadata)?);
    findings.finish("mimari")
}

fn clippy(ctx: &mut Ctx) -> anyhow::Result<()> {
    cmd::run(cmd::cargo(&ctx.root).args([
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--locked",
        "--",
        "-D",
        "warnings",
    ]))
    .context("clippy uyarıları hata sayılır; `#[allow]` yasak, gerekçeli `#[expect]` kullanın")
}

fn dependencies(ctx: &mut Ctx) -> anyhow::Result<()> {
    let metadata = Metadata::load(&ctx.root)?;
    deps::check(&ctx.root, &metadata)?.finish("beyaz liste")?;
    cmd::run(cmd::cargo(&ctx.root).args(["deny", "--all-features", "--locked", "check"]))?;
    cmd::run(cmd::cargo(&ctx.root).args(["machete"]))
        .context("kullanılmayan bağımlılık var; Cargo.toml'dan kaldırın")?;
    // `--no-dev-deps` kullanılmaz: manifestten dev-bağımlılıkları geçici olarak silip
    // `Cargo.lock`'u değiştirir ve `--locked` ile çakışır. Resolver 3'te dev-bağımlılık
    // özellikleri normal derlemeyle zaten birleşmediği için ek bir şey yakalamaz.
    cmd::run(cmd::cargo(&ctx.root).args([
        "hack",
        "check",
        "--workspace",
        "--each-feature",
        "--locked",
    ]))
}

fn tests(ctx: &mut Ctx) -> anyhow::Result<()> {
    ctx.lcov = Some(coverage::run_tests(&ctx.root)?);
    Ok(())
}

fn diff_coverage(ctx: &mut Ctx) -> anyhow::Result<()> {
    let (Some(base), Some(tree), Some(lcov)) = (&ctx.base, &ctx.tree, &ctx.lcov) else {
        bail!("iç hata: taban, ağaç veya lcov hazır değil");
    };
    let text = diff::diff(&ctx.root, base, tree, &["crates"])?;
    let lcov_text = std::fs::read_to_string(lcov)?;
    let hits = coverage::parse_lcov(&ctx.root, &lcov_text);
    let findings = coverage::check(&hits, &diff::added_lines(&text));
    ctx.diff = Some(text);
    findings.finish("diff kapsaması")
}

fn mutation(ctx: &mut Ctx) -> anyhow::Result<()> {
    let text = ctx.diff.as_deref().context("iç hata: diff hazır değil")?;
    mutants::run(&ctx.root, text)?.finish("mutasyon")
}

fn performance(ctx: &mut Ctx) -> anyhow::Result<()> {
    let metadata = Metadata::load(&ctx.root)?;
    let benches: Vec<String> = metadata
        .packages
        .iter()
        .flat_map(|p| p.targets.iter().map(move |t| (p, t)))
        .filter(|(_, t)| t.kind.iter().any(|k| k == "bench"))
        .map(|(p, t)| format!("{}::{}", p.name, t.name))
        .collect();
    let mut findings = Findings::default();
    if benches.is_empty() {
        findings.note("atlandı: tanımlı sıcak yol benchmark'ı yok (Gungraun kapısı M3'te)");
    } else {
        findings.error(format!(
            "benchmark hedefleri var ({}) ama Gungraun karşılaştırması henüz kurulmadı; \
             kapı insan tarafından xtask'a eklenmeden benchmark eklenemez.",
            benches.join(", ")
        ));
    }
    findings.finish("performans")
}

fn docs(ctx: &mut Ctx) -> anyhow::Result<()> {
    cmd::run(
        cmd::cargo(&ctx.root)
            .env("RUSTDOCFLAGS", "-D warnings")
            .args([
                "doc",
                "--workspace",
                "--no-deps",
                "--all-features",
                "--locked",
            ]),
    )
    .context("belge uyarıları hata sayılır")
}
