//! Server-Sent Events 증분 파서 (SRS-LLM-004).

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
}

#[derive(Default)]
pub struct SseParser {
    buf: Vec<u8>,
    event: Option<String>,
    data: Vec<String>,
}

impl SseParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// 수신 바이트를 넣고 완성된 이벤트를 꺼낸다. 줄 경계에서만 UTF-8 해석.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<SseEvent> {
        self.buf.extend_from_slice(bytes);
        let mut out = Vec::new();
        while let Some(pos) = self.buf.iter().position(|&b| b == b'\n') {
            let mut line: Vec<u8> = self.buf.drain(..=pos).collect();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            let line = String::from_utf8_lossy(&line).into_owned();
            self.line(&line, &mut out);
        }
        out
    }

    /// 스트림 종료 시 남은 이벤트 처리.
    pub fn finish(&mut self) -> Vec<SseEvent> {
        let mut out = Vec::new();
        if !self.buf.is_empty() {
            let rest = String::from_utf8_lossy(&std::mem::take(&mut self.buf)).into_owned();
            self.line(rest.trim_end_matches('\r'), &mut out);
        }
        self.dispatch(&mut out);
        out
    }

    fn line(&mut self, line: &str, out: &mut Vec<SseEvent>) {
        if line.is_empty() {
            self.dispatch(out);
            return;
        }
        if line.starts_with(':') {
            return;
        }
        let (field, value) = match line.split_once(':') {
            Some((f, v)) => (f, v.strip_prefix(' ').unwrap_or(v)),
            None => (line, ""),
        };
        match field {
            "event" => self.event = Some(value.to_string()),
            "data" => self.data.push(value.to_string()),
            _ => {}
        }
    }

    fn dispatch(&mut self, out: &mut Vec<SseEvent>) {
        if self.data.is_empty() && self.event.is_none() {
            return;
        }
        out.push(SseEvent {
            event: self.event.take(),
            data: std::mem::take(&mut self.data).join("\n"),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_events_split_across_chunks() {
        let mut p = SseParser::new();
        let mut events = p.feed(b"event: response.output_text.delta\ndata: {\"delta\":\"he");
        assert!(events.is_empty());
        events.extend(p.feed(b"llo\"}\n\n: ping\n\nevent: done\r\ndata: a\r\ndata: b\r\n\r\n"));
        assert_eq!(
            events,
            vec![
                SseEvent {
                    event: Some("response.output_text.delta".into()),
                    data: "{\"delta\":\"hello\"}".into()
                },
                SseEvent {
                    event: Some("done".into()),
                    data: "a\nb".into()
                },
            ]
        );
    }

    #[test]
    fn utf8_split_inside_line_is_preserved() {
        let bytes = "data: 안녕\n\n".as_bytes();
        let mut p = SseParser::new();
        let mut events = p.feed(&bytes[..8]);
        events.extend(p.feed(&bytes[8..]));
        assert_eq!(events[0].data, "안녕");
    }

    #[test]
    fn finish_flushes_pending_event() {
        let mut p = SseParser::new();
        p.feed(b"data: [DONE]");
        assert_eq!(
            p.finish(),
            vec![SseEvent {
                event: None,
                data: "[DONE]".into()
            }]
        );
    }
}
