//! 데몬 피어링(소비 면) — 사이드카가 코어 데몬(soksak-ptyd)의 클라이언트로서 tee 를
//! 구독하고 봉인-블롭을 밀어 넣는다(SPEC.md §6). control 소켓(NDJSON 요청/응답)과
//! stream 소켓(hello 후 length-prefixed tee 프레임) 두 면을 연다.
//!
//! 데몬 wire 소비는 엔진-불가지다(바이트만 나른다) — wezterm 엔진 교체와 무관하다.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;

use interprocess::local_socket::{prelude::*, GenericFilePath, RecvHalf, SendHalf, Stream};

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use serde_json::Value;

use crate::proto;

fn other(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::Other, msg.into())
}

fn read_token(home: &Path) -> io::Result<String> {
    let path = proto::token_path(home);
    let t = std::fs::read_to_string(&path).map_err(|e| {
        other(format!("daemon token unreadable at {}: {e} (is soksak-ptyd up?)", path.display()))
    })?;
    let t = t.trim().to_string();
    if t.is_empty() {
        return Err(other(format!("daemon token empty at {}", path.display())));
    }
    Ok(t)
}

/// 한 세션(ListSessions 응답 항목) — 어느 tee 를 구독할지의 좌표.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session: u64,
    pub pane_id: String,
    pub window_label: Option<String>,
}

/// control 소켓 클라이언트 — hello 후 NDJSON 요청/응답. 사이드카가 쓰는 요청만
/// 타입 헬퍼로 감싼다; 임의 요청은 [`ControlClient::request`] 로 보낸다(통합 드라이버용).
pub struct ControlClient {
    writer: SendHalf,
    reader: BufReader<RecvHalf>,
}

impl ControlClient {
    /// control 소켓에 연결하고 hello 를 교환한다. 데몬 미가동·토큰 부재·버전 스큐는
    /// 전부 명시 에러(무음 아님).
    pub fn connect(home: &Path) -> io::Result<Self> {
        let token = read_token(home)?;
        let path = proto::control_socket_path(home);
        let name = path.as_os_str().to_fs_name::<GenericFilePath>()?;
        let stream = Stream::connect(name).map_err(|e| {
            other(format!("cannot reach daemon control socket {}: {e}", path.display()))
        })?;
        let (recv, send) = stream.split();
        let mut c = ControlClient { writer: send, reader: BufReader::new(recv) };
        c.send_line(&proto::hello(&token, None, false))?;
        let reply = c.read_reply()?;
        proto::require_ok(&reply).map_err(other)?;
        Ok(c)
    }

    fn send_line(&mut self, v: &Value) -> io::Result<()> {
        let mut line = serde_json::to_vec(v)?;
        line.push(b'\n');
        self.writer.write_all(&line)?;
        self.writer.flush()
    }

    fn read_reply(&mut self) -> io::Result<Value> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line)?;
        if n == 0 {
            return Err(other("daemon closed the control connection"));
        }
        serde_json::from_str(line.trim()).map_err(|e| other(format!("reply parse: {e}")))
    }

    /// 임의 요청 1왕복. 사이드카 본연이 아닌 요청(세션 스폰·write 등, 통합 드라이버)도
    /// 이 경로로 보낸다 — control 소켓은 어떤 태그된 요청도 받는다.
    pub fn request(&mut self, req: Value) -> io::Result<Value> {
        self.send_line(&req)?;
        self.read_reply()
    }

    /// 살아 있는 세션 목록 — 어느 tee 를 구독할지.
    pub fn list_sessions(&mut self) -> io::Result<Vec<SessionInfo>> {
        let reply = self.request(serde_json::json!({ "op": "listSessions" }))?;
        proto::require_ok(&reply).map_err(other)?;
        let arr = reply
            .get("data")
            .and_then(|d| d.get("sessions"))
            .and_then(|s| s.as_array())
            .ok_or_else(|| other("listSessions: no sessions array"))?;
        Ok(arr
            .iter()
            .filter_map(|s| {
                Some(SessionInfo {
                    session: s.get("session")?.as_u64()?,
                    pane_id: s.get("paneId")?.as_str()?.to_string(),
                    window_label: s.get("windowLabel").and_then(|w| w.as_str()).map(String::from),
                })
            })
            .collect())
    }

    /// 봉인-블롭 푸시(SPEC §6.3) — 사이드카는 키를 만지지 않는다. 데몬이 봉인·원자쓰기.
    pub fn store_blob(
        &mut self,
        window_label: Option<&str>,
        pane_id: &str,
        bytes: &[u8],
    ) -> io::Result<()> {
        let reply = self.request(serde_json::json!({
            "op": "storeBlob",
            "windowLabel": window_label,
            "paneId": pane_id,
            "bytesB64": B64.encode(bytes),
        }))?;
        proto::require_ok(&reply).map_err(other)
    }
}

