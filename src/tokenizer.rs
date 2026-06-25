use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Indent,
    Dedent,
    Newline,
    Ident(String),
    IntLit(String),
    FloatLit(String),
    StrLit(String),
    FStrLit(String),

    KwDef,
    KwClass,
    KwIf,
    KwElif,
    KwElse,
    KwFor,
    KwWhile,
    KwBreak,
    KwContinue,
    KwReturn,
    KwPass,
    KwImport,
    KwFrom,
    KwAs,
    KwTrue,
    KwFalse,
    KwNone,
    KwIn,
    KwIs,
    KwNot,
    KwAnd,
    KwOr,
    KwLambda,
    KwDel,
    KwTry,
    KwExcept,
    KwFinally,
    KwRaise,
    KwWith,
    KwYield,

    Plus,
    Minus,
    Star,
    Slash,
    DoubleSlash,
    Percent,
    DoubleStar,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    Eq,
    EqEq,
    BangEq,
    Less,
    Greater,
    LessEq,
    GreaterEq,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Dot,
    Colon,
    Arrow,
    At,
    Semicolon,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Ident(s) => write!(f, "IDENT({})", s),
            Token::IntLit(s) => write!(f, "INT({})", s),
            Token::FloatLit(s) => write!(f, "FLOAT({})", s),
            Token::StrLit(s) => write!(f, "STR({})", s),
            Token::FStrLit(s) => write!(f, "FSTR({})", s),
            _ => write!(f, "{:?}", self),
        }
    }
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn tokenize_line(line: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        if ch == '\n' || ch == '\r' {
            i += 1;
            continue;
        }

        // Comment check is done by the pre-processor, so # here is inside a string.
        // We handle it as a regular character by falling through to the string/ident checks below.

        if (ch == 'f' || ch == 'F')
            && i + 1 < chars.len()
            && (chars[i + 1] == '"' || chars[i + 1] == '\'')
        {
            let quote = chars[i + 1];
            i += 2;
            let mut s = String::new();
            while i < chars.len() {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    match chars[i + 1] {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        'r' => s.push('\r'),
                        '\\' => s.push('\\'),
                        '"' => s.push('"'),
                        '\'' => s.push('\''),
                        c => {
                            s.push('\\');
                            s.push(c);
                        }
                    }
                    i += 2;
                    continue;
                }
                if chars[i] == quote {
                    i += 1;
                    break;
                }
                s.push(chars[i]);
                i += 1;
            }
            tokens.push(Token::FStrLit(s));
            continue;
        }

        if is_ident_start(ch) {
            let mut s = String::new();
            while i < chars.len() && is_ident_continue(chars[i]) {
                s.push(chars[i]);
                i += 1;
            }
            tokens.push(match s.as_str() {
                "def" => Token::KwDef,
                "class" => Token::KwClass,
                "if" => Token::KwIf,
                "elif" => Token::KwElif,
                "else" => Token::KwElse,
                "for" => Token::KwFor,
                "while" => Token::KwWhile,
                "break" => Token::KwBreak,
                "continue" => Token::KwContinue,
                "return" => Token::KwReturn,
                "pass" => Token::KwPass,
                "import" => Token::KwImport,
                "from" => Token::KwFrom,
                "del" => Token::KwDel,
                "as" => Token::KwAs,
                "True" => Token::KwTrue,
                "False" => Token::KwFalse,
                "None" => Token::KwNone,
                "in" => Token::KwIn,
                "is" => Token::KwIs,
                "not" => Token::KwNot,
                "and" => Token::KwAnd,
                "or" => Token::KwOr,
                "lambda" => Token::KwLambda,
                "try" => Token::KwTry,
                "except" => Token::KwExcept,
                "finally" => Token::KwFinally,
                "raise" => Token::KwRaise,
                "with" => Token::KwWith,
                "yield" => Token::KwYield,

                _ => Token::Ident(s),
            });
            continue;
        }

        if ch.is_ascii_digit() {
            let mut s = String::new();
            let mut is_float = false;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                if chars[i] == '.' {
                    if is_float {
                        break;
                    }
                    is_float = true;
                }
                s.push(chars[i]);
                i += 1;
            }
            if is_float {
                tokens.push(Token::FloatLit(s));
            } else {
                tokens.push(Token::IntLit(s));
            }
            continue;
        }

        if ch == '"' || ch == '\'' {
            let quote = ch;
            let mut s = String::new();
            i += 1;
            while i < chars.len() {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    match chars[i + 1] {
                        'n' => {
                            s.push('\n');
                            i += 2;
                        }
                        't' => {
                            s.push('\t');
                            i += 2;
                        }
                        'r' => {
                            s.push('\r');
                            i += 2;
                        }
                        '\\' => {
                            s.push('\\');
                            i += 2;
                        }
                        '"' => {
                            s.push('"');
                            i += 2;
                        }
                        '\'' => {
                            s.push('\'');
                            i += 2;
                        }
                        'x' => {
                            if i + 3 < chars.len() {
                                let hex: String = chars[i + 2..i + 4].iter().collect();
                                if let Ok(val) = u8::from_str_radix(&hex, 16) {
                                    s.push(val as char);
                                    i += 4;
                                    continue;
                                }
                            }
                            s.push('\\');
                            s.push('x');
                            i += 2;
                        }
                        c => {
                            s.push('\\');
                            s.push(c);
                            i += 2;
                        }
                    }
                    continue;
                }
                if chars[i] == quote {
                    i += 1;
                    break;
                }
                s.push(chars[i]);
                i += 1;
            }
            tokens.push(Token::StrLit(s));
            continue;
        }

        if i + 1 < chars.len() {
            let two = format!("{}{}", ch, chars[i + 1]);
            match two.as_str() {
                "//" => {
                    tokens.push(Token::DoubleSlash);
                    i += 2;
                    continue;
                }
                "**" => {
                    tokens.push(Token::DoubleStar);
                    i += 2;
                    continue;
                }
                "==" => {
                    tokens.push(Token::EqEq);
                    i += 2;
                    continue;
                }
                "!=" => {
                    tokens.push(Token::BangEq);
                    i += 2;
                    continue;
                }
                "<=" => {
                    tokens.push(Token::LessEq);
                    i += 2;
                    continue;
                }
                ">=" => {
                    tokens.push(Token::GreaterEq);
                    i += 2;
                    continue;
                }
                "+=" => {
                    tokens.push(Token::PlusEq);
                    i += 2;
                    continue;
                }
                "-=" => {
                    tokens.push(Token::MinusEq);
                    i += 2;
                    continue;
                }
                "*=" => {
                    tokens.push(Token::StarEq);
                    i += 2;
                    continue;
                }
                "/=" => {
                    tokens.push(Token::SlashEq);
                    i += 2;
                    continue;
                }
                "%=" => {
                    tokens.push(Token::PercentEq);
                    i += 2;
                    continue;
                }
                "->" => {
                    tokens.push(Token::Arrow);
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }

        match ch {
            '+' => tokens.push(Token::Plus),
            '-' => tokens.push(Token::Minus),
            '*' => tokens.push(Token::Star),
            '/' => tokens.push(Token::Slash),
            '%' => tokens.push(Token::Percent),
            '=' => tokens.push(Token::Eq),
            '!' => return Err("unexpected character '!'".to_string()),
            '<' => tokens.push(Token::Less),
            '>' => tokens.push(Token::Greater),
            '(' => tokens.push(Token::LParen),
            ')' => tokens.push(Token::RParen),
            '[' => tokens.push(Token::LBracket),
            ']' => tokens.push(Token::RBracket),
            '{' => tokens.push(Token::LBrace),
            '}' => tokens.push(Token::RBrace),
            ',' => tokens.push(Token::Comma),
            '.' => tokens.push(Token::Dot),
            ':' => tokens.push(Token::Colon),
            '@' => tokens.push(Token::At),
            ';' => tokens.push(Token::Semicolon),
            _ => return Err(format!("unexpected character '{}'", ch)),
        }
        i += 1;
    }

    Ok(tokens)
}

