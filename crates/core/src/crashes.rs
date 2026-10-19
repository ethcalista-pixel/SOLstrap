//! Local process-exit evidence. Never stores join links, raw logs or dumps.
use anyhow::{ensure, Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
mod process;
const FILE: &str = "sessions.json";
const MAX: usize = 32;
#[derive(Debug, Serialize, Deserialize)]
pub struct LaunchStatus {
    pub at: u64,
    pub message: String,
}
pub fn launch_status(dir: &Path) -> Result<Option<LaunchStatus>> {
    let path = dir.join("session-launch-status.json");
    if !path.try_exists()? {
        return Ok(None);
    }
    ensure!(
        fs::metadata(&path)?.len() <= 4096,
        "Launch status file exceeds its size limit"
    );
    Ok(Some(crate::read_json(&path)?))
}
pub fn note_launch(dir: &Path, message: &str) {
    let message = message
        .chars()
        .filter(|c| !c.is_control())
        .take(256)
        .collect();
    let _ = crate::write_json(
        &dir.join("session-launch-status.json"),
        &LaunchStatus { at: now(), message },
    );
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    Waiting,
    Running,
    Exited,
    LaunchFailed,
    NotObserved,
    Interrupted,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub pid: u32,
    pub created: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clue {
    pub file_name: String,
    pub shared_folder: bool,
    pub partial: bool,
    pub access_violation: usize,
    pub allocation_failure: usize,
    pub dns_failure: usize,
    pub tls_failure: usize,
    pub connection_failure: usize,
    pub exceptions: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub started: u64,
    pub updated: u64,
    pub ended: Option<u64>,
    pub state: State,
    pub player: Option<Identity>,
    pub player_started: Option<u64>,
    pub exit_code: Option<u32>,
    pub clues: Vec<Clue>,
    root: PathBuf,
    baseline: Vec<Identity>,
    log_baseline: Vec<String>,
    claimed: bool,
}
#[derive(Serialize, Deserialize)]
struct Store {
    version: u32,
    sessions: Vec<Session>,
}
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub fn history(dir: &Path) -> Result<Vec<Session>> {
    let path = dir.join(FILE);
    if !path.try_exists()? {
        return Ok(vec![]);
    }
    ensure!(
        fs::metadata(&path)?.len() <= 1024 * 1024,
        "Session history exceeds its size limit"
    );
    let store: Store = crate::read_json(&path)?;
    ensure!(
        store.version == 1 && store.sessions.len() <= MAX,
        "Unsupported session history version or size"
    );
    for session in &store.sessions {
        ensure!(
            session.id.len() <= 64
                && session.clues.len() <= 4
                && session.log_baseline.len() <= 4
                && session.baseline.len() <= 128,
            "Invalid session record"
        );
    }
    Ok(store.sessions)
}
fn edit<T>(dir: &Path, change: impl FnOnce(&mut Vec<Session>) -> Result<T>) -> Result<T> {
    fs::create_dir_all(dir)?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join("sessions.lock"))?;
    lock.try_lock_exclusive()
        .context("Another session update is running")?;
    let mut sessions = history(dir)?;
    let value = change(&mut sessions)?;
    crate::write_json(
        &dir.join(FILE),
        &Store {
            version: 1,
            sessions,
        },
    )?;
    Ok(value)
}
fn update(dir: &Path, id: &str, change: impl FnOnce(&mut Session)) -> Result<()> {
    edit(dir, |sessions| {
        let s = sessions
            .iter_mut()
            .find(|s| s.id == id)
            .context("Session is no longer retained")?;
        change(s);
        s.updated = now();
        Ok(())
    })
}
fn logs(root: &Path) -> crate::diagnostics::CoreGuiReport {
    let shared = std::env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("Roblox/logs"));
    crate::diagnostics::inspect_coregui_with_shared(root, shared.as_deref())
}
pub fn prepare(dir: &Path, root: &Path) -> Result<String> {
    let started = now();
    let baseline = process::list(root)?
        .into_iter()
        .map(|p| p.identity.clone())
        .collect::<Vec<_>>();
    ensure!(
        baseline.len() <= 128,
        "Too many player processes to monitor"
    );
    let report = logs(root);
    let log_baseline = report
        .logs
        .iter()
        .map(|l| format!("{}:{}", report.log_folder.display(), l.file_name))
        .collect();
    let id = format!(
        "{}-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(),
        std::process::id()
    );
    let session = Session {
        id: id.clone(),
        started,
        updated: started,
        ended: None,
        state: State::Waiting,
        player: None,
        player_started: None,
        exit_code: None,
        clues: vec![],
        root: root.to_owned(),
        baseline,
        log_baseline,
        claimed: false,
    };
    edit(dir, |sessions| {
        sessions.insert(0, session);
        trim_history(sessions)?;
        Ok(())
    })?;
    Ok(id)
}
fn trim_history(sessions: &mut Vec<Session>) -> Result<()> {
    while sessions.len() > MAX {
        let at = sessions
            .iter()
            .rposition(|s| {
                !matches!(s.state, State::Waiting | State::Running)
                    || now().saturating_sub(s.updated) > 90
            })
            .context("All retained session slots are actively monitored")?;
        sessions.remove(at);
    }
    Ok(())
}
pub fn failed(dir: &Path, id: &str) {
    let _ = update(dir, id, |s| {
        s.state = State::LaunchFailed;
        s.ended = Some(now());
    });
}
pub fn observer_failed(dir: &Path, id: &str) {
    let _ = update(dir, id, |s| {
        s.state = State::Interrupted;
        s.ended = Some(now());
    });
}
fn eligible(identity: &Identity, session: &Session) -> bool {
    !session.baseline.contains(identity)
        && identity.created / 10_000_000 >= 11644473600 + session.started.saturating_sub(2)
}
fn finish(dir: &Path, id: &str, code: Option<u32>, session: &Session) -> Result<()> {
    let end = now();
    let report = logs(&session.root);
    let clues = related_clues(session, &report, end);
    update(dir, id, |s| {
        s.state = if code.is_some() {
            State::Exited
        } else {
            State::Interrupted
        };
        s.exit_code = code;
        s.ended = Some(end);
        s.clues = clues;
    })
}
fn related_clues(
    session: &Session,
    report: &crate::diagnostics::CoreGuiReport,
    end: u64,
) -> Vec<Clue> {
    report
        .logs
        .iter()
        .filter(|l| {
            !session.log_baseline.contains(&format!(
                "{}:{}",
                report.log_folder.display(),
                l.file_name
            )) && l
                .modified_unix_seconds
                .is_some_and(|t| t >= session.started && t <= end.saturating_add(10))
        })
        .map(|l| Clue {
            file_name: l.file_name.clone(),
            shared_folder: report.shared_log_folder,
            partial: l.partial,
            access_violation: l.access_violation_markers,
            allocation_failure: l.out_of_memory_markers,
            dns_failure: l.dns_failure_markers,
            tls_failure: l.tls_failure_markers,
            connection_failure: l.connection_failure_markers,
            exceptions: l.exception_markers,
        })
        .collect()
}
/// Run in a separate native launcher process. It owns only query/wait handles.
pub fn watch(dir: &Path, id: &str) -> Result<()> {
    let session = edit(dir, |sessions| {
        let s = sessions
            .iter_mut()
            .find(|s| s.id == id)
            .context("Unknown session")?;
        ensure!(
            !s.claimed && s.state == State::Waiting,
            "Session observer already started"
        );
        s.claimed = true;
        Ok(s.clone())
    })?;
    let result = observe(dir, id, &session);
    if result.is_err() {
        observer_failed(dir, id);
    }
    result
}
fn observe(dir: &Path, id: &str, session: &Session) -> Result<()> {
    let deadline = std::time::Instant::now();
    let mut heartbeat = std::time::Instant::now();
    let player = loop {
        let mut players = process::list(&session.root)?;
        players.sort_by_key(|p| p.identity.created);
        if let Some(player) = players.into_iter().find(|p| eligible(&p.identity, session)) {
            break player;
        }
        if deadline.elapsed().as_secs() >= 90 {
            update(dir, id, |s| {
                s.state = State::NotObserved;
                s.ended = Some(now());
            })?;
            return Ok(());
        }
        if heartbeat.elapsed().as_secs() >= 15 {
            update(dir, id, |_| {})?;
            heartbeat = std::time::Instant::now();
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    };
    update(dir, id, |s| {
        s.state = State::Running;
        s.player = Some(player.identity.clone());
        s.player_started = Some(player.identity.created / 10_000_000 - 11644473600);
    })?;
    loop {
        if let Some(code) = player.exit_code()? {
            std::thread::sleep(std::time::Duration::from_millis(750));
            return finish(dir, id, Some(code), session);
        }
        if deadline.elapsed().as_secs() >= 24 * 60 * 60 {
            return finish(dir, id, None, session);
        }
        if heartbeat.elapsed().as_secs() >= 15 {
            update(dir, id, |_| {})?;
            heartbeat = std::time::Instant::now();
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
#[derive(Debug, Serialize)]
pub struct Explanation {
    pub title: String,
    pub evidence: String,
    pub next_step: String,
    pub abnormal: bool,
}
pub fn explain(s: &Session, time: u64) -> Explanation {
    let stale =
        matches!(s.state, State::Waiting | State::Running) && time.saturating_sub(s.updated) > 90;
    let (title, evidence, next, abnormal) = if stale {
        ("Monitoring update missing","No observer update for over 90 seconds. Sleep, interruption, or an exited observer can cause this; the player outcome is unknown.","Join through the website again to start a new observation. This is not proof of a crash.",false)
    } else {
        match(s.state,s.exit_code){
        (State::Waiting,_) => ("Waiting for a new player","The website launcher was forwarded; waiting up to 90 seconds for a new process at the configured Ferns player path.","Join normally. Existing player processes are excluded to avoid assigning another session's exit to this join.",false),
        (State::Running,_) => ("Player is being monitored","A matching player process and creation time were observed. The native observer remains active when the settings window is closed.","If the player closes unexpectedly, reopen Diagnostics or refresh here for its recorded exit.",false),
        (State::Exited,Some(0)) => ("Player exited successfully","Windows reported exit code 0 for the observed player process. An unexpected disconnect can still occur without a nonzero exit.","If the game disconnected, compare the possible log clues with what happened on screen.",false),
        (State::Exited,Some(0xc0000005)) => ("Player exited with access-violation status","Windows reported 0xC0000005: an invalid memory access. The exit code does not identify the faulty module.","Record the experience and last action. Compare a reproduction with optional shader changes restored and share the sanitized report with the Ferns developer.",true),
        (State::Exited,Some(0xc00000fd)) => ("Player exited with stack-overflow status","Windows reported 0xC00000FD. This can result from excessive recursion or stack usage; the responsible component is unknown.","Record a repeatable trigger and the client build for the Ferns developer.",true),
        (State::Exited,Some(0xc0000017)) => ("Player exited with allocation-failure status","Windows reported 0xC0000017 (insufficient virtual memory). This alone does not establish a memory leak.","Check available memory and repeat with fewer background applications. Record whether the same experience triggers it.",true),
        (State::Exited,Some(_)) => ("Player exited with a nonzero status","The observed player returned a nonzero exit code. A forced termination can also do this, so it is not proof of a software crash.","Record the last action and inspect the possible log clues. Export the summary for the Ferns developer if it repeats.",true),
        (State::LaunchFailed,_) => ("Ferns launcher could not start","The original launcher failed to spawn. No player exit was observed.","Check the configured installation and whether the original launcher exists, then try another website join.",false),
        (State::NotObserved,_) => ("No new player observed","No new matching player process was captured within 90 seconds. The launcher may have reused an existing player, failed before observation, or taken longer to start.","This does not establish a crash. Fully exit Ferns before the next join if you want a distinct observed session.",false),
        _ => ("Player outcome unknown","The observer stopped or could not read a final exit status.","Monitoring may have been interrupted or reached its 24-hour limit. A new website join starts another observation.",false),
    }
    };
    Explanation {
        title: title.into(),
        evidence: format!(
            "{evidence}{}",
            s.exit_code
                .map(|c| format!(" Exit code: 0x{c:08X} ({c})."))
                .unwrap_or_default()
        ),
        next_step: next.into(),
        abnormal,
    }
}
/// Export deliberately omits installation paths, process baselines and raw logs.
pub fn export(s: &Session, path: &Path) -> Result<()> {
    crate::write_json(
        path,
        &serde_json::json!({"schema_version":1,"started_unix_seconds":s.started,"player_started_unix_seconds":s.player_started,"ended_unix_seconds":s.ended,"state":s.state,"exit_code":s.exit_code,"explanation":explain(s,now()),"possible_log_clues":s.clues,"log_ownership_verified":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launch_receipt_is_bounded_and_invalid_data_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        assert!(launch_status(dir.path()).unwrap().is_none());
        note_launch(dir.path(), &format!("ready\n{}", "x".repeat(300)));
        let status = launch_status(dir.path()).unwrap().unwrap();
        assert_eq!(status.message.chars().count(), 256);
        assert!(!status.message.contains('\n'));
        assert!(status.at > 0);
        let path = dir.path().join("session-launch-status.json");
        fs::write(&path, b"invalid").unwrap();
        assert!(launch_status(dir.path()).is_err());
        fs::write(&path, vec![b' '; 4097]).unwrap();
        assert!(launch_status(dir.path()).is_err());
    }
    fn session() -> Session {
        Session {
            id: "test".into(),
            started: 100,
            updated: 110,
            ended: None,
            state: State::Waiting,
            player: None,
            player_started: None,
            exit_code: None,
            clues: vec![],
            root: PathBuf::from("private-root"),
            baseline: vec![],
            log_baseline: vec![],
            claimed: false,
        }
    }
    #[test]
    fn exit_statuses_and_missing_updates_do_not_overclaim() {
        let mut s = session();
        s.state = State::Exited;
        s.ended = Some(120);
        for code in [0, 0xc0000005, 0xc00000fd, 0xc0000017, 259, 1] {
            s.exit_code = Some(code);
            let explanation = explain(&s, 120);
            assert_eq!(explanation.abnormal, code != 0);
            assert!(explanation.evidence.contains(&format!("0x{code:08X}")));
        }
        s.state = State::Running;
        s.exit_code = None;
        assert!(!explain(&s, 300).abnormal);
        assert!(explain(&s, 300).evidence.contains("unknown"));
        s.state = State::NotObserved;
        assert!(!explain(&s, 120).abnormal);
    }
    #[test]
    fn excludes_old_processes_and_handles_pid_reuse() {
        let mut s = session();
        let old = Identity {
            pid: 9,
            created: (11644473600 + 99) * 10_000_000,
        };
        s.baseline.push(old.clone());
        assert!(!eligible(&old, &s));
        assert!(eligible(
            &Identity {
                pid: 9,
                created: (11644473600 + 101) * 10_000_000
            },
            &s
        ));
        assert!(!eligible(
            &Identity {
                pid: 10,
                created: (11644473600 + 50) * 10_000_000
            },
            &s
        ));
    }
    #[test]
    fn duplicate_observers_do_not_interrupt_the_owner() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = session();
        s.claimed = true;
        s.state = State::Running;
        edit(dir.path(), |sessions| {
            sessions.push(s);
            Ok(())
        })
        .unwrap();
        assert!(watch(dir.path(), "test").is_err());
        assert_eq!(history(dir.path()).unwrap()[0].state, State::Running);
    }
    #[test]
    fn existing_logs_are_excluded_and_new_logs_only_supply_structured_clues() {
        let temp = tempfile::tempdir().unwrap();
        let folder = temp.path().join("Roblox/logs");
        fs::create_dir_all(&folder).unwrap();
        fs::write(
            folder.join("old_Player_test.log"),
            b"EXCEPTION_ACCESS_VIOLATION\n",
        )
        .unwrap();
        fs::write(
            folder.join("new_Player_test.log"),
            b"connection timed out\nsecret-join-ticket-must-not-export\n",
        )
        .unwrap();
        let report = crate::diagnostics::inspect_coregui(temp.path());
        let mut s = session();
        s.started = now().saturating_sub(2);
        s.log_baseline
            .push(format!("{}:old_Player_test.log", folder.display()));
        let clues = related_clues(&s, &report, now());
        assert_eq!(clues.len(), 1);
        assert_eq!(clues[0].connection_failure, 1);
        assert_eq!(clues[0].access_violation, 0);
        assert!(!serde_json::to_string(&clues)
            .unwrap()
            .contains("secret-join-ticket"));
    }
    #[test]
    fn store_is_bounded_corruption_preserved_and_export_excludes_private_state() {
        let dir = tempfile::tempdir().unwrap();
        for n in 0..40 {
            let mut s = session();
            s.id = n.to_string();
            edit(dir.path(), |sessions| {
                sessions.insert(0, s);
                sessions.truncate(MAX);
                Ok(())
            })
            .unwrap();
        }
        assert_eq!(history(dir.path()).unwrap().len(), MAX);
        let report = dir.path().join("report.json");
        export(&session(), &report).unwrap();
        let text = fs::read_to_string(report).unwrap();
        assert!(!text.contains("private-root"));
        assert!(!text.contains("baseline"));
        fs::write(dir.path().join(FILE), b"broken").unwrap();
        assert!(update(dir.path(), "0", |_| {}).is_err());
        assert_eq!(fs::read(dir.path().join(FILE)).unwrap(), b"broken");
    }
    #[test]
    fn retention_preserves_a_live_observer_amid_completed_joins() {
        let mut running = session();
        running.state = State::Running;
        running.updated = now();
        running.id = "live".into();
        let mut sessions = vec![running];
        for n in 0..80 {
            let mut s = session();
            s.id = n.to_string();
            s.state = State::NotObserved;
            sessions.insert(0, s);
            trim_history(&mut sessions).unwrap();
        }
        assert_eq!(sessions.len(), MAX);
        assert!(sessions.iter().any(|s| s.id == "live"));
    }
    #[cfg(windows)]
    #[test]
    #[ignore = "requires explicitly compiled local exit-probe fixture; never launches Ferns"]
    fn controlled_process_exit_is_captured_and_wrong_paths_ignored() {
        let helper = PathBuf::from(
            std::env::var_os("SOLSTRAP_TEST_EXIT_HELPER").expect("explicit helper exe"),
        );
        for (name, code) in [
            ("RobloxPlayerBeta.exe", 0u32),
            ("Ferns.exe", 0xc0000005),
            ("Ferns.exe", 259),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("client");
            let dir = temp.path().join("state");
            fs::create_dir_all(root.join("Player")).unwrap();
            fs::copy(&helper, root.join("Player").join(name)).unwrap();
            let alien = temp.path().join("alien/Player");
            fs::create_dir_all(&alien).unwrap();
            fs::copy(&helper, alien.join(name)).unwrap();
            let mut other = std::process::Command::new(alien.join(name))
                .arg("1")
                .spawn()
                .unwrap();
            let id = prepare(&dir, &root).unwrap();
            let worker_dir = dir.clone();
            let worker_id = id.clone();
            let watcher = std::thread::spawn(move || watch(&worker_dir, &worker_id));
            let mut child = std::process::Command::new(root.join("Player").join(name))
                .arg(format!("{code:X}"))
                .spawn()
                .unwrap();
            let pid = child.id();
            child.wait().unwrap();
            other.wait().unwrap();
            watcher.join().unwrap().unwrap();
            let sessions = history(&dir).unwrap();
            let s = &sessions[0];
            assert_eq!(s.state, State::Exited);
            assert_eq!(s.exit_code, Some(code));
            assert_eq!(s.player.as_ref().unwrap().pid, pid);
            assert!(s.player_started.is_some());
            assert!(s.ended.is_some());
        }
    }
}
