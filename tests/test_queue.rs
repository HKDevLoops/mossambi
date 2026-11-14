//! Conversion queue reducer + ffmpeg progress protocol parser tests (W-7).

use mossambi::codec::{parse_ffmpeg_speed, FfmpegProgressParser};
use mossambi::core::conversion_queue::{ConversionQueue, QueueItemStatus};
use std::path::PathBuf;

fn sample_files(n: usize) -> Vec<PathBuf> {
    (1..=n)
        .map(|i| PathBuf::from(format!("C:/tmp/file{}.png", i)))
        .collect()
}

#[test]
fn test_queue_enqueue_and_summary() {
    let mut queue = ConversionQueue::new();
    let staged = queue.enqueue_batch(sample_files(3), "FLAC");
    assert_eq!(staged.len(), 3);
    assert_eq!(staged[0].0, 1); // ids start at 1
    let summary = queue.summary();
    assert_eq!(summary.queued, 3);
    assert_eq!(summary.total(), 3);
    assert!(queue.has_active());
}

#[test]
fn test_queue_progress_promotes_and_clamps() {
    let mut queue = ConversionQueue::new();
    let staged = queue.enqueue_batch(sample_files(1), "MP3");
    let (id, _) = staged[0].clone();

    // First progress promotes Queued -> Running and clamps 0..=1.
    queue.on_progress(id, 2.5, "speed 1.5x");
    let items = queue.items();
    assert_eq!(items[0].status, QueueItemStatus::Running);
    assert!((items[0].progress - 1.0).abs() < f32::EPSILON);
    assert_eq!(items[0].status_text, "speed 1.5x");

    queue.on_progress(id, 0.5, "50% | speed 1.5x");
    let items = queue.items();
    assert!((items[0].progress - 0.5).abs() < f32::EPSILON);
}

#[test]
fn test_queue_done_failed_terminal_idempotence() {
    let mut queue = ConversionQueue::new();
    let staged = queue.enqueue_batch(sample_files(2), "WAV");
    let (id_a, _) = staged[0].clone();
    let (id_b, _) = staged[1].clone();

    queue.on_item_done(id_a, vec![PathBuf::from("C:/tmp/out.wav")], "Converted");
    let items = queue.items();
    assert_eq!(items[0].status, QueueItemStatus::Done);
    assert_eq!(items[0].outputs.len(), 1);
    assert!((items[0].progress - 1.0).abs() < f32::EPSILON);

    // Terminal states ignore further updates.
    queue.on_item_failed(id_a, "late error");
    assert_eq!(queue.items()[0].status, QueueItemStatus::Done);

    queue.on_item_failed(id_b, "ffmpeg failed: bad args");
    let items = queue.items();
    assert_eq!(items[1].status, QueueItemStatus::Failed);
    assert!(items[1].error_detail.as_deref().unwrap().contains("ffmpeg"));
}

#[test]
fn test_queue_cancel_transitions() {
    let mut queue = ConversionQueue::new();
    let staged = queue.enqueue_batch(sample_files(3), "MP4");
    let (running_id, _) = staged[0].clone();

    queue.on_progress(running_id, 0.4, "40% | speed 2.0x");
    queue.request_cancel();

    let items = queue.items();
    // Running item shows the transitional Stopping chip.
    assert_eq!(items[0].status, QueueItemStatus::Stopping);
    // Queued items are cancelled outright.
    assert_eq!(items[1].status, QueueItemStatus::Cancelled);
    assert_eq!(items[2].status, QueueItemStatus::Cancelled);

    // The worker confirms the stop.
    queue.on_item_cancelled(running_id);
    assert_eq!(queue.items()[0].status, QueueItemStatus::Cancelled);
}

#[test]
fn test_queue_task_stalled() {
    let mut queue = ConversionQueue::new();
    let staged = queue.enqueue_batch(sample_files(3), "FLAC");
    let (running_id, _) = staged[0].clone();
    queue.on_progress(running_id, 0.2, "20%");

    queue.on_task_stalled("Task 'StudioConversion' exceeded 30 seconds without progress.");

    let items = queue.items();
    assert_eq!(items[0].status, QueueItemStatus::Failed);
    assert!(items[0]
        .error_detail
        .as_deref()
        .unwrap()
        .contains("30 seconds"));
    // Queued items never get outcomes -> Cancelled.
    assert_eq!(items[1].status, QueueItemStatus::Cancelled);
    assert_eq!(items[2].status, QueueItemStatus::Cancelled);
}

