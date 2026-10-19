use crate::{diagnostics::CoreGuiReport, discovery::Installation};
#[cfg(any(windows, test))]
use anyhow::Context;
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct Detection {
    pub time_utc: String,
    pub event_id: u32,
    pub threat: String,
    pub action: String,
}

#[derive(Debug, Serialize)]
pub struct DefenderReport {
    pub readable: bool,
    pub limited: bool,
    pub error: Option<String>,
    pub events: Vec<Detection>,
}

#[derive(Debug, Serialize)]
pub struct Finding {
    pub title: String,
    pub evidence: String,
    pub next_step: String,
}

#[derive(Debug, Serialize)]
pub struct HealthReport {
    pub defender: DefenderReport,
    pub startup: Vec<Finding>,
    pub session: Vec<Finding>,
}

pub fn inspect(installation: &Installation, logs: &CoreGuiReport) -> HealthReport {
    let target = installation.root.join("Player/FernsLauncher.exe");
    let defender = match read_defender(&target) {
        Ok(report) => report,
        Err(error) => DefenderReport {
            readable: false,
            limited: false,
            error: Some(format!("{error:#}")),
            events: Vec::new(),
        },
    };
    let startup = startup_findings(installation, &defender);
    let session = session_findings(logs);
    HealthReport {
        defender,
        startup,
        session,
    }
}

fn startup_findings(installation: &Installation, defender: &DefenderReport) -> Vec<Finding> {
    let mut findings = Vec::new();
    if installation.android_guest_library {
        findings.push(Finding {
            title: "Android guest library detected".into(),
            evidence: "This installation contains an ELF libroblox.so in the Android app directory. Diagnostics reads the compatibility host's client.log; Windows exit monitoring describes the host process only.".into(),
            next_step: "Treat previous Windows-client shader and CoreGui tests as unverified for this build. A guest failure can occur while the Windows host remains open; compare current log evidence with the host exit report.".into(),
        });
    }
    if !installation.detected {
        findings.push(Finding { title: "Game executable is missing".into(), evidence: "The selected folder contains neither Player/Ferns.exe nor Player/RobloxPlayerBeta.exe.".into(), next_step: "Check the selected installation folder. Website integration needs a complete client installation.".into() });
    }
    for launcher in installation
        .launchers
        .iter()
        .filter(|launcher| !launcher.exists)
    {
        let recent = if launcher.protocol == "ferns2026" {
            defender.events.iter().find(|event| {
                event.event_id == 1117 && event.action.eq_ignore_ascii_case("Quarantine")
            })
        } else {
            None
        };
        if let Some(event) = recent {
            findings.push(Finding {
                title: "Player launcher missing after a recorded quarantine".into(),
                evidence: format!("Defender recorded {} with action {} at {}. The player launcher is currently absent.", event.threat, event.action, event.time_utc),
                next_step: "Ask the Ferns developer to review the detection and provide a verified build. Reinstalling the same package may repeat the removal. SOLstrap does not restore quarantined files or change protection settings.".into(),
            });
        } else {
            findings.push(Finding { title: format!("{} launcher is missing", launcher.protocol), evidence: "The original launcher required by this website route is absent.".into(), next_step: "Confirm the client location and obtain the original launcher from the Ferns developer. A missing file alone does not establish why it disappeared.".into() });
        }
    }
    findings
}

fn session_findings(report: &CoreGuiReport) -> Vec<Finding> {
    let mut findings = Vec::new();
    for log in &report.logs {
        let categories = [
            (log.access_violation_markers, "Access-violation evidence", "The log contains an explicit access-violation marker. This does not identify the faulty component.", "Record the client build and recent changes for the Ferns developer. Use original client files when reproducing after the launcher is reviewed."),
            (log.out_of_memory_markers, "Allocation-failure evidence", "The log contains a recognized out-of-memory or allocation-failure marker.", "Record available memory and the experience that was running. This is evidence to investigate, not proof of a memory leak."),
            (log.dns_failure_markers, "Hostname-resolution evidence", "The log contains a recognized DNS/hostname-resolution failure marker.", "Check whether the Ferns service hostname resolves and whether other users can reach it. A local log cannot establish a server-wide outage."),
            (log.tls_failure_markers, "Certificate-validation evidence", "The log contains a recognized TLS certificate-validation failure marker.", "Check system time and ask the Ferns operator to review the affected service certificate. Do not disable certificate verification as a fix."),
            (log.connection_failure_markers, "Connection-failure evidence", "The log contains a recognized connection failure or timeout marker.", "Compare the time with Ferns service status and other network failures. This does not by itself distinguish a local problem from a server problem."),
        ];
        for (count, title, explanation, next_step) in categories {
            if count > 0 {
                findings.push(Finding {
                    title: title.into(),
                    evidence: format!(
                        "{}: {} matching line(s). {}{}",
                        log.file_name,
                        count,
                        explanation,
                        if log.partial {
                            " Only the log head and tail were inspected."
                        } else {
                            ""
                        }
                    ),
                    next_step: next_step.into(),
                });
            }
        }
        if log.exception_markers > 0 && log.access_violation_markers == 0 {
            findings.push(Finding { title: "Exception evidence".into(), evidence: format!("{} contains {} exception marker(s). No specific cause was identified.", log.file_name, log.exception_markers), next_step: "Preserve the original log locally and record the last action before the failure. An exception marker alone is not proof of a crash or its cause.".into() });
        }
    }
    findings
}

