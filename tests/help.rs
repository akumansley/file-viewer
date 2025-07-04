use rexpect::spawn;

#[test]
fn shows_help_when_no_file_provided() -> anyhow::Result<()> {
    let mut p = spawn("target/debug/file-viewer --headless", Some(30000))?;
    p.exp_string("Usage: file-viewer")?;
    Ok(())
}
