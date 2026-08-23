# WezTerm 적격성 변경 기록

## 전각 문자 줄바꿈

Revision `eebf29473eb5b7a07c9cb5c833d42fa90fb00777`은 남은 칸이 하나일 때 폭 2 grapheme을 배치하기 전에 다음 줄로 넘긴다. 변경하지 않은 CJK 폭 fixture가 이 상태를 검증하며, 일곱 fixture 적합성 suite는 7/7을 통과한다.

## Feed 처리량

Mirror는 처음에 계약 corpus를 약 68 MB/s로 처리했다. Feed 경로를 최적화한 뒤 약 102 MB/s를 기록했고 변경하지 않은 계약 소유 성능 예산을 통과했다. 현재 적격성은 이 과거 측정치가 아니라 `tests/bench.rs`로 판정한다.
