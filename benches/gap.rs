//! Generative Artifact Protocol (GAP) benchmarks — apply-engine speed on the
//! real fixtures in assets/evals/apply-engine/.
//!
//! Run: cargo bench --bench gap
//! Filter to one case: cargo bench --bench gap -- "edit_replace/case_0002"
//! Scale sweep (grow artifacts 2-4x): GAP_BENCH_SCALES=1,2,3,4 cargo bench --bench gap
//!
//! Payload sizes (wire/content bytes per envelope) are reported by
//! `just payload-report`, not here.

use std::fs;
use std::path::PathBuf;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use gap::apply::{self, apply_edit, TextResolver};
use gap::gap::{EditOp, Envelope};

// ── Fixture loading ────────────────────────────────────────────────────────

const EDIT_FAMILIES: &[&str] = &[
    "edit-replace",
    "edit-multi",
    "edit-delete",
    "edit-section-single",
    "edit-section-multi",
];

struct Fixture {
    case: String,
    artifact: String,
    synthesize: Vec<Envelope>,
    /// (family name, one op-list per envelope in the family's JSONL file).
    edits: Vec<(&'static str, Vec<Vec<EditOp>>)>,
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/evals/apply-engine")
}

fn load_fixture(case_dir: &str) -> Fixture {
    let base = fixtures_dir().join(case_dir);
    let artifact =
        fs::read_to_string(base.join("artifacts/dashboard.html")).expect("read artifact");

    let parse_envelopes = |name: &str| -> Vec<Envelope> {
        let path = base.join("envelopes").join(format!("{name}.jsonl"));
        fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| Envelope::from_json(l).expect("parse envelope"))
            .collect()
    };

    let parse_edit_ops = |envs: Vec<Envelope>| -> Vec<Vec<EditOp>> {
        envs.into_iter()
            .map(|env| {
                env.content
                    .into_iter()
                    .map(|v| serde_json::from_value::<EditOp>(v).expect("parse EditOp"))
                    .collect()
            })
            .collect()
    };

    // Pre-flight: bench only envelopes that actually apply. The legacy
    // edit-replace/multi/delete fixtures target raw source lines instead of
    // `<gap:target>` ids, so they can never resolve; benching their error
    // path would measure nothing useful.
    let resolver = TextResolver {
        format: "text/html".to_string(),
    };
    let mut skipped = 0usize;
    let edits: Vec<(&'static str, Vec<Vec<EditOp>>)> = EDIT_FAMILIES
        .iter()
        .map(|family| {
            let all = parse_edit_ops(parse_envelopes(family));
            let n = all.len();
            let ok: Vec<Vec<EditOp>> = all
                .into_iter()
                .filter(|ops| apply_edit(&resolver, &artifact, ops).is_ok())
                .collect();
            skipped += n - ok.len();
            (*family, ok)
        })
        .collect();
    if skipped > 0 {
        eprintln!("case {case_dir}: skipped {skipped} envelope(s) whose targets do not resolve");
    }

    Fixture {
        case: case_dir.into(),
        artifact,
        synthesize: parse_envelopes("synthesize"),
        edits,
    }
}

fn all_fixtures() -> Vec<Fixture> {
    let dir = fixtures_dir();
    let mut cases: Vec<String> = fs::read_dir(&dir)
        .expect("read apply-engine dir")
        .filter_map(|e| {
            let e = e.ok()?;
            if e.file_type().ok()?.is_dir() {
                Some(e.file_name().to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();
    cases.sort();
    let fixtures: Vec<Fixture> = cases.into_iter().map(|c| load_fixture(&c)).collect();
    assert!(
        !fixtures.is_empty(),
        "no fixtures found under {}",
        dir.display()
    );
    fixtures
}

// ── Scale helpers ──────────────────────────────────────────────────────────

/// Artifact size multipliers. Defaults to the unscaled fixture; set
/// GAP_BENCH_SCALES=1,2,3,4 to sweep larger artifacts.
fn scales() -> Vec<usize> {
    std::env::var("GAP_BENCH_SCALES")
        .map(|s| s.split(',').filter_map(|v| v.trim().parse().ok()).collect())
        .unwrap_or_else(|_| vec![1])
}

/// Grow an artifact without duplicating `<gap:target>` ids (duplicate ids make
/// scaled inputs unrepresentative: resolution would always hit the first
/// copy). Repeats the first `<tbody>` contents when marker-free, else inserts
/// comment padding ahead of the first marker so both document size and marker
/// scan distance grow.
fn scale_artifact(html: &str, multiplier: usize) -> String {
    if multiplier <= 1 {
        return html.to_string();
    }
    if let Some(start) = html.find("<tbody>") {
        if let Some(rel_end) = html[start..].find("</tbody>") {
            let inner = &html[start + 7..start + rel_end];
            if !inner.contains("<gap:target ") {
                let repeated = inner.repeat(multiplier);
                return format!(
                    "{}{}{}",
                    &html[..start + 7],
                    repeated,
                    &html[start + rel_end..]
                );
            }
        }
    }
    let pad = "<!-- pad -->".repeat(html.len() * (multiplier - 1) / 12 + 1);
    match html.find("<gap:target ") {
        Some(i) => format!("{}{}{}", &html[..i], pad, &html[i..]),
        None => format!("{pad}{html}"),
    }
}

// ── Benchmarks ─────────────────────────────────────────────────────────────

fn bench_full_copy(c: &mut Criterion) {
    let fixtures = all_fixtures();
    let scales = scales();

    let mut group = c.benchmark_group("full_copy");
    for f in &fixtures {
        for &scale in &scales {
            let scaled = scale_artifact(&f.artifact, scale);
            let label = format!("case_{}/{}x_{}B", f.case, scale, scaled.len());
            group.bench_with_input(BenchmarkId::from_parameter(&label), &scaled, |b, html| {
                b.iter(|| {
                    let copy = html.to_string();
                    std::hint::black_box(copy);
                })
            });
        }
    }
    group.finish();
}

/// Full `apply::apply` on the synthesize envelope — the cost of establishing
/// an artifact (content copy + handle construction with target extraction).
fn bench_synthesize(c: &mut Criterion) {
    let fixtures = all_fixtures();

    let mut group = c.benchmark_group("synthesize");
    for f in &fixtures {
        for (i, env) in f.synthesize.iter().enumerate() {
            let label = format!("case_{}/env_{}", f.case, i);
            group.bench_with_input(BenchmarkId::from_parameter(&label), env, |b, env| {
                b.iter(|| apply::apply(None, env).unwrap())
            });
        }
    }
    group.finish();
}

fn bench_edits(c: &mut Criterion) {
    let fixtures = all_fixtures();
    let scales = scales();
    let resolver = TextResolver {
        format: "text/html".to_string(),
    };

    for family in EDIT_FAMILIES {
        let mut group = c.benchmark_group(family.replace('-', "_"));
        for f in &fixtures {
            let Some((_, env_ops)) = f.edits.iter().find(|(name, _)| name == family) else {
                continue;
            };
            for (i, ops) in env_ops.iter().enumerate() {
                for &scale in &scales {
                    let scaled = scale_artifact(&f.artifact, scale);
                    let label = format!("case_{}/env_{}/{}x_{}B", f.case, i, scale, scaled.len());
                    group.bench_with_input(
                        BenchmarkId::from_parameter(&label),
                        &scaled,
                        |b, html| b.iter(|| apply_edit(&resolver, html, ops).unwrap()),
                    );
                }
            }
        }
        group.finish();
    }
}

criterion_group!(benches, bench_full_copy, bench_synthesize, bench_edits);
criterion_main!(benches);
