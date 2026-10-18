use anyhow::{Context, Result};
use serde::Serialize;
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::SystemTime,
};

const SECTION_BYTES: u64 = 256 * 1024;
const MAX_LOGS: usize = 4;

#[derive(Debug, Serialize)]
pub struct LogEvidence {
    pub file_name: String,
    pub bytes: u64,
    pub inspected_bytes: usize,
    pub partial: bool,
    pub modified_unix_seconds: Option<u64>,
    pub core_debug_markers: usize,
    pub script_started_marker: bool,
    pub chrome_require_succeeded: bool,
    pub chrome_require_failed: bool,
    pub probe_started: bool,
    pub probe_write_succeeded: bool,
    pub probe_write_failed: bool,
    pub exception_markers: usize,
    pub access_violation_markers: usize,
    pub out_of_memory_markers: usize,
    pub dns_failure_markers: usize,
    pub tls_failure_markers: usize,
    pub connection_failure_markers: usize,
}

#[derive(Debug, Serialize)]
pub struct CoreGuiReport {
    pub script_path: PathBuf,
    pub loose_script_present: bool,
    pub log_folder: PathBuf,
    pub logs: Vec<LogEvidence>,
    pub scan_errors: usize,
}

pub fn inspect_coregui(root: &Path) -> CoreGuiReport {
    let script_path =
        root.join("Player/content/scripts/CoreScripts/PerformanceStatsManagerScript.lua");
    let log_folder = root.join("Roblox/logs");
    let mut report = CoreGuiReport {
        loose_script_present: script_path.is_file(),
        script_path,
        log_folder,
        logs: Vec::new(),
        scan_errors: 0,
    };
    let entries = match fs::read_dir(&report.log_folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return report,
        Err(_) => {
            report.scan_errors += 1;
            return report;
        }
    };
    let mut files: Vec<_> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let kind = entry.file_type().ok()?;
            if !kind.is_file() || kind.is_symlink() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.contains("_Player_") || !name.ends_with(".log") {
                return None;
            }
            let metadata = entry.metadata().ok()?;
            Some((
                metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                entry.path(),
            ))
        })
        .collect();
    files.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    for (_, path) in files.into_iter().take(MAX_LOGS) {
        match inspect_log(&path) {
            Ok(log) => report.logs.push(log),
            Err(_) => report.scan_errors += 1,
        }
    }
    report
}

fn inspect_log(path: &Path) -> Result<LogEvidence> {
    let mut file = File::open(path).context("Cannot open player log")?;
    let metadata = file.metadata()?;
    let bytes = metadata.len();
    let mut sections = Vec::new();
    let partial = bytes > SECTION_BYTES * 2;
    if partial {
        let mut head = Vec::new();
        (&mut file).take(SECTION_BYTES).read_to_end(&mut head)?;
        // Keep complete lines only. A marker split at a boundary is not valid evidence.
        if let Some(end) = head.iter().rposition(|byte| *byte == b'\n') {
            head.truncate(end + 1);
        } else {
            head.clear();
        }
        sections.push(head);
        file.seek(SeekFrom::Start(bytes - SECTION_BYTES))?;
        let mut tail = Vec::new();
        (&mut file).take(SECTION_BYTES).read_to_end(&mut tail)?;
        if let Some(start) = tail.iter().position(|byte| *byte == b'\n') {
            tail.drain(..=start);
        } else {
            tail.clear();
        }
        sections.push(tail);
    } else {
        let mut contents = Vec::new();
        (&mut file)
            .take(SECTION_BYTES * 2)
            .read_to_end(&mut contents)?;
        sections.push(contents);
    }
    let mut evidence = LogEvidence {
        file_name: path
            .file_name()
            .context("Log has no filename")?
            .to_string_lossy()
            .into_owned(),
        bytes,
        inspected_bytes: sections.iter().map(Vec::len).sum(),
        partial,
        modified_unix_seconds: metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs()),
        core_debug_markers: 0,
        script_started_marker: false,
        chrome_require_succeeded: false,
        chrome_require_failed: false,
        probe_started: false,
        probe_write_succeeded: false,
        probe_write_failed: false,
        exception_markers: 0,
        access_violation_markers: 0,
        out_of_memory_markers: 0,
        dns_failure_markers: 0,
        tls_failure_markers: 0,
        connection_failure_markers: 0,
    };
    for section in sections {
        scan_markers(&String::from_utf8_lossy(&section), &mut evidence);
    }
    Ok(evidence)
}

