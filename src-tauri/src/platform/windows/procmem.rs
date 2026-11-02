//! Private working set of a process: the RAM only it uses, which is Task Manager's Memory
//! column. Summing it over processes doesn't count shared pages twice the way working set
//! would, and unlike private bytes (commit) it leaves out memory reserved but never touched.
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX2};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

pub fn private_ws(pid: u32) -> Option<u64> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut m = PROCESS_MEMORY_COUNTERS_EX2::default();
        let ok = K32GetProcessMemoryInfo(h, &mut m as *mut _ as *mut PROCESS_MEMORY_COUNTERS, size_of::<PROCESS_MEMORY_COUNTERS_EX2>() as u32).as_bool();
        let _ = CloseHandle(h);
        ok.then_some(m.PrivateWorkingSetSize as u64)
    }
}
