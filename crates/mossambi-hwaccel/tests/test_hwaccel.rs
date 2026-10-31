use mossambi_hwaccel::detect_hardware_capabilities;

#[test]
fn test_hw_detection_non_empty() {
    let hw = detect_hardware_capabilities();
    assert!(!hw.ffmpeg_hwaccel_flag.is_empty());
    assert!(!hw.h264_encoder.is_empty());
    assert!(!hw.hevc_encoder.is_empty());
}
