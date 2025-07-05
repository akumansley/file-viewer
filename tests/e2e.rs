use rexpect::spawn;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_e2e() -> anyhow::Result<()> {
    let mut file = NamedTempFile::new()?;
    writeln!(file, "hello world")?;

    let mut p = spawn(
        &format!(
            "target/debug/file-viewer {} --headless",
            file.path().to_str().unwrap()
        ),
        Some(30000),
    )?;

    p.exp_string("hello world")?;

    Ok(())
}
