#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Enter,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Escape,
    CtrlA,
    CtrlC,
    CtrlD,
    CtrlE,
    CtrlK,
    CtrlU,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub key: Key,
}

impl KeyEvent {
    pub fn new(key: Key) -> Self {
        Self { key }
    }

    pub fn from_buf(buf: &[u8]) -> Option<KeyEvent> {
        match parse_prefix(buf) {
            ParseResult::Complete { key, consumed } if consumed == buf.len() => {
                Some(Self::new(key))
            }
            _ => None,
        }
    }
}

pub struct KeyParser {
    pending: Vec<u8>,
}

impl KeyParser {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Vec<KeyEvent> {
        self.pending.extend_from_slice(bytes);

        let mut events = Vec::new();
        loop {
            match parse_prefix(&self.pending) {
                ParseResult::Complete { key, consumed } => {
                    events.push(KeyEvent::new(key));
                    self.pending.drain(..consumed).for_each(drop);
                }
                ParseResult::Incomplete => break,
                ParseResult::Invalid { consumed } => {
                    self.pending.drain(..consumed).for_each(drop);
                }
            }
        }

        events
    }

    pub fn flush(&mut self) -> Vec<KeyEvent> {
        if self.pending.as_slice() == [0x1b] {
            self.pending.clear();
            return vec![KeyEvent::new(Key::Escape)];
        }

        Vec::new()
    }
}

impl Default for KeyParser {
    fn default() -> Self {
        Self::new()
    }
}

enum ParseResult {
    Complete { key: Key, consumed: usize },
    Incomplete,
    Invalid { consumed: usize },
}

fn parse_prefix(buf: &[u8]) -> ParseResult {
    let Some(&first) = buf.first() else {
        return ParseResult::Incomplete;
    };

    match first {
        0x1b => parse_escape(buf),
        b'\r' | b'\n' => complete(Key::Enter, 1),
        0x7f | 0x08 => complete(Key::Backspace, 1),
        0x01 => complete(Key::CtrlA, 1),
        0x03 => complete(Key::CtrlC, 1),
        0x04 => complete(Key::CtrlD, 1),
        0x05 => complete(Key::CtrlE, 1),
        0x0b => complete(Key::CtrlK, 1),
        0x15 => complete(Key::CtrlU, 1),
        0x20..=0x7e => complete(Key::Char(first as char), 1),
        _ => ParseResult::Invalid { consumed: 1 },
    }
}

fn parse_escape(buf: &[u8]) -> ParseResult {
    if buf.len() < 2 {
        return ParseResult::Incomplete;
    }

    match buf[1] {
        b'[' | b'O' => {}
        _ => return ParseResult::Invalid { consumed: 2 },
    }

    if buf.len() < 3 {
        return ParseResult::Incomplete;
    }

    match buf[2] {
        b'A' => return complete(Key::Up, 3),
        b'B' => return complete(Key::Down, 3),
        b'C' => return complete(Key::Right, 3),
        b'D' => return complete(Key::Left, 3),
        b'H' => return complete(Key::Home, 3),
        b'F' => return complete(Key::End, 3),
        _ => {}
    }

    if !buf[2].is_ascii_digit() && buf[2] != b';' {
        return ParseResult::Invalid { consumed: 3 };
    }

    let mut index = 3;
    while index < buf.len() {
        let byte = buf[index];

        if (0x40..=0x7e).contains(&byte) {
            let key = if index == 3 && byte == b'~' {
                match buf[2] {
                    b'1' => Some(Key::Home),
                    b'3' => Some(Key::Delete),
                    b'4' => Some(Key::End),
                    _ => None,
                }
            } else {
                None
            };

            return match key {
                Some(key) => complete(key, index + 1),
                None => ParseResult::Invalid {
                    consumed: index + 1,
                },
            };
        }

        index += 1;
    }

    ParseResult::Incomplete
}

