use cep_indexer::{build_index, SearchEngine};
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

fn temporary_index_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("opencep-benchmark-{}-{nonce}", std::process::id()))
}

#[test]
#[ignore = "diagnostic benchmark; run explicitly with --release --ignored --nocapture"]
fn measures_repeated_fixture_searches() {
    const ITERATIONS: u32 = 10_000;

    let index_path = temporary_index_path();
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    build_index(&fixtures, &index_path, "benchmark-fixture").expect("fixture index must build");
    let engine = SearchEngine::open(&index_path).expect("fixture index must open");
    let started = Instant::now();

    for _ in 0..ITERATIONS {
        let results = engine
            .search("SP", "Sao Paulo", "Paulsta", 10)
            .expect("fixture search must succeed");
        assert_eq!(results[0].cep, "01310-100");
    }

    let elapsed = started.elapsed();
    println!(
        "{ITERATIONS} fixture searches in {elapsed:?}; average {:?}",
        elapsed / ITERATIONS
    );

    std::fs::remove_dir_all(index_path).expect("temporary index must be removable");
}
