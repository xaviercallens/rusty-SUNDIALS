//! Transport estimators from tracked-vortex CSV files (`vortex::csv` format), printed as JSON.
//!
//!     cargo run --release -p qf-pgpe --example transport_estimate -- --l 64 --q 1,-1 FILE.csv [FILE2.csv ...]
//!
//! Options: `--l L` (box side, default 64), `--q q1,q2,...` (charges, default `1,-1,1,-1`), `--lag`, `--t-settle`,
//! `--d-valid`, `--lags-eta a,b,c` (defaults: the registered values of `transport_estimators.analyse_tracks`).
//! All files are analysed together as one set of tracks (blocks = tracks x halves).
use qf_pgpe::transport::{NAMES, Options, analyse_tracks, read_csv_track};
use serde_json::json;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (mut l, mut q, mut opts) = (64.0, vec![1, -1, 1, -1], Options::default());
    let mut files = Vec::new();
    fn list<T: std::str::FromStr>(s: &str) -> Result<Vec<T>, String> {
        s.split(',')
            .map(|v| {
                v.trim()
                    .parse::<T>()
                    .map_err(|_| format!("cannot parse '{v}'"))
            })
            .collect()
    }
    while let Some(a) = args.next() {
        let mut val = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match a.as_str() {
            "--l" => l = val("--l")?.parse().map_err(|e| format!("--l: {e}"))?,
            "--q" => q = list(&val("--q")?)?,
            "--lag" => opts.lag = val("--lag")?.parse().map_err(|e| format!("--lag: {e}"))?,
            "--t-settle" => {
                opts.t_settle = val("--t-settle")?
                    .parse()
                    .map_err(|e| format!("--t-settle: {e}"))?
            }
            "--d-valid" => {
                opts.d_valid = val("--d-valid")?
                    .parse()
                    .map_err(|e| format!("--d-valid: {e}"))?
            }
            "--lags-eta" => opts.lags_eta = list(&val("--lags-eta")?)?,
            f if !f.starts_with("--") => files.push(f.to_string()),
            other => return Err(format!("unknown option {other}")),
        }
    }
    if files.is_empty() {
        return Err("usage: transport_estimate --l 64 --q 1,-1 FILE.csv [FILE2.csv ...]".into());
    }
    let tracks = files
        .iter()
        .map(|f| {
            read_csv_track(
                &std::fs::read_to_string(f).map_err(|e| format!("{f}: {e}"))?,
                &q,
            )
            .map_err(|e| format!("{f}: {e}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let e = analyse_tracks(&tracks, l, &opts)?;
    let mut out = serde_json::Map::new();
    for (i, n) in NAMES.iter().enumerate() {
        out.insert(n.to_string(), json!(e.values()[i]));
        out.insert(format!("{n}_se"), json!(e.errors()[i]));
    }
    out.insert("n_blocks".into(), json!(e.n_blocks));
    out.insert("lag".into(), json!(e.lag));
    out.insert("lags_eta".into(), json!(e.lags_eta));
    out.insert("msd".into(), json!(e.msd));
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    Ok(())
}
