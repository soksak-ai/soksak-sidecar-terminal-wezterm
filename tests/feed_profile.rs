// 축 ① 만 재는 계측 하니스 — 게이트가 아니다(판정은 tests/bench.rs 가 한다).
//
// 예산 게이트는 실 데몬을 세워 수요를 재므로 한 번에 수십 초가 든다. feed 경로를 고치는
// 동안에는 그 왕복이 너무 길다. 이 하니스는 계약의 같은 코퍼스를 같은 미러에 먹이되 feed 만
// 잰다 — 고친 것이 벌었는지를 몇 초 안에 답한다.
//
//   cargo test --release --test feed_profile -- --ignored --nocapture
//
// SOKSAK_FEED_SECS 를 주면 그 초 동안 코퍼스를 되먹인다 — 프로파일러(sample(1))가 붙을 창을
// 열어 두기 위한 모드다(중앙값이 아니라 평균을 낸다).
//
//   SOKSAK_FEED_SECS=12 cargo test --release --test feed_profile -- --ignored --nocapture

use std::time::{Duration, Instant};

use soksak_contract_terminal::bench::corpus;
use soksak_contract_terminal::corpus::{COLS, ROWS};
use soksak_sidecar_terminal_wezterm::Mirror;

// 코퍼스를 한 덩어리로 먹인다(계약 벤치와 같은 모양).
fn feed_once(corpus: &[u8]) -> f64 {
    feed_chunked(corpus, corpus.len())
}

// 실전은 한 덩어리가 아니다 — 데몬의 tee 는 조각으로 배달한다. 조각마다 무는 비용(엔진 진입·
// answerback 배수 배리어)이 있으면 한 덩어리 측정에는 안 보이고 실전에서만 드러난다. 그래서
// 조각 크기를 주고 잰다.
fn feed_chunked(corpus: &[u8], chunk: usize) -> f64 {
    let mut m = Mirror::new(COLS, ROWS);
    let t = Instant::now();
    for c in corpus.chunks(chunk.max(1)) {
        m.feed(c);
    }
    let secs = t.elapsed().as_secs_f64();
    std::hint::black_box(&m);
    (corpus.len() as f64 / 1e6) / secs
}

fn median_of_7(corpus: &[u8], chunk: usize) -> f64 {
    let mut v: Vec<f64> = (0..7).map(|_| feed_chunked(corpus, chunk)).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[3]
}

#[test]
#[ignore]
fn feed_profile() {
    let corpus = corpus();
    println!("corpus {:.2} MB", corpus.len() as f64 / 1e6);

    if let Ok(secs) = std::env::var("SOKSAK_FEED_SECS") {
        let secs: u64 = secs.parse().expect("SOKSAK_FEED_SECS");
        let deadline = Instant::now() + Duration::from_secs(secs);
        let (mut n, mut sum) = (0u64, 0.0);
        while Instant::now() < deadline {
            sum += feed_once(&corpus);
            n += 1;
        }
        println!("feed {:.1} MB/s (mean of {n} runs over {secs}s)", sum / n as f64);
        return;
    }

    println!("feed {:.1} MB/s (one buffer, median of 7)", median_of_7(&corpus, corpus.len()));
    // 데몬이 실제로 배달하는 모양 — 조각이 작아질수록 조각당 고정비가 드러난다. 8192 는
    // 데몬이 PTY 를 읽는 버퍼 크기이고 tee 는 읽은 만큼 그대로 흘리므로, 실전의 조각이 그것이다.
    for chunk in [256 * 1024, 64 * 1024, 16 * 1024, 8 * 1024, 4 * 1024] {
        println!(
            "feed {:.1} MB/s (chunks of {:>6} B, median of 7){}",
            median_of_7(&corpus, chunk),
            chunk,
            if chunk == 8 * 1024 { "  <- 데몬의 PTY 읽기 크기" } else { "" }
        );
    }
}
