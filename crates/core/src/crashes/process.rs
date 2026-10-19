use super::*;
#[cfg(windows)]
mod native {
    use super::*;
    use windows_sys::Win32::{
        Foundation::{
            CloseHandle, FILETIME, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0, WAIT_TIMEOUT,
        },
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
                TH32CS_SNAPPROCESS,
            },
            Threading::{
                GetExitCodeProcess, GetProcessTimes, OpenProcess, QueryFullProcessImageNameW,
                WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
    };
    struct Handle(HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    pub struct Player {
        handle: Handle,
        pub identity: Identity,
    }
    impl Player {
        pub fn exit_code(&self) -> Result<Option<u32>> {
            unsafe {
                match WaitForSingleObject(self.handle.0, 0) {
                    WAIT_TIMEOUT => Ok(None),
                    WAIT_OBJECT_0 => {
                        let mut code = 0;
                        ensure!(
                            GetExitCodeProcess(self.handle.0, &mut code) != 0,
                            "Cannot query final player status"
                        );
                        Ok(Some(code))
                    }
                    _ => anyhow::bail!("Cannot wait on player"),
                }
            }
        }
    }
    pub fn list(root: &Path) -> Result<Vec<Player>> {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            ensure!(
                snapshot != INVALID_HANDLE_VALUE,
                "Cannot inspect player processes"
            );
            let snapshot = Handle(snapshot);
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut more = Process32FirstW(snapshot.0, &mut entry);
            let mut players = vec![];
            while more != 0 {
                let n = entry
                    .szExeFile
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(entry.szExeFile.len());
                if String::from_utf16_lossy(&entry.szExeFile[..n])
                    .eq_ignore_ascii_case("RobloxPlayerBeta.exe")
                {
                    // QUERY_LIMITED_INFORMATION + SYNCHRONIZE only; no memory/process writes.
                    let h = OpenProcess(
                        PROCESS_QUERY_LIMITED_INFORMATION | 0x00100000,
                        0,
                        entry.th32ProcessID,
                    );
                    if !h.is_null() {
                        let handle = Handle(h);
                        let mut path = vec![0u16; 32768];
                        let mut len = path.len() as u32;
                        if QueryFullProcessImageNameW(h, 0, path.as_mut_ptr(), &mut len) != 0
                            && crate::launch::same_file_path(
                                Path::new(&String::from_utf16_lossy(&path[..len as usize])),
                                &root.join("Player/RobloxPlayerBeta.exe"),
                            )
                        {
                            let mut created: FILETIME = std::mem::zeroed();
                            let mut exit: FILETIME = std::mem::zeroed();
                            let mut kernel: FILETIME = std::mem::zeroed();
                            let mut user: FILETIME = std::mem::zeroed();
                            if GetProcessTimes(h, &mut created, &mut exit, &mut kernel, &mut user)
                                != 0
                            {
                                players.push(Player {
                                    handle,
                                    identity: Identity {
                                        pid: entry.th32ProcessID,
                                        created: ((created.dwHighDateTime as u64) << 32)
                                            | created.dwLowDateTime as u64,
                                    },
                                });
                            }
                        }
                    }
                }
                more = Process32NextW(snapshot.0, &mut entry);
            }
            Ok(players)
        }
    }
}
#[cfg(windows)]
pub use native::*;
#[cfg(not(windows))]
pub struct Player {
    pub identity: Identity,
}
#[cfg(not(windows))]
impl Player {
    pub fn exit_code(&self) -> Result<Option<u32>> {
        anyhow::bail!("Process observation requires Windows")
    }
}
#[cfg(not(windows))]
pub fn list(_: &Path) -> Result<Vec<Player>> {
    anyhow::bail!("Process observation requires Windows")
}
