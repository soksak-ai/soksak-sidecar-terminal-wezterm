# WezTerm qualification history

## Native legacy mouse protocols

Release 0.0.38 consumes common terminal Kit v0.0.34 at immutable commit
`20fb2d73d13e5bcde592380d3052c5d2204a592f` and WezTerm provider commit
`17c7f4aa77e43ad14459cfe6f5da76b1a0a57a2f`. The provider now owns distinct DEC9 and DEC1001
state, query/reset behavior, and native phase encoding. The sidecar reads that public engine state;
it does not alias either mode to DEC1000, DEC1002, or DEC1003. Kit-owned `mouse_reporting()` and
`reports_pointer()` are the route policy boundary.

## Wide-character wrapping

Revision `eebf29473eb5b7a07c9cb5c833d42fa90fb00777` changed width-two grapheme handling: when one column remains, the grapheme wraps before placement. The unchanged CJK-width fixture verifies that state, and the seven-fixture conformance suite passes 7 of 7.

## Feed throughput

The mirror initially processed the contract corpus at approximately 68 MB/s. After optimizing the feed path it measured approximately 102 MB/s and cleared the unchanged contract-owned performance budget. Current qualification is determined by `tests/bench.rs`, not by these historical measurements.