/// 데몬 control 소켓에 붙어 데몬이 죽을 때까지 블록한다. control 은 요청/응답이라 서버는
/// 무요청 연결에 아무것도 쓰지 않으므로, read 반환 = EOF/에러 = 데몬 사망 이벤트다(폴링 0).
/// 사이드카의 존재 이유는 데몬 피어링이다 — 데몬이 죽으면(재부팅 모사 등) 이 사이드카는 죽은
/// 피어를 붙든 좀비가 되어 싱글턴으로 신선 사이드카를 막는다. 호출자(main)는 이 반환 즉시
/// exit(0) 해 서비스 소켓을 놓고, 앱의 다음 ensureSidecar 스폰이 새 데몬에 붙는 신선 인스턴스를 띄운다.
pub fn block_until_daemon_dies(home: &Path) -> io::Result<()> {
    let token = read_token(home)?;
    let path = proto::control_socket_path(home);
    let name = path.as_os_str().to_fs_name::<GenericFilePath>()?;
    let stream = Stream::connect(name)?;
    let (recv, send) = stream.split();
    let mut writer = send;
    let mut reader = BufReader::new(recv);
    let mut line = serde_json::to_vec(&proto::hello(&token, None, false))?;
    line.push(b'\n');
    writer.write_all(&line)?;
    writer.flush()?;
    let mut ack = String::new();
    if reader.read_line(&mut ack)? == 0 {
        return Ok(()); // 데몬이 hello 전에 닫음 = 이미 죽음
    }
    // 이후 아무 요청도 보내지 않는다 — read 가 반환하면 데몬이 연결을 닫았다(사망).
    let mut sink = [0u8; 64];
    loop {
        match reader.read(&mut sink) {
            Ok(0) | Err(_) => return Ok(()), // EOF/에러 = 데몬 사망
            Ok(_) => continue,               // 규약 밖 바이트는 무시
        }
    }
}

/// tee 프레임(SPEC §6.2) — 원시 사본 또는 백프레셔 유실 마커.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeeFrame {
    Data(Vec<u8>),
    Gap { from_seq: u64, to_seq: u64 },
}

/// stream 소켓 tee 구독 — hello{subscribe} 후 length-prefixed 프레임을 EOF 까지 읽는다.
pub struct TeeStream {
    reader: BufReader<RecvHalf>,
    // The send half is unused after the subscribe hello, but holding it keeps the
    // connection's write side open so the split does not shut it down early.
    _writer: SendHalf,
    session: u64,
    // 구독 시점의 데몬 링 head(ack.startSeq) — 소비자 consumed_seq 의 정확한 기점. 이후
    // 프레임 길이만큼 전진하면 좌표가 데몬 링과 정합한다(warm 핸드오프, SPEC §6.4).
    start_seq: u64,
    // 구독 순간 데몬이 원자 캡처한 링 backlog(ack.seedB64) — start_seq 이전 retained 출력.
    // 미드-세션 구독의 근접-birth 씨앗이다: 미러에 선주입해 구독 전 화면을 메운다. 링은
    // 유계라 부분 씨앗(retained window)이고, start_seq 부터의 프레임과 겹치지 않는다.
    seed: Vec<u8>,
}

impl TeeStream {
    pub fn subscribe(home: &Path, session: u64) -> io::Result<Self> {
        let token = read_token(home)?;
        let path = proto::stream_socket_path(home);
        let name = path.as_os_str().to_fs_name::<GenericFilePath>()?;
        let stream = Stream::connect(name).map_err(|e| {
            other(format!("cannot reach daemon stream socket {}: {e}", path.display()))
        })?;
        let (recv, send) = stream.split();
        let mut writer = send;
        let mut reader = BufReader::new(recv);
        // hello{subscribe:true, session} → ack 1줄(NDJSON) → 이후 프레임.
        let mut line = serde_json::to_vec(&proto::hello(&token, Some(session), true))?;
        line.push(b'\n');
        writer.write_all(&line)?;
        writer.flush()?;
        let mut ack = String::new();
        if reader.read_line(&mut ack)? == 0 {
            return Err(other("daemon closed before subscribe ack"));
        }
        let ack: Value =
            serde_json::from_str(ack.trim()).map_err(|e| other(format!("subscribe ack parse: {e}")))?;
        proto::require_ok(&ack).map_err(other)?;
        // startSeq 부재는 구버전 데몬 — 0 으로 떨어지되(정합성 약화) 조용히 셰이프를 깨진 않는다.
        let start_seq = ack.get("data").and_then(|d| d.get("startSeq")).and_then(|v| v.as_u64()).unwrap_or(0);
        // seedB64 부재는 backlog 없음(birth 구독 또는 구버전 데몬) — 빈 씨앗으로 떨어진다.
        let seed = ack
            .get("data")
            .and_then(|d| d.get("seedB64"))
            .and_then(|v| v.as_str())
            .and_then(|s| B64.decode(s).ok())
            .unwrap_or_default();
        Ok(TeeStream { reader, _writer: writer, session, start_seq, seed })
    }

    pub fn session(&self) -> u64 {
        self.session
    }

    /// 구독 시점 데몬 링 head — 소비자 consumed_seq 의 기점.
    pub fn start_seq(&self) -> u64 {
        self.start_seq
    }

