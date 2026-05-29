<gap:target id="rust-csv-cli-tool">
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, BufWriter};

<gap:target id="error-module">
use thiserror::Error;

<gap:target id="app-error-type">
#[derive(Debug, Error)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Invalid filter expression: {0}")]
    InvalidFilter(String),

    #[error("Unknown column: {0}")]
    UnknownColumn(String),
}
</gap:target>
</gap:target>

<gap:target id="cli-arguments-module">
<gap:target id="cli-args-struct">#[derive(Parser, Debug)]
#[command(name = "csv-tool", version, about = "Process and analyze CSV files")]
pub struct CliArgs {
    #[arg(short, long)]
    pub input: String,

    #[arg(short, long, value_parser = ["table", "json", "csv"], default_value = "table")]
    pub output_format: String,

    #[arg(short = 'c', long, value_delimiter = ',')]
    pub columns: Vec<String>,

    #[arg(short = 'f', long)]
    pub filter: Option<String>,

    #[arg(short = 's', long)]
    pub sort_column: Option<String>,

    #[arg(short = 'l', long)]
    pub limit: Option<usize>,

    #[arg(long, default_value_t = 10)]
    pub head: usize,
}</gap:target>
</gap:target>

<gap:target id="data-models-module">
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub fields: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationResult {
    pub group_key: String,
    pub count: usize,
    pub sum: Option<f64>,
    pub avg: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}
</gap:target>

<gap:target id="filter-expression-module">
#[derive(Debug, Clone)]
pub enum Operator {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    Contains,
}

<gap:target id="filter-ast-struct">
#[derive(Debug, Clone)]
pub struct FilterExpr {
    pub column: String,
    pub op: Operator,
    pub value: String,
}
</gap:target>

<gap:target id="filter-parser-function">
pub fn parse_filter_expr(input: &str) -> Result<FilterExpr, AppError> {
    let ops = [
        (" contains ", Operator::Contains),
        ("==", Operator::Eq),
        ("!=", Operator::Ne),
        (">=", Operator::Ge),
        ("<=", Operator::Le),
        (">", Operator::Gt),
        ("<", Operator::Lt),
    ];

    for (token, op) in ops {
        if let Some((left, right)) = input.split_once(token) {
            let column = left.trim().to_string();
            let value = right.trim().trim_matches('"').trim_matches('\'').to_string();
            if column.is_empty() || value.is_empty() {
                return Err(AppError::InvalidFilter(input.to_string()));
            }
            return Ok(FilterExpr { column, op, value });
        }
    }

    Err(AppError::InvalidFilter(input.to_string()))
}
</gap:target>

<gap:target id="filter-evaluator-function">
pub fn matches_filter(record: &Record, expr: &FilterExpr) -> Result<bool, AppError> {
    let left = record
        .fields
        .get(&expr.column)
        .ok_or_else(|| AppError::UnknownColumn(expr.column.clone()))?;

    let result = match expr.op {
        Operator::Eq => left == &expr.value,
        Operator::Ne => left != &expr.value,
        Operator::Contains => left.contains(&expr.value),
        Operator::Gt | Operator::Ge | Operator::Lt | Operator::Le => {
            let lhs = left.parse::<f64>().map_err(|_| AppError::Parse(left.clone()))?;
            let rhs = expr.value.parse::<f64>().map_err(|_| AppError::Parse(expr.value.clone()))?;
            match expr.op {
                Operator::Gt => lhs > rhs,
                Operator::Ge => lhs >= rhs,
                Operator::Lt => lhs < rhs,
                Operator::Le => lhs <= rhs,
                _ => unreachable!(),
            }
        }
    };

    Ok(result)
}
</gap:target>
</gap:target>

<gap:target id="csv-processor-module">
pub struct CsvProcessor {
    pub headers: Vec<String>,
    pub records: Vec<Record>,
}

<gap:target id="csv-processor-impl">
impl CsvProcessor {
    pub fn from_path(path: &str) -> Result<Self, AppError> {
        let file = File::open(path)?;
        let mut rdr = csv::Reader::from_reader(BufReader::new(file));

        let headers = rdr
            .headers()?
            .iter()
            .map(|h| h.to_string())
            .collect::<Vec<_>>();

        let mut records = Vec::new();
        for row in rdr.records() {
            let row = row?;
            let mut fields = HashMap::new();
            for (h, v) in headers.iter().zip(row.iter()) {
                fields.insert(h.clone(), v.to_string());
            }
            records.push(Record { fields });
        }

        Ok(Self { headers, records })
    }

    pub fn select_columns(&self, columns: &[String]) -> Result<Self, AppError> {
        if columns.is_empty() {
            return Ok(Self {
                headers: self.headers.clone(),
                records: self.records.clone(),
            });
        }

        for col in columns {
            if !self.headers.contains(col) {
                return Err(AppError::UnknownColumn(col.clone()));
            }
        }

        let records = self
            .records
            .iter()
            .map(|rec| {
                let fields = columns
                    .iter()
                    .filter_map(|c| rec.fields.get(c).map(|v| (c.clone(), v.clone())))
                    .collect::<HashMap<_, _>>();
                Record { fields }
            })
            .collect::<Vec<_>>();

        Ok(Self {
            headers: columns.to_vec(),
            records,
        })
    }

    pub fn filter(&self, expr: &FilterExpr) -> Result<Self, AppError> {
        let mut records = Vec::new();
        for rec in &self.records {
            if matches_filter(rec, expr)? {
                records.push(rec.clone());
            }
        }
        Ok(Self {
            headers: self.headers.clone(),
            records,
        })
    }

