use anyhow::{bail, ensure, Result};
use std::path::{Path, PathBuf};

pub fn protocol(uri: &str) -> Result<&'static str> {
    // Preserve the original payload byte-for-byte after checking its envelope.
    // Never include payloads in errors: website joins may contain authentication tickets.
    ensure!(
        uri.len() <= 30000 && !uri.is_empty(),
        "Launch link has an invalid length"
    );
    ensure!(
        !uri.chars()
            .any(|c| c.is_control() || c.is_whitespace() || c == '"'),
        "Launch link contains invalid characters"
    );
    let (scheme, payload) = uri
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("Launch link is missing a protocol"))?;
    ensure!(!payload.is_empty(), "Launch link has no payload");
    if scheme.eq_ignore_ascii_case("ferns2026") {
        Ok("ferns2026")
    } else if scheme.eq_ignore_ascii_case("ferns-launcher") {
        Ok("ferns-launcher")
    } else {
        bail!("Only Ferns launch links are supported")
    }
}

pub fn parse_original(command: &str) -> Result<PathBuf> {
    let rest = command.strip_prefix('"').ok_or_else(|| {
        anyhow::anyhow!("Original launcher command must have a quoted executable path")
    })?;
    let (target, args) = rest
        .split_once('"')
        .ok_or_else(|| anyhow::anyhow!("Original launcher command is incomplete"))?;
    ensure!(
        matches!(args.trim(), "\"%1\"" | "%1"),
        "Original launcher uses an unsupported argument format; integration was not changed"
    );
    let path = PathBuf::from(target);
    ensure!(
        path.is_absolute()
            && path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe")),
        "Original launcher is not an absolute executable path"
    );
    Ok(path)
}

pub fn handler_command(executable: &Path) -> Result<String> {
    let text = executable
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Launcher path is not valid Unicode"))?;
    ensure!(
        executable.is_absolute() && !text.contains(['"', '\r', '\n']),
        "Launcher path is invalid"
    );
    Ok(format!("\"{text}\" \"%1\""))
}

pub fn same_file_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_envelope_without_exposing_tickets() {
        assert_eq!(
            protocol("FERNS2026:1+ticket:secret+place:5").unwrap(),
            "ferns2026"
        );
        for uri in [
            "https://example.com/secret",
            "ferns2026:",
            "ferns2026:secret\" --flag",
            "ferns2026:secret\n",
        ] {
            let error = protocol(uri).unwrap_err().to_string();
            assert!(!error.contains("secret"));
        }
    }

    #[test]
    fn preserves_encoded_payload() {
        let uri = "ferns-launcher://join?ticket=a%22b%20c&place=2";
        assert_eq!(protocol(uri).unwrap(), "ferns-launcher");
    }

    #[test]
    fn rejects_overlong_payload() {
        assert!(protocol(&format!("ferns2026:{}", "a".repeat(30000))).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn only_accepts_known_launcher_argument_shape() {
        assert_eq!(
            parse_original("\"C:\\Ferns\\FernsLauncher.exe\" \"%1\"").unwrap(),
            PathBuf::from("C:\\Ferns\\FernsLauncher.exe")
        );
        assert!(parse_original("\"C:\\Ferns\\FernsLauncher.exe\" --extra %1").is_err());
        assert!(parse_original("cmd /c %1").is_err());
    }
}
