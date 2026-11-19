//! Conversion queue command handlers: add, list, run.
//! Persists a lightweight job list to disk and wraps core ConversionQueue
//! semantics plus the codec transcode entry point. Pure ASCII by contract.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

use crate::cli::QueueCommand;
use crate::codec::{transcode_file, LosslessMode};
use crate::core::config_dir;

/// A single persisted queue job. The core `QueueItemStatus` is not serializable,
/// so status is stored as its label string.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueueJob {
    id: usize,
    source: PathBuf,
    target: String,
    status: String,
}

fn queue_path() -> PathBuf {
    config_dir().join("cli_queue.json")
}

fn load_jobs() -> Result<Vec<QueueJob>> {
    let path = queue_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| anyhow!("failed to read conversion queue: {}", e))?;
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&raw).map_err(|e| anyhow!("corrupt conversion queue: {}", e))
}

fn save_jobs(jobs: &[QueueJob]) -> Result<()> {
    let path = queue_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| anyhow!("failed to create queue directory: {}", e))?;
    }
    let raw = serde_json::to_string_pretty(jobs)?;
    std::fs::write(&path, raw).map_err(|e| anyhow!("failed to write conversion queue: {}", e))
}

pub fn handle_queue(action: QueueCommand, as_json: bool) -> Result<()> {
    match action {
        QueueCommand::Add { files, target } => {
            let mut jobs = load_jobs()?;
            let next_id = jobs.iter().map(|j| j.id).max().unwrap_or(0) + 1;

            // Wrap the core queue reducer to derive stable ids and validate the batch.
            let mut core_queue = crate::core::ConversionQueue::new();
            let staged = core_queue.enqueue_batch(files.clone(), &target);

            for (offset, (_, source)) in staged.into_iter().enumerate() {
                jobs.push(QueueJob {
                    id: next_id + offset,
                    source,
                    target: target.clone(),
                    status: "Queued".to_string(),
                });
            }
            save_jobs(&jobs)?;

            let added: Vec<String> = jobs
                .iter()
                .rev()
                .take(files.len())
                .map(|j| format!("#{} {} -> {}", j.id, j.source.display(), j.target))
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "status": "success", "operation": "queue-add", "added": added.len()
                    }))?
                );
            } else {
                for line in &added {
                    println!("Queued {}", line);
                }
            }
            Ok(())
        }
        QueueCommand::List => {
            let jobs = load_jobs()?;
            if as_json {
                let items: Vec<Value> = jobs.iter().map(job_to_json).collect();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "count": items.len(), "jobs": items
                    }))?
                );
            } else {
                if jobs.is_empty() {
                    println!("Conversion queue is empty.");
                } else {
                    println!("Conversion queue ({} job(s)):", jobs.len());
                    for j in &jobs {
                        println!(
                            "  #{} [{}] {} -> {}",
                            j.id,
                            j.status,
                            j.source.display(),
                            j.target
                        );
                    }
                }
            }
            Ok(())
        }
        QueueCommand::Run => {
            let mut jobs = load_jobs()?;
            for job in jobs.iter_mut() {
                if job.status == "Done" {
                    continue;
                }
                if !job.source.exists() {
                    job.status = "Failed".to_string();
                    continue;
                }
                match transcode_file(&job.source, &job.target, LosslessMode::StrictBitExact, None) {
                    Ok(output) => {
                        job.status = "Done".to_string();
                        if !as_json {
                            println!("#{} done: {}", job.id, output.display());
                        }
                    }
                    Err(e) => {
                        job.status = format!("Failed: {}", e);
                        if !as_json {
                            eprintln!("#{} failed: {}", job.id, e);
                        }
                    }
                }
            }
            save_jobs(&jobs)?;
            if as_json {
                let items: Vec<Value> = jobs.iter().map(job_to_json).collect();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({ "jobs": items }))?
                );
            } else {
                println!("Conversion queue run complete.");
            }
            Ok(())
        }
    }
}

type Value = serde_json::Value;

fn job_to_json(j: &QueueJob) -> Value {
    json!({
        "id": j.id,
        "source": j.source.display().to_string(),
        "target": j.target,
        "status": j.status
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_serialization_roundtrip() {
        let job = QueueJob {
            id: 7,
            source: PathBuf::from("a.txt"),
            target: "pdf".to_string(),
            status: "Queued".to_string(),
        };
        let raw = serde_json::to_string(&job).unwrap();
        let back: QueueJob = serde_json::from_str(&raw).unwrap();
        assert_eq!(back.id, 7);
        assert_eq!(back.target, "pdf");
        assert_eq!(back.status, "Queued");
    }
}