#[test]
fn test_queue_retry_failed() {
    let mut queue = ConversionQueue::new();
    let staged = queue.enqueue_batch(sample_files(2), "FLAC");
    let (id_a, _) = staged[0].clone();
    let (id_b, _) = staged[1].clone();

    queue.on_item_failed(id_a, "boom");
    queue.on_item_done(id_b, Vec::new(), "Converted");

    let retry = queue.take_failed_for_retry();
    assert_eq!(retry.len(), 1);
    assert_eq!(retry[0].0, id_a);
    assert_eq!(retry[0].2, "FLAC");

    // The failed row is re-queued with diagnostics cleared.
    let items = queue.items();
    assert_eq!(items[0].status, QueueItemStatus::Queued);
    assert!(items[0].error_detail.is_none());
    assert_eq!(items[1].status, QueueItemStatus::Done);

    // Second retry pass returns nothing.
    assert!(queue.take_failed_for_retry().is_empty());
}

#[test]
fn test_queue_clear_finished_and_all_outputs() {
    let mut queue = ConversionQueue::new();
    let staged = queue.enqueue_batch(sample_files(3), "WAV");
    let (id_a, _) = staged[0].clone();
    let (id_b, _) = staged[1].clone();
    let (id_c, _) = staged[2].clone();

    queue.on_item_done(id_a, vec![PathBuf::from("C:/tmp/a.wav")], "Converted");
    queue.on_item_failed(id_b, "boom");
    queue.on_progress(id_c, 0.1, "10%");

    assert_eq!(queue.all_outputs().len(), 1);

    queue.clear_finished();
    let items = queue.items();
    // Only the still-active item remains.
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].status, QueueItemStatus::Running);
    assert_eq!(queue.summary().done, 0);
    assert!(queue.all_outputs().is_empty());
}

#[test]
fn test_queue_planned_route() {
    let mut queue = ConversionQueue::new();
    let staged = queue.enqueue_batch(sample_files(1), "PDF");
    let (id, _) = staged[0].clone();

    queue.set_planned_route(id, "DOCX -> PDF".to_string());
    assert_eq!(
        queue.items()[0].planned_route.as_deref(),
        Some("DOCX -> PDF")
    );

    // Unknown ids are ignored safely.
    queue.set_planned_route(999, "nope".to_string());
}

#[test]
fn test_ffmpeg_progress_parser_determinate() {
    let mut parser = FfmpegProgressParser::new(Some(100.0));
    assert_eq!(parser.feed_line("frame=123"), None);
    assert_eq!(parser.feed_line("fps=45"), None);
    assert_eq!(parser.feed_line("out_time_us=50000000"), None);
    assert_eq!(parser.feed_line("speed=1.5x"), None);

    let (fraction, text) = parser.feed_line("progress=continue").unwrap();
    assert!((fraction - 0.5).abs() < 0.001);
    assert!(text.contains("50%"));
    assert!(text.contains("speed 1.50x"));

    // Each block starts fresh.
    assert_eq!(parser.feed_line("out_time_us=100000000"), None);
    let (fraction, _) = parser.feed_line("progress=end").unwrap();
    assert!((fraction - 1.0).abs() < 0.001);
}

#[test]
fn test_ffmpeg_progress_parser_indeterminate() {
    let mut parser = FfmpegProgressParser::new(None);
    assert!(!parser.has_duration());
    assert_eq!(parser.feed_line("out_time_us=999999"), None);
    assert_eq!(parser.feed_line("speed=N/A"), None); // rejected -> no speed
    let (fraction, text) = parser.feed_line("progress=continue").unwrap();
    assert_eq!(fraction, 0.0);
    assert!(text.contains("working"));
}

#[test]
fn test_ffmpeg_progress_parser_out_time_ms_is_micros() {
    let mut parser = FfmpegProgressParser::new(Some(10.0));
    assert_eq!(parser.feed_line("total_size=12345"), None);
    assert_eq!(parser.feed_line("no_equals_sign_here"), None);
    // out_time_ms is a historical misnomer: the value is microseconds.
    assert_eq!(parser.feed_line("out_time_ms=2500000"), None);
    let (fraction, _) = parser.feed_line("progress=continue").unwrap();
    assert!((fraction - 0.25).abs() < 0.001);
}

#[test]
fn test_parse_ffmpeg_speed() {
    assert_eq!(parse_ffmpeg_speed("1.5x"), Some(1.5));
    assert_eq!(parse_ffmpeg_speed("2x"), Some(2.0));
    assert_eq!(parse_ffmpeg_speed("0.25x"), Some(0.25));
    assert_eq!(parse_ffmpeg_speed("N/A"), None);
    assert_eq!(parse_ffmpeg_speed("garbage"), None);
}
