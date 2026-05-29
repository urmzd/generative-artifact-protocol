use clap::{Parser, ValueEnum};
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("csv error: {0}")]
    Csv(#[from] csv::Error),
    #[error("invalid filter expression: {0}")]
    InvalidFilter(String),
    #[error("missing column: {0}")]
    MissingColumn(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("output error: {0}")]
    Output(String),
}

#[derive(Parser, Debug)]
#[command(name = "csvtool", version, about = "Process and analyze CSV files")]
struct Args {
    #[arg(short, long)]
    input: PathBuf,

    #[arg(short, long, value_enum, default_value = "table")]
    format: OutputFormat,

    #[arg(short, long, value_delimiter = ',')]
    columns: Vec<String>,

    #[arg(short, long)]
    filter: Option<String>,

    #[arg(short, long)]
    sort: Option<String>,

    #[arg(short, long)]
    limit: Option<usize>,

    #[arg(long, default_value_t = 10)]
    head: usize,

    #[arg(long)]
    group_by: Option<String>,

    #[arg(long, value_delimiter = ',')]
    aggregate: Vec<AggregateSpec>,
}

#[derive(Clone, Debug, ValueEnum)]
enum OutputFormat {
    Table,
    Json,
    Csv,
    Markdown,
}

#[derive(Clone, Debug)]
struct Record {
    values: HashMap<String, String>,
}

impl Record {
    fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }
}

#[derive(Clone, Debug)]
enum Op {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    Contains,
}

#[derive(Clone, Debug)]
struct FilterExpr {
    column: String,
    op: Op,
    value: String,
}

impl FilterExpr {
    fn parse(input: &str) -> Result<Self, CliError> {
        let ops = [
            ("==", Op::Eq),
            ("!=", Op::Ne),
            (">=", Op::Ge),
            ("<=", Op::Le),
            (">", Op::Gt),
            ("<", Op::Lt),
            ("~", Op::Contains),
        ];
        for (sym, op) in ops {
            if let Some(idx) = input.find(sym) {
                let column = input[..idx].trim().to_string();
                let value = input[idx + sym.len()..].trim().trim_matches('"').to_string();
                if column.is_empty() || value.is_empty() {
                    return Err(CliError::InvalidFilter(input.to_string()));
                }
                return Ok(Self { column, op, value });
            }
        }
        Err(CliError::InvalidFilter(input.to_string()))
    }

