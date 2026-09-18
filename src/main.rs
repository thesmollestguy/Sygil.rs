use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{Read, Write, self};
use std::path::Path;
use std::process;

const SYGIL_VERSION: i16 = -1013;

#[derive(Debug)]
enum SygilError {
    Syntax(String),
    Format(String),
    Version(String),
    Runtime(String),
}

impl std::fmt::Display for SygilError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SygilError::Syntax(msg) => write!(f, "SygilSyntaxError: {}", msg),
            SygilError::Format(msg) => write!(f, "FormatError: {}", msg),
            SygilError::Version(msg) => write!(f, "VersionError: {}", msg),
            SygilError::Runtime(msg) => write!(f, "RuntimeError: {}", msg),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
enum Token {
    Separator = 0x01,
    Default = 0x02,
    Comma = 0x03,
    LCurl = 0x06,
    RCurl = 0x07,
    LBrack = 0x08,
    RBrack = 0x09,
    LPipe = 0x0A,
    RPipe = 0x0B,
    LAngle = 0x0C,
    RAngle = 0x0D,
    LQuote = 0x0E,
    RQuote = 0x0F,
    DeclVar = 0x10,
    UseVar = 0x11,
    DeclClass = 0x15,
    UseClass = 0x16,
    DeclFunc = 0x1A,
    Call = 0x1B,
    Int = 0x30,
    Float = 0x31,
    Long = 0x32,
    True = 0x35,
    False = 0x36,
    Short = 0x38,
    Sub = 0x40,
    Add = 0x41,
    Div = 0x42,
    Mult = 0x43,
    Pow = 0x44,
    Equ = 0x50,
    Neq = 0x51,
    And = 0x52,
    Or = 0x53,
    LParen = 0x60,
    RParen = 0x61,
}

impl Token {
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Token::Separator),
            0x02 => Some(Token::Default),
            0x03 => Some(Token::Comma),
            0x06 => Some(Token::LCurl),
            0x07 => Some(Token::RCurl),
            0x08 => Some(Token::LBrack),
            0x09 => Some(Token::RBrack),
            0x0A => Some(Token::LPipe),
            0x0B => Some(Token::RPipe),
            0x0C => Some(Token::LAngle),
            0x0D => Some(Token::RAngle),
            0x0E => Some(Token::LQuote),
            0x0F => Some(Token::RQuote),
            0x10 => Some(Token::DeclVar),
            0x11 => Some(Token::UseVar),
            0x15 => Some(Token::DeclClass),
            0x16 => Some(Token::UseClass),
            0x1A => Some(Token::DeclFunc),
            0x1B => Some(Token::Call),
            0x30 => Some(Token::Int),
            0x31 => Some(Token::Float),
            0x32 => Some(Token::Long),
            0x35 => Some(Token::True),
            0x36 => Some(Token::False),
            0x38 => Some(Token::Short),
            0x40 => Some(Token::Sub),
            0x41 => Some(Token::Add),
            0x42 => Some(Token::Div),
            0x43 => Some(Token::Mult),
            0x44 => Some(Token::Pow),
            0x50 => Some(Token::Equ),
            0x51 => Some(Token::Neq),
            0x52 => Some(Token::And),
            0x53 => Some(Token::Or),
            0x60 => Some(Token::LParen),
            0x61 => Some(Token::RParen),
            _ => None,
        }
    }
}

fn format_ver_num(ver: i16) -> String {
    let width = 5;
    let zfilled = format!("{:0>width$}", ver.abs().to_string(), width = width);
    let ver_nums: Vec<&str> = zfilled.char_indices()
        .map(|(i, c)| &zfilled[i..i + c.len_utf8()])
        .collect();
    if ver.abs() != ver {
        format!("{}-{}.{}{}.{}", "Alpha", ver_nums[1], ver_nums[2], ver_nums[3], ver_nums[4])
    } else {
        format!("{}.{}{}.{}", ver_nums[1], ver_nums[2], ver_nums[3], ver_nums[4])
    }
}

struct Checker {
    source: Vec<char>,
    pos: usize,
    in_quote: bool,
    in_pipe: bool,
    expecting: Option<Vec<&'static str>>,
    prev_token: Option<&'static str>,
    line: usize,
    line_pos: usize,
}

impl Checker {
    fn new(path: &str) -> Result<Self, SygilError> {
        let source_str = fs::read_to_string(path)
            .map_err(|e| SygilError::Runtime(format!("Could not read file {}: {}", path, e)))?;
        Ok(Self {
            source: source_str.chars().collect(),
            pos: 0,
            in_quote: false,
            in_pipe: false,
            expecting: None,
            prev_token: None,
            line: 1,
            line_pos: 1,
        })
    }

    fn get_expanded_id(token_id: &str) -> String {
        let s_key;
        match token_id {
            "DECL_VAR" => s_key = "DECLARE VARIABLE (':')",
            "USE_CLASS" => s_key = "USE CLASS ('@')",
            "CALL" => s_key = "CALL FUNCTION ('_')",
            "LCURL" => s_key = "LEFT CURLY BRACKET ('{')",
            "RCURL" => s_key = "RIGHT CURLY BRACKET ('}')",
            "LBRACK" => s_key = "LEFT BRACKET ('[')",
            "RBRACK" => s_key = "RIGHT BRACKET (']')",
            "LPAREN" => s_key = "LEFT PARENTHESIS ('(')",
            "RPAREN" => s_key = "RIGHT PARENTHESIS (')')",
            "LANGLE" => s_key = "LEFT ANGLE BRACKET ('<')",
            "RANGLE" => s_key = "RIGHT ANGLE BRACKET ('>')",
            "DEFAULT" => s_key = "TILDE ('~')",
            "LPIPE" => s_key = "LEFT PIPE ('|')",
            "RPIPE" => s_key = "RIGHT PIPE ('|')",
            "SEPARATOR" => s_key = "SEMICOLON (';')",
            "USE_VAR" => s_key = "USE VARIABLE ('$')",
            "SUB" => s_key = "SUBTRACT ('-')",
            "ADD" => s_key = "ADD ('+')",
            "MULT" => s_key = "MULTIPLY ('*')",
            "DIV" => s_key = "DIVIDE ('/')",
            "POW" => s_key = "EXPONENTIATE ('^')",
            "COMMA" => s_key = "COMMA (',')",
            "NEQ" => s_key = "NOT EQUAL ('!')",
            "EQU" => s_key = "EQUAL ('=')",
            "OR" => s_key = "OR ('&')",
            "AND" => s_key = "AND ('&&')",
            _ => s_key = token_id
        }
        s_key.to_string()
    }