#[cfg(any(windows, test))]
fn parse_detection(xml: &str, target: &Path) -> Result<Option<Detection>> {
    let doc = roxmltree::Document::parse(xml).context("Could not parse a Defender event")?;
    let data = |name: &str| {
        doc.descendants()
            .find(|node| node.has_tag_name("Data") && node.attribute("Name") == Some(name))
            .and_then(|node| node.text())
            .unwrap_or_default()
    };
    let normalized = target.to_string_lossy().replace('/', "\\").to_lowercase();
    let matching = data("Path").split(';').any(|entry| {
        let entry = entry.trim().replace('/', "\\").to_lowercase();
        let Some(file) = entry.strip_prefix("file:_") else {
            return false;
        };
        file == normalized || file == format!("{normalized}.new")
    });
    if !matching {
        return Ok(None);
    }
    let event_id = doc
        .descendants()
        .find(|node| node.has_tag_name("EventID"))
        .and_then(|node| node.text())
        .and_then(|text| text.parse().ok())
        .unwrap_or(0);
    if !matches!(event_id, 1116 | 1117) {
        return Ok(None);
    }
    let clean = |text: &str| {
        text.chars()
            .filter(|character| !character.is_control())
            .take(180)
            .collect::<String>()
    };
    let time_utc = doc
        .descendants()
        .find(|node| node.has_tag_name("TimeCreated"))
        .and_then(|node| node.attribute("SystemTime"))
        .map(clean)
        .unwrap_or_default();
    Ok(Some(Detection {
        time_utc,
        event_id,
        threat: clean(data("Threat Name")),
        action: clean(data("Action Name")),
    }))
}

#[cfg(windows)]
fn read_defender(target: &Path) -> Result<DefenderReport> {
    use anyhow::ensure;
    use windows_sys::Win32::{
        Foundation::{GetLastError, ERROR_INSUFFICIENT_BUFFER, ERROR_NO_MORE_ITEMS},
        System::EventLog::*,
    };
    struct Handle(EVT_HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                EvtClose(self.0);
            }
        }
    }
    let wide = |value: &str| value.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let channel = wide("Microsoft-Windows-Windows Defender/Operational");
    let query = wide("*[System[(EventID=1116 or EventID=1117) and TimeCreated[timediff(@SystemTime) <= 604800000]]]");
    // Native read-only event-log access; no child shell, service changes, or security settings.
    let raw = unsafe {
        EvtQuery(
            0,
            channel.as_ptr(),
            query.as_ptr(),
            EvtQueryChannelPath | EvtQueryReverseDirection,
        )
    };
    ensure!(
        raw != 0,
        "Cannot read Defender history: {}",
        std::io::Error::last_os_error()
    );
    let result = Handle(raw);
    let mut events = Vec::new();
    let mut limited = true;
    for _ in 0..128 {
        let mut event = 0;
        let mut returned = 0;
        let ok = unsafe { EvtNext(result.0, 1, &mut event, 0, 0, &mut returned) };
        if ok == 0 {
            if unsafe { GetLastError() } == ERROR_NO_MORE_ITEMS {
                limited = false;
                break;
            }
            anyhow::bail!(
                "Cannot enumerate Defender history: {}",
                std::io::Error::last_os_error()
            );
        }
        let event = Handle(event);
        let mut required = 0;
        let mut properties = 0;
        let ok = unsafe {
            EvtRender(
                0,
                event.0,
                EvtRenderEventXml,
                0,
                std::ptr::null_mut(),
                &mut required,
                &mut properties,
            )
        };
        ensure!(
            ok == 0
                && unsafe { GetLastError() } == ERROR_INSUFFICIENT_BUFFER
                && required <= 512 * 1024,
            "Could not size a Defender event"
        );
        let mut buffer = vec![0u16; (required as usize).div_ceil(2)];
        let ok = unsafe {
            EvtRender(
                0,
                event.0,
                EvtRenderEventXml,
                required,
                buffer.as_mut_ptr().cast(),
                &mut required,
                &mut properties,
            )
        };
        ensure!(ok != 0, "Could not read a Defender event");
        let end = buffer
            .iter()
            .position(|character| *character == 0)
            .unwrap_or(buffer.len());
        let xml = String::from_utf16_lossy(&buffer[..end]);
        if let Some(detection) = parse_detection(&xml, target)? {
            events.push(detection);
        }
    }
    Ok(DefenderReport {
        readable: true,
        limited,
        error: None,
        events,
    })
}