    fn matches(&self, record: &Record) -> bool {
        let lhs = match record.get(&self.column) {
            Some(v) => v,
            None => return false,
        };
        match self.op {
            Op::Eq => lhs == self.value,
            Op::Ne => lhs != self.value,
            Op::Contains => lhs.contains(&self.value),
            Op::Gt | Op::Ge | Op::Lt | Op::Le => {
                let l = lhs.parse::<f64>();
                let r = self.value.parse::<f64>();
                match (l, r) {
                    (Ok(l), Ok(r)) => match self.op {
                        Op::Gt => l > r,
                        Op::Ge => l >= r,
                        Op::Lt => l < r,
                        Op::Le => l <= r,
                        _ => false,
                    },
                    _ => match self.op {
                        Op::Gt => lhs > self.value.as_str(),
                        Op::Ge => lhs >= self.value.as_str(),
                        Op::Lt => lhs < self.value.as_str(),
                        Op::Le => lhs <= self.value.as_str(),
                        _ => false,
                    },
                }
            }
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct AggregationResult {
    group: String,
    metric: String,
    column: String,
    value: f64,
}

#[derive(Clone, Debug)]
enum AggKind {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

#[derive(Clone, Debug)]
struct AggregateSpec {
    kind: AggKind,
    column: String,
}

impl std::str::FromStr for AggregateSpec {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let (kind_str, column) = s
            .split_once(':')
            .ok_or_else(|| format!("expected kind:column, got {s}"))?;
        let kind = match kind_str.to_lowercase().as_str() {
            "count" => AggKind::Count,
            "sum" => AggKind::Sum,
            "avg" => AggKind::Avg,
            "min" => AggKind::Min,
            "max" => AggKind::Max,
            _ => return Err(format!("unknown aggregation kind {kind_str}")),
        };
        Ok(Self {
            kind,
            column: column.to_string(),
        })
    }
}

fn read_csv(path: &PathBuf) -> Result<Vec<Record>, CliError> {
    let file = File::open(path)?;
    let mut rdr = csv::Reader::from_reader(BufReader::new(file));
    let headers = rdr.headers()?.clone();
    let mut out = Vec::new();
    for row in rdr.records() {
        let row = row?;
        let mut values = HashMap::new();
        for (h, v) in headers.iter().zip(row.iter()) {
            values.insert(h.to_string(), v.to_string());
        }
        out.push(Record { values });
    }
    Ok(out)
}

fn select_columns(records: &[Record], cols: &[String]) -> Result<Vec<Record>, CliError> {
    if cols.is_empty() {
        return Ok(records.to_vec());
    }
    let mut out = Vec::with_capacity(records.len());
    for r in records {
        let mut values = HashMap::new();
        for c in cols {
            let v = r
                .get(c)
                .ok_or_else(|| CliError::MissingColumn(c.clone()))?;
            values.insert(c.clone(), v.to_string());
        }
        out.push(Record { values });
    }
    Ok(out)
}

fn filter_records(records: &[Record], filter: &Option<FilterExpr>) -> Vec<Record> {
    match filter {
        None => records.to_vec(),
        Some(f) => records.iter().cloned().filter(|r| f.matches(r)).collect(),
    }
}

fn sort_records(records: &mut [Record], column: &str) {
    records.sort_by(|a, b| {
        let av = a.get(column).unwrap_or("");
        let bv = b.get(column).unwrap_or("");
        let ap = av.parse::<f64>();
        let bp = bv.parse::<f64>();
        match (ap, bp) {
            (Ok(a), Ok(b)) => a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal),
            _ => av.cmp(bv),
        }
    });
}

fn limit_records(mut records: Vec<Record>, limit: Option<usize>) -> Vec<Record> {
    if let Some(n) = limit {
        records.truncate(n);
    }
    records
}

fn head_records(mut records: Vec<Record>, head: usize) -> Vec<Record> {
    records.truncate(head);
    records
}

fn aggregate(records: &[Record], group_by: &str, specs: &[AggregateSpec]) -> Result<Vec<AggregationResult>, CliError> {
    let mut groups: HashMap<String, Vec<&Record>> = HashMap::new();
    for r in records {
        let group = r.get(group_by).unwrap_or("").to_string();
        groups.entry(group).or_default().push(r);
    }

    let mut results = Vec::new();
    for (group, items) in groups {
        for spec in specs {
            let value = match spec.kind {
                AggKind::Count => items.len() as f64,
                AggKind::Sum | AggKind::Avg | AggKind::Min | AggKind::Max => {
                    let mut nums = Vec::new();
                    for r in &items {
                        let raw = r
                            .get(&spec.column)
                            .ok_or_else(|| CliError::MissingColumn(spec.column.clone()))?;
                        let n = raw.parse::<f64>().map_err(|_| {
                            CliError::Parse(format!("cannot parse {} as number", raw))
                        })?;
                        nums.push(n);
                    }
                    if nums.is_empty() {
                        0.0
                    } else {
                        match spec.kind {
                            AggKind::Sum => nums.iter().sum(),
                            AggKind::Avg => nums.iter().sum::<f64>() / nums.len() as f64,
                            AggKind::Min => nums.iter().cloned().fold(f64::INFINITY, f64::min),
                            AggKind::Max => nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                            AggKind::Count => unreachable!(),
                        }
                    }
                }
            };
            results.push(AggregationResult {
                group: group.clone(),
                metric: format!("{:?}", spec.kind).to_lowercase(),
                column: spec.column.clone(),
                value,
            });
        }
    }
    Ok(results)
}

fn ordered_headers(records: &[Record]) -> Vec<String> {
    let mut headers: Vec<String> = records.first().map(|r| r.values.keys().cloned().collect()).unwrap_or_else(Vec::new);
    headers.sort();
    headers
}

fn format_table(records: &[Record]) -> String {
    if records.is_empty() {
        return String::from("(no records)");
    }
    let headers = ordered_headers(records);

    let mut widths: HashMap<String, usize> = HashMap::new();
    for h in &headers {
        widths.insert(h.clone(), h.len());
    }
    for r in records {
        for h in &headers {
            let len = r.get(h).unwrap_or("").len();
            widths.entry(h.clone()).and_modify(|w| *w = (*w).max(len));
        }
    }

    let mut out = String::new();
    for h in &headers {
        let w = widths[h];
        out.push_str(&format!("{:width$} ", h, width = w));
    }
    out.push('\n');
    for h in &headers {
        let w = widths[h];
        out.push_str(&format!("{:-<width$} ", "", width = w));
    }
    out.push('\n');
    for r in records {
        for h in &headers {
            let w = widths[h];
            out.push_str(&format!("{:width$} ", r.get(h).unwrap_or(""), width = w));
        }
        out.push('\n');
    }
    out
}

fn escape_markdown_cell(s: &str) -> String {
    s.replace('|', r"\|").replace('\n', "<br>")
}

fn format_markdown(records: &[Record]) -> String {
    if records.is_empty() {
        return String::from("_No records_");
    }

    let headers = ordered_headers(records);
    let mut out = String::new();

    out.push('|');
    for h in &headers {
        out.push(' ');
        out.push_str(&escape_markdown_cell(h));
        out.push(' ');
        out.push('|');
    }
    out.push('\n');

    out.push('|');
    for _ in &headers {
        out.push_str(" --- |");
    }
    out.push('\n');

    for r in records {
        out.push('|');
        for h in &headers {
            out.push(' ');
            out.push_str(&escape_markdown_cell(r.get(h).unwrap_or("")));
            out.push(' ');
            out.push('|');
        }
        out.push('\n');
    }

    out
}

fn format_json<T: Serialize>(value: &T) -> Result<String, CliError> {
    serde_json::to_string_pretty(value).map_err(|e| CliError::Output(e.to_string()))
}

fn format_csv(records: &[Record]) -> Result<String, CliError> {
    let mut wtr = csv::Writer::from_writer(Vec::new());
    if records.is_empty() {
        return Ok(String::new());
    }
    let headers = ordered_headers(records);
    wtr.write_record(&headers)?;
    for r in records {
        let row: Vec<&str> = headers.iter().map(|h| r.get(h).unwrap_or("")).collect();
        wtr.write_record(&row)?;
    }
    let bytes = wtr.into_inner().map_err(|e| CliError::Output(e.to_string()))?;
    String::from_utf8(bytes).map_err(|e| CliError::Output(e.to_string()))
}

fn run(args: Args) -> Result<(), CliError> {
    let mut records = read_csv(&args.input)?;

    let filter = match args.filter {
        Some(ref f) => Some(FilterExpr::parse(f)?),
        None => None,
    };

    records = filter_records(&records, &filter);
    records = select_columns(&records, &args.columns)?;
    if let Some(ref c) = args.sort {
        sort_records(&mut records, c);
    }
    records = limit_records(records, args.limit);
    records = head_records(records, args.head);

    if let Some(group_by) = args.group_by.as_deref() {
        let aggs = aggregate(&records, group_by, &args.aggregate)?;
        match args.format {
            OutputFormat::Json => {
                let s = format_json(&aggs)?;
                println!("{s}");
            }
            OutputFormat::Csv => {
                let mut wtr = csv::Writer::from_writer(io::stdout());
                for a in aggs {
                    wtr.serialize(a)?;
                }
                wtr.flush()?;
            }
            OutputFormat::Table | OutputFormat::Markdown => {
                let s = format_json(&aggs)?;
                println!("{s}");
            }
        }
        return Ok(());
    }

    match args.format {
        OutputFormat::Table => print!("{}", format_table(&records)),
        OutputFormat::Markdown => print!("{}", format_markdown(&records)),
        OutputFormat::Json => println!("{}", format_json(&records)?),
        OutputFormat::Csv => print!("{}", format_csv(&records)?),
    }
    Ok(())
}

fn main() {
    let args = Args::parse();
    if let Err(e) = run(args) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(pairs: &[(&str, &str)]) -> Record {
        Record {
            values: pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    #[test]
    fn parse_filter_eq() {
        let f = FilterExpr::parse(r#"name==\"alice\""#).unwrap();
        assert_eq!(f.column, "name");
    }

    #[test]
    fn filter_matches_numeric() {
        let f = FilterExpr::parse("age>=18").unwrap();
        let r = record(&[("age", "20")]);
        assert!(f.matches(&r));
    }

    #[test]
    fn filter_rejects_missing_column() {
        let f = FilterExpr::parse("age>=18").unwrap();
        let r = record(&[("name", "bob")]);
        assert!(!f.matches(&r));
    }

    #[test]
    fn select_columns_works() {
        let r = record(&[("a", "1"), ("b", "2")]);
        let out = select_columns(&[r], &[String::from("a")]).unwrap();
        assert_eq!(out[0].get("a"), Some("1"));
        assert_eq!(out[0].get("b"), None);
    }

    #[test]
    fn aggregate_sum() {
        let rs = vec![
            record(&[("group", "x"), ("value", "1")]),
            record(&[("group", "x"), ("value", "2")]),
        ];
        let specs = vec![AggregateSpec {
            kind: AggKind::Sum,
            column: "value".into(),
        }];
        let out = aggregate(&rs, "group", &specs).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].value, 3.0);
    }

    #[test]
    fn head_limits_records() {
        let rs = vec![
            record(&[("a", "1")]),
            record(&[("a", "2")]),
            record(&[("a", "3")]),
        ];
        let out = head_records(rs, 2);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].get("a"), Some("1"));
        assert_eq!(out[1].get("a"), Some("2"));
    }

    #[test]
    fn markdown_formatter_generates_github_style_table() {
        let rs = vec![
            record(&[("name", "alice"), ("age", "30")]),
            record(&[("name", "bob"), ("age", "25")]),
        ];
        let out = format_markdown(&rs);
        assert!(out.contains("| age | name |") || out.contains("| name | age |"));
        assert!(out.contains("---"));
        assert!(out.contains("alice"));
        assert!(out.contains("bob"));
    }
}