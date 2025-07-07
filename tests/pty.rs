use rexpect::spawn;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
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

#[test]
fn test_interactive_command_q() -> anyhow::Result<()> {
    let mut file = NamedTempFile::new()?;
    writeln!(file, "hello world")?;

    let mut p = spawn(
        &format!("target/debug/file-viewer {}", file.path().display()),
        Some(5_000),
    )?;

    // Wait for app to render
    std::thread::sleep(std::time::Duration::from_millis(200));
    p.send(":q\r")?; // send colon, q, and Enter
    p.flush()?;
    p.exp_eof()?;

    Ok(())
}

#[test]
fn test_custom_command_execution() -> anyhow::Result<()> {
    let mut file = NamedTempFile::new()?;
    writeln!(file, "hello world")?;

    let out = NamedTempFile::new()?;

    let mut script = NamedTempFile::new()?;
    writeln!(script, "#!/bin/sh")?;
    writeln!(script, "echo \"$@\" >> {}", out.path().display())?;
    script.flush()?;
    let script_path = script.into_temp_path();
    let mut perms = std::fs::metadata(&script_path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script_path, perms)?;

    let cmd_spec = format!("foo:{} {{line}} {{col}} {{args}}", script_path.display());
    eprintln!("cmd_spec={}", cmd_spec);

    let mut p = spawn(
        &format!(
            "target/debug/file-viewer {} --command \"{}\"",
            file.path().display(),
            cmd_spec
        ),
        Some(5_000),
    )?;

    std::thread::sleep(std::time::Duration::from_millis(200));
    p.send(":foo test-arg\r")?;
    p.flush()?;
    // wait for command to run
    std::thread::sleep(std::time::Duration::from_millis(500));
    p.send("q")?;
    p.flush()?;
    p.exp_eof()?;

    let output = std::fs::read_to_string(out.path())?;
    eprintln!("output read: {}", output.trim());
    assert_eq!(output.trim(), "1 1 test-arg");

    Ok(())
}