    fn get_collapsed_id(token_id: &str) -> String {
        let s_key;
        match token_id {
            "DECL_VAR" => s_key = "''",
            "USE_CLASS" => s_key = "@",
            "CALL" => s_key = "_",
            "LCURL" => s_key = "{",
            "RCURL" => s_key = "}",
            "LBRACK" => s_key = "[",
            "RBRACK" => s_key = "]",
            "LANGLE" => s_key = "<",
            "RANGLE" => s_key = ">",
            "DEFAULT" => s_key = "~",
            "LPIPE" => s_key = "|",
            "RPIPE" => s_key = "|",
            "LPAREN" => s_key = "(",
            "RPAREN" => s_key = ")",
            "SEPARATOR" => s_key = ";",
            "USE_VAR" => s_key = "$",
            "SUB" => s_key = "-",
            "ADD" => s_key = "+",
            "MULT" => s_key = "*",
            "DIV" => s_key = "/",
            "POW" => s_key = "^",
            "COMMA" => s_key = ",",
            "NEQ" => s_key = "!",
            "EQU" => s_key = "=",
            "OR" => s_key = "&",
            "AND" => s_key = "&&",
            "DECL_CLASS" => s_key = ":@",
            "DECL_FUNC" => s_key = "::",
            "TRUE" => s_key = "?1",
            "FALSE" => s_key = "?0",
            "LQUOTE" => s_key = "\"",
            "RQUOTE" => s_key = "\"",
            _ => s_key = token_id
        }
        s_key.to_string()
    }

    fn inc_pos(&mut self, amount: usize) {
        self.pos += amount;
        self.line_pos += amount;
    }

    fn peek(&self) -> char {
        if self.pos + 1 < self.source.len() {
            self.source[self.pos + 1]
        } else {
            '\0'
        }
    }

    fn expect(&mut self, token_keys: Option<Vec<&'static str>>) {
        self.expecting = token_keys;
    }

    fn handle_identifier(&mut self, check_reserved: bool) -> Result<(usize, String), SygilError> {
        let mut name = String::new();
        while self.pos < self.source.len()
            && (self.source[self.pos].is_alphanumeric() || self.source[self.pos] == '_')
        {
            name.push(self.source[self.pos]);
            self.inc_pos(1);
        }

        if !name.is_empty() && check_reserved {
            let reserved = ["__global__", "__function__", "__parent__", "print", "return", "if", "while"];
            if reserved.contains(&name.as_str()) {
                return Err(SygilError::Syntax(format!(
                    "\"{}\" is a reserved keyword in all contexts",
                    name
                )));
            }
        }
        Ok((name.len(), name))
    }

    fn handle_string_content(&mut self) {
        while self.pos < self.source.len() && self.source[self.pos] != '"' {
            self.inc_pos(1);
        }
    }

    fn handle_number(&mut self) {
        while self.pos < self.source.len()
            && (self.source[self.pos].is_digit(10) || self.source[self.pos] == '.')
        {
            self.inc_pos(1);
        }
    }

    fn get_expects(s_key: &str) -> Option<Vec<&'static str>> {
        match s_key {
            "DECL_VAR" => Some(vec!["SEPARATOR", "COMMA", "RANGLE"]),
            "DECL_FUNC" => Some(vec!["LANGLE"]),
            "CALL" => Some(vec!["LANGLE"]),
            "DECL_CLASS" => Some(vec!["SEPARATOR"]),
            "USE_CLASS" => Some(vec!["USE_CLASS", "USE_VAR", "CALL", "DECL_CLASS", "DECL_FUNC", "DECL_VAR"]),
            _ => None,
        }
    }

    fn check(&mut self) -> Result<(), SygilError> {
        let mut commenting = 0;

        while self.pos < self.source.len() {
            let char = self.source[self.pos];

            if char == '\n' {
                self.line += 1;
                self.line_pos = 0;
            }

            if char == '#' {
                commenting = 1;
                self.inc_pos(1);
                continue;
            } else if char == '`' {
                commenting = if commenting == 2 { 0 } else { 2 };
                self.inc_pos(1);
                continue;
            } else if char.is_whitespace() {
                self.inc_pos(1);
                if commenting == 1 && char == '\n' {
                    commenting = 0;
                }
                continue;
            } else if commenting > 0 {
                self.inc_pos(1);
                continue;
            }

            let s_key;

            if char == ':' && self.peek() == ':' {
                self.inc_pos(2);
                self.handle_identifier(true)?;
                s_key = "DECL_FUNC";
            } else if char == '?' && (self.peek() == '0' || self.peek() == '1') {
                s_key = if self.peek() == '1' { "TRUE" } else { "FALSE" };
                self.inc_pos(2);
            } else if char == ':' && self.peek() == '@' {
                self.inc_pos(2);
                self.handle_identifier(true)?;
                s_key = "DECL_CLASS";
            } else if char == '&' && self.peek() == '&'{
                self.inc_pos(2);
                s_key = "AND";
            } else if char == '"' {
                s_key = if self.in_quote { "RQUOTE" } else { "LQUOTE" };
                self.in_quote = !self.in_quote;
                self.inc_pos(1);
                if self.in_quote {
                    self.handle_string_content();
                }
            } else if char == '|' {
                s_key = if self.in_pipe { "RPIPE" } else { "LPIPE" };
                self.in_pipe = !self.in_pipe;
                self.inc_pos(1);
            } else {
                match char {
                    ':' => s_key = "DECL_VAR",
                    '@' => s_key = "USE_CLASS",
                    '_' => s_key = "CALL",
                    '{' => s_key = "LCURL",
                    '}' => s_key = "RCURL",
                    '[' => s_key = "LBRACK",
                    ']' => s_key = "RBRACK",
                    '<' => s_key = "LANGLE",
                    '>' => s_key = "RANGLE",
                    '(' => s_key = "LPAREN",
                    ')' => s_key = "RPAREN",
                    '~' => s_key = "DEFAULT",
                    ';' => s_key = "SEPARATOR",
                    '$' => s_key = "USE_VAR",
                    '-' => s_key = "SUB",
                    '+' => s_key = "ADD",
                    '*' => s_key = "MULT",
                    '/' => s_key = "DIV",
                    '^' => s_key = "POW",
                    ',' => s_key = "COMMA",
                    '!' => s_key = "NEQ",
                    '=' => s_key = "EQU",
                    '&' => s_key = "OR",
                    _ if char.is_digit(10) => {
                        self.handle_number();
                        continue;
                    }
                    _ => {
                        self.inc_pos(1);
                        continue;
                    }
                }
                self.inc_pos(1);

                if s_key == "CALL" || s_key == "USE_VAR" || s_key == "USE_CLASS" {
                    self.handle_identifier(false)?;
                } else if s_key == "DECL_VAR" {
                    let (name_len, _) = self.handle_identifier(true)?;
                    if name_len == 2 {
                        return Err(SygilError::Syntax(
                            "Variables may not be 2 characters long.".to_string(),
                        ));
                    }
                }
            }

            if !s_key.is_empty() {
                if let Some(expected) = &self.expecting {
                    if !expected.contains(&s_key) {
                        let mut expect_list = Vec::new();
                        for expstr in expected {
                            expect_list.push(Checker::get_collapsed_id(expstr));
                        }
                        return Err(SygilError::Syntax(format!(
                            "Token after {:?} was not in {:?} ({}, {})",
                            Checker::get_expanded_id(self.prev_token.unwrap()), expect_list, self.line, self.line_pos
                        )));
                    }
                }
                self.prev_token = Some(s_key);
                self.expect(Self::get_expects(s_key));
            }
        }
        Ok(())
    }
}

struct Tokenizer {
    source: Vec<char>,
    pos: usize,
    in_quote: bool,
    in_pipe: bool,
}

