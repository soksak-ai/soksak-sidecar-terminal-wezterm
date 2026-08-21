use soksak_sidecar_terminal_wezterm::Mirror;

fn stream() -> Vec<u8> {
    let mut stream = b"primary line one\r\nprimary line two\r\n".to_vec();
    stream.extend_from_slice(b"\x1b[1;31mred\x1b[0m tail\r\n");
    stream.extend_from_slice(b"\x1b[?1049h");
    stream.extend_from_slice(b"\x1b[2J\x1b[HALT SCREEN BODY\r\n\x1b[?2004h");
    stream
}

#[test]
fn every_two_chunk_split_matches_one_feed() {
    let stream = stream();
    let expected = {
        let mut mirror = Mirror::new(80, 24);
        mirror.feed(&stream);
        mirror.rehydrate()
    };
    for cut in 0..=stream.len() {
        let mut mirror = Mirror::new(80, 24);
        mirror.feed(&stream[..cut]);
        mirror.feed(&stream[cut..]);
        assert_eq!(mirror.rehydrate(), expected, "split at {cut}");
    }
}

#[test]
fn every_alt_entry_three_chunk_split_matches_one_feed() {
    let stream = stream();
    let expected = {
        let mut mirror = Mirror::new(80, 24);
        mirror.feed(&stream);
        mirror.rehydrate()
    };
    let enter = stream
        .windows(8)
        .position(|value| value == b"\x1b[?1049h")
        .unwrap();
    for first in enter..enter + 8 {
        for second in first..enter + 8 {
            let mut mirror = Mirror::new(80, 24);
            mirror.feed(&stream[..first]);
            mirror.feed(&stream[first..second]);
            mirror.feed(&stream[second..]);
            assert_eq!(mirror.rehydrate(), expected, "splits at {first}/{second}");
        }
    }
}
