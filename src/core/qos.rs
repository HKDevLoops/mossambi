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

/// Discovered CPU core topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuTopology {
    /// Total logical processing units available.
    pub total_logical_cores: usize,
    /// Number of identified or estimated Performance cores.
    pub p_cores: usize,
    /// Number of identified or estimated Efficiency cores.
    pub e_cores: usize,
    /// True when heterogeneous core classes are detected.
    pub is_hybrid: bool,
}

impl CpuTopology {
    /// Formats a descriptive label for UI settings.
    pub fn description(&self) -> String {
        if self.is_hybrid {
            format!(
                "Hybrid ({} cores: {} P-cores + {} E-cores)",
                self.total_logical_cores, self.p_cores, self.e_cores
            )
        } else {
            format!("{} cores", self.total_logical_cores)
        }
    }
}

/// Detects CPU topology and core distribution using system APIs.
pub fn detect_cpu_topology() -> CpuTopology {
    let total = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4);

    #[cfg(target_os = "windows")]
    {
        win32_topology::detect_windows_topology(total)
    }

    #[cfg(not(target_os = "windows"))]
    {
        CpuTopology {
            total_logical_cores: total,
            p_cores: total,
            e_cores: 0,
            is_hybrid: false,
        }
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

    pub fn detect_windows_topology(total: usize) -> CpuTopology {
        let mut len: u32 = 0;
        unsafe {
            let _ = GetLogicalProcessorInformationEx(
                RELATION_PROCESSOR_CORE,
                std::ptr::null_mut(),
                &mut len,
            );
        }
        if len == 0 {
            return CpuTopology {
                total_logical_cores: total,
                p_cores: total,
                e_cores: 0,
                is_hybrid: false,
            };
        }

        let mut buf = vec![0u8; len as usize];
        let ok = unsafe {
            GetLogicalProcessorInformationEx(
                RELATION_PROCESSOR_CORE,
                buf.as_mut_ptr(),
                &mut len,
            )
        };
        if ok == 0 {
            return CpuTopology {
                total_logical_cores: total,
                p_cores: total,
                e_cores: 0,
                is_hybrid: false,
            };
        }

        let mut p_count = 0usize;
        let mut e_count = 0usize;
        let mut offset = 0usize;
        while offset + std::mem::size_of::<SystemLogicalProcessorInformationEx>() <= buf.len() {
            let info = unsafe {
                &*(buf.as_ptr().add(offset) as *const SystemLogicalProcessorInformationEx)
            };
            if info.size == 0 {
                break;
            }
            if info.relationship == RELATION_PROCESSOR_CORE {
                if info.processor_core.efficiency_class == 0 {
                    e_count += 1;
                } else {
                    p_count += 1;
                }
            }
            offset += info.size as usize;
        }

        if p_count > 0 && e_count > 0 {
            CpuTopology {
                total_logical_cores: total,
                p_cores: p_count,
                e_cores: e_count,
                is_hybrid: true,
            }
        } else {
            CpuTopology {
                total_logical_cores: total,
                p_cores: total,
                e_cores: 0,
                is_hybrid: false,
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
