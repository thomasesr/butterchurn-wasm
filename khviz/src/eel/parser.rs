use super::lexer::{Lexer, Token};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Lt,
    Gt,
    BitAnd,
    BitOr,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    Var(String),
    Assign {
        var: String,
        val: Box<Expr>,
    },
    CompoundAssign {
        var: String,
        op: BinOp,
        val: Box<Expr>,
    },
    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
    Sequence(Vec<Expr>),
}

pub type Ast = Expr;

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let t = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn eat(&mut self, tok: &Token) -> bool {
        if self.peek() == tok {
            self.advance();
            true
        } else {
            false
        }
    }

    fn parse_program(&mut self) -> Ast {
        let mut stmts = Vec::new();
        while *self.peek() != Token::Eof {
            self.eat(&Token::Semicolon);
            if *self.peek() == Token::Eof {
                break;
            }
            stmts.push(self.parse_expr());
            self.eat(&Token::Semicolon);
        }
        match stmts.len() {
            0 => Expr::Number(0.0),
            1 => stmts.remove(0),
            _ => Expr::Sequence(stmts),
        }
    }

    fn parse_expr(&mut self) -> Expr {
        self.parse_comma()
    }

    fn parse_comma(&mut self) -> Expr {
        let mut exprs = vec![self.parse_assign()];
        while self.eat(&Token::Comma) {
            exprs.push(self.parse_assign());
        }
        if exprs.len() == 1 {
            exprs.remove(0)
        } else {
            Expr::Sequence(exprs)
        }
    }

    fn parse_assign(&mut self) -> Expr {
        if let Token::Ident(name) = self.peek().clone() {
            let saved = self.pos;
            self.advance();
            let maybe_op = match self.peek() {
                Token::Eq => Some(None),
                Token::PlusEq => Some(Some(BinOp::Add)),
                Token::MinusEq => Some(Some(BinOp::Sub)),
                Token::StarEq => Some(Some(BinOp::Mul)),
                Token::SlashEq => Some(Some(BinOp::Div)),
                Token::PercentEq => Some(Some(BinOp::Mod)),
                Token::CaretEq => Some(Some(BinOp::Pow)),
                _ => None,
            };
            if let Some(op_opt) = maybe_op {
                self.advance();
                let val = self.parse_assign();
                return match op_opt {
                    None => Expr::Assign {
                        var: name,
                        val: Box::new(val),
                    },
                    Some(op) => Expr::CompoundAssign {
                        var: name,
                        op,
                        val: Box::new(val),
                    },
                };
            }
            self.pos = saved;
        }
        self.parse_bitor()
    }

    fn parse_bitor(&mut self) -> Expr {
        let mut lhs = self.parse_bitand();
        while self.eat(&Token::Pipe) {
            let rhs = self.parse_bitand();
            lhs = Expr::BinOp {
                op: BinOp::BitOr,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        lhs
    }

    fn parse_bitand(&mut self) -> Expr {
        let mut lhs = self.parse_cmp();
        while self.eat(&Token::Amp) {
            let rhs = self.parse_cmp();
            lhs = Expr::BinOp {
                op: BinOp::BitAnd,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        lhs
    }

    fn parse_cmp(&mut self) -> Expr {
        let lhs = self.parse_additive();
        match self.peek() {
            Token::Lt => {
                self.advance();
                let rhs = self.parse_additive();
                Expr::BinOp {
                    op: BinOp::Lt,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }
            }
            Token::Gt => {
                self.advance();
                let rhs = self.parse_additive();
                Expr::BinOp {
                    op: BinOp::Gt,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }
            }
            _ => lhs,
        }
    }

    fn parse_additive(&mut self) -> Expr {
        let mut lhs = self.parse_multiplicative();
        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_multiplicative();
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        lhs
    }

    fn parse_multiplicative(&mut self) -> Expr {
        let mut lhs = self.parse_power();
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_power();
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        lhs
    }

    fn parse_power(&mut self) -> Expr {
        let base = self.parse_unary();
        if self.eat(&Token::Caret) {
            let exp = self.parse_power();
            Expr::BinOp {
                op: BinOp::Pow,
                lhs: Box::new(base),
                rhs: Box::new(exp),
            }
        } else {
            base
        }
    }

    fn parse_unary(&mut self) -> Expr {
        match self.peek() {
            Token::Minus => {
                self.advance();
                let e = self.parse_unary();
                Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(e),
                }
            }
            Token::Bang => {
                self.advance();
                let e = self.parse_unary();
                Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(e),
                }
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Expr {
        match self.peek().clone() {
            Token::Number(n) => {
                self.advance();
                Expr::Number(n)
            }
            Token::Ident(name) => {
                self.advance();
                if self.eat(&Token::LParen) {
                    let mut args = Vec::new();
                    if *self.peek() != Token::RParen && *self.peek() != Token::Eof {
                        args.push(self.parse_assign());
                        while self.eat(&Token::Comma) {
                            if *self.peek() == Token::RParen || *self.peek() == Token::Eof {
                                break;
                            }
                            args.push(self.parse_assign());
                        }
                    }
                    self.eat(&Token::RParen);
                    Expr::Call { name, args }
                } else {
                    Expr::Var(name)
                }
            }
            Token::LParen => {
                self.advance();
                let e = self.parse_expr();
                self.eat(&Token::RParen);
                e
            }
            _ => {
                self.advance();
                Expr::Number(0.0)
            }
        }
    }
}

pub fn parse(src: &str) -> Ast {
    let mut lex = Lexer::new(src);
    let tokens = lex.tokenize();
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}
