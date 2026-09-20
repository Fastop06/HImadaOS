// Command Line Tokenizer & Redirection Parser for HimadaOS

pub const MAX_TOKENS: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RedirectKind {
    None,
    Truncate, // >
    Append,   // >>
}

pub struct ParsedCommand<'a> {
    pub cmd: &'a str,
    pub args: &'a str,
    pub redirect_file: Option<&'a str>,
    pub redirect_kind: RedirectKind,
}

pub fn parse_command<'a>(line: &'a str) -> ParsedCommand<'a> {
    let trimmed = line.trim();

    // Check for output redirection (>> or >)
    let (clean_line, redirect_file, redirect_kind) = if let Some(idx) = trimmed.find(">>") {
        let cmd_part = trimmed[..idx].trim();
        let file_part = trimmed[idx + 2..].trim();
        let f = if file_part.is_empty() { None } else { Some(file_part) };
        (cmd_part, f, RedirectKind::Append)
    } else if let Some(idx) = trimmed.find('>') {
        let cmd_part = trimmed[..idx].trim();
        let file_part = trimmed[idx + 1..].trim();
        let f = if file_part.is_empty() { None } else { Some(file_part) };
        (cmd_part, f, RedirectKind::Truncate)
    } else {
        (trimmed, None, RedirectKind::None)
    };

    // Extract command and arguments
    let (cmd, args) = if let Some(idx) = clean_line.find(' ') {
        (&clean_line[..idx], clean_line[idx + 1..].trim())
    } else {
        (clean_line, "")
    };

    ParsedCommand {
        cmd,
        args,
        redirect_file,
        redirect_kind,
    }
}

/// Splits arguments while honoring quotes: "hello world" or 'hello world'
pub fn split_quoted_args<'a>(args: &'a str, out: &mut [&'a str; MAX_TOKENS]) -> usize {
    let mut count = 0;
    let bytes = args.as_bytes();
    let mut i = 0;

    while i < bytes.len() && count < MAX_TOKENS {
        // Skip leading whitespace
        while i < bytes.len() && bytes[i] == b' ' {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        if bytes[i] == b'"' {
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != b'"' {
                i += 1;
            }
            out[count] = &args[start..i];
            count += 1;
            if i < bytes.len() {
                i += 1; // skip closing quote
            }
        } else if bytes[i] == b'\'' {
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != b'\'' {
                i += 1;
            }
            out[count] = &args[start..i];
            count += 1;
            if i < bytes.len() {
                i += 1;
            }
        } else {
            let start = i;
            while i < bytes.len() && bytes[i] != b' ' {
                i += 1;
            }
            out[count] = &args[start..i];
            count += 1;
        }
    }

    count
}