fn complete(key: Key, consumed: usize) -> ParseResult {
    ParseResult::Complete { key, consumed }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(bytes: &[u8]) -> Vec<Key> {
        let mut parser = KeyParser::new();
        parser
            .feed(bytes)
            .into_iter()
            .map(|event| event.key)
            .collect()
    }

    #[test]
    fn printable_characters() {
        assert_eq!(keys(b"ab"), vec![Key::Char('a'), Key::Char('b')]);
        assert_eq!(keys(b" "), vec![Key::Char(' ')]);
    }

    #[test]
    fn simple_keys() {
        assert_eq!(keys(b"\r"), vec![Key::Enter]);
        assert_eq!(keys(b"\n"), vec![Key::Enter]);
        assert_eq!(keys(&[0x7f]), vec![Key::Backspace]);
        assert_eq!(keys(&[0x08]), vec![Key::Backspace]);
    }

    #[test]
    fn control_keys() {
        assert_eq!(keys(&[0x01]), vec![Key::CtrlA]);
        assert_eq!(keys(&[0x03]), vec![Key::CtrlC]);
        assert_eq!(keys(&[0x04]), vec![Key::CtrlD]);
        assert_eq!(keys(&[0x05]), vec![Key::CtrlE]);
        assert_eq!(keys(&[0x0b]), vec![Key::CtrlK]);
        assert_eq!(keys(&[0x15]), vec![Key::CtrlU]);
    }

    #[test]
    fn arrows_home_and_end() {
        assert_eq!(keys(b"\x1b[A"), vec![Key::Up]);
        assert_eq!(keys(b"\x1b[B"), vec![Key::Down]);
        assert_eq!(keys(b"\x1b[C"), vec![Key::Right]);
        assert_eq!(keys(b"\x1b[D"), vec![Key::Left]);
        assert_eq!(keys(b"\x1b[H"), vec![Key::Home]);
        assert_eq!(keys(b"\x1b[F"), vec![Key::End]);
        assert_eq!(keys(b"\x1bOA"), vec![Key::Up]);
        assert_eq!(keys(b"\x1bOB"), vec![Key::Down]);
    }

    #[test]
    fn tilde_sequences() {
        assert_eq!(keys(b"\x1b[1~"), vec![Key::Home]);
        assert_eq!(keys(b"\x1b[3~"), vec![Key::Delete]);
        assert_eq!(keys(b"\x1b[4~"), vec![Key::End]);
    }

    #[test]
    fn multiple_keys_in_one_feed() {
        assert_eq!(
            keys(b"a\x1b[A\x7f"),
            vec![Key::Char('a'), Key::Up, Key::Backspace]
        );
    }

    #[test]
    fn split_escape_sequence() {
        let mut parser = KeyParser::new();

        assert!(parser.feed(b"\x1b[3").is_empty());
        assert_eq!(
            parser
                .feed(b"~")
                .into_iter()
                .map(|event| event.key)
                .collect::<Vec<_>>(),
            vec![Key::Delete]
        );
    }

    #[test]
    fn lone_escape_flushes() {
        let mut parser = KeyParser::new();

        assert!(parser.feed(b"\x1b").is_empty());
        assert_eq!(
            parser
                .flush()
                .into_iter()
                .map(|event| event.key)
                .collect::<Vec<_>>(),
            vec![Key::Escape]
        );
        assert!(parser.flush().is_empty());
    }

    #[test]
    fn invalid_sequences_are_dropped() {
        assert!(keys(b"\x1bx").is_empty());
        assert!(keys(b"\x1b[5~").is_empty());
    }

    #[test]
    fn from_buf_requires_a_single_complete_key() {
        assert_eq!(
            KeyEvent::from_buf(b"a").map(|event| event.key),
            Some(Key::Char('a'))
        );
        assert_eq!(
            KeyEvent::from_buf(b"\x1b[3~").map(|event| event.key),
            Some(Key::Delete)
        );
        assert_eq!(KeyEvent::from_buf(b""), None);
        assert_eq!(KeyEvent::from_buf(b"ab"), None);
        assert_eq!(KeyEvent::from_buf(b"\x1b"), None);
        assert_eq!(KeyEvent::from_buf(b"\x1bx"), None);
    }
}
