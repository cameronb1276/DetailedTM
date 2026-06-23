use std::collections::{HashMap, HashSet};
use thiserror::Error;
use windows_sys::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
    PdhOpenQueryW, PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE_ITEM_W,
    PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA,
};

const GPU_COUNTER_PATH: &str = r"\GPU Engine(*)\Utilization Percentage";

#[derive(Debug, Error)]
pub enum GpuCollectionError {
    #[error("PDH could not open the GPU query (status 0x{0:08X})")]
    OpenQuery(u32),
    #[error("the GPU Engine performance counter is unavailable (status 0x{0:08X})")]
    AddCounter(u32),
    #[error("PDH GPU sampling failed (status 0x{0:08X})")]
    Collect(u32),
    #[error("PDH could not size GPU counter data (status 0x{0:08X})")]
    Size(u32),
    #[error("PDH could not read GPU counter data (status 0x{0:08X})")]
    Read(u32),
}

pub struct GpuCollector {
    query: PDH_HQUERY,
    counter: PDH_HCOUNTER,
    initialization_error: Option<u32>,
}

impl GpuCollector {
    pub fn new() -> Self {
        let mut collector = Self {
            query: std::ptr::null_mut(),
            counter: std::ptr::null_mut(),
            initialization_error: None,
        };

        let open_status = unsafe { PdhOpenQueryW(std::ptr::null(), 0, &mut collector.query) };
        if open_status != 0 {
            collector.initialization_error = Some(open_status);
            return collector;
        }

        let path = wide_null(GPU_COUNTER_PATH);
        let add_status = unsafe {
            PdhAddEnglishCounterW(collector.query, path.as_ptr(), 0, &mut collector.counter)
        };
        if add_status != 0 {
            collector.initialization_error = Some(add_status);
        }
        collector
    }

    pub fn collect(&mut self, pids: &[u32]) -> Result<HashMap<u32, f32>, GpuCollectionError> {
        if let Some(status) = self.initialization_error {
            return if self.query.is_null() {
                Err(GpuCollectionError::OpenQuery(status))
            } else {
                Err(GpuCollectionError::AddCounter(status))
            };
        }

        let collect_status = unsafe { PdhCollectQueryData(self.query) };
        if collect_status != 0 {
            return Err(GpuCollectionError::Collect(collect_status));
        }

        let mut byte_size = 0_u32;
        let mut item_count = 0_u32;
        let size_status = unsafe {
            PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut byte_size,
                &mut item_count,
                std::ptr::null_mut(),
            )
        };
        if size_status != PDH_MORE_DATA {
            return Err(GpuCollectionError::Size(size_status));
        }

        // A usize-backed allocation gives the PDH item array pointer alignment,
        // while the byte count also reserves PDH's trailing UTF-16 names.
        let words = (byte_size as usize).div_ceil(std::mem::size_of::<usize>());
        let mut storage = vec![0_usize; words.max(1)];
        let items = storage.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
        let read_status = unsafe {
            PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut byte_size,
                &mut item_count,
                items,
            )
        };
        if read_status != 0 {
            return Err(GpuCollectionError::Read(read_status));
        }

        let wanted: HashSet<u32> = pids.iter().copied().collect();
        let mut usage: HashMap<u32, f32> = pids.iter().copied().map(|pid| (pid, 0.0)).collect();
        let values = unsafe { std::slice::from_raw_parts(items, item_count as usize) };
        for item in values {
            if item.FmtValue.CStatus != PDH_CSTATUS_VALID_DATA
                && item.FmtValue.CStatus != PDH_CSTATUS_NEW_DATA
            {
                continue;
            }
            let instance = unsafe { wide_ptr_to_string(item.szName.cast_const()) };
            let Some(pid) = pid_from_instance(&instance) else {
                continue;
            };
            if !wanted.contains(&pid) {
                continue;
            }
            let value = unsafe { item.FmtValue.Anonymous.doubleValue };
            if value.is_finite() && value >= 0.0 {
                *usage.entry(pid).or_default() += value as f32;
            }
        }

        // Multiple GPU engines can report simultaneously. The UI represents a
        // process-wide percentage, so keep the displayed value in 0..=100.
        usage
            .values_mut()
            .for_each(|value| *value = value.min(100.0));
        Ok(usage)
    }
}

impl Drop for GpuCollector {
    fn drop(&mut self) {
        if !self.query.is_null() {
            unsafe { PdhCloseQuery(self.query) };
        }
    }
}

fn pid_from_instance(instance: &str) -> Option<u32> {
    let parts: Vec<&str> = instance.split('_').collect();
    parts
        .windows(2)
        .find(|pair| pair[0].eq_ignore_ascii_case("pid"))
        .and_then(|pair| pair[1].parse().ok())
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn wide_ptr_to_string(pointer: *const u16) -> String {
    if pointer.is_null() {
        return String::new();
    }
    let mut length = 0;
    while length < 4096 && unsafe { *pointer.add(length) } != 0 {
        length += 1;
    }
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(pointer, length) })
}

#[cfg(test)]
mod tests {
    use super::pid_from_instance;

    #[test]
    fn extracts_pid_from_gpu_engine_instance() {
        assert_eq!(
            pid_from_instance("pid_1234_luid_0x00000000_0x0000_phys_0_eng_1_engtype_3D"),
            Some(1234)
        );
        assert_eq!(pid_from_instance("not_a_gpu_instance"), None);
    }
}
