//! What the running app costs, read from outside it the way Task Manager does.

use std::os::windows::io::{AsRawHandle, RawHandle};
use std::process::Child;
use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::{ERROR_SUCCESS, FILETIME, HANDLE};
use windows::Win32::System::Performance::{
    PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY,
    PDH_MORE_DATA, PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData,
    PdhGetFormattedCounterArrayW, PdhOpenQueryW,
};
use windows::Win32::System::Threading::GetProcessTimes;
use windows::core::{HSTRING, PCWSTR};

/// Windows counts CPU time in steps of 100 ns.
const CPU_TICKS_PER_SECOND: f64 = 10_000_000.0;

fn ticks(time: FILETIME) -> u64 {
    (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
}

/// CPU time the process has used so far, in all its threads.
fn cpu_time(process: RawHandle) -> Duration {
    let (mut created, mut exited, mut kernel, mut user) = Default::default();
    // SAFETY: every pointer is a live local, and the caller's `Child` keeps the handle open.
    unsafe {
        GetProcessTimes(
            HANDLE(process),
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        )
    }
    .expect("read the app's CPU time");
    Duration::from_secs_f64((ticks(kernel) + ticks(user)) as f64 / CPU_TICKS_PER_SECOND)
}

/// The share of one CPU core `child` used during `span`, as a percentage.
pub fn cpu_percent_of_one_core(child: &Child, span: Duration) -> f64 {
    let process = child.as_raw_handle();
    let before = cpu_time(process);
    thread::sleep(span);
    let used = cpu_time(process) - before;
    used.as_secs_f64() / span.as_secs_f64() * 100.0
}

/// The counter behind Task Manager's GPU column, summed over every GPU engine the process has used.
pub struct GpuUse {
    query: PDH_HQUERY,
    counter: PDH_HCOUNTER,
}

impl GpuUse {
    pub fn of(pid: u32) -> Self {
        let path = HSTRING::from(format!(r"\GPU Engine(pid_{pid}_*)\Utilization Percentage"));
        let mut query = PDH_HQUERY::default();
        // SAFETY: `query` is a live local that Windows writes during the call.
        let opened = unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) };
        assert_eq!(opened, ERROR_SUCCESS.0, "open a performance query");
        let mut counter = PDH_HCOUNTER::default();
        // SAFETY: `query` was just opened, and `path` and `counter` outlive the call.
        let added = unsafe { PdhAddEnglishCounterW(query, &path, 0, &mut counter) };
        assert_eq!(added, ERROR_SUCCESS.0, "add the GPU counter");
        let gpu = Self { query, counter };
        gpu.collect();
        gpu
    }

    fn collect(&self) {
        // SAFETY: `self.query` stays open until `drop`.
        let collected = unsafe { PdhCollectQueryData(self.query) };
        assert_eq!(collected, ERROR_SUCCESS.0, "read the GPU counter");
    }

    /// The average percentage since the last call, or since `of`.
    pub fn percent(&self) -> f64 {
        self.collect();
        let (mut bytes, mut count) = (0, 0);
        // SAFETY: both sizes are live locals, and no buffer is passed yet.
        let status = unsafe {
            PdhGetFormattedCounterArrayW(self.counter, PDH_FMT_DOUBLE, &mut bytes, &mut count, None)
        };
        if status != PDH_MORE_DATA {
            // No engine has been used yet, so there's nothing to add up.
            return 0.0;
        }
        // Eight-byte items, because the array is followed by the instance names it points into.
        let mut buffer = vec![0u64; (bytes as usize).div_ceil(size_of::<u64>())];
        let items = buffer.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
        // SAFETY: `buffer` holds at least the `bytes` Windows asked for, aligned for an item.
        let status = unsafe {
            PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut bytes,
                &mut count,
                Some(items),
            )
        };
        assert_eq!(status, ERROR_SUCCESS.0, "format the GPU counter");
        (0..count as usize)
            .map(|index| {
                // SAFETY: Windows wrote `count` items at the start of `buffer`.
                let item = unsafe { items.add(index).read() };
                if item.FmtValue.CStatus == PDH_CSTATUS_VALID_DATA {
                    // SAFETY: PDH_FMT_DOUBLE selects the double in this union.
                    unsafe { item.FmtValue.Anonymous.doubleValue }
                } else {
                    0.0
                }
            })
            .sum()
    }
}

impl Drop for GpuUse {
    fn drop(&mut self) {
        // SAFETY: this value owns the query, and `drop` runs exactly once.
        unsafe { PdhCloseQuery(self.query) };
    }
}