#[cfg(not(windows))]
fn read_defender(_: &Path) -> Result<DefenderReport> {
    Ok(DefenderReport {
        readable: false,
        limited: false,
        error: Some("Defender event history is available only on Windows".into()),
        events: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xml(path: &str) -> String {
        format!(
            r#"<Event xmlns="http://schemas.microsoft.com/win/2004/08/events/event"><System><EventID>1117</EventID><TimeCreated SystemTime="2026-10-18T20:39:18Z"/></System><EventData><Data Name="Path">{path}</Data><Data Name="Threat Name">Trojan:Example</Data><Data Name="Action Name">Quarantine</Data><Data Name="Unrelated">private-secret</Data></EventData></Event>"#
        )
    }

    #[test]
    fn detection_matches_only_exact_launcher_or_update_path() {
        let target = Path::new("C:/Ferns/Player/FernsLauncher.exe");
        for path in [
            "file:_C:\\Ferns\\Player\\FernsLauncher.exe",
            "file:_c:\\ferns\\player\\fernslauncher.exe.new; startup:_another-shortcut",
        ] {
            let record = parse_detection(&xml(path), target).unwrap().unwrap();
            assert_eq!(record.action, "Quarantine");
            assert!(!serde_json::to_string(&record)
                .unwrap()
                .contains("private-secret"));
        }
        for path in [
            "file:_C:\\Other\\FernsLauncher.exe",
            "file:_C:\\Ferns\\Player\\FernsLauncher.exe.unrelated",
        ] {
            assert!(parse_detection(&xml(path), target).unwrap().is_none());
        }
    }

    #[test]
    fn malformed_xml_is_not_treated_as_clean_history() {
        assert!(
            parse_detection("not xml", Path::new("C:/Ferns/Player/FernsLauncher.exe")).is_err()
        );
    }

    #[test]
    fn reinstalled_launcher_does_not_inherit_a_missing_file_diagnosis() {
        let folder = tempfile::tempdir().unwrap();
        std::fs::create_dir(folder.path().join("Player")).unwrap();
        std::fs::create_dir(folder.path().join("Client")).unwrap();
        for name in [
            "Player/RobloxPlayerBeta.exe",
            "Player/FernsLauncher.exe",
            "Client/FernsLauncher.exe",
        ] {
            std::fs::write(folder.path().join(name), b"test").unwrap();
        }
        let installation = crate::discovery::inspect(&crate::settings::Settings {
            client_root: Some(folder.path().to_owned()),
            ..Default::default()
        })
        .unwrap();
        let history = DefenderReport {
            readable: true,
            limited: false,
            error: None,
            events: vec![parse_detection(
                &xml("file:_C:\\Ferns\\Player\\FernsLauncher.exe"),
                Path::new("C:/Ferns/Player/FernsLauncher.exe"),
            )
            .unwrap()
            .unwrap()],
        };
        assert!(startup_findings(&installation, &history).is_empty());
        std::fs::remove_file(folder.path().join("Player/FernsLauncher.exe")).unwrap();
        let missing = crate::discovery::inspect(&crate::settings::Settings {
            client_root: Some(folder.path().to_owned()),
            ..Default::default()
        })
        .unwrap();
        assert!(startup_findings(&missing, &history)[0]
            .title
            .contains("recorded quarantine"));
        let unknown = startup_findings(
            &missing,
            &DefenderReport {
                events: Vec::new(),
                ..history
            },
        );
        assert!(!unknown[0].title.contains("quarantine"));
    }
}
