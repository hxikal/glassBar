//! GPU 3D-engine utilization from Windows performance counters (what Task Manager shows).
use std::sync::Mutex;
use windows::core::w;
use windows::Win32::System::Performance::*;

struct Query { q: PDH_HQUERY, c: PDH_HCOUNTER }
// PDH handles are plain process-wide handles; we only use them under the mutex.
unsafe impl Send for Query {}

static QUERY: Mutex<Option<Query>> = Mutex::new(None);
// Its own query: two callers collecting one query would break each other's rate deltas.
static QUERY_PIDS: Mutex<Option<Query>> = Mutex::new(None);

fn open() -> Option<Query> {
    unsafe {
        let mut q = PDH_HQUERY::default();
        if PdhOpenQueryW(None, 0, &mut q) != 0 { return None; }
        let mut c = PDH_HCOUNTER::default();
        if PdhAddEnglishCounterW(q, w!("\\GPU Engine(*engtype_3D)\\Utilization Percentage"), 0, &mut c) != 0 {
            let _ = PdhCloseQuery(q);
            return None;
        }
        PdhCollectQueryData(q); // first sample primes the rate counter
        Some(Query { q, c })
    }
}

/// (process id, utilization) for every 3D engine instance; None if counters are unavailable.
fn read(slot: &Mutex<Option<Query>>) -> Option<Vec<(Option<u32>, f64)>> {
    let mut guard = slot.lock().unwrap();
    if guard.is_none() { *guard = open(); }
    let Query { q, c } = guard.as_ref()?;
    unsafe {
        if PdhCollectQueryData(*q) != 0 { return None; }
        let (mut size, mut count) = (0u32, 0u32);
        // First call asks for the buffer size (PDH_MORE_DATA).
        PdhGetFormattedCounterArrayW(*c, PDH_FMT_DOUBLE, &mut size, &mut count, None);
        if size == 0 { return Some(vec![]); }
        let mut buf = vec![0u8; size as usize];
        let items = buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
        if PdhGetFormattedCounterArrayW(*c, PDH_FMT_DOUBLE, &mut size, &mut count, Some(items)) != 0 { return None; }
        Some(std::slice::from_raw_parts(items, count as usize).iter()
            .map(|i| (i.szName.to_string().ok().and_then(|n| pid_of(&n)), i.FmtValue.Anonymous.doubleValue))
            .collect())
    }
}

/// The process id in a GPU Engine instance name ("pid_1234_luid_…_engtype_3D").
pub fn pid_of(instance: &str) -> Option<u32> {
    let rest = instance.strip_prefix("pid_")?;
    rest[..rest.find('_').unwrap_or(rest.len())].parse().ok()
}

/// Sum of 3D engine utilization across processes, 0..100. First call returns ~0.
pub fn usage() -> Option<f32> {
    Some(read(&QUERY)?.iter().map(|(_, v)| v).sum::<f64>().clamp(0.0, 100.0) as f32)
}

/// 3D engine utilization of each of these processes (same order), 0..100. First call returns ~0.
pub fn usage_each(pids: &[u32]) -> Option<Vec<f32>> {
    let all = read(&QUERY_PIDS)?;
    // A fold from 0, not sum(): an empty f64 sum is -0.0, which reads "-0%".
    Some(pids.iter().map(|&p| all.iter().filter(|(q, _)| *q == Some(p))
        .fold(0.0, |a, (_, v)| a + v).clamp(0.0, 100.0) as f32).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pid_of_reads_engine_instance_names() {
        assert_eq!(pid_of("pid_1234_luid_0x00000000_0x0000D1A5_phys_0_eng_0_engtype_3D"), Some(1234));
        assert_eq!(pid_of("_Total"), None);
        assert_eq!(pid_of("pid_abc_luid"), None);
        assert_eq!(pid_of(""), None);
    }
}