#[derive(Debug, Clone)]
pub struct TokenWithMeta {
    pub token: Token,
    pub line: usize,
    pub col: usize,
}

fn find_triple_string(
    raw_lines: &[&str],
    start: usize,
    quote: char,
) -> Result<(usize, String), String> {
    let q3: String = (0..3).map(|_| quote).collect();
    let raw = raw_lines[start];
    let line_no_comment = if let Some(pos) = raw.find('#') {
        &raw[..pos]
    } else {
        raw
    };
    let Some(start_pos) = line_no_comment.find(&q3) else {
        return Err("internal: find_triple_string called but no opening triple quote".to_string());
    };
    let before = &line_no_comment[..start_pos];
    let after_start = &line_no_comment[start_pos + 3..];

    let mut content = String::new();
    let rest = after_start.trim();
    if rest.starts_with('\\') {
        let tail = rest[1..].trim_end();
        if !tail.is_empty() {
            content.push_str(&format!("\n{}", tail));
        }
    } else if !rest.is_empty() && !rest.starts_with(quote) {
        content.push_str(rest.trim_end());
    }

    let mut i = start;
    let mut closed = false;
    loop {
        if content.contains(&q3) {
            let pos = content.find(&q3).unwrap();
            content.truncate(pos);
            closed = true;
            break;
        }
        i += 1;
        if i >= raw_lines.len() {
            break;
        }
        let raw_line = raw_lines[i];
        let line = if let Some(comment_pos) = raw_line.find('#') {
            &raw_line[..comment_pos]
        } else {
            raw_line
        };
        if let Some(pos) = line.find(&q3) {
            let before_close = &line[..pos];
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str(before_close);
            closed = true;
            i += 1;
            break;
        }
        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(line.trim_end());
    }
    if !closed {
        return Err("unterminated triple-quoted string".to_string());
    }
    let other_quote = if quote == '"' { '\'' } else { '"' };
    let content_escaped = content
        .replace('\\', "\\\\")
        .replace(other_quote, &other_quote.to_string())
        .replace(quote, &format!("\\{}", quote))
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace('\r', "\\r");
    let after_close = if closed && i > 0 {
        let close_line_idx = i - 1;
        let close_line = if let Some(pos) = raw_lines[close_line_idx].find('#') {
            &raw_lines[close_line_idx][..pos]
        } else {
            raw_lines[close_line_idx]
        };
        if let Some(pos) = close_line.find(&q3) {
            close_line[pos + 3..].to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    let replacement = format!("{}\"{}\"{}", before, content_escaped, after_close);
    Ok((i - 1, replacement))
}

fn preprocess_triple_strings(raw_lines: &[&str]) -> Result<Vec<String>, String> {
    let mut lines: Vec<String> = raw_lines.iter().map(|s| s.to_string()).collect();
    let mut i = 0;
    while i < lines.len() {
        let line_no_comment = if let Some(pos) = lines[i].find('#') {
            lines[i][..pos].to_string()
        } else {
            lines[i].clone()
        };
        if line_no_comment.contains("\"\"\"") {
            let (consumed, replacement) = find_triple_string(&raw_lines, i, '"')?;
            lines[i] = replacement;
            for j in (i + 1)..=consumed.min(lines.len() - 1) {
                lines[j] = String::new();
            }
            i = consumed + 1;
        } else if line_no_comment.contains("'''") {
            let (consumed, replacement) = find_triple_string(&raw_lines, i, '\'')?;
            lines[i] = replacement;
            for j in (i + 1)..=consumed.min(lines.len() - 1) {
                lines[j] = String::new();
            }
            i = consumed + 1;
            for j in (i + 1)..=consumed.min(lines.len() - 1) {
                lines[j] = String::new();
            }
            i = consumed + 1;
        } else {
            i += 1;
        }
    }
    Ok(lines)
}

pub fn tokenize(source: &str) -> Result<Vec<TokenWithMeta>, String> {
    let mut result = Vec::new();
    let raw_lines: Vec<&str> = source.lines().collect();
    let processed_lines = preprocess_triple_strings(&raw_lines)?;
    let lines: Vec<&str> = processed_lines.iter().map(|s| s.as_str()).collect();
    let mut indent_stack: Vec<usize> = vec![0];
    let mut paren_depth: Vec<usize> = Vec::new();
    let mut line_tokens: Vec<(usize, Vec<TokenWithMeta>)> = Vec::new();

    for (line_idx, raw_line) in lines.iter().enumerate() {
        // Find the first '#' not inside a string literal
        let comment_pos = raw_line.char_indices().fold(None, |acc, (i, c)| {
            if acc.is_some() {
                return acc;
            }
            if c == '#' {
                // Check we're not in a string by scanning from start
                let mut in_single = false;
                let mut in_double = false;
                let mut prev = ' ';
                for (_, pc) in raw_line[..i].char_indices() {
                    if pc == '\\' && prev == '\\' {
                        prev = ' ';
                        continue;
                    }
                    if pc == '\\' {
                        prev = '\\';
                        continue;
                    }
                    prev = pc;
                    if pc == '\'' && !in_double {
                        in_single = !in_single;
                    }
                    if pc == '"' && !in_single {
                        in_double = !in_double;
                    }
                }
                if !in_single && !in_double {
                    Some(i)
                } else {
                    None
                }
            } else {
                None
            }
        });
        let line = if let Some(pos) = comment_pos {
            &raw_line[..pos]
        } else {
            raw_line
        };

        let trimmed = line.trim_end();

        if trimmed.is_empty() {
            continue;
        }

        let leading = line.len() - line.trim_start().len();
        let content = trimmed;

        let starts_in_paren = !paren_depth.is_empty();

        let toks = tokenize_line(content)?;
        let mut meta_toks = Vec::new();
        for tok in toks {
            match &tok {
                Token::LParen | Token::LBracket | Token::LBrace => {
                    paren_depth.push(1);
                }
                Token::RParen | Token::RBracket | Token::RBrace => {
                    paren_depth.pop();
                }
                _ => {}
            }
            meta_toks.push(TokenWithMeta {
                token: tok,
                line: line_idx + 1,
                col: 0,
            });
        }

        if starts_in_paren {
            if !meta_toks.is_empty() {
                line_tokens.push((line_idx, meta_toks));
            }
            continue;
        }

        let last_indent = *indent_stack.last().unwrap_or(&0);

        if leading > last_indent {
            indent_stack.push(leading);
            let indent_tok = TokenWithMeta {
                token: Token::Indent,
                line: line_idx + 1,
                col: 0,
            };
            if let Some(last) = line_tokens.last_mut() {
                if !last.1.is_empty() {
                    if let Some(end) = last.1.last_mut() {
                        if matches!(end.token, Token::Colon) {
                            last.1.push(indent_tok);
                        } else {
                            last.1.push(TokenWithMeta {
                                token: Token::Newline,
                                line: line_idx + 1,
                                col: 0,
                            });
                            last.1.push(indent_tok);
                        }
                    } else {
                        last.1.push(TokenWithMeta {
                            token: Token::Newline,
                            line: line_idx + 1,
                            col: 0,
                        });
                        last.1.push(indent_tok);
                    }
                }
            }
        } else if leading < last_indent {
            while let Some(&top) = indent_stack.last() {
                if top > leading {
                    indent_stack.pop();
                    if let Some(last) = line_tokens.last_mut() {
                        last.1.push(TokenWithMeta {
                            token: Token::Dedent,
                            line: line_idx + 1,
                            col: 0,
                        });
                    }
                } else {
                    break;
                }
            }
        }

        if line_tokens.last().map_or(true, |(_, t)| {
            t.is_empty() || !matches!(t.last().unwrap().token, Token::Indent | Token::Dedent)
        }) {
            if !line_tokens.is_empty() {
                let last_line = line_tokens.last_mut().unwrap();
                if !last_line.1.is_empty()
                    && !matches!(
                        last_line.1.last().unwrap().token,
                        Token::Indent | Token::Dedent | Token::Newline
                    )
                {
                    if !meta_toks.is_empty() || (meta_toks.is_empty() && !content.trim().is_empty())
                    {
                        let mut push_newline = true;
                        if let Some(nl) = last_line.1.last() {
                            if matches!(nl.token, Token::Newline) {
                                push_newline = false;
                            }
                        }
                        if push_newline {
                            last_line.1.push(TokenWithMeta {
                                token: Token::Newline,
                                line: line_idx + 1,
                                col: 0,
                            });
                        }
                    }
                }
            }
        }

        if !meta_toks.is_empty() {
            line_tokens.push((line_idx, meta_toks));
        }
    }

    while indent_stack.len() > 1 {
        indent_stack.pop();
        line_tokens.push((
            lines.len(),
            vec![TokenWithMeta {
                token: Token::Dedent,
                line: lines.len(),
                col: 0,
            }],
        ));
    }

    for (_, toks) in &line_tokens {
        result.extend(toks.iter().cloned());
    }

    Ok(result)
}
