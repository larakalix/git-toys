//! Measure bounded history reads against an existing synthetic repository.
use git_toys_engine::{types::HistoryQuery, Repository};
use std::{path::Path, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("Pass a repository directory")?;
    let start = Instant::now();
    let repo = Repository::open(Path::new(&path))?;
    let open = start.elapsed();
    let start = Instant::now();
    let commits = repo.history(&HistoryQuery {
        revision: None,
        search: None,
        path: None,
        skip: 0,
        limit: 200,
    })?;
    let elapsed = start.elapsed();
    println!(
        "{}",
        serde_json::json!({"rows":commits.len(),"openMs":open.as_secs_f64()*1000.0,"historyMs":elapsed.as_secs_f64()*1000.0,"serializedBytes":serde_json::to_vec(&commits)?.len()})
    );
    if commits.len() != 200 || elapsed.as_secs_f64() > 2.0 {
        return Err("History acceptance target was not met".into());
    }
    Ok(())
}
