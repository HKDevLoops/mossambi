//! Conversion-route command handler (Dijkstra shortest path).
//! Wraps core::route_planner. Pure ASCII by contract.

use anyhow::{anyhow, Result};
use serde_json::json;

use crate::core::route_planner::find_conversion_route;

pub fn handle_route(from: &str, to: &str, as_json: bool) -> Result<()> {
    let route = find_conversion_route(from, to)
        .ok_or_else(|| anyhow!("no conversion route from '{}' to '{}'", from, to))?;

    let steps: Vec<String> = route.steps.iter().map(|s| format!("{:?}", s)).collect();
    let path = route.display_path();

    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "from": from,
                "to": to,
                "hops": route.hop_count(),
                "cost": route.total_cost,
                "path": path,
                "steps": steps
            }))?
        );
    } else {
        println!("Route: {}", path);
        println!("  Hops: {}  Cost: {}", route.hop_count(), route.total_cost);
    }
    Ok(())
}