    /// 구독 순간 데몬이 원자 캡처한 링 backlog — start_seq 이전 retained 출력의 근접-birth
    /// 씨앗. 소비자가 미러에 선주입해 구독 전 화면을 메운다(비면 birth 구독).
    pub fn seed(&self) -> &[u8] {
        &self.seed
    }

    /// 다음 프레임 — `[kind:u8][len:u32 BE][payload]`. EOF(세션 종료·연결 끝) → None.
    pub fn next_frame(&mut self) -> io::Result<Option<TeeFrame>> {
        let mut kind = [0u8; 1];
        match self.reader.read_exact(&mut kind) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        }
        let mut len_be = [0u8; 4];
        self.reader.read_exact(&mut len_be)?;
        let len = u32::from_be_bytes(len_be) as usize;
        let mut payload = vec![0u8; len];
        self.reader.read_exact(&mut payload)?;
        decode_frame(kind[0], &payload).map(Some)
    }
}

/// 프레임 바디 디코드(kind + payload) — 소켓과 분리된 순수 함수라 하니스 없이 단위
/// 테스트가 가능하다(프레임 파싱이 소비 계약의 실체).
pub fn decode_frame(kind: u8, payload: &[u8]) -> io::Result<TeeFrame> {
    match kind {
        proto::TEE_FRAME_DATA => Ok(TeeFrame::Data(payload.to_vec())),
        proto::TEE_FRAME_GAP => {
            let v: Value =
                serde_json::from_slice(payload).map_err(|e| other(format!("gap payload: {e}")))?;
            let from_seq = v.get("fromSeq").and_then(|x| x.as_u64()).ok_or_else(|| other("gap: no fromSeq"))?;
            let to_seq = v.get("toSeq").and_then(|x| x.as_u64()).ok_or_else(|| other("gap: no toSeq"))?;
            Ok(TeeFrame::Gap { from_seq, to_seq })
        }
        k => Err(other(format!("unknown tee frame kind {k}"))),
    }
}

/// tee 프레임 1개를 `[kind][len BE][payload]` 로 인코딩(데몬 encode_tee_frame 과 동형).
/// 통합·단위 테스트가 데몬 없이 프레임 스트림을 합성할 때 쓴다.
pub fn encode_frame(kind: u8, payload: &[u8], out: &mut Vec<u8>) {
    out.push(kind);
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(payload);
}

#[cfg(test)]
mod tests {
    use super::*;

    // 프레임 디코드 = 소비 계약의 핵. data/gap 을 인코딩→디코드 라운드트립으로 못박는다
    // (데몬 없이도 검증 가능한 seam).
    #[test]
    fn data_and_gap_frames_round_trip() {
        assert_eq!(
            decode_frame(proto::TEE_FRAME_DATA, b"hello").unwrap(),
            TeeFrame::Data(b"hello".to_vec())
        );
        let gap = serde_json::to_vec(&serde_json::json!({ "fromSeq": 10, "toSeq": 25 })).unwrap();
        assert_eq!(
            decode_frame(proto::TEE_FRAME_GAP, &gap).unwrap(),
            TeeFrame::Gap { from_seq: 10, to_seq: 25 }
        );
    }

    #[test]
    fn unknown_frame_kind_is_a_loud_error() {
        assert!(decode_frame(9, b"").is_err(), "unknown kind must not be silently dropped");
    }

    // 인코더(데몬 포맷)로 만든 바이트를 길이-접두 파서가 그대로 되읽는다 — [kind][len BE]
    // [payload] 프레이밍이 데몬과 어긋나면 여기서 깨진다.
    #[test]
    fn length_prefix_framing_parses_encoder_output() {
        let mut buf = Vec::new();
        encode_frame(proto::TEE_FRAME_DATA, b"ab", &mut buf);
        encode_frame(proto::TEE_FRAME_DATA, b"", &mut buf);
        let gap = serde_json::to_vec(&serde_json::json!({ "fromSeq": 1, "toSeq": 4 })).unwrap();
        encode_frame(proto::TEE_FRAME_GAP, &gap, &mut buf);

        // 소켓 대신 메모리 커서로 같은 파싱 루프를 돈다.
        let mut cur = std::io::Cursor::new(buf);
        let mut frames = Vec::new();
        loop {
            let mut kind = [0u8; 1];
            match cur.read_exact(&mut kind) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => panic!("{e}"),
            }
            let mut len_be = [0u8; 4];
            cur.read_exact(&mut len_be).unwrap();
            let len = u32::from_be_bytes(len_be) as usize;
            let mut payload = vec![0u8; len];
            cur.read_exact(&mut payload).unwrap();
            frames.push(decode_frame(kind[0], &payload).unwrap());
        }
        assert_eq!(
            frames,
            vec![
                TeeFrame::Data(b"ab".to_vec()),
                TeeFrame::Data(vec![]),
                TeeFrame::Gap { from_seq: 1, to_seq: 4 },
            ]
        );
    }
}
