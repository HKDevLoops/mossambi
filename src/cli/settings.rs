//! Generic settings command handler: get/set/list every configuration field.
//! Serializes MossambiConfig to serde_json::Value so all fields (appearance,
//! Fields, motion, donut, ...) are covered without hand-maintaining a key list.
//! Pure ASCII by contract.

use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};

use crate::cli::SettingsCommand;
use crate::core::MossambiConfig;

fn config_value() -> Result<Value> {
    serde_json::to_value(MossambiConfig::load())
        .map_err(|e| anyhow!("failed to serialize configuration: {}", e))
}

fn get_dotted<'a>(root: &'a Value, key: &str) -> Option<&'a Value> {
    let mut cur = root;
    for seg in key.split('.') {
        cur = cur.get(seg)?;
    }
    Some(cur)
}

fn set_dotted(root: &mut Value, key: &str, value: Value) -> Result<()> {
    let segs: Vec<&str> = key.split('.').collect();
    if segs.is_empty() {
        bail!("empty settings key");
    }
    let mut cur = root;
    for seg in &segs[..segs.len() - 1] {
        let next = cur
            .get_mut(*seg)
            .ok_or_else(|| anyhow!("unknown settings group '{}'", seg))?;
        cur = next;
    }
    let leaf = segs[segs.len() - 1];
    let obj = cur
        .as_object_mut()
        .ok_or_else(|| anyhow!("settings key '{}' is not an object", key))?;
    if !obj.contains_key(leaf) {
        bail!("unknown settings key '{}'", key);
    }
    obj.insert(leaf.to_string(), value);
    Ok(())
}

fn parse_value(raw: &str) -> Value {
    serde_json::from_str::<Value>(raw).unwrap_or_else(|_| Value::String(raw.to_string()))
}

fn flatten(prefix: &str, value: &Value, out: &mut Vec<(String, String)>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let path = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                flatten(&path, v, out);
            }
        }
        Value::Array(_) => out.push((prefix.to_string(), value.to_string())),
        other => {
            let s = match other {
                Value::String(s) => s.clone(),
                _ => other.to_string(),
            };
            out.push((prefix.to_string(), s));
        }
    }
}

pub fn handle_settings(action: SettingsCommand, as_json: bool) -> Result<()> {
    match action {
        SettingsCommand::List { prefix } => {
            let root = config_value()?;
            let base = match prefix {
                Some(ref p) => get_dotted(&root, p)
                    .ok_or_else(|| anyhow!("unknown settings prefix '{}'", p))?,
                None => &root,
            };
            let mut flat = Vec::new();
            let key_prefix = prefix.unwrap_or_default();
            flatten(&key_prefix, base, &mut flat);
            flat.sort_by(|a, b| a.0.cmp(&b.0));

            if as_json {
                let obj: serde_json::Map<String, Value> = flat
                    .into_iter()
                    .map(|(k, v)| (k, Value::String(v)))
                    .collect();
                println!("{}", serde_json::to_string_pretty(&Value::Object(obj))?);
            } else {
                for (k, v) in &flat {
                    println!("{} = {}", k, v);
                }
            }
            Ok(())
        }
        SettingsCommand::Get { key } => {
            let root = config_value()?;
            let value =
                get_dotted(&root, &key).ok_or_else(|| anyhow!("unknown settings key '{}'", key))?;
            if as_json {
                println!("{}", serde_json::to_string_pretty(value)?);
            } else {
                match value {
                    Value::String(s) => println!("{}", s),
                    other => println!("{}", other),
                }
            }
            Ok(())
        }
        SettingsCommand::Set { key, value } => {
            let mut root = config_value()?;
            let new_value = parse_value(&value);
            set_dotted(&mut root, &key, new_value)?;
            let cfg: MossambiConfig = serde_json::from_value(root)
                .map_err(|e| anyhow!("failed to apply settings change: {}", e))?;
            cfg.save()?;
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "status": "saved", "key": key, "value": value
                    }))?
                );
            } else {
                println!("Saved: {} = {}", key, value);
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_dotted_navigates_nested_keys() {
        let v = json!({ "a": { "b": { "c": 7 } }, "list": [1, 2] });
        assert_eq!(get_dotted(&v, "a.b.c"), Some(&json!(7)));
        assert_eq!(get_dotted(&v, "list"), Some(&json!([1, 2])));
        assert_eq!(get_dotted(&v, "a.x"), None);
    }

    #[test]
    fn set_dotted_updates_leaf_and_rejects_unknown() {
        let mut v = json!({ "a": { "b": 1 } });
        assert!(set_dotted(&mut v, "a.b", json!(2)).is_ok());
        assert_eq!(get_dotted(&v, "a.b"), Some(&json!(2)));
        assert!(set_dotted(&mut v, "a.missing", json!(3)).is_err());
    }

    #[test]
    fn parse_value_handles_json_and_plain_strings() {
        assert_eq!(parse_value("42"), json!(42));
        assert_eq!(parse_value("true"), json!(true));
        assert_eq!(parse_value("rangalipi"), json!("rangalipi"));
    }
}
