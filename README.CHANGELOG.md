# WezTerm qualification history

## Wide-character wrapping

Revision `eebf29473eb5b7a07c9cb5c833d42fa90fb00777` changed width-two grapheme handling: when one column remains, the grapheme wraps before placement. The unchanged CJK-width fixture verifies that state, and the seven-fixture conformance suite passes 7 of 7.

## Feed throughput

The mirror initially processed the contract corpus at approximately 68 MB/s. After optimizing the feed path it measured approximately 102 MB/s and cleared the unchanged contract-owned performance budget. Current qualification is determined by `tests/bench.rs`, not by these historical measurements.
