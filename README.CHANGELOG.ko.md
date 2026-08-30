# WezTerm 적격성 변경 기록

## 레거시 mouse protocol의 native 상태

0.0.38 릴리스는 공통 터미널 Kit v0.0.34의 immutable commit
`20fb2d73d13e5bcde592380d3052c5d2204a592f`와 WezTerm provider commit
`17c7f4aa77e43ad14459cfe6f5da76b1a0a57a2f`를 사용한다. Provider가 DEC9와 DEC1001의
서로 다른 상태, query/reset 동작, native phase encoding을 소유한다. Sidecar는 그 공개 엔진
상태를 읽으며 두 mode를 DEC1000, DEC1002, DEC1003 중 어느 것에도 alias하지 않는다. Route
정책 경계는 Kit이 소유한 `mouse_reporting()`과 `reports_pointer()`다.

## 전각 문자 줄바꿈

Revision `eebf29473eb5b7a07c9cb5c833d42fa90fb00777`은 남은 칸이 하나일 때 폭 2 grapheme을 배치하기 전에 다음 줄로 넘긴다. 변경하지 않은 CJK 폭 fixture가 이 상태를 검증하며, 일곱 fixture 적합성 suite는 7/7을 통과한다.

## Feed 처리량

Mirror는 처음에 계약 corpus를 약 68 MB/s로 처리했다. Feed 경로를 최적화한 뒤 약 102 MB/s를 기록했고 변경하지 않은 계약 소유 성능 예산을 통과했다. 현재 적격성은 이 과거 측정치가 아니라 `tests/bench.rs`로 판정한다.
