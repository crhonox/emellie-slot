use std::fs::File;
use std::io::Read;
use std::path::Path;

const TITLE_LEN: usize = 12;
const CODE_OFF: usize = 0x0c;
const CODE_LEN: usize = 4;

/// The DS cart header's game title, at its very start.
pub fn title(rom: &Path) -> Option<String> {
    let head = head(rom)?;
    text_from_bytes(&head[..TITLE_LEN])
}

/// The four-letter game code after the title, `ADAE` for Pokémon Diamond (USA).
pub fn code(rom: &Path) -> Option<String> {
    let head = head(rom)?;
    text_from_bytes(&head[CODE_OFF..CODE_OFF + CODE_LEN])
}

fn head(rom: &Path) -> Option<[u8; CODE_OFF + CODE_LEN]> {
    let mut buf = [0u8; CODE_OFF + CODE_LEN];
    File::open(rom).ok()?.read_exact(&mut buf).ok()?;
    Some(buf)
}

fn text_from_bytes(buf: &[u8]) -> Option<String> {
    let end = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    let text = std::str::from_utf8(&buf[..end]).ok()?.trim();
    (!text.is_empty()).then(|| text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_and_code_come_from_the_header() {
        let dir = tempfile::tempdir().unwrap();
        let rom = dir.path().join("game.nds");
        let mut bytes = b"POKEMON D\0\0\0ADAE01".to_vec();
        bytes.resize(0x200, 0);
        std::fs::write(&rom, bytes).unwrap();
        assert_eq!(title(&rom).as_deref(), Some("POKEMON D"));
        assert_eq!(code(&rom).as_deref(), Some("ADAE"));
    }

    #[test]
    fn a_file_too_short_for_a_header_has_none() {
        let dir = tempfile::tempdir().unwrap();
        let rom = dir.path().join("short.nds");
        std::fs::write(&rom, b"POKE").unwrap();
        assert_eq!(title(&rom), None);
        assert_eq!(code(&rom), None);
    }
}