impl Tokenizer {
    fn new(path: &str) -> Result<Self, SygilError> {
        let source_str = fs::read_to_string(path)
            .map_err(|e| SygilError::Runtime(format!("Could not read file {}: {}", path, e)))?;
        Ok(Self {
            source: source_str.chars().collect(),
            pos: 0,
            in_quote: false,
            in_pipe: false,
        })
    }

    fn peek(&self) -> char {
        if self.pos + 1 < self.source.len() {
            self.source[self.pos + 1]
        } else {
            '\0'
        }
    }

    fn handle_identifier(&mut self, out_file: &mut File) -> Result<(), std::io::Error> {
        let mut name = String::new();
        while self.pos < self.source.len()
            && (self.source[self.pos].is_alphanumeric() || self.source[self.pos] == '_')
        {
            name.push(self.source[self.pos]);
            self.pos += 1;
        }

        if !name.is_empty() {
            let name_bytes = name.as_bytes();
            out_file.write_all(&[name_bytes.len() as u8])?;
            out_file.write_all(name_bytes)?;
        }
        Ok(())
    }

    fn handle_string_content(&mut self, out_file: &mut File) -> Result<(), std::io::Error> {
        let mut content = String::new();
        while self.pos < self.source.len() && self.source[self.pos] != '"' {
            content.push(self.source[self.pos]);
            self.pos += 1;
        }

        if !content.is_empty() {
            out_file.write_all(content.as_bytes())?;
        }
        Ok(())
    }

    fn handle_number(&mut self, out_file: &mut File) -> Result<(), std::io::Error> {
        let mut num_str = String::new();
        while self.pos < self.source.len()
            && (self.source[self.pos].is_digit(10) || self.source[self.pos] == '.')
        {
            num_str.push(self.source[self.pos]);
            self.pos += 1;
        }

        if !num_str.is_empty() {
            if num_str.contains('.') {
                if let Ok(f_val) = num_str.parse::<f64>() {
                    out_file.write_all(&[Token::Float as u8])?;
                    out_file.write_all(&f_val.to_be_bytes())?;
                }
            } else if let Ok(i_val) = num_str.parse::<i64>() {
                if i_val.abs() > 2147483647 {
                    out_file.write_all(&[Token::Long as u8])?;
                    out_file.write_all(&(i_val as i64).to_be_bytes())?;
                } else if i_val.abs() < 32768 {
                    out_file.write_all(&[Token::Short as u8])?;
                    out_file.write_all(&(i_val as i16).to_be_bytes())?;
                } else {
                    out_file.write_all(&[Token::Int as u8])?;
                    out_file.write_all(&(i_val as i32).to_be_bytes())?;
                }
            }
        }
        Ok(())
    }

    fn tokenize(&mut self, output_path: &str) -> Result<(), SygilError> {
        let mut out_file = File::create(output_path).map_err(|e| {
            SygilError::Runtime(format!("Could not create output file {}: {}", output_path, e))
        })?;

        out_file.write_all(b"sygil").map_err(|e| SygilError::Runtime(e.to_string()))?;
        out_file.write_all(&SYGIL_VERSION.to_be_bytes()).map_err(|e| SygilError::Runtime(e.to_string()))?;

        let mut commenting = 0;

        while self.pos < self.source.len() {
            let char = self.source[self.pos];

            if char == '#' {
                commenting = 1;
                self.pos += 1;
                continue;
            }
            if char == '`' {
                commenting = if commenting == 2 { 0 } else { 2 };
                self.pos += 1;
                continue;
            }
            if char.is_whitespace() {
                self.pos += 1;
                if commenting == 1 && char == '\n' {
                    commenting = 0;
                }
                continue;
            }

            if commenting > 0 {
                self.pos += 1;
                continue;
            }

            if char == ':' && self.peek() == ':' {
                out_file.write_all(&[Token::DeclFunc as u8]).unwrap();
                self.pos += 2;
                self.handle_identifier(&mut out_file).unwrap();
                continue;
            }

            if char == '?' && (self.peek() == '0' || self.peek() == '1') {
                let token = if self.peek() == '1' { Token::True } else { Token::False };
                out_file.write_all(&[token as u8]).unwrap();
                self.pos += 2;
                continue;
            }

            if char == ':' && self.peek() == '@' {
                out_file.write_all(&[Token::DeclClass as u8]).unwrap();
                self.pos += 2;
                self.handle_identifier(&mut out_file).unwrap();
                continue;
            }

            if char == '&' && self.peek() == '&' {
                let _ = out_file.write(&[Token::And as u8]);
                self.pos+=2;
                continue;
            }

            if char == '"' {
                let token = if self.in_quote { Token::RQuote } else { Token::LQuote };
                out_file.write_all(&[token as u8]).unwrap();
                self.in_quote = !self.in_quote;
                self.pos += 1;
                if self.in_quote {
                    self.handle_string_content(&mut out_file).unwrap();
                }
                continue;
            }

            if char == '|' {
                let token = if self.in_pipe { Token::RPipe } else { Token::LPipe };
                out_file.write_all(&[token as u8]).unwrap();
                self.in_pipe = !self.in_pipe;
                self.pos += 1;
                continue;
            }

            let s_key = match char {
                ':' => Some(Token::DeclVar),
                '@' => Some(Token::UseClass),
                '_' => Some(Token::Call),
                '{' => Some(Token::LCurl),
                '}' => Some(Token::RCurl),
                '[' => Some(Token::LBrack),
                ']' => Some(Token::RBrack),
                '<' => Some(Token::LAngle),
                '>' => Some(Token::RAngle),
                '~' => Some(Token::Default),
                '(' => Some(Token::LParen),
                ')' => Some(Token::RParen),
                ';' => Some(Token::Separator),
                '$' => Some(Token::UseVar),
                '-' => Some(Token::Sub),
                '+' => Some(Token::Add),
                '*' => Some(Token::Mult),
                '/' => Some(Token::Div),
                '^' => Some(Token::Pow),
                ',' => Some(Token::Comma),
                '!' => Some(Token::Neq),
                '=' => Some(Token::Equ),
                '&' => Some(Token::Or),
                _ => None,
            };

            if let Some(token) = s_key {
                out_file.write_all(&[token.clone() as u8]).unwrap();
                self.pos += 1;

                if token == Token::DeclVar
                    || token == Token::Call
                    || token == Token::UseVar
                    || token == Token::UseClass
                {
                    self.handle_identifier(&mut out_file).unwrap();
                }
                continue;
            }

            if char.is_digit(10) {
                self.handle_number(&mut out_file).unwrap();
                continue;
            }

            self.pos += 1;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
enum SygilValue {
    Integer(i32),
    Long(i64),
    Short(i16),
    Float(f64),
    Boolean(bool),
    String(String),
    //List(Vec<SygilValue>),
    Null,
}

impl std::fmt::Display for SygilValue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SygilValue::Integer(v) => write!(f, "{}", v),
            SygilValue::Float(v) => write!(f, "{}", v),
            SygilValue::Boolean(v) => write!(f, "{}", if *v { "True" } else { "False" }),
            SygilValue::String(v) => write!(f, "{}", v),
            SygilValue::Long(v) => write!(f, "{}", v),
            SygilValue::Short(v) => write!(f, "{}", v),
            /*SygilValue::List(v) => {
                let strings: Vec<String> = v.iter().map(|item| item.to_string()).collect();
                write!(f, "[{}]", strings.join(", "))
            }*/
            SygilValue::Null => write!(f, "null"),
        }
    }
}

