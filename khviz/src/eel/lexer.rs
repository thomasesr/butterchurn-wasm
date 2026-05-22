#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    Eq,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    CaretEq,
    Lt,
    Gt,
    Amp,
    Pipe,
    Bang,
    Comma,
    Semicolon,
    LParen,
    RParen,
    Eof,
}

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(src: &str) -> Self {
        Self {
            input: src.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<char> {
        self.input.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.input.get(self.pos).copied();
        self.pos += 1;
        ch
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while self.peek().map_or(false, |c| c.is_whitespace()) {
                self.advance();
            }
            if self.peek() == Some('/') && self.peek2() == Some('/') {
                while self.peek().map_or(false, |c| c != '\n') {
                    self.advance();
                }
                continue;
            }
            if self.peek() == Some('/') && self.peek2() == Some('*') {
                self.pos += 2;
                while self.pos + 1 < self.input.len() {
                    if self.input[self.pos] == '*' && self.input[self.pos + 1] == '/' {
                        self.pos += 2;
                        break;
                    }
                    self.pos += 1;
                }
                continue;
            }
            break;
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            match self.peek() {
                None => {
                    tokens.push(Token::Eof);
                    break;
                }
                Some(c) => {
                    if let Some(tok) = self.next_token(c) {
                        tokens.push(tok);
                    }
                }
            }
        }
        tokens
    }

    fn next_token(&mut self, c: char) -> Option<Token> {
        if c.is_ascii_digit()
            || (c == '.' && self.peek2().map_or(false, |d| d.is_ascii_digit()))
        {
            return Some(self.read_number());
        }
        if c.is_alphabetic() || c == '_' {
            return Some(self.read_ident());
        }
        self.advance();
        let tok = match c {
            '+' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::PlusEq
                } else {
                    Token::Plus
                }
            }
            '-' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::MinusEq
                } else {
                    Token::Minus
                }
            }
            '*' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::StarEq
                } else {
                    Token::Star
                }
            }
            '/' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::SlashEq
                } else {
                    Token::Slash
                }
            }
            '%' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::PercentEq
                } else {
                    Token::Percent
                }
            }
            '^' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::CaretEq
                } else {
                    Token::Caret
                }
            }
            '=' => Token::Eq,
            '<' => Token::Lt,
            '>' => Token::Gt,
            '&' => Token::Amp,
            '|' => Token::Pipe,
            '!' => Token::Bang,
            ',' => Token::Comma,
            ';' => Token::Semicolon,
            '(' => Token::LParen,
            ')' => Token::RParen,
            _ => return None,
        };
        Some(tok)
    }

    fn read_number(&mut self) -> Token {
        let start = self.pos;
        while self.peek().map_or(false, |c| c.is_ascii_digit() || c == '.') {
            self.advance();
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            self.advance();
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.advance();
            }
            while self.peek().map_or(false, |c| c.is_ascii_digit()) {
                self.advance();
            }
        }
        let s: String = self.input[start..self.pos].iter().collect();
        Token::Number(s.parse::<f64>().unwrap_or(0.0))
    }

    fn read_ident(&mut self) -> Token {
        let start = self.pos;
        while self
            .peek()
            .map_or(false, |c| c.is_alphanumeric() || c == '_')
        {
            self.advance();
        }
        let name: String = self.input[start..self.pos].iter().collect();
        Token::Ident(name)
    }
}
