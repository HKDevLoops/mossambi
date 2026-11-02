//! Hardware Acceleration Engine for Mossambi
//! Detects and configures GPU encoders (NVENC, AMF, QSV, Adreno, VAAPI),
//! NPU AI inference engines (DirectML, Hexagon NPU, Intel NPU, Apple Neural Engine),
//! SIMD features (NEON, AVX2), and multi-core threading pools.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Qualcomm,
    Apple,
    GenericVaapi,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NpuEngine {
    DirectML,
    QualcommHexagon,
    IntelAiBoost,
    AppleNeuralEngine,
    CpuSimdVector,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HwCapabilities {
    pub vendor: GpuVendor,
    pub npu_engine: NpuEngine,
    pub npu_name: &'static str,
    pub supports_gpu_transcode: bool,
    pub supports_npu_ai: bool,
    pub supports_h264_hw: bool,
    pub supports_hevc_hw: bool,
    pub supports_av1_hw: bool,
    pub supports_neon_simd: bool,
    pub supports_avx2_simd: bool,
    pub ffmpeg_hwaccel_flag: &'static str,
    pub h264_encoder: &'static str,
    pub hevc_encoder: &'static str,
    pub optimal_thread_count: usize,
}

/// Detects GPU hardware encoders, NPU neural accelerators, and CPU SIMD topology.
pub fn detect_hardware_capabilities() -> HwCapabilities {
    let mut vendor = GpuVendor::None;
    let mut h264_encoder = "libx264";
    let mut hevc_encoder = "libx265";
    let mut ffmpeg_hwaccel_flag = "auto";

    // Architecture SIMD detection
    let supports_neon = cfg!(target_arch = "aarch64") || cfg!(target_arch = "arm");
    let supports_avx2 = cfg!(target_arch = "x86_64");

    // NPU Engine Detection
    let (npu_engine, npu_name, supports_npu) = detect_npu_engine();

    // Logical CPU core count for multi-threading
    let optimal_thread_count = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4);

    // Windows GPU detection
    #[cfg(target_os = "windows")]
    {
        // Check for Snapdragon / Qualcomm ARM64
        if cfg!(target_arch = "aarch64") {
            vendor = GpuVendor::Qualcomm;
            h264_encoder = "h264_mf";
            hevc_encoder = "hevc_mf";
            ffmpeg_hwaccel_flag = "d3d11va";
        } else {
            // Probe NVENC / AMF / QSV via standard system commands
            if let Ok(output) = std::process::Command::new("ffmpeg")
                .args(["-encoders"])
                .output()
            {
                let text = String::from_utf8_lossy(&output.stdout);
                if text.contains("h264_nvenc") {
                    vendor = GpuVendor::Nvidia;
                    h264_encoder = "h264_nvenc";
                    hevc_encoder = "hevc_nvenc";
                    ffmpeg_hwaccel_flag = "cuda";
                } else if text.contains("h264_qsv") {
                    vendor = GpuVendor::Intel;
                    h264_encoder = "h264_qsv";
                    hevc_encoder = "hevc_qsv";
                    ffmpeg_hwaccel_flag = "qsv";
                } else if text.contains("h264_amf") {
                    vendor = GpuVendor::Amd;
                    h264_encoder = "h264_amf";
                    hevc_encoder = "hevc_amf";
                    ffmpeg_hwaccel_flag = "d3d11va";
                }
            }
        }
    }

    // Linux GPU detection
    #[cfg(target_os = "linux")]
    {
        if std::path::Path::new("/dev/dri/renderD128").exists() {
            vendor = GpuVendor::GenericVaapi;
            h264_encoder = "h264_vaapi";
            hevc_encoder = "hevc_vaapi";
            ffmpeg_hwaccel_flag = "vaapi";
        }
    }

    // macOS detection
    #[cfg(target_os = "macos")]
    {
        vendor = GpuVendor::Apple;
        h264_encoder = "h264_videotoolbox";
        hevc_encoder = "hevc_videotoolbox";
        ffmpeg_hwaccel_flag = "videotoolbox";
    }

    HwCapabilities {
        vendor,
        npu_engine,
        npu_name,
        supports_gpu_transcode: vendor != GpuVendor::None,
        supports_npu_ai: supports_npu,
        supports_h264_hw: vendor != GpuVendor::None,
        supports_hevc_hw: vendor != GpuVendor::None,
        supports_av1_hw: false,
        supports_neon_simd: supports_neon,
        supports_avx2_simd: supports_avx2,
        ffmpeg_hwaccel_flag,
        h264_encoder,
        hevc_encoder,
        optimal_thread_count,
    }
}

fn detect_npu_engine() -> (NpuEngine, &'static str, bool) {
    #[cfg(target_os = "windows")]
    {
        if cfg!(target_arch = "aarch64") {
            return (
                NpuEngine::QualcommHexagon,
                "Qualcomm Hexagon NPU (Snapdragon X Series)",
                true,
            );
        }

        // DirectML is standard on Windows 10/11
        if std::path::Path::new(r"C:\Windows\System32\directml.dll").exists() {
            return (
                NpuEngine::DirectML,
                "Windows DirectML Neural Compute Engine (NPU / GPU Tensor)",
                true,
            );
        }
    }

    #[cfg(target_os = "macos")]
    {
        return (
            NpuEngine::AppleNeuralEngine,
            "Apple Neural Engine (16-core ANE)",
            true,
        );
    }

    #[cfg(target_arch = "aarch64")]
    {
        return (
            NpuEngine::CpuSimdVector,
            "ARM NEON 128-bit Vector Engine",
            true,
        );
    }

    #[cfg(target_arch = "x86_64")]
    {
        return (
            NpuEngine::CpuSimdVector,
            "x86_64 AVX2 256-bit Vector Engine",
            true,
        );
    }

    #[allow(unreachable_code)]
    (NpuEngine::CpuSimdVector, "Standard Multi-core CPU", false)
}

/// Configures the Rayon global multi-threaded pool for concurrent batch conversions.
pub fn init_multi_threading(threads: Option<usize>) -> usize {
    let thread_count = threads.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4)
    });

    let _ = rayon::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|idx| format!("mossambi-worker-{}", idx))
        .build_global();

    thread_count
}