impl SygilValue {
    fn add(&self, other: &SygilValue) -> SygilValue {
        match (self, other) {
            (SygilValue::Integer(a), SygilValue::Integer(b)) => SygilValue::Integer(a + b),
            (SygilValue::Float(a), SygilValue::Float(b)) => SygilValue::Float(a + b),
            (SygilValue::String(a), SygilValue::String(b)) => SygilValue::String(format!("{}{}", a, b)),
            (SygilValue::Short(a), SygilValue::Short(b)) => SygilValue::Integer((a + b) as i32),
            (SygilValue::Short(a), SygilValue::Integer(b)) => SygilValue::Integer(((*a as i32)  + b) as i32),
            (SygilValue::Integer(a), SygilValue::Short(b)) => SygilValue::Integer(((*b as i32)  + a) as i32),
            (SygilValue::Integer(a), SygilValue::Float(b)) => SygilValue::Float((*a as f64) + b),
            (SygilValue::Float(a), SygilValue::Integer(b)) => SygilValue::Float((*b as f64) + a),
            (SygilValue::Short(a), SygilValue::Float(b)) => SygilValue::Float((*a as f64) + b),
            (SygilValue::Float(a), SygilValue::Short(b)) => SygilValue::Float((*b as f64) + a),
            (SygilValue::Long(a), SygilValue::Float(b)) => SygilValue::Float((*a as f64) + b),
            (SygilValue::Float(a), SygilValue::Long(b)) => SygilValue::Float((*b as f64) + a),
            (SygilValue::Long(a), SygilValue::Long(b)) => SygilValue::Long((a + b) as i64),
            (SygilValue::Short(a), SygilValue::Long(b)) => SygilValue::Long((*a as i64)  + b),
            (SygilValue::Long(a), SygilValue::Short(b)) => SygilValue::Long((*b as i64)  + a),
            (SygilValue::Integer(a), SygilValue::Long(b)) => SygilValue::Long((*a as i64)  + b),
            (SygilValue::Long(a), SygilValue::Integer(b)) => SygilValue::Long((*b as i64)  + a),
            _ => SygilValue::Null,
        }
    }
    fn sub(&self, other: &SygilValue) -> SygilValue {
        match (self, other) {
            (SygilValue::Integer(a), SygilValue::Integer(b)) => SygilValue::Integer(a - b),
            (SygilValue::Float(a), SygilValue::Float(b)) => SygilValue::Float(a - b),
            (SygilValue::Short(a), SygilValue::Short(b)) => SygilValue::Integer((a - b) as i32),
            (SygilValue::Short(a), SygilValue::Integer(b)) => SygilValue::Integer((*a as i32) - b),
            (SygilValue::Integer(a), SygilValue::Short(b)) => SygilValue::Integer((*b as i32) - a),
            (SygilValue::Integer(a), SygilValue::Float(b)) => SygilValue::Float((*a as f64) - b),
            (SygilValue::Float(a), SygilValue::Integer(b)) => SygilValue::Float((*b as f64) - a),
            (SygilValue::Short(a), SygilValue::Float(b)) => SygilValue::Float((*a as f64) - b),
            (SygilValue::Float(a), SygilValue::Short(b)) => SygilValue::Float((*b as f64) - a),
            (SygilValue::Long(a), SygilValue::Float(b)) => SygilValue::Float((*a as f64) - b),
            (SygilValue::Float(a), SygilValue::Long(b)) => SygilValue::Float((*b as f64) - a),
            (SygilValue::Long(a), SygilValue::Long(b)) => SygilValue::Long((a - b) as i64),
            (SygilValue::Short(a), SygilValue::Long(b)) => SygilValue::Long((*a as i64) - b),
            (SygilValue::Long(a), SygilValue::Short(b)) => SygilValue::Long((*b as i64) - a),
            (SygilValue::Integer(a), SygilValue::Long(b)) => SygilValue::Long((*a as i64) - b),
            (SygilValue::Long(a), SygilValue::Integer(b)) => SygilValue::Long((*b as i64) - a),
            _ => SygilValue::Null,
        }
    }
    fn mult(&self, other: &SygilValue) -> SygilValue {
        match (self, other) {
            (SygilValue::Integer(a), SygilValue::Integer(b)) => SygilValue::Long((*a as i64) * (*b as i64)),
            (SygilValue::Float(a), SygilValue::Float(b)) => SygilValue::Float(a * b),
            (SygilValue::Short(a), SygilValue::Short(b)) => SygilValue::Integer((*a as i32) * (*b as i32)),
            (SygilValue::Short(a), SygilValue::Integer(b)) => SygilValue::Long((*a as i64) * (*b as i64)),
            (SygilValue::Integer(a), SygilValue::Short(b)) => SygilValue::Long((*a as i64) * (*b as i64)),
            (SygilValue::Integer(a), SygilValue::Float(b)) => SygilValue::Float((*a as f64) - b),
            (SygilValue::Float(a), SygilValue::Integer(b)) => SygilValue::Float((*b as f64) - a),
            (SygilValue::Short(a), SygilValue::Float(b)) => SygilValue::Float((*a as f64) - b),
            (SygilValue::Float(a), SygilValue::Short(b)) => SygilValue::Float((*b as f64) - a),
            (SygilValue::Long(a), SygilValue::Float(b)) => SygilValue::Float((*a as f64) - b),
            (SygilValue::Float(a), SygilValue::Long(b)) => SygilValue::Float((*b as f64) - a),
            (SygilValue::Long(a), SygilValue::Long(b)) => SygilValue::Long(a * b),
            (SygilValue::Short(a), SygilValue::Long(b)) => SygilValue::Long((*a as i64) * b),
            (SygilValue::Long(a), SygilValue::Short(b)) => SygilValue::Long((*b as i64) * a),
            (SygilValue::Integer(a), SygilValue::Long(b)) => SygilValue::Long((*a as i64) * b),
            (SygilValue::Long(a), SygilValue::Integer(b)) => SygilValue::Long((*b as i64) * a),
            _ => SygilValue::Null,
        }
    }
    fn div(&self, other: &SygilValue) -> SygilValue {
        match (self, other) {
            (SygilValue::Short(a), SygilValue::Short(b)) => 
                if *b!=(0 as i16) { SygilValue::Float((*a as f64) / (*b as f64)) } else { SygilValue::Null },
            (SygilValue::Short(a), SygilValue::Integer(b)) => 
                if *b!=(0 as i32) { SygilValue::Float((*a as f64) / (*b as f64)) } else { SygilValue::Null },
            (SygilValue::Short(a), SygilValue::Long(b)) => 
                if *b!=(0 as i64) { SygilValue::Float((*a as f64) / (*b as f64)) } else { SygilValue::Null },
            (SygilValue::Short(a), SygilValue::Float(b)) => 
                if *b!=(0 as f64) { SygilValue::Float((*a as f64) / (*b as f64)) } else { SygilValue::Null },
            (SygilValue::Integer(a), SygilValue::Short(b)) => 
                if *b!=(0 as i16) { SygilValue::Float((*a as f64) / (*b as f64)) } else { SygilValue::Null },
            (SygilValue::Integer(a), SygilValue::Integer(b)) => 
                if *b!=(0 as i32) { SygilValue::Float((*a as f64) / (*b as f64)) } else { SygilValue::Null },
            (SygilValue::Integer(a), SygilValue::Long(b)) => 
                if *b!=(0 as i64) { SygilValue::Float((*a as f64) / (*b as f64)) } else { SygilValue::Null },
            (SygilValue::Integer(a), SygilValue::Float(b)) => 
                if *b!=(0 as f64) { SygilValue::Float((*a as f64) / (*b as f64)) } else { SygilValue::Null },
            (SygilValue::Long(a), SygilValue::Short(b)) => 
                if *b!=(0 as i16) { SygilValue::Float((*a as f64) / (*b as f64)) } else { SygilValue::Null },
            (SygilValue::Long(a), SygilValue::Integer(b)) => 
                if *b!=(0 as i32) { SygilValue::Float((*a as f64) / (*b as f64)) } else { SygilValue::Null },
            (SygilValue::Long(a), SygilValue::Long(b)) => 
                if *b!=(0 as i64) { SygilValue::Float((*a as f64) / (*b as f64)) } else { SygilValue::Null },
            (SygilValue::Long(a), SygilValue::Float(b)) => 
                if *b!=(0 as f64) { SygilValue::Float((*a as f64) / (*b as f64)) } else { SygilValue::Null },
            (SygilValue::Float(a), SygilValue::Short(b)) => 
                if *b!=(0 as i16) { SygilValue::Float((*a as f64) / (*b as f64)) } else { SygilValue::Null },
            (SygilValue::Float(a), SygilValue::Integer(b)) => 
                if *b!=(0 as i32) { SygilValue::Float((*a as f64) / (*b as f64)) } else { SygilValue::Null },
            (SygilValue::Float(a), SygilValue::Long(b)) => 
                if *b!=(0 as i64) { SygilValue::Float((*a as f64) / (*b as f64)) } else { SygilValue::Null },
            (SygilValue::Float(a), SygilValue::Float(b)) => 
                if *b!=(0 as f64) { SygilValue::Float((*a as f64) / (*b as f64)) } else { SygilValue::Null },
            _ => SygilValue::Null,
        }
    }
    fn unwrap(&self) -> bool {
        match self {
            SygilValue::Boolean(b) => *b,
            _ => panic!("Unwrap called on non-boolean")
        }
    }
}

