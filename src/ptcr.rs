use anyhow::{Result, anyhow};
use regex::Regex;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Span {
    File,
    Line(usize),
    Point {
        line: usize,
        col: usize,
    },
    LineRange {
        start: usize,
        end: usize,
    },
    ColumnRange {
        line: usize,
        start_col: usize,
        end_col: usize,
    },
    MultiLine {
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub path: PathBuf,
    pub span: Span,
    pub body: Vec<String>,
}

fn header_regex() -> &'static Regex {
    static REGEX: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
        Regex::new(r"^([^\s:]+):(\*|(\d+)(?:\.(\d+))?(?:-(\d+)(?:\.(\d+))?)?)$").unwrap()
    });
    &REGEX
}

pub fn parse(input: &str) -> Result<Vec<Record>> {
    let mut records = Vec::new();
    let mut current: Option<Record> = None;

    for line in input.lines() {
        if let Some(caps) = header_regex().captures(line) {
            if let Some(rec) = current.take() {
                records.push(rec);
            }
            let path = caps.get(1).unwrap().as_str();
            let span = if caps.get(3).is_none() {
                Span::File
            } else {
                let start_line: usize = caps.get(3).unwrap().as_str().parse()?;
                let start_col = caps.get(4).map(|m| m.as_str().parse::<usize>().unwrap());
                match caps.get(5) {
                    None => {
                        if let Some(col) = start_col {
                            Span::Point {
                                line: start_line,
                                col,
                            }
                        } else {
                            Span::Line(start_line)
                        }
                    }
                    Some(end_line_match) => {
                        let end_line: usize = end_line_match.as_str().parse()?;
                        let end_col = caps.get(6).map(|m| m.as_str().parse::<usize>().unwrap());
                        match (start_col, end_col) {
                            (None, None) => Span::LineRange {
                                start: start_line,
                                end: end_line,
                            },
                            (Some(sc), Some(ec)) => {
                                if start_line == end_line {
                                    Span::ColumnRange {
                                        line: start_line,
                                        start_col: sc,
                                        end_col: ec,
                                    }
                                } else {
                                    Span::MultiLine {
                                        start_line,
                                        start_col: sc,
                                        end_line,
                                        end_col: ec,
                                    }
                                }
                            }
                            _ => return Err(anyhow!("invalid span")),
                        }
                    }
                }
            };
            current = Some(Record {
                path: PathBuf::from(path),
                span,
                body: Vec::new(),
            });
        } else if let Some(rec) = current.as_mut() {
            rec.body.push(line.to_string());
        }
    }
    if let Some(rec) = current {
        records.push(rec);
    }
    Ok(records)
}

#[allow(dead_code)]
pub fn read_file(path: &Path) -> Result<Vec<Record>> {
    let mut s = String::new();
    File::open(path)?.read_to_string(&mut s)?;
    parse(&s)
}

#[allow(dead_code)]
fn span_to_string(span: &Span) -> String {
    match span {
        Span::File => "*".to_string(),
        Span::Line(l) => format!("{}", l),
        Span::Point { line, col } => format!("{}.{}", line, col),
        Span::LineRange { start, end } => format!("{}-{}", start, end),
        Span::ColumnRange {
            line,
            start_col,
            end_col,
        } => {
            format!("{}.{}-{}.{}", line, start_col, line, end_col)
        }
        Span::MultiLine {
            start_line,
            start_col,
            end_line,
            end_col,
        } => format!("{}.{}-{}.{}", start_line, start_col, end_line, end_col),
    }
}

#[allow(dead_code)]
pub fn write_file(path: &Path, records: &[Record]) -> Result<()> {
    let mut f = File::create(path)?;
    for (i, rec) in records.iter().enumerate() {
        if i > 0 {
            writeln!(f)?;
        }
        writeln!(f, "{}:{}", rec.path.display(), span_to_string(&rec.span))?;
        for line in &rec.body {
            writeln!(f, "{}", line)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_record() {
        let input = "src/main.rs:1\nhello";
        let recs = parse(input).unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].path, PathBuf::from("src/main.rs"));
        assert_eq!(recs[0].span, Span::Line(1));
        assert_eq!(recs[0].body, vec!["hello".to_string()]);
    }
}