fn scan_markers(text: &str, evidence: &mut LogEvidence) {
    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("exception_access_violation") || lower.contains("0xc0000005") {
            evidence.access_violation_markers += 1;
        }
        if lower.contains("std::bad_alloc")
            || lower.contains("outofmemoryexception")
            || lower.contains("out of memory")
        {
            evidence.out_of_memory_markers += 1;
        }
        if lower.contains("curle_couldnt_resolve_host")
            || lower.contains("could not resolve host")
            || lower.contains("getaddrinfo failed")
        {
            evidence.dns_failure_markers += 1;
        }
        if lower.contains("certificate_verify_failed")
            || lower.contains("certificate verify failed")
            || lower.contains("certificate has expired")
        {
            evidence.tls_failure_markers += 1;
        }
        if lower.contains("curle_couldnt_connect")
            || lower.contains("connection timed out")
            || lower.contains("connection refused")
        {
            evidence.connection_failure_markers += 1;
        }
        if let Some((_, marker)) = line.split_once("[SOLSTRAP_PROBE_V1]") {
            evidence.probe_started |= marker.trim() == "started";
            if let Some((_, value)) = marker.split_once("CoreGui write ok=") {
                let first = value.split_whitespace().next().unwrap_or_default();
                evidence.probe_write_succeeded |= first == "true";
                evidence.probe_write_failed |= first == "false";
            }
        }
        if let Some((_, marker)) = line.split_once("[COREDBG]") {
            evidence.core_debug_markers += 1;
            evidence.script_started_marker |= marker.contains("loose CoreScript RUNNING");
            if let Some((_, value)) = marker.split_once("require Chrome ok=") {
                let first = value.split_whitespace().next().unwrap_or_default();
                evidence.chrome_require_succeeded |= first == "true";
                evidence.chrome_require_failed |= first == "false";
            }
        }
        if line.contains("Unhandled exception") || line.contains("EXCEPTION_ACCESS_VIOLATION") {
            evidence.exception_markers += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log_file(root: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let folder = root.join("Roblox/logs");
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn missing_logs_are_not_claimed_as_script_failure() {
        let temp = tempfile::tempdir().unwrap();
        let report = inspect_coregui(temp.path());
        assert!(report.logs.is_empty());
        assert_eq!(report.scan_errors, 0);
        assert!(!report.loose_script_present);
    }

    #[test]
    fn extracts_only_structured_evidence_and_ignores_other_logs() {
        let temp = tempfile::tempdir().unwrap();
        log_file(temp.path(), "version_Player_A_last.log", b"ticket=private-secret\n[COREDBG] loose CoreScript RUNNING at CoreScript security\n[COREDBG] require Chrome ok= true err/val= table\n");
        log_file(
            temp.path(),
            "launcher.log",
            b"[COREDBG] require Chrome ok= false\n",
        );
        let report = inspect_coregui(temp.path());
        assert_eq!(report.logs.len(), 1);
        assert!(report.logs[0].script_started_marker);
        assert!(report.logs[0].chrome_require_succeeded);
        assert!(!report.logs[0].chrome_require_failed);
        let exported = serde_json::to_string(&report).unwrap();
        assert!(!exported.contains("private-secret"));
    }

    #[test]
    fn inspects_large_log_head_and_tail_without_merging_boundary_fragments() {
        let temp = tempfile::tempdir().unwrap();
        let mut content = b"[COREDBG] loose CoreScript RUNNING\n".to_vec();
        content.extend(vec![b'x'; SECTION_BYTES as usize * 3]);
        content.extend_from_slice(b"\nEXCEPTION_ACCESS_VIOLATION\n[COREDBG] require Chrome ok= false err/val= unavailable\n");
        let path = log_file(temp.path(), "version_Player_A_last.log", &content);
        let result = inspect_log(&path).unwrap();
        assert!(result.partial);
        assert!(result.inspected_bytes <= SECTION_BYTES as usize * 2);
        assert!(result.script_started_marker);
        assert!(result.chrome_require_failed);
        assert_eq!(result.exception_markers, 1);
    }

    #[test]
    fn probe_status_needs_an_explicit_boolean_result() {
        let temp = tempfile::tempdir().unwrap();
        let path = log_file(temp.path(), "version_Player_B_last.log", b"[SOLSTRAP_PROBE_V1] started\n[SOLSTRAP_PROBE_V1] CoreGui write ok= true\n[COREDBG] unrelated true\n");
        let result = inspect_log(&path).unwrap();
        assert!(result.probe_started && result.probe_write_succeeded);
        assert!(!result.probe_write_failed);
        assert!(!result.chrome_require_succeeded);
    }

    #[test]
    fn failure_clues_are_separate_and_raw_text_is_not_exported() {
        let temp = tempfile::tempdir().unwrap();
        let path = log_file(temp.path(), "version_Player_C_last.log", b"EXCEPTION_ACCESS_VIOLATION 0xc0000005\nstd::bad_alloc\nCURLE_COULDNT_RESOLVE_HOST ticket=private-secret\nCERTIFICATE_VERIFY_FAILED\nconnection timed out\n");
        let result = inspect_log(&path).unwrap();
        assert_eq!(result.access_violation_markers, 1);
        assert_eq!(result.out_of_memory_markers, 1);
        assert_eq!(result.dns_failure_markers, 1);
        assert_eq!(result.tls_failure_markers, 1);
        assert_eq!(result.connection_failure_markers, 1);
        assert!(!serde_json::to_string(&result)
            .unwrap()
            .contains("private-secret"));
    }

    #[test]
    fn generic_connection_and_memory_words_are_not_failure_markers() {
        let temp = tempfile::tempdir().unwrap();
        let path = log_file(temp.path(), "version_Player_D_last.log", b"memory usage: 1500 MB\nconnection established\ncertificate loaded\ntimeout configured: 30\n");
        let result = inspect_log(&path).unwrap();
        assert_eq!(
            result.access_violation_markers
                + result.out_of_memory_markers
                + result.dns_failure_markers
                + result.tls_failure_markers
                + result.connection_failure_markers,
            0
        );
    }
}
