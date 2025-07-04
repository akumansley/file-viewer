use rexpect::spawn;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_interactive_hello_world() -> anyhow::Result<()> {
    let mut file = NamedTempFile::new()?;
    writeln!(file, "hello world")?;

    let mut p = spawn(
        &format!("target/debug/file-viewer {}", file.path().display()),
        Some(5_000),
    )?;

    // Give the application a moment to render.
    std::thread::sleep(std::time::Duration::from_millis(200));
    p.send("q")?;
    p.flush()?;
    p.exp_eof()?;

    Ok(())
}
