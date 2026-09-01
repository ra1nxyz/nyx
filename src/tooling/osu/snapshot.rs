use serde::{Deserialize, Serialize};
use std::path::Path;

const SNAPSHOT_PATH: &str = "data/osu_rank.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct RankSnapshot {
    pub user_id: u64,
    pub username: String,
    pub global_rank: Option<u32>,
    pub pp: f64,
    pub last_visit: Option<String>,
    pub checked_at: String,
}

pub fn rank_change(
    old: Option<u32>,
    new: Option<u32>,
) -> Option<i32> {
    match (old, new) {
        (Some(old), Some(new)) => {
            Some(old as i32 - new as i32)
        }
        _ => None,
    }
}

pub fn load() -> Result<Option<RankSnapshot>, Box<dyn std::error::Error + Send + Sync>> {
    if !Path::new(SNAPSHOT_PATH).exists() {
        return Ok(None);
    }

    let data = std::fs::read_to_string(SNAPSHOT_PATH)?;
    let snapshot = serde_json::from_str(&data)?;

    Ok(Some(snapshot))
}

pub fn save(
    snapshot: &RankSnapshot,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::fs::create_dir_all("data")?;

    let data = serde_json::to_string_pretty(snapshot)?;

    std::fs::write(SNAPSHOT_PATH, data)?;

    Ok(())
}