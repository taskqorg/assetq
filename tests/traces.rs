//! Replays every lifecycle trace through the executable model (R6, A4).

#[test]
fn every_trace_replays() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/traces");
    let on_disk = std::fs::read_dir(&dir).unwrap().filter(|e| e.as_ref().unwrap().path().extension().is_some_and(|x| x == "json")).count();
    assert_eq!(on_disk, pipe::traces::TRACES.len(), "embed every trace file in src/traces.rs");
    let failures: Vec<String> =
        pipe::traces::TRACES.iter().filter_map(|(name, raw)| pipe::traces::run(raw).err().map(|e| format!("{name}: {e}"))).collect();
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn trace_runner_detects_a_wrong_expectation() {
    let happy = pipe::traces::TRACES.iter().find(|(n, _)| *n == "happy-path.json").unwrap().1;
    let raw = happy.replace("\"succeeded\"", "\"failed\"");
    assert!(pipe::traces::run(&raw).is_err());
}

#[test]
fn every_capacity_trace_replays() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/traces/capacity");
    assert_eq!(
        std::fs::read_dir(&dir).unwrap().count(),
        pipe::traces::CAPACITY_TRACES.len(),
        "embed every capacity trace in src/traces.rs"
    );
    let failures: Vec<String> = pipe::traces::CAPACITY_TRACES
        .iter()
        .filter_map(|(name, raw)| pipe::image_worker::capacity::replay(raw).err().map(|e| format!("{name}: {e}")))
        .collect();
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn capacity_trace_runner_detects_a_wrong_expectation() {
    let trace = pipe::traces::CAPACITY_TRACES.iter().find(|(n, _)| *n == "cancel-retains-occupancy.json").unwrap().1;
    assert!(pipe::image_worker::capacity::replay(&trace.replace("\"launched\"", "\"free\"")).is_err());
    assert!(pipe::image_worker::capacity::replay(&trace.replace("\"reject\":\"unproved\"", "\"note\":\"\"")).is_err());
}