#[derive(Clone, Debug)]
struct SygilFunction {
    params: Vec<String>,
    code: Vec<u8>,
    defaults: Vec<SygilValue>,
}

#[derive(Clone, Debug)]
struct Context {
    variables: HashMap<String, SygilValue>,
    functions: HashMap<String, SygilFunction>,
}

impl Context {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
        }
    }
}

#[derive(Clone)]
struct ReadableByteBuffer {
    bytes: Vec<u8>,
    pos: usize,
}

impl ReadableByteBuffer {
    fn new(bytes: Vec<u8>) -> Self {
        Self { bytes, pos: 0 }
    }

    fn read(&mut self, amount: usize) -> Vec<u8> {
        let end = std::cmp::min(self.pos + amount, self.bytes.len());
        let data = self.bytes[self.pos..end].to_vec();
        self.pos = end;
        data
    }

    fn read_u8(&mut self) -> Option<u8> {
        if self.pos < self.bytes.len() {
            let val = self.bytes[self.pos];
            self.pos += 1;
            Some(val)
        } else {
            None
        }
    }

    /*fn peek(&self, ahead: usize) -> Option<u8> {
        if self.pos + ahead - 1 < self.bytes.len() {
            Some(self.bytes[self.pos + ahead - 1])
        } else {
            None
        }
    }*/

    fn eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }
}

struct VM {
    tokens: ReadableByteBuffer,
    contexts: HashMap<String, Context>,
    active_context: String,
    return_val: Option<SygilValue>,
    old_tokens: Vec<ReadableByteBuffer>,
    func_depth: u32,
}

impl VM {
    fn new(file_path: &str) -> Result<Self, SygilError> {
        let mut file = File::open(file_path)
            .map_err(|e| SygilError::Runtime(format!("Could not open file: {}", e)))?;
        
        let mut check = vec![0; 7];
        file.read_exact(&mut check).map_err(|_| SygilError::Format("File too short".to_string()))?;
        
        if &check[..5] != b"sygil" {
            return Err(SygilError::Format("Not a valid compiled Sygil file.".to_string()));
        }
        
        let version_bytes = [check[5], check[6]];
        let version = i16::from_be_bytes(version_bytes);
        if version != SYGIL_VERSION {
            return Err(SygilError::Version(format!(
                "Wrong version ({}, correct is {})", format_ver_num(version), format_ver_num(SYGIL_VERSION)
            )));
        }

        let mut rest_of_file = Vec::new();
        file.read_to_end(&mut rest_of_file).unwrap();

        let mut contexts = HashMap::new();
        contexts.insert("__global__".to_string(), Context::new());

        Ok(Self {
            tokens: ReadableByteBuffer::new(rest_of_file),
            contexts,
            active_context: "__global__".to_string(),
            return_val: None,
            old_tokens: Vec::new(),
            func_depth: 0 as u32
        })
    }

    fn depth_id(mut n: u32) -> String {
        if n == 0 {
            return "0".to_string();
        }
        const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
        let mut result = Vec::new();
        while n > 0 {
            let remainder = (n % 36) as usize;
            result.push(DIGITS[remainder]);
            n /= 36;
        }
        result.reverse();
        String::from_utf8(result).unwrap()
    }

    fn read_str_from_vm(&mut self) -> String {
        let mut data = Vec::new();
        while let Some(b) = self.tokens.read_u8() {
            if b == Token::RQuote as u8 {
                break;
            }
            data.push(b);
        }
        String::from_utf8_lossy(&data).into_owned()
    }

    fn read_exp_from_vm(&mut self) -> Vec<u8> {
        let mut data = Vec::new();
        let mut skip = 0;
        while let Some(b) = self.tokens.read_u8() {
            if skip == 0 {
                if b == Token::RBrack as u8 {
                    break;
                } else if b == Token::Long as u8 || b == Token::Float as u8 {
                    skip = 8;
                } else if b == Token::Short as u8 {
                    skip = 2;
                } else if b == Token::Int as u8 {
                    skip = 4;
                }
            }
            data.push(b);
        }
        data
    }

    fn read_func_code(&mut self) -> Vec<u8> {
        let mut data = Vec::new();
        let mut depth = 0;
        loop {
            if let Some(b) = self.tokens.read_u8() {
                if b == Token::RPipe as u8 && depth == 0 {
                    break;
                }
                data.push(b);
                if b == 0x0A { depth += 1; } 
                else if b == 0x0B { depth -= 1; }
            } else {
                break;
            }
        }
        data
    }
    
    fn get_new_func_params(&mut self) -> Vec<u8> {
        let mut data = Vec::new();
        while let Some(b) = self.tokens.read_u8() {
            if b == Token::RAngle as u8 {
                break;
            }
            data.push(b);
        }
        data
    }