    pub fn sort_by_column(&mut self, column: &str) -> Result<(), AppError> {
        if !self.headers.contains(&column.to_string()) {
            return Err(AppError::UnknownColumn(column.to_string()));
        }

        self.records.sort_by(|a, b| {
            a.fields
                .get(column)
                .cmp(&b.fields.get(column))
        });

        Ok(())
    }

    pub fn limit(&self, n: usize) -> Self {
        let records = self.records.iter().take(n).cloned().collect::<Vec<_>>();
        Self {
            headers: self.headers.clone(),
            records,
        }
    }

    pub fn aggregate_by_group(
        &self,
        group_column: &str,
        value_column: &str,
    ) -> Result<Vec<AggregationResult>, AppError> {
        if !self.headers.contains(&group_column.to_string()) {
            return Err(AppError::UnknownColumn(group_column.to_string()));
        }
        if !self.headers.contains(&value_column.to_string()) {
            return Err(AppError::UnknownColumn(value_column.to_string()));
        }

        let mut map: HashMap<String, Vec<f64>> = HashMap::new();
        for rec in &self.records {
            let group_key = rec.fields.get(group_column).cloned().unwrap_or_default();
            let value = rec
                .fields
                .get(value_column)
                .ok_or_else(|| AppError::UnknownColumn(value_column.to_string()))?
                .parse::<f64>()
                .map_err(|_| AppError::Parse(value_column.to_string()))?;
            map.entry(group_key).or_default().push(value);
        }

        let mut out = Vec::new();
        for (group_key, values) in map {
            let count = values.len();
            let sum: f64 = values.iter().sum();
            let avg = if count == 0 { None } else { Some(sum / count as f64) };
            let min = values.iter().cloned().reduce(f64::min);
            let max = values.iter().cloned().reduce(f64::max);
            out.push(AggregationResult {
                group_key,
                count,
                sum: Some(sum),
                avg,
                min,
                max,
            });
        }

        Ok(out)
    }
}
</gap:target>
</gap:target>

<gap:target id="output-formatters-module">
pub fn print_table(records: &[Record], headers: &[String]) {
    let widths = headers
        .iter()
        .map(|h| {
            let max_value = records
                .iter()
                .filter_map(|r| r.fields.get(h))
                .map(|v| v.len())
                .max()
                .unwrap_or(0);
            h.len().max(max_value)
        })
        .collect::<Vec<_>>();

    let sep = widths
        .iter()
        .map(|w| "-".repeat(*w + 2))
        .collect::<Vec<_>>()
        .join("+");

    println!("{}", sep);
    for (i, h) in headers.iter().enumerate() {
        print!(" {:width$} ", h, width = widths[i]);
        if i + 1 != headers.len() {
            print!("|");
        }
    }
    println!();
    println!("{}", sep);

    for rec in records {
        for (i, h) in headers.iter().enumerate() {
            let v = rec.fields.get(h).cloned().unwrap_or_default();
            print!(" {:width$} ", v, width = widths[i]);
            if i + 1 != headers.len() {
                print!("|");
            }
        }
        println!();
    }
}

pub fn print_json(records: &[Record]) -> Result<(), AppError> {
    let stdout = io::stdout();
    let writer = BufWriter::new(stdout.lock());
    serde_json::to_writer_pretty(writer, records)?;
    Ok(())
}

pub fn print_csv(records: &[Record], headers: &[String]) -> Result<(), AppError> {
    let stdout = io::stdout();
    let mut wtr = csv::Writer::from_writer(BufWriter::new(stdout.lock()));
    wtr.write_record(headers)?;
    for rec in records {
        let row = headers
            .iter()
            .map(|h| rec.fields.get(h).cloned().unwrap_or_default())
            .collect::<Vec<_>>();
        wtr.write_record(row)?;
    }
    wtr.flush()?;
    Ok(())
}
</gap:target>

<gap:target id="main-function-module">    if let Some(limit) = args.limit {
        processor = processor.limit(limit);
    }

    processor = processor.limit(args.head);

    match args.output_format.as_str() {</gap:target>

<gap:target id="unit-tests-module">
#[cfg(test)]
mod tests {
    use super::*;

    <gap:target id="filter-parser-tests">
    #[test]
    fn parses_equality_filter() {
        let expr = parse_filter_expr("age == 30").unwrap();
        assert_eq!(expr.column, "age");
        assert!(matches!(expr.op, Operator::Eq));
        assert_eq!(expr.value, "30");
    }

    #[test]
    fn parses_contains_filter() {
        let expr = parse_filter_expr("name contains \"John\"").unwrap();
        assert_eq!(expr.column, "name");
        assert!(matches!(expr.op, Operator::Contains));
        assert_eq!(expr.value, "John");
    }
    </gap:target>

    <gap:target id="filter-evaluator-tests">
    #[test]
    fn evaluates_numeric_filter() {
        let mut fields = HashMap::new();
        fields.insert("age".to_string(), "35".to_string());
        let record = Record { fields };
        let expr = FilterExpr {
            column: "age".to_string(),
            op: Operator::Gt,
            value: "30".to_string(),
        };

        assert_eq!(matches_filter(&record, &expr).unwrap(), true);
    }

    #[test]
    fn evaluates_string_filter() {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), "Alice".to_string());
        let record = Record { fields };
        let expr = FilterExpr {
            column: "name".to_string(),
            op: Operator::Eq,
            value: "Alice".to_string(),
        };

        assert_eq!(matches_filter(&record, &expr).unwrap(), true);
    }
    </gap:target>
}
</gap:target>
</gap:target>