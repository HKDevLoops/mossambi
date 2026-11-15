//! Hybrid CPU Topology & System-Level Thread QoS (Quality of Service)
//!
//! Provides system-level thread routing for heterogeneous CPU architectures
//! (Intel P/E cores, AMD Zen4/Zen4c, Apple Silicon Performance/Efficiency):
//! - Performance cores (P-cores / Super-cores): routed for conversion, transcode,
//!   raster rendering, and heavy computation via elevated thread priority and disabled
//!   power throttling.
//! - Efficiency cores (E-cores): routed for background tasks (indexing, watchers,
//!   hash prewarming, trash audits) via Windows EcoQoS (ThreadPowerThrottling).

/// Thread Quality of Service target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThreadQoS {
    /// High-performance routing: unthrottled execution targeting Performance cores.
    Performance,
    /// EcoQoS / background routing: power-throttled execution targeting Efficiency cores.
    Efficiency,
    /// Default OS scheduling.
    Default,
}

/// Discovered CPU core topology and system architecture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuTopology {
    /// Total logical processing units available.
    pub total_logical_cores: usize,
    /// Number of physical cores detected.
    pub physical_cores: usize,
    /// Number of Performance cores (or P-core allocation).
    pub p_cores: usize,
    /// Number of Efficiency cores (or E-core allocation).
    pub e_cores: usize,
    /// Number of Super / Prime cores (or super allocation).
    pub super_cores: usize,
    /// True when heterogeneous hardware core classes are detected.
    pub is_hybrid: bool,
    /// CPU Brand/Model name.
    pub cpu_name: String,
}

impl CpuTopology {
    /// Formats a descriptive label for UI settings.
    pub fn description(&self) -> String {
        if self.is_hybrid {
            if self.super_cores > 0 {
                format!(
                    "Hybrid ({}T: {} P + {} E + {} Super)",
                    self.total_logical_cores, self.p_cores, self.e_cores, self.super_cores
                )
            } else {
                format!(
                    "Hybrid ({}T: {} P-cores + {} E-cores)",
                    self.total_logical_cores, self.p_cores, self.e_cores
                )
            }
        } else if self.super_cores > 0 {
            format!(
                "Classical ({}T: {} P + {} E-EcoQoS + {} Super UI)",
                self.total_logical_cores, self.p_cores, self.e_cores, self.super_cores
            )
        } else {
            format!(
                "Classical ({}T: {} Performance + {} EcoQoS)",
                self.total_logical_cores, self.p_cores, self.e_cores
            )
        }
    }

    /// Detailed description of architecture for About / Diagnostics.
    pub fn detailed_summary(&self) -> String {
        if self.is_hybrid {
            format!(
                "{} - Heterogeneous Hybrid Architecture: {} logical processors ({} P-cores, {} E-cores, {} Super-cores)",
                self.cpu_name, self.total_logical_cores, self.p_cores, self.e_cores, self.super_cores
            )
        } else {
            format!(
                "{} - Classical Architecture ({} cores / {} threads): Partitioned as {} Performance workers (50%), {} Background EcoQoS workers, and {} Super-core UI worker",
                self.cpu_name, self.physical_cores, self.total_logical_cores, self.p_cores, self.e_cores, self.super_cores
            )
        }
    }
}

/// Queries CPU brand string via x86 CPUID or generic fallback.
pub fn query_cpu_brand_string() -> String {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        #[cfg(target_arch = "x86")]
        use std::arch::x86::__cpuid;
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::__cpuid;

        #[allow(unused_unsafe)]
        {
            let max_ext = unsafe { __cpuid(0x80000000) }.eax;
            if max_ext >= 0x80000004 {
                let mut bytes = Vec::with_capacity(48);
                for func in 0x80000002..=0x80000004 {
                    let res = unsafe { __cpuid(func) };
                    for reg in [res.eax, res.ebx, res.ecx, res.edx] {
                        bytes.extend_from_slice(&reg.to_le_bytes());
                    }
                }
                let s = String::from_utf8_lossy(&bytes);
                let trimmed = s.trim_matches('\0').trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }
    "Multi-Core Processor".to_string()
}

/// Detects CPU topology and core distribution using system APIs.
pub fn detect_cpu_topology() -> CpuTopology {
    let total = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4);
    let cpu_name = query_cpu_brand_string();

    #[cfg(target_os = "windows")]
    {
        win32_topology::detect_windows_topology(total, cpu_name)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let (p_cores, e_cores, super_cores) = partition_classical_cores(total);
        CpuTopology {
            total_logical_cores: total,
            physical_cores: (total / 2).max(1),
            p_cores,
            e_cores,
            super_cores,
            is_hybrid: false,
            cpu_name,
        }
    }
}