    fn read_func_params_call(&mut self) -> Vec<u8> {
        let mut data = Vec::new();
        let mut depth = 0;
        while depth > -1 {
            if let Some(b) = self.tokens.read_u8() {
                if b == Token::LAngle as u8 { depth += 1; }
                else if b == Token::RAngle as u8 { depth -= 1; }
                data.push(b);
            } else {
                break;
            }
        }
        data
    }

    fn split_func_params(&self, params: &[u8]) -> Vec<Vec<u8>> {
        let mut split = Vec::new();
        let mut cur_param = Vec::new();
        let mut f_depth = 0;
        let mut l_skip = 0;

        for &p in params {
            if l_skip > 0 {
                l_skip -= 1;
                cur_param.push(p);
                continue;
            }
            if p == Token::Comma as u8 && f_depth < 1 {
                split.push(cur_param.clone());
                cur_param.clear();
            } else {
                if p == Token::LAngle as u8 && l_skip == 0 { f_depth += 1; }
                else if p == Token::RAngle as u8 && l_skip == 0 { f_depth -= 1; }
                else if (p == Token::UseVar as u8 || p == Token::DeclVar as u8 || p == Token::Call as u8 || p == Token::UseClass as u8) && l_skip == 0 {
                    l_skip = 1;
                }
                else if (p == Token::Int as u8 || p == Token::Float as u8) && l_skip == 0 { l_skip = 4; }
                else if p == Token::Short as u8 && l_skip == 0 { l_skip = 2; }
                else if p == Token::Long as u8 && l_skip == 0 { l_skip = 8; }
                cur_param.push(p);
            }
        }
        split.push(cur_param);
        
        if let Some(last) = split.last() {
            if last.is_empty() {
                split.pop();
            }
        }
        split
    }

    fn eval_cond(&mut self, exp_bytes: Vec<u8>) -> SygilValue {
        let mut output = SygilValue::Null;
        let mut mode = "set";
        let mut exp = ReadableByteBuffer::new(exp_bytes);
        let starting_context = self.active_context.clone();
        let fin = false;

        while !exp.eof() && !fin {
            if let Some(b) = exp.read_u8() {
                match Token::from_u8(b) {
                    Some(Token::UseVar) => {
                        let name_len = exp.read_u8().unwrap_or(0) as usize;
                        let var_name_bytes = exp.read(name_len);
                        let var_name = String::from_utf8_lossy(&var_name_bytes).into_owned();
                        
                        let var_data = self.contexts.get(&self.active_context)
                            .and_then(|c| c.variables.get(&var_name))
                            .cloned()
                            .unwrap_or(SygilValue::Null);

                        match mode {
                            "set" => output = var_data,
                            "equ" => output = output.add(&var_data),
                            "neq" => output = output.sub(&var_data),
                            _ => {}
                        }
                        self.active_context = starting_context.clone();
                    },
                    Some(Token::Int) => {
                        let data = exp.read(4);
                        if data.len() == 4 {
                            let val = i32::from_be_bytes(data.try_into().unwrap()) as i32;
                            let v = SygilValue::Integer(val);
                            match mode {
                                "set" => output = v,
                                "equ" => output = SygilValue::Boolean(output == v),
                                "neq" => output = output.sub(&v),
                                _ => {}
                            }
                        }
                    },
                    Some(Token::Short) => {
                        let data = exp.read(2);
                        if data.len() == 2 {
                            let val = i16::from_be_bytes(data.try_into().unwrap()) as i16;
                            let v = SygilValue::Short(val);
                            match mode {
                                "set" => output = v,
                                "equ" => output = SygilValue::Boolean(output == v),
                                "neq" => output = SygilValue::Boolean(output != v),
                                _ => {}
                            }
                        }
                    },
                    Some(Token::Long) => {
                        let data = exp.read(8);
                        if data.len() == 8 {
                            let val = i64::from_be_bytes(data.try_into().unwrap()) as i64;
                            let v = SygilValue::Long(val);
                            match mode {
                                "set" => output = v,
                                "equ" => output = SygilValue::Boolean(output == v),
                                "neq" => output = SygilValue::Boolean(output != v),
                                _ => {}
                            }
                        }
                    },
                    Some(Token::LQuote) => {
                        let mut str_data = Vec::new();
                        while let Some(nb) = exp.read_u8() {
                            if nb == Token::RQuote as u8 { break; }
                            str_data.push(nb);
                        }
                        let s = SygilValue::String(String::from_utf8_lossy(&str_data).into_owned());
                        match mode {
                            "set" => output = s.clone(),
                            "equ" => output = SygilValue::Boolean(output == s),
                            "neq" => output = SygilValue::Boolean(output != s),
                            _ => {}
                        }
                    },
                    Some(Token::LCurl) => {
                        let mut data = Vec::new();
                        loop {
                            let b = exp.read(1);
                            if b.is_empty() {
                                break;
                            }
                            let a = b[0];
                            if a == Token::RCurl as u8 {
                                break;
                            } 
                            data.push(a);
                        }
                        let cond_bool = self.eval_cond(data);

                        match mode {
                            "set" => output = cond_bool,
                            "or" => output = SygilValue::Boolean(cond_bool.unwrap() || output.unwrap()),
                            "and" => output = SygilValue::Boolean(cond_bool.unwrap() && output.unwrap()),
                            _ => {}
                        }
                    }
                    Some(Token::True) => {
                        match mode {
                            "set" => output = SygilValue::Boolean(true),
                            "or" => output = SygilValue::Boolean(output.unwrap() || true),
                            "and" => output = SygilValue::Boolean(output.unwrap() && true),
                            "equ" => output = SygilValue::Boolean(output.unwrap() == true),
                            _ => {}
                        }
                    }
                    Some(Token::False) => {
                        match mode {
                            "set" => output = SygilValue::Boolean(false),
                            "or" => output = SygilValue::Boolean(output.unwrap() || false),
                            "and" => output = SygilValue::Boolean(output.unwrap() && false),
                            "equ" => output = SygilValue::Boolean(output.unwrap() == false),
                            _ => {}
                        }
                    }
                    Some(Token::Or) => mode = "or",
                    Some(Token::And) => mode = "and",
                    Some(Token::Equ) => mode = "equ",
                    Some(Token::Neq) => mode = "neq",
                    _ => {}
                }
            }
        }
        output
    }

