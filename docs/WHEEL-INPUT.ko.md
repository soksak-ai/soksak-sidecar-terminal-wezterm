# 네이티브 휠 입력

공통 터미널 Kit은 장치 delta를 터미널 셀 단위의 정수 step으로 바꾸고 일반 scrollback을
소유한다. 이 provider에는 PTY로 보내는 두 route, 즉 mouse report와 alternate scroll만
전달한다. Provider는 로컬 scrollback을 인코딩하지 않는다.

Mouse report route에서 provider는 live terminal model에 휠 이벤트를 전달한다. DEC9, DEC1000,
DEC1001, DEC1002, DEC1003은 서로 다른 native protocol flag이며, 공통 Kit의 공개
`mouse_reporting()` helper가 event를 encoder에 넣을지 결정한다. 그 뒤 model의 현재 coordinate
encoding이 SGR cell report, 기본 X10 report 또는 UTF-8 확장 좌표 report를 선택한다. 수직·수평
방향, 반복 step, 셀 위치와 modifier는 model의 native 입력으로 유지된다. Meta는 model의 Alt
modifier로 매핑되며 DEC9은 modifier를 억제한다. 고정된 model은 decimal mouse mode 1015를
구현하지 않으며 provider도 이를 위한 generic encoder를 추가하지 않는다.

Alternate scroll route에서는 alternate screen과 DEC mode 1007이 켜져 있고 mouse reporting은
꺼져 있어야 한다. 정규화된 step 하나마다 네이티브 휠 이벤트 하나를 보내며 model은 현재
cursor-key mode가 선택한 cursor sequence를 낸다. Route 선택 뒤 mouse mode가 켜지면 mouse
report가 우선하므로 오래된 alternate-scroll route를 거부한다.

Provider는 전달받은 모든 route를 mirror seat의 lock 안에서 live parsed mode와 다시 대조한다.
Mouse tracking, alternate screen 또는 mode 1007이 바뀌기 전에 선택된 route는
`WHEEL_MODE_CHANGED`로 실패하며 다른 route로 다시 인코딩하지 않는다.

Owner acceptance matrix는 `tests/wheel_input.rs`에 있다. `make verify
TARGET=aarch64-apple-darwin`이 canonical gate의 나머지 검사와 함께 실행한다.