/// Classical homogeneous CPU core partitioner (e.g. AMD Ryzen 4800H 8C/16T):
/// - ~50% allocated to Performance tasks (transcoding, heavy indexing, hashing)
/// - ~40-50% allocated to Efficiency background tasks (EcoQoS file watching, cache)
/// - ~10% (1 core) allocated to latency-critical Super-core UI responsiveness
pub fn partition_classical_cores(total: usize) -> (usize, usize, usize) {
    if total >= 8 {
        let super_cores = (total * 10 / 100).clamp(1, 2);
        let p_cores = (total * 50 / 100).max(1);
        let e_cores = total.saturating_sub(p_cores + super_cores).max(1);
        (p_cores, e_cores, super_cores)
    } else if total >= 4 {
        let p_cores = total / 2;
        let e_cores = total.saturating_sub(p_cores);
        (p_cores, e_cores, 0)
    } else {
        (total.max(1), 0, 0)
    }
}

/// Sets the QoS and core scheduling routing for the current calling thread.
pub fn set_current_thread_qos(qos: ThreadQoS) {
    #[cfg(target_os = "windows")]
    {
        win32_qos::apply_windows_thread_qos(qos);
    }

    #[cfg(target_os = "macos")]
    {
        darwin_qos::apply_darwin_thread_qos(qos);
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = qos;
    }
}

#[cfg(target_os = "windows")]
mod win32_qos {
    use super::ThreadQoS;

    #[repr(C)]
    struct ThreadPowerThrottlingState {
        version: u32,
        control_mask: u32,
        state_mask: u32,
    }