    fn eval_exp(&mut self, exp_bytes: Vec<u8>) -> SygilValue {
        let mut output = SygilValue::Null;
        let mut mode = "set";
        let mut exp = ReadableByteBuffer::new(exp_bytes);
        let starting_context = self.active_context.clone();

        while !exp.eof() {
            if let Some(b) = exp.read_u8() {
                match Token::from_u8(b) {
                    Some(Token::UseVar) => {
                        let name_len = exp.read_u8().unwrap_or(0) as usize;
                        let var_name_bytes = exp.read(name_len);
                        let var_name = String::from_utf8_lossy(&var_name_bytes).into_owned();
                        
                        let var_data = self.contexts.get(&self.active_context)
                            .and_then(|c| c.variables.get(&var_name))
                            .cloned()
                            .unwrap_or(SygilValue::Null);

                        match mode {
                            "set" => output = var_data,
                            "add" => output = output.add(&var_data),
                            "sub" => output = output.sub(&var_data),
                            "mult" => output = output.mult(&var_data),
                            "div" => output = output.div(&var_data),
                            _ => {}
                        }
                        self.active_context = starting_context.clone();
                    },
                    Some(Token::Float) => {
                        let data = exp.read(8);
                        if data.len() == 8 {
                            let val = f64::from_be_bytes(data.try_into().unwrap()) as f64;
                            println!("{}", val);
                            let v = SygilValue::Float(val);
                            match mode {
                                "set" => output = v,
                                "add" => output = output.add(&v),
                                "sub" => output = output.sub(&v),
                                "mult" => output = output.mult(&v),
                                "div" => output = output.div(&v),
                                _ => {}
                            }
                        }
                    }
                    Some(Token::Int) => {
                        let data = exp.read(4);
                        if data.len() == 4 {
                            let val = i32::from_be_bytes(data.try_into().unwrap()) as i32;
                            let v = SygilValue::Integer(val);
                            match mode {
                                "set" => output = v,
                                "add" => output = output.add(&v),
                                "sub" => output = output.sub(&v),
                                "mult" => output = output.mult(&v),
                                "div" => output = output.div(&v),
                                _ => {}
                            }
                        }
                    },
                    Some(Token::Short) => {
                        let data = exp.read(2);
                        if data.len() == 2 {
                            let val = i16::from_be_bytes(data.try_into().unwrap()) as i16;
                            let v = SygilValue::Short(val);
                            match mode {
                                "set" => output = v,
                                "add" => output = output.add(&v),
                                "sub" => output = output.sub(&v),
                                "mult" => output = output.mult(&v),
                                "div" => output = output.div(&v),
                                _ => {}
                            }
                        }
                    },
                    Some(Token::Long) => {
                        let data = exp.read(8);
                        if data.len() == 8 {
                            let val = i64::from_be_bytes(data.try_into().unwrap()) as i64;
                            let v = SygilValue::Long(val);
                            match mode {
                                "set" => output = v,
                                "add" => output = output.add(&v),
                                "sub" => output = output.sub(&v),
                                "mult" => output = output.mult(&v),
                                "div" => output = output.div(&v),
                                _ => {}
                            }
                        }
                    },
                    Some(Token::LQuote) => {
                        let mut str_data = Vec::new();
                        while let Some(nb) = exp.read_u8() {
                            if nb == Token::RQuote as u8 { break; }
                            str_data.push(nb);
                        }
                        let s = SygilValue::String(String::from_utf8_lossy(&str_data).into_owned());
                        match mode {
                            "set" => output = s.clone(),
                            "add" => output = output.add(&s),
                            _ => {}
                        }
                    },
                    Some(Token::Call) => {
                        let current_tokens = self.tokens.clone();
                        self.tokens = exp.clone();
                        self.op_call();
                        exp = self.tokens.clone();
                        self.tokens = current_tokens;
                        
                        let data = self.return_val.clone().unwrap_or(SygilValue::Null);
                        match mode {
                            "set" => output = data,
                            "add" => output = output.add(&data),
                            "sub" => output = output.sub(&data),
                            "mult" => output = output.mult(&data),
                            "div" => output = output.div(&data),
                            _ => {}
                        }
                    },
                    Some(Token::LCurl) => {
                        let mut data = Vec::new();
                        loop {
                            let b = exp.read(1);
                            if b.is_empty() {
                                break;
                            }
                            let a = b[0];
                            if a == Token::RCurl as u8 {
                                break;
                            } 
                            data.push(a);
                        }
                        output = self.eval_cond(data);
                    },
                    Some(Token::Sub) => mode = "sub",
                    Some(Token::Add) => mode = "add",
                    Some(Token::Div) => mode = "div",
                    Some(Token::Mult) => mode = "mult",
                    Some(Token::Pow) => mode = "pow",
                    Some(Token::True) => output = SygilValue::Boolean(true),
                    Some(Token::False) => output = SygilValue::Boolean(false),
                    _ => {}
                }
            }
        }
        output
    }

    fn eval_func_params(&mut self, params: &[u8]) -> Vec<SygilValue> {
        let split = self.split_func_params(params);
        return split.into_iter().map(|p| self.eval_exp(p)).collect()
    }

    fn op_set_variable(&mut self) {
        let name_len = self.tokens.read_u8().unwrap_or(0) as usize;
        let var_name_bytes = self.tokens.read(name_len);
        let var_name = String::from_utf8_lossy(&var_name_bytes).into_owned();
        
        self.tokens.read(1);
        let n_byte = self.tokens.read_u8().unwrap_or(0);
        
        let mut data = SygilValue::Null;
        
        match Token::from_u8(n_byte) {
            Some(Token::Int) => {
                let bytes = self.tokens.read(4);
                data = SygilValue::Integer(i32::from_be_bytes(bytes.try_into().unwrap()) as i32);
            },
            Some(Token::Float) => {
                let bytes = self.tokens.read(8);
                data = SygilValue::Float(f64::from_be_bytes(bytes.try_into().unwrap()));
            },
            Some(Token::Short) => {
                let bytes = self.tokens.read(2);
                data = SygilValue::Short(i16::from_be_bytes(bytes.try_into().unwrap()) as i16);
            },
            Some(Token::Long) => {
                let bytes = self.tokens.read(8);
                data = SygilValue::Long(i64::from_be_bytes(bytes.try_into().unwrap()) as i64);
            },
            Some(Token::LBrack) => {
                let exp = self.read_exp_from_vm();
                data = self.eval_exp(exp);
            },
            Some(Token::LQuote) => {
                let s = self.read_str_from_vm();
                data = SygilValue::String(s);
                println!("{}", data);
            },
            Some(Token::True) => data = SygilValue::Boolean(true),
            Some(Token::False) => data = SygilValue::Boolean(false),
            _ => {}
        }

        if let Some(ctx) = self.contexts.get_mut(&self.active_context) {
            ctx.variables.insert(var_name, data);
        }
    }

    fn op_set_function(&mut self) {
        let name_len = self.tokens.read_u8().unwrap_or(0) as usize;
        let func_name_bytes = self.tokens.read(name_len);
        let func_name = String::from_utf8_lossy(&func_name_bytes).into_owned();
        
        self.tokens.read(1);
        
        let param_bytes = self.get_new_func_params();
        let mut params_split = self.split_func_params(&param_bytes);
        
        let mut param_names = Vec::new();
        let mut defaults = Vec::new();

        for p in params_split.iter_mut() {
            if let Some(idx) = p.iter().position(|&x| x == Token::Default as u8) {
                let default_exp = p[idx..].to_vec();
                defaults.push(self.eval_exp(default_exp));
                *p = p[..idx].to_vec();
            } else {
                defaults.push(SygilValue::Null);
            }
            
            if p.len() > 2 {
                let name = String::from_utf8_lossy(&p[2..]).into_owned();
                param_names.push(name);
            } else if p.len() > 0 {
                param_names.push(String::from_utf8_lossy(p).into_owned());
            }
        }

        self.tokens.read(1);
        let code = self.read_func_code();

        if let Some(ctx) = self.contexts.get_mut(&self.active_context) {
            ctx.functions.insert(func_name, SygilFunction {
                params: param_names,
                code,
                defaults
            });
        }
    }

