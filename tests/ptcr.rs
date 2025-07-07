#[path = "../src/ptcr.rs"]
mod ptcr;
use std::path::PathBuf;

#[test]
fn parse_example() {
    let input = "file.txt:1\nhello";
    let recs = ptcr::parse(input).unwrap();
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].path, PathBuf::from("file.txt"));
    assert_eq!(recs[0].body, vec!["hello".to_string()]);
}