    const THREAD_POWER_THROTTLING: u32 = 1;
    const THREAD_POWER_THROTTLING_EXECUTION_SPEED: u32 = 0x1;
    const THREAD_PRIORITY_ABOVE_NORMAL: i32 = 1;
    const THREAD_PRIORITY_BELOW_NORMAL: i32 = -1;
    const THREAD_PRIORITY_NORMAL: i32 = 0;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentThread() -> isize;
        fn SetThreadPriority(h_thread: isize, n_priority: i32) -> i32;
        fn SetThreadInformation(
            h_thread: isize,
            thread_information_class: u32,
            thread_information: *const std::ffi::c_void,
            thread_information_size: u32,
        ) -> i32;
    }

    pub fn apply_windows_thread_qos(qos: ThreadQoS) {
        #[allow(unused_unsafe)]
        unsafe {
            let t = GetCurrentThread();
            match qos {
                ThreadQoS::Performance => {
                    // Disable execution speed power throttling -> routes to P-cores
                    let state = ThreadPowerThrottlingState {
                        version: 1,
                        control_mask: THREAD_POWER_THROTTLING_EXECUTION_SPEED,
                        state_mask: 0,
                    };
                    let _ = SetThreadInformation(
                        t,
                        THREAD_POWER_THROTTLING,
                        &state as *const ThreadPowerThrottlingState as *const std::ffi::c_void,
                        std::mem::size_of::<ThreadPowerThrottlingState>() as u32,
                    );
                    let _ = SetThreadPriority(t, THREAD_PRIORITY_ABOVE_NORMAL);
                }
                ThreadQoS::Efficiency => {
                    // Enable EcoQoS execution speed throttling -> routes to E-cores
                    let state = ThreadPowerThrottlingState {
                        version: 1,
                        control_mask: THREAD_POWER_THROTTLING_EXECUTION_SPEED,
                        state_mask: THREAD_POWER_THROTTLING_EXECUTION_SPEED,
                    };
                    let _ = SetThreadInformation(
                        t,
                        THREAD_POWER_THROTTLING,
                        &state as *const ThreadPowerThrottlingState as *const std::ffi::c_void,
                        std::mem::size_of::<ThreadPowerThrottlingState>() as u32,
                    );
                    let _ = SetThreadPriority(t, THREAD_PRIORITY_BELOW_NORMAL);
                }
                ThreadQoS::Default => {
                    let _ = SetThreadPriority(t, THREAD_PRIORITY_NORMAL);
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
mod win32_topology {
    use super::CpuTopology;

    const RELATION_PROCESSOR_CORE: u32 = 0;

    #[repr(C)]
    struct ProcessorCoreInfo {
        flags: u8,
        efficiency_class: u8,
        reserved: [u8; 20],
        group_count: u16,
    }

    #[repr(C)]
    struct SystemLogicalProcessorInformationEx {
        relationship: u32,
        size: u32,
        processor_core: ProcessorCoreInfo,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetLogicalProcessorInformationEx(
            relationship_type: u32,
            buffer: *mut u8,
            returned_length: *mut u32,
        ) -> i32;
    }

    pub fn detect_windows_topology(total: usize, cpu_name: String) -> CpuTopology {
        let mut len: u32 = 0;
        #[allow(unused_unsafe)]
        unsafe {
            let _ = GetLogicalProcessorInformationEx(
                RELATION_PROCESSOR_CORE,
                std::ptr::null_mut(),
                &mut len,
            );
        }
        if len == 0 {
            let (p_cores, e_cores, super_cores) = super::partition_classical_cores(total);
            return CpuTopology {
                total_logical_cores: total,
                physical_cores: (total / 2).max(1),
                p_cores,
                e_cores,
                super_cores,
                is_hybrid: false,
                cpu_name,
            };
        }

        let mut buf = vec![0u8; len as usize];
        let ok = unsafe {
            GetLogicalProcessorInformationEx(RELATION_PROCESSOR_CORE, buf.as_mut_ptr(), &mut len)
        };
        if ok == 0 {
            let (p_cores, e_cores, super_cores) = super::partition_classical_cores(total);
            return CpuTopology {
                total_logical_cores: total,
                physical_cores: (total / 2).max(1),
                p_cores,
                e_cores,
                super_cores,
                is_hybrid: false,
                cpu_name,
            };
        }

        let mut p_count = 0usize;
        let mut e_count = 0usize;
        let mut super_count = 0usize;
        let mut physical_cores = 0usize;
        let mut offset = 0usize;
        while offset + std::mem::size_of::<SystemLogicalProcessorInformationEx>() <= buf.len() {
            let info = unsafe {
                &*(buf.as_ptr().add(offset) as *const SystemLogicalProcessorInformationEx)
            };
            if info.size == 0 {
                break;
            }
            if info.relationship == RELATION_PROCESSOR_CORE {
                physical_cores += 1;
                match info.processor_core.efficiency_class {
                    0 => e_count += 1,
                    1 => p_count += 1,
                    _ => super_count += 1,
                }
            }
            offset += info.size as usize;
        }

        if (p_count > 0 && e_count > 0) || (super_count > 0 && (p_count > 0 || e_count > 0)) {
            CpuTopology {
                total_logical_cores: total,
                physical_cores: physical_cores.max(1),
                p_cores: p_count.max(1),
                e_cores: e_count,
                super_cores: super_count,
                is_hybrid: true,
                cpu_name,
            }
        } else {
            // Classical homogeneous processor (e.g. AMD Ryzen 4800H 8C/16T, Intel 10th Gen)
            let (p_cores, e_cores, super_cores) = super::partition_classical_cores(total);
            CpuTopology {
                total_logical_cores: total,
                physical_cores: physical_cores.max((total / 2).max(1)),
                p_cores,
                e_cores,
                super_cores,
                is_hybrid: false,
                cpu_name,
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod darwin_qos {
    use super::ThreadQoS;

    const QOS_CLASS_USER_INTERACTIVE: u32 = 0x21;
    const QOS_CLASS_BACKGROUND: u32 = 0x09;
    const QOS_CLASS_DEFAULT: u32 = 0x15;

    extern "C" {
        fn pthread_set_qos_class_self_np(qos_class: u32, relative_priority: i32) -> i32;
    }

    pub fn apply_darwin_thread_qos(qos: ThreadQoS) {
        let code = match qos {
            ThreadQoS::Performance => QOS_CLASS_USER_INTERACTIVE,
            ThreadQoS::Efficiency => QOS_CLASS_BACKGROUND,
            ThreadQoS::Default => QOS_CLASS_DEFAULT,
        };
        #[allow(unused_unsafe)]
        unsafe {
            let _ = pthread_set_qos_class_self_np(code, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topology_detection_sane() {
        let topo = detect_cpu_topology();
        assert!(topo.total_logical_cores >= 1);
        assert!(topo.p_cores >= 1);
        let desc = topo.description();
        assert!(!desc.is_empty());
    }

    #[test]
    fn test_thread_qos_calls_without_panic() {
        set_current_thread_qos(ThreadQoS::Performance);
        set_current_thread_qos(ThreadQoS::Efficiency);
        set_current_thread_qos(ThreadQoS::Default);
    }
}
