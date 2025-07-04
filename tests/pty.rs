use rexpect::spawn;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_interactive_hello_world() -> anyhow::Result<()> {
    let mut file = NamedTempFile::new()?;
    writeln!(file, "hello world")?;

    let mut p = spawn(
        &format!(
            "target/debug/file-viewer {} --headless",
            file.path().to_str().unwrap()
        ),
        Some(5000),
    )?;

    // Verify the output shows the file contents.
    p.exp_regex("hello world")?;
    p.exp_eof()?;

    Ok(())
}
