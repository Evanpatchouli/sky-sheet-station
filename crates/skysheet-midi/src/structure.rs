//! Validate framing before midly: its strict mode still accepts some incomplete
//! metadata payloads and does not enforce our complete-file/EOT policy.
pub(crate) const MAX_FILE_BYTES: usize = 32 * 1024 * 1024;

pub(crate) fn validate(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err("MIDI exceeds this converter's 32 MiB resource limit".into());
    }
    let mut cursor = Cursor { bytes, pos: 0 };
    if cursor.take(4)? != b"MThd" {
        return Err("expected SMF MThd header (containers are unsupported)".into());
    }
    if cursor.u32()? != 6 {
        return Err("SMF header length must be 6".into());
    }
    let header = cursor.take(6)?;
    let format = u16::from_be_bytes([header[0], header[1]]);
    let count = u16::from_be_bytes([header[2], header[3]]);
    let division = u16::from_be_bytes([header[4], header[5]]);
    if format > 1 {
        return Err(format!(
            "unsupported SMF Format {format}; only Format 0/1 supported"
        ));
    }
    if count == 0 || (format == 0 && count != 1) {
        return Err("invalid SMF track count".into());
    }
    if division & 0x8000 != 0 {
        return Err("SMPTE time division is unsupported; it is not PPQ".into());
    }
    if division == 0 {
        return Err("PPQ time division must be positive".into());
    }
    for track in 0..count {
        if cursor.take(4)? != b"MTrk" {
            return Err(format!("track {track}: expected MTrk chunk"));
        }
        let length = cursor.u32()? as usize;
        let data = cursor.take(length)?;
        validate_track(data).map_err(|e| format!("track {track}: {e}"))?;
    }
    if cursor.pos != bytes.len() {
        return Err("trailing data or undeclared tracks after final chunk".into());
    }
    Ok(())
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.pos.checked_add(n).ok_or("chunk length overflow")?;
        let value = self
            .bytes
            .get(self.pos..end)
            .ok_or_else(|| format!("truncated data at byte {}", self.pos))?;
        self.pos = end;
        Ok(value)
    }
    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn vlq(&mut self) -> Result<usize, String> {
        let mut value = 0;
        for _ in 0..4 {
            let byte = self.byte()?;
            value = (value << 7) | usize::from(byte & 127);
            if byte & 128 == 0 {
                return Ok(value);
            }
        }
        Err("VLQ exceeds four bytes".into())
    }
}

fn validate_track(bytes: &[u8]) -> Result<(), String> {
    let mut cursor = Cursor { bytes, pos: 0 };
    let mut running = None;
    while cursor.pos < bytes.len() {
        cursor.vlq()?;
        let first = cursor.byte()?;
        let status = if first < 128 {
            cursor.pos -= 1;
            running.ok_or("data byte without running status")?
        } else {
            first
        };
        match status {
            0x80..=0xef => {
                running = Some(status);
                let n = if status & 0xe0 == 0xc0 { 1 } else { 2 };
                if cursor.take(n)?.iter().any(|b| b & 128 != 0) {
                    return Err("channel data byte exceeds 7 bits".into());
                }
            }
            0xff => {
                running = None;
                let kind = cursor.byte()?;
                if kind >= 128 {
                    return Err("meta type exceeds 7 bits".into());
                }
                let len = cursor.vlq()?;
                let expected = match kind {
                    0 => Some(2),
                    0x20 | 0x21 => Some(1),
                    0x2f => Some(0),
                    0x51 => Some(3),
                    0x54 => Some(5),
                    0x58 => Some(4),
                    0x59 => Some(2),
                    _ => None,
                };
                if expected.is_some_and(|n| n != len) {
                    return Err(format!("meta {kind:#x}: invalid payload length {len}"));
                }
                let payload = cursor.take(len)?;
                if kind == 0x20 && payload[0] > 15 {
                    return Err("meta channel prefix exceeds 15".into());
                }
                if kind == 0x21 && payload[0] > 127 {
                    return Err("meta port exceeds 127".into());
                }
                if kind == 0x59 && (!(-7..=7).contains(&(payload[0] as i8)) || payload[1] > 1) {
                    return Err("invalid key signature metadata".into());
                }
                if kind == 0x2f {
                    if cursor.pos != bytes.len() {
                        return Err("events/data after EndOfTrack".into());
                    }
                    return Ok(());
                }
            }
            0xf0 | 0xf7 => {
                running = None;
                let n = cursor.vlq()?;
                cursor.take(n)?;
            }
            _ => return Err(format!("unsupported SMF status {status:#x}")),
        }
    }
    Err("missing EndOfTrack".into())
}