    fn op_call(&mut self) -> Option<&'static str> {
        let name_len = self.tokens.read_u8().unwrap_or(0) as usize;
        let func_name_bytes = self.tokens.read(name_len);
        let func_name = String::from_utf8_lossy(&func_name_bytes).into_owned();
        
        self.tokens.read(1);
        let param_bytes = self.read_func_params_call();
        let clean_params = if param_bytes.last() == Some(&(Token::RAngle as u8)) {
            &param_bytes[..param_bytes.len()-1]
        } else {
            &param_bytes[..]
        };
        
        let params_eval = self.eval_func_params(clean_params);
        self.return_val = None;

        let mut func_to_call = None;

        if func_name == "print" {
            for p in &params_eval {
                match p {
                    SygilValue::String(_a) => print!("{}", p),
                    _ => print!("{} ", p),
                }
            }
            println!();
            return None;
        } else if func_name == "return" {
            if let Some(v) = params_eval.get(0) {
                self.return_val = Some(v.clone());
            }
            return Some("RET");
        } else if func_name == "if" {
            if let Some(v) = params_eval.get(0) {
                self.tokens.read(1);
                let cond_code = self.read_func_code();
                if v.unwrap() {
                    self.run(Some(ReadableByteBuffer::new(cond_code)));
                }
            }
        } else if func_name == "input" {
            for p in &params_eval {
                match p {
                    SygilValue::String(_a) => print!("{}", p),
                    _ => print!("{} ", p),
                }
            }
            println!();

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to fetch input");
            self.return_val = Some(SygilValue::String(input.trim().to_string()));
            return Some("RET");
        }

        if let Some(ctx) = self.contexts.get(&self.active_context) {
            if let Some(f) = ctx.functions.get(&func_name) {
                func_to_call = Some(f.clone());
            }
        }

        if let Some(f_data) = func_to_call {
            self.contexts.insert(("_FUNC".to_owned()+&VM::depth_id(self.func_depth)).to_string(), Context::new());
            let prev_context = self.active_context.clone();
            self.active_context = ("_FUNC".to_owned()+&VM::depth_id(self.func_depth)).to_string();

            if let Some(func_ctx) = self.contexts.get_mut(&self.active_context) {
                for (i, p_name) in f_data.params.iter().enumerate() {
                    if i < params_eval.len() {
                        func_ctx.variables.insert(p_name.clone(), params_eval[i].clone());
                    } else if i < f_data.defaults.len() {
                        func_ctx.variables.insert(p_name.clone(), f_data.defaults[i].clone());
                    }
                }
            }

            self.run(Some(ReadableByteBuffer::new(f_data.code)));
            self.active_context = prev_context;
            self.contexts.remove(&("_FUNC".to_owned()+&VM::depth_id(self.func_depth)));
        }

        if let Some(ctx) = self.contexts.get(&self.active_context) {
            if let Some(_f) = ctx.functions.get(&(("_IF".to_owned()+&VM::depth_id(self.func_depth)).to_string())) {
                self.contexts.remove(&("_IF".to_owned()+&VM::depth_id(self.func_depth)));
            }
        }

        None
    }

    fn op_set_class(&mut self) {
        let name_len = self.tokens.read_u8().unwrap_or(0) as usize;
        let class_name_bytes = self.tokens.read(name_len);
        let class_name_base = String::from_utf8_lossy(&class_name_bytes).into_owned();
        let class_name = format!("{}.{}", self.active_context, class_name_base);
        
        let sub_objects = self.read_func_code();
        self.contexts.insert(class_name.clone(), Context::new());
        
        let prev_context = self.active_context.clone();
        self.active_context = class_name;
        self.run(Some(ReadableByteBuffer::new(sub_objects)));
        self.active_context = prev_context;
    }

    fn run(&mut self, code: Option<ReadableByteBuffer>) -> Option<&'static str> {
        if let Some(c) = code {
            self.old_tokens.push(self.tokens.clone());
            self.tokens = c;
        }

        while !self.tokens.eof() {
            if let Some(op) = self.tokens.read_u8() {
                match Token::from_u8(op) {
                    Some(Token::DeclVar) => self.op_set_variable(),
                    Some(Token::DeclFunc) => self.op_set_function(),
                    Some(Token::Call) => {
                        if self.op_call() == Some("RET") { break; }
                    },
                    Some(Token::DeclClass) => self.op_set_class(),
                    _ => {}
                }
            }
        }

        if !self.old_tokens.is_empty() {
            self.tokens = self.old_tokens.pop().unwrap();
        }
        Some("RET")
    }
}

fn compile_to_syc(path: &str) -> Result<String, SygilError> {
    let mut checker = Checker::new(path)?;
    checker.check()?;
    
    let path_obj = Path::new(path);
    let dir = path_obj.parent().unwrap_or_else(|| Path::new(""));
    let file_stem = path_obj.file_stem().unwrap().to_str().unwrap();
    
    let n_dir = dir.join("__syc__");
    fs::create_dir_all(&n_dir).map_err(|e| SygilError::Runtime(e.to_string()))?;
    
    let out_path = n_dir.join(format!("{}.syc", file_stem));
    let out_path_str = out_path.to_str().unwrap().to_string();
    
    let mut tokenizer = Tokenizer::new(path)?;
    tokenizer.tokenize(&out_path_str)?;
    
    Ok(out_path_str)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        println!("Sygil {}", format_ver_num(SYGIL_VERSION));
        println!(
"Usage: 
sygil compile <file.sy> <out.syc>
sygil run <file.syc>
sygil validate <file.sy>"
        );
        process::exit(0);
    }

    let command = &args[1];

    match command.as_str() {
        "documentation" => {
            print!("")
        }
        "compile" => {
            if args.len() < 4 {
                println!("Error: 'compile' requires input and output paths.");
                process::exit(1);
            }
            if let Err(e) = Checker::new(&args[2]).and_then(|mut c| c.check()) {
                println!("{}", e);
                process::exit(1);
            }
            if let Err(e) = Tokenizer::new(&args[2]).and_then(|mut t| t.tokenize(&args[3])) {
                println!("{}", e);
                process::exit(1);
            }
            println!("Compiled successfully to {}", args[3]);
        }
        "run" => {
            if args.len() < 3 {
                println!("Error: 'run' requires a file path.");
                process::exit(1);
            }
            let input_path = &args[2];
            let path_to_run = if input_path.ends_with(".sy") {
                match compile_to_syc(input_path) {
                    Ok(p) => p,
                    Err(e) => {
                        println!("{}", e);
                        process::exit(1);
                    }
                }
            } else {
                input_path.clone()
            };

            match VM::new(&path_to_run) {
                Ok(mut vm) => {
                    vm.run(None);
                }
                Err(e) => {
                    println!("{}", e);
                    process::exit(1);
                }
            }
        }
        "validate" => {
            if args.len() < 3 {
                println!("Error: 'validate' requires a file path.");
                process::exit(1);
            }
            match Checker::new(&args[2]).and_then(|mut c| c.check()) {
                Ok(_) => println!("Syntax is valid."),
                Err(e) => {
                    println!("{}", e);
                    process::exit(1);
                }
            }
        }
        _ => {
            println!("Unknown command: \"{}\". Use compile, run, or validate.", command);
        }
    }
}