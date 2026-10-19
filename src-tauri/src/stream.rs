//! Incremental SSE parsing. Buffer bytes until a full line exists so UTF-8
//! characters split between network packets are never decoded prematurely.
const MAX_EVENT_BYTES: usize = 1024 * 1024;

#[derive(Default)]
pub struct SseDecoder {
    pending: Vec<u8>,
    data: Vec<String>,
    data_size: usize,
}

impl SseDecoder {
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<String>, String> {
        self.pending.extend_from_slice(bytes);
        let mut events = Vec::new();
        let mut start = 0;
        // Iterate over the complete lines without repeatedly shifting the buffer.
        while let Some(offset) = self.pending[start..].iter().position(|&byte| byte == b'\n') {
            let end = start + offset;
            let line = std::str::from_utf8(&self.pending[start..end])
                .map_err(|_| "服务返回了无效的 UTF-8 数据。")?
                .trim_end_matches('\r')
                .to_owned();
            self.line(&line, &mut events)?;
            start = end + 1;
        }
        self.pending.drain(..start);
        if self.pending.len() + self.data_size > MAX_EVENT_BYTES {
            return Err("服务返回的单条事件过大。".into());
        }
        Ok(events)
    }

    pub fn finish(&mut self) -> Result<Vec<String>, String> {
        let mut events = Vec::new();
        let pending = std::mem::take(&mut self.pending);
        if !pending.is_empty() {
            let line =
                std::str::from_utf8(&pending).map_err(|_| "服务返回了不完整的 UTF-8 数据。")?;
            self.line(line.trim_end_matches('\r'), &mut events)?;
        }
        self.line("", &mut events)?;
        Ok(events)
    }

    fn line(&mut self, line: &str, events: &mut Vec<String>) -> Result<(), String> {
        if line.is_empty() {
            if !self.data.is_empty() {
                events.push(self.data.join("\n"));
                self.data.clear();
                self.data_size = 0;
            }
        } else if let Some(value) = line.strip_prefix("data:") {
            let value = value.strip_prefix(' ').unwrap_or(value);
            self.data_size += value.len();
            if self.data_size > MAX_EVENT_BYTES {
                return Err("服务返回的单条事件过大。".into());
            }
            self.data.push(value.to_owned());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_chinese_split_at_every_possible_byte_boundary() {
        let wire = ": keepalive\r\ndata: {\"text\":\"你好🌿\"}\r\n\r\ndata: [DONE]\n\n".as_bytes();
        for split in 0..=wire.len() {
            let mut decoder = SseDecoder::default();
            let mut events = decoder.push(&wire[..split]).unwrap();
            events.extend(decoder.push(&wire[split..]).unwrap());
            events.extend(decoder.finish().unwrap());
            assert_eq!(events, ["{\"text\":\"你好🌿\"}", "[DONE]"], "split={split}");
        }
    }

    #[test]
    fn handles_multiline_data_and_unterminated_final_event() {
        let mut decoder = SseDecoder::default();
        assert!(decoder
            .push(b"event: message\ndata: first\ndata:second")
            .unwrap()
            .is_empty());
        assert_eq!(decoder.finish().unwrap(), ["first\nsecond"]);
    }

    #[test]
    fn rejects_unbounded_or_invalid_data() {
        assert!(SseDecoder::default()
            .push(&vec![b'a'; MAX_EVENT_BYTES + 1])
            .is_err());
        assert!(SseDecoder::default().push(b"data: \xff\n\n").is_err());
    }
}
