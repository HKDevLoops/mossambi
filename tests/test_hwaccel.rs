use mossambi::hwaccel::{detect_hardware_capabilities, init_multi_threading};

#[test]
fn test_hw_detection_non_empty() {
    let hw = detect_hardware_capabilities();
    assert!(!hw.ffmpeg_hwaccel_flag.is_empty());
    assert!(!hw.h264_encoder.is_empty());
    assert!(!hw.hevc_encoder.is_empty());
    assert!(!hw.npu_name.is_empty());
    assert!(hw.optimal_thread_count > 0);

    let threads = init_multi_threading(Some(4));
    assert_eq!(threads, 4);
}
