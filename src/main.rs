use std::collections::HashSet;
use std::env;
use std::fmt;
use std::fs;
use std::ptr;

use thiserror::Error;

#[derive(Debug, Clone, Copy)]
struct Span {
    line: usize,
    col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Keyword {
    SharedContract,
    Contract,
    State,
    Var,
    On,
    If,
    Else,
    Call,
    Let,
    Send,
    Transition,
    Violation,
    Drop,
}

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Ident(String),
    Number(u64),
    Keyword(Keyword),
    LBrace,
    RBrace,
    LParen,
    RParen,
    Colon,
    Semicolon,
    Comma,
    Dot,
    Plus,
    Minus,
    EqEq,
    Greater,
    Less,
    Assign,
    Arrow,
    Eof,
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    span: Span,
}

#[derive(Debug, Error)]
enum CompileError {
    #[error("Lex error at {line}:{col}: {msg}")]
    Lex {
        line: usize,
        col: usize,
        msg: String,
    },
    #[error("Parse error at {line}:{col}: {msg}")]
    Parse {
        line: usize,
        col: usize,
        msg: String,
    },
    #[error("Linearity error: {0}")]
    Verify(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
struct Program {
    items: Vec<Item>,
}

#[derive(Debug, Clone)]
enum Item {
    SharedContract(ContractDecl),
    Contract(ContractDecl),
}

#[derive(Debug, Clone)]
struct ContractDecl {
    name: String,
    states: Vec<StateDecl>,
}

#[derive(Debug, Clone)]
struct StateDecl {
    name: String,
    members: Vec<StateMember>,
}

#[derive(Debug, Clone)]
enum StateMember {
    VarDecl(VarDecl),
    Handler(HandlerDecl),
}

#[derive(Debug, Clone)]
struct VarDecl {
    name: String,
    ty: String,
    init: Expr,
}

#[derive(Debug, Clone)]
struct HandlerDecl {
    name: String,
    params: Vec<Param>,
    body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
struct Param {
    name: String,
    ty: String,
}

#[derive(Debug, Clone)]
enum Stmt {
    If {
        cond: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
    },
    Call(Expr),
    Let {
        name: String,
        expr: Expr,
    },
    Send(Expr),
    Transition(String),
    Violation(Vec<Stmt>),
    Drop(Expr),
    Expr(Expr),
}

#[derive(Debug, Clone)]
enum Expr {
    Number(u64),
    Path(String),
    Call { target: String, args: Vec<Expr> },
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Eq(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Number(n) => write!(f, "{}", n),
            Expr::Path(p) => write!(f, "{}", p),
            Expr::Call { target, args } => {
                let joined = args
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{}({})", target, joined)
            }
            Expr::Add(l, r) => write!(f, "{} + {}", l, r),
            Expr::Sub(l, r) => write!(f, "{} - {}", l, r),
            Expr::Eq(l, r) => write!(f, "{} == {}", l, r),
            Expr::Gt(l, r) => write!(f, "{} > {}", l, r),
            Expr::Lt(l, r) => write!(f, "{} < {}", l, r),
        }
    }
}

struct Lexer {
    src: Vec<char>,
    idx: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    fn new(input: &str) -> Self {
        Self {
            src: input.chars().collect(),
            idx: 0,
            line: 1,
            col: 1,
        }
    }

    fn tokenize(mut self) -> Result<Vec<Token>, CompileError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_ws_and_comments();
            let span = Span {
                line: self.line,
                col: self.col,
            };
            let token = match self.peek() {
                None => Token {
                    kind: TokenKind::Eof,
                    span,
                },
                Some('{') => {
                    self.bump();
                    Token {
                        kind: TokenKind::LBrace,
                        span,
                    }
                }
                Some('}') => {
                    self.bump();
                    Token {
                        kind: TokenKind::RBrace,
                        span,
                    }
                }
                Some('(') => {
                    self.bump();
                    Token {
                        kind: TokenKind::LParen,
                        span,
                    }
                }
                Some(')') => {
                    self.bump();
                    Token {
                        kind: TokenKind::RParen,
                        span,
                    }
                }
                Some(':') => {
                    self.bump();
                    Token {
                        kind: TokenKind::Colon,
                        span,
                    }
                }
                Some(';') => {
                    self.bump();
                    Token {
                        kind: TokenKind::Semicolon,
                        span,
                    }
                }
                Some(',') => {
                    self.bump();
                    Token {
                        kind: TokenKind::Comma,
                        span,
                    }
                }
                Some('.') => {
                    self.bump();
                    Token {
                        kind: TokenKind::Dot,
                        span,
                    }
                }
                Some('+') => {
                    self.bump();
                    Token {
                        kind: TokenKind::Plus,
                        span,
                    }
                }
                Some('-') if self.peek_next() == Some('>') => {
                    self.bump();
                    self.bump();
                    Token {
                        kind: TokenKind::Arrow,
                        span,
                    }
                }
                Some('-') => {
                    self.bump();
                    Token {
                        kind: TokenKind::Minus,
                        span,
                    }
                }
                Some('=') if self.peek_next() == Some('=') => {
                    self.bump();
                    self.bump();
                    Token {
                        kind: TokenKind::EqEq,
                        span,
                    }
                }
                Some('=') => {
                    self.bump();
                    Token {
                        kind: TokenKind::Assign,
                        span,
                    }
                }
                Some('>') => {
                    self.bump();
                    Token {
                        kind: TokenKind::Greater,
                        span,
                    }
                }
                Some('<') => {
                    self.bump();
                    Token {
                        kind: TokenKind::Less,
                        span,
                    }
                }
                Some(ch) if ch.is_ascii_digit() => {
                    let n = self.lex_number()?;
                    Token {
                        kind: TokenKind::Number(n),
                        span,
                    }
                }
                Some(ch) if is_ident_start(ch) => {
                    let ident = self.lex_ident();
                    let kind = match ident.as_str() {
                        "shared_contract" => TokenKind::Keyword(Keyword::SharedContract),
                        "contract" => TokenKind::Keyword(Keyword::Contract),
                        "state" => TokenKind::Keyword(Keyword::State),
                        "var" => TokenKind::Keyword(Keyword::Var),
                        "on" => TokenKind::Keyword(Keyword::On),
                        "if" => TokenKind::Keyword(Keyword::If),
                        "else" => TokenKind::Keyword(Keyword::Else),
                        "call" => TokenKind::Keyword(Keyword::Call),
                        "let" => TokenKind::Keyword(Keyword::Let),
                        "send" => TokenKind::Keyword(Keyword::Send),
                        "transition" => TokenKind::Keyword(Keyword::Transition),
                        "violation" => TokenKind::Keyword(Keyword::Violation),
                        "drop" => TokenKind::Keyword(Keyword::Drop),
                        _ => TokenKind::Ident(ident),
                    };
                    Token { kind, span }
                }
                Some(other) => {
                    return Err(CompileError::Lex {
                        line: span.line,
                        col: span.col,
                        msg: format!("unexpected character '{}'", other),
                    })
                }
            };
            let at_eof = matches!(token.kind, TokenKind::Eof);
            tokens.push(token);
            if at_eof {
                break;
            }
        }
        Ok(tokens)
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while matches!(self.peek(), Some(ch) if ch.is_whitespace()) {
                self.bump();
            }
            if self.peek() == Some('/') && self.peek_next() == Some('/') {
                while !matches!(self.peek(), None | Some('\n')) {
                    self.bump();
                }
                continue;
            }
            break;
        }
    }

    fn lex_number(&mut self) -> Result<u64, CompileError> {
        let start = Span {
            line: self.line,
            col: self.col,
        };
        let mut buf = String::new();
        while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
            buf.push(self.bump().unwrap_or('0'));
        }
        buf.parse::<u64>().map_err(|_| CompileError::Lex {
            line: start.line,
            col: start.col,
            msg: "invalid number".to_string(),
        })
    }

    fn lex_ident(&mut self) -> String {
        let mut buf = String::new();
        while matches!(self.peek(), Some(ch) if is_ident_continue(ch)) {
            buf.push(self.bump().unwrap_or('_'));
        }
        buf
    }

    fn peek(&self) -> Option<char> {
        self.src.get(self.idx).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.src.get(self.idx + 1).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.src.get(self.idx).copied()?;
        self.idx += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    is_ident_start(ch) || ch.is_ascii_digit()
}

struct Parser {
    tokens: Vec<Token>,
    idx: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, idx: 0 }
    }

    fn parse_program(&mut self) -> Result<Program, CompileError> {
        let mut items = Vec::new();
        while !self.is_eof() {
            items.push(self.parse_item()?);
        }
        Ok(Program { items })
    }

    fn parse_item(&mut self) -> Result<Item, CompileError> {
        match self.peek_kind() {
            TokenKind::Keyword(Keyword::SharedContract) => {
                self.bump();
                Ok(Item::SharedContract(self.parse_contract_body()?))
            }
            TokenKind::Keyword(Keyword::Contract) => {
                self.bump();
                Ok(Item::Contract(self.parse_contract_body()?))
            }
            _ => self.error_here("expected 'shared_contract' or 'contract'"),
        }
    }

    fn parse_contract_body(&mut self) -> Result<ContractDecl, CompileError> {
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;
        let mut states = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            states.push(self.parse_state_decl()?);
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(ContractDecl { name, states })
    }

    fn parse_state_decl(&mut self) -> Result<StateDecl, CompileError> {
        self.expect_keyword(Keyword::State)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;
        let mut members = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            members.push(self.parse_state_member()?);
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(StateDecl { name, members })
    }

    fn parse_state_member(&mut self) -> Result<StateMember, CompileError> {
        match self.peek_kind() {
            TokenKind::Keyword(Keyword::Var) => {
                self.bump();
                let name = self.expect_ident()?;
                self.expect(&TokenKind::Colon)?;
                let ty = self.expect_ident()?;
                self.expect(&TokenKind::Assign)?;
                let init = self.parse_expr()?;
                self.expect(&TokenKind::Semicolon)?;
                Ok(StateMember::VarDecl(VarDecl { name, ty, init }))
            }
            TokenKind::Keyword(Keyword::On) => {
                self.bump();
                let name = self.expect_ident()?;
                self.expect(&TokenKind::LParen)?;
                let mut params = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        let pname = self.expect_ident()?;
                        self.expect(&TokenKind::Colon)?;
                        let pty = self.expect_ident()?;
                        params.push(Param {
                            name: pname,
                            ty: pty,
                        });
                        if self.check(&TokenKind::Comma) {
                            self.bump();
                            continue;
                        }
                        break;
                    }
                }
                self.expect(&TokenKind::RParen)?;
                let body = self.parse_block()?;
                Ok(StateMember::Handler(HandlerDecl { name, params, body }))
            }
            _ => self.error_here("expected state member: 'var' or 'on'"),
        }
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, CompileError> {
        self.expect(&TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, CompileError> {
        match self.peek_kind() {
            TokenKind::Keyword(Keyword::If) => {
                self.bump();
                self.expect(&TokenKind::LParen)?;
                let cond = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                let then_branch = self.parse_block()?;
                let else_branch = if self.matches_keyword(Keyword::Else) {
                    self.bump();
                    Some(self.parse_block()?)
                } else {
                    None
                };
                Ok(Stmt::If {
                    cond,
                    then_branch,
                    else_branch,
                })
            }
            TokenKind::Keyword(Keyword::Call) => {
                self.bump();
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::Semicolon)?;
                Ok(Stmt::Call(expr))
            }
            TokenKind::Keyword(Keyword::Let) => {
                self.bump();
                let name = self.expect_ident()?;
                self.expect(&TokenKind::Assign)?;
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::Semicolon)?;
                Ok(Stmt::Let { name, expr })
            }
            TokenKind::Keyword(Keyword::Send) => {
                self.bump();
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::Semicolon)?;
                Ok(Stmt::Send(expr))
            }
            TokenKind::Keyword(Keyword::Transition) => {
                self.bump();
                self.expect(&TokenKind::Arrow)?;
                let target = self.expect_ident()?;
                self.expect(&TokenKind::Semicolon)?;
                Ok(Stmt::Transition(target))
            }
            TokenKind::Keyword(Keyword::Violation) => {
                self.bump();
                Ok(Stmt::Violation(self.parse_block()?))
            }
            TokenKind::Keyword(Keyword::Drop) => {
                self.bump();
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::Semicolon)?;
                Ok(Stmt::Drop(expr))
            }
            _ => {
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::Semicolon)?;
                Ok(Stmt::Expr(expr))
            }
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, CompileError> {
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.parse_additive()?;
        loop {
            if self.check(&TokenKind::EqEq) {
                self.bump();
                let right = self.parse_additive()?;
                left = Expr::Eq(Box::new(left), Box::new(right));
                continue;
            }
            if self.check(&TokenKind::Greater) {
                self.bump();
                let right = self.parse_additive()?;
                left = Expr::Gt(Box::new(left), Box::new(right));
                continue;
            }
            if self.check(&TokenKind::Less) {
                self.bump();
                let right = self.parse_additive()?;
                left = Expr::Lt(Box::new(left), Box::new(right));
                continue;
            }
            break;
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.parse_primary()?;
        loop {
            if self.check(&TokenKind::Plus) {
                self.bump();
                let right = self.parse_primary()?;
                left = Expr::Add(Box::new(left), Box::new(right));
                continue;
            }
            if self.check(&TokenKind::Minus) {
                self.bump();
                let right = self.parse_primary()?;
                left = Expr::Sub(Box::new(left), Box::new(right));
                continue;
            }
            break;
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, CompileError> {
        match self.peek_kind() {
            TokenKind::Number(n) => {
                let value = *n;
                self.bump();
                Ok(Expr::Number(value))
            }
            TokenKind::Ident(_) => {
                let path = self.parse_path()?;
                if self.check(&TokenKind::LParen) {
                    self.bump();
                    let mut args = Vec::new();
                    if !self.check(&TokenKind::RParen) {
                        loop {
                            args.push(self.parse_expr()?);
                            if self.check(&TokenKind::Comma) {
                                self.bump();
                                continue;
                            }
                            break;
                        }
                    }
                    self.expect(&TokenKind::RParen)?;
                    Ok(Expr::Call { target: path, args })
                } else {
                    Ok(Expr::Path(path))
                }
            }
            TokenKind::LParen => {
                self.bump();
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }
            _ => self.error_here("expected expression"),
        }
    }

    fn parse_path(&mut self) -> Result<String, CompileError> {
        let mut parts = vec![self.expect_ident()?];
        while self.check(&TokenKind::Dot) {
            self.bump();
            parts.push(self.expect_ident()?);
        }
        Ok(parts.join("."))
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<(), CompileError> {
        if self.check(kind) {
            self.bump();
            Ok(())
        } else {
            self.error_here(&format!("expected {:?}", kind))
        }
    }

    fn expect_keyword(&mut self, kw: Keyword) -> Result<(), CompileError> {
        if self.matches_keyword(kw) {
            self.bump();
            Ok(())
        } else {
            self.error_here("expected keyword")
        }
    }

    fn expect_ident(&mut self) -> Result<String, CompileError> {
        match self.peek_kind() {
            TokenKind::Ident(s) => {
                let out = s.clone();
                self.bump();
                Ok(out)
            }
            _ => self.error_here("expected identifier"),
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(kind)
    }

    fn matches_keyword(&self, kw: Keyword) -> bool {
        matches!(self.peek_kind(), TokenKind::Keyword(k) if *k == kw)
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.tokens[self.idx].kind
    }

    fn bump(&mut self) {
        if !self.is_eof() {
            self.idx += 1;
        }
    }

    fn is_eof(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }

    fn error_here<T>(&self, msg: &str) -> Result<T, CompileError> {
        let span = self.tokens[self.idx].span;
        Err(CompileError::Parse {
            line: span.line,
            col: span.col,
            msg: msg.to_string(),
        })
    }
}

fn parse_source(source: &str) -> Result<Program, CompileError> {
    let tokens = Lexer::new(source).tokenize()?;
    Parser::new(tokens).parse_program()
}

#[derive(Debug)]
struct VerifyIssue(String);

fn verify_program(program: &Program) -> Result<(), CompileError> {
    let mut issues = Vec::new();
    for item in &program.items {
        let contract = match item {
            Item::Contract(c) | Item::SharedContract(c) => c,
        };
        let contract_name = &contract.name;
        for state in &contract.states {
            validate_state_members(contract_name, state, &mut issues);
            for member in &state.members {
                if let StateMember::Handler(handler) = member {
                    verify_violation_exhaustiveness(contract_name, &state.name, handler, &mut issues);
                    let mut linear_env: HashSet<String> = HashSet::new();
                    for p in &handler.params {
                        if is_linear_type(&p.ty) {
                            linear_env.insert(p.name.clone());
                        }
                    }
                    let mut states = vec![linear_env.clone()];
                    for stmt in &handler.body {
                        states = apply_stmt(stmt, &states, &mut linear_env, &mut issues, &handler.name);
                    }
                    for remaining in states {
                        if !remaining.is_empty() {
                            issues.push(VerifyIssue(format!(
                                "handler '{}.{}.{}' leaks linear values on one path: {:?}",
                                contract_name, state.name, handler.name, remaining
                            )));
                        }
                    }
                }
            }
        }
    }

    if issues.is_empty() {
        Ok(())
    } else {
        let joined = issues
            .into_iter()
            .map(|i| i.0)
            .collect::<Vec<_>>()
            .join("\n");
        Err(CompileError::Verify(joined))
    }
}

fn is_linear_type(ty: &str) -> bool {
    ty.starts_with("Linear")
}

fn validate_state_members(contract_name: &str, state: &StateDecl, issues: &mut Vec<VerifyIssue>) {
    let mut vars_seen = HashSet::new();
    for member in &state.members {
        if let StateMember::VarDecl(var) = member {
            if !vars_seen.insert(var.name.clone()) {
                issues.push(VerifyIssue(format!(
                    "duplicate state variable '{}.{}.{}'",
                    contract_name, state.name, var.name
                )));
            }
            if is_linear_type(&var.ty) {
                issues.push(VerifyIssue(format!(
                    "state variable '{}.{}.{}' cannot use linear type '{}'",
                    contract_name, state.name, var.name, var.ty
                )));
            }
            if expr_returns_linear(&var.init) && !is_linear_type(&var.ty) {
                issues.push(VerifyIssue(format!(
                    "state variable '{}.{}.{}' initializes linear value into non-linear type '{}'",
                    contract_name, state.name, var.name, var.ty
                )));
            }
        }
    }
}

fn verify_violation_exhaustiveness(
    contract_name: &str,
    state_name: &str,
    handler: &HandlerDecl,
    issues: &mut Vec<VerifyIssue>,
) {
    let has_linear_params = handler.params.iter().any(|p| is_linear_type(&p.ty));
    if !has_linear_params {
        return;
    }

    if !contains_violation(&handler.body) {
        issues.push(VerifyIssue(format!(
            "handler '{}.{}.{}' has linear input but no explicit violation block",
            contract_name, state_name, handler.name
        )));
    }

    ensure_if_exhaustive_with_violation(
        &handler.body,
        contract_name,
        state_name,
        &handler.name,
        issues,
    );
}

fn contains_violation(stmts: &[Stmt]) -> bool {
    for stmt in stmts {
        match stmt {
            Stmt::Violation(_) => return true,
            Stmt::If {
                then_branch,
                else_branch,
                ..
            } => {
                if contains_violation(then_branch) {
                    return true;
                }
                if let Some(else_b) = else_branch {
                    if contains_violation(else_b) {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    false
}

fn ensure_if_exhaustive_with_violation(
    stmts: &[Stmt],
    contract_name: &str,
    state_name: &str,
    handler_name: &str,
    issues: &mut Vec<VerifyIssue>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::If {
                else_branch,
                then_branch,
                ..
            } => {
                let Some(else_b) = else_branch else {
                    issues.push(VerifyIssue(format!(
                        "handler '{}.{}.{}' uses if without else; unhandled input path",
                        contract_name, state_name, handler_name
                    )));
                    ensure_if_exhaustive_with_violation(
                        then_branch,
                        contract_name,
                        state_name,
                        handler_name,
                        issues,
                    );
                    continue;
                };

                if !contains_violation(else_b) {
                    issues.push(VerifyIssue(format!(
                        "handler '{}.{}.{}' has else-path without violation handling",
                        contract_name, state_name, handler_name
                    )));
                }

                ensure_if_exhaustive_with_violation(
                    then_branch,
                    contract_name,
                    state_name,
                    handler_name,
                    issues,
                );
                ensure_if_exhaustive_with_violation(
                    else_b,
                    contract_name,
                    state_name,
                    handler_name,
                    issues,
                );
            }
            Stmt::Violation(body) => {
                ensure_if_exhaustive_with_violation(
                    body,
                    contract_name,
                    state_name,
                    handler_name,
                    issues,
                );
            }
            _ => {}
        }
    }
}

fn apply_stmt(
    stmt: &Stmt,
    states: &[HashSet<String>],
    linear_env: &mut HashSet<String>,
    issues: &mut Vec<VerifyIssue>,
    handler_name: &str,
) -> Vec<HashSet<String>> {
    let mut out = Vec::new();
    for state in states {
        out.extend(apply_stmt_single(
            stmt,
            state,
            linear_env,
            issues,
            handler_name,
        ));
    }
    out
}

fn apply_stmt_single(
    stmt: &Stmt,
    state: &HashSet<String>,
    linear_env: &mut HashSet<String>,
    issues: &mut Vec<VerifyIssue>,
    handler_name: &str,
) -> Vec<HashSet<String>> {
    let mut st = state.clone();
    match stmt {
        Stmt::If {
            cond,
            then_branch,
            else_branch,
        } => {
            consume_in_expr(cond, &mut st, linear_env, issues, handler_name);
            let then_states = apply_block(then_branch, vec![st.clone()], linear_env, issues, handler_name);
            let else_states = if let Some(else_body) = else_branch {
                apply_block(else_body, vec![st], linear_env, issues, handler_name)
            } else {
                vec![st]
            };
            [then_states, else_states].concat()
        }
        Stmt::Call(expr) => {
            consume_in_expr(expr, &mut st, linear_env, issues, handler_name);
            vec![st]
        }
        Stmt::Let { name, expr } => {
            if let Expr::Path(p) = expr {
                if linear_env.contains(p) {
                    consume_var(p, &mut st, issues, handler_name);
                    linear_env.insert(name.clone());
                    st.insert(name.clone());
                    return vec![st];
                }
            }
            consume_in_expr(expr, &mut st, linear_env, issues, handler_name);
            if expr_returns_linear(expr) {
                linear_env.insert(name.clone());
                st.insert(name.clone());
            }
            vec![st]
        }
        Stmt::Send(expr) | Stmt::Drop(expr) => {
            consume_in_expr(expr, &mut st, linear_env, issues, handler_name);
            if let Expr::Path(p) = expr {
                if linear_env.contains(p) {
                    consume_var(p, &mut st, issues, handler_name);
                }
            }
            vec![st]
        }
        Stmt::Transition(_) => vec![st],
        Stmt::Violation(body) => apply_block(body, vec![st], linear_env, issues, handler_name),
        Stmt::Expr(expr) => {
            consume_in_expr(expr, &mut st, linear_env, issues, handler_name);
            vec![st]
        }
    }
}

fn apply_block(
    block: &[Stmt],
    mut states: Vec<HashSet<String>>,
    linear_env: &mut HashSet<String>,
    issues: &mut Vec<VerifyIssue>,
    handler_name: &str,
) -> Vec<HashSet<String>> {
    for stmt in block {
        states = apply_stmt(stmt, &states, linear_env, issues, handler_name);
    }
    states
}

fn consume_in_expr(
    expr: &Expr,
    state: &mut HashSet<String>,
    linear_env: &HashSet<String>,
    issues: &mut Vec<VerifyIssue>,
    handler_name: &str,
) {
    match expr {
        Expr::Number(_) | Expr::Path(_) => {}
        Expr::Add(l, r) | Expr::Sub(l, r) | Expr::Eq(l, r) | Expr::Gt(l, r) | Expr::Lt(l, r) => {
            consume_in_expr(l, state, linear_env, issues, handler_name);
            consume_in_expr(r, state, linear_env, issues, handler_name);
        }
        Expr::Call { args, .. } => {
            for arg in args {
                consume_in_expr(arg, state, linear_env, issues, handler_name);
                if let Expr::Path(p) = arg {
                    if linear_env.contains(p) {
                        consume_var(p, state, issues, handler_name);
                    }
                }
            }
        }
    }
}

fn consume_var(
    var: &str,
    state: &mut HashSet<String>,
    issues: &mut Vec<VerifyIssue>,
    handler_name: &str,
) {
    if !state.remove(var) {
        issues.push(VerifyIssue(format!(
            "use-after-move for '{}' in handler '{}'",
            var, handler_name
        )));
    }
}

fn expr_returns_linear(expr: &Expr) -> bool {
    match expr {
        Expr::Call { target, .. } => target.starts_with("create_"),
        _ => false,
    }
}

fn eval_const_expr(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Number(n) => i64::try_from(*n).ok(),
        Expr::Add(l, r) => Some(eval_const_expr(l)? + eval_const_expr(r)?),
        Expr::Sub(l, r) => Some(eval_const_expr(l)? - eval_const_expr(r)?),
        Expr::Path(_) | Expr::Call { .. } | Expr::Eq(_, _) | Expr::Gt(_, _) | Expr::Lt(_, _) => {
            None
        }
    }
}

fn eval_const_bool(expr: &Expr) -> Option<bool> {
    match expr {
        Expr::Eq(l, r) => Some(eval_const_expr(l)? == eval_const_expr(r)?),
        Expr::Gt(l, r) => Some(eval_const_expr(l)? > eval_const_expr(r)?),
        Expr::Lt(l, r) => Some(eval_const_expr(l)? < eval_const_expr(r)?),
        _ => Some(eval_const_expr(expr)? != 0),
    }
}

fn find_first_const_expr(stmts: &[Stmt]) -> Option<&Expr> {
    for stmt in stmts {
        match stmt {
            Stmt::Expr(expr) | Stmt::Send(expr) => {
                if eval_const_expr(expr).is_some() {
                    return Some(expr);
                }
            }
            Stmt::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                if let Some(cond_value) = eval_const_bool(cond) {
                    if cond_value {
                        if let Some(found) = find_first_const_expr(then_branch) {
                            return Some(found);
                        }
                    } else if let Some(else_b) = else_branch {
                        if let Some(found) = find_first_const_expr(else_b) {
                            return Some(found);
                        }
                    }
                    continue;
                }

                if let Some(found) = find_first_const_expr(then_branch) {
                    return Some(found);
                }
                if let Some(else_b) = else_branch {
                    if let Some(found) = find_first_const_expr(else_b) {
                        return Some(found);
                    }
                }
            }
            Stmt::Violation(body) => {
                if let Some(found) = find_first_const_expr(body) {
                    return Some(found);
                }
            }
            Stmt::Call(_) | Stmt::Let { .. } | Stmt::Transition(_) | Stmt::Drop(_) => {}
        }
    }
    None
}

fn collect_add_terms(expr: &Expr, out: &mut Vec<i64>) -> Result<(), CompileError> {
    match expr {
        Expr::Number(n) => {
            let value = i64::try_from(*n).map_err(|_| {
                CompileError::Verify("native backend: number too large for i64".to_string())
            })?;
            out.push(value);
            Ok(())
        }
        Expr::Add(l, r) => {
            collect_add_terms(l, out)?;
            collect_add_terms(r, out)
        }
        Expr::Sub(l, r) => {
            collect_add_terms(l, out)?;
            let mut rhs = Vec::new();
            collect_add_terms(r, &mut rhs)?;
            for value in rhs {
                out.push(-value);
            }
            Ok(())
        }
        Expr::Path(_)
        | Expr::Call { .. }
        | Expr::Eq(_, _)
        | Expr::Gt(_, _)
        | Expr::Lt(_, _) => Err(CompileError::Verify(
            "native backend supports only constant additive expressions".to_string(),
        )),
    }
}

fn build_x86_64_return_expr(expr: &Expr) -> Result<Vec<u8>, CompileError> {
    let mut terms = Vec::new();
    collect_add_terms(expr, &mut terms)?;
    if terms.is_empty() {
        return Err(CompileError::Verify(
            "native backend: expression has no terms".to_string(),
        ));
    }

    let mut code = vec![0x48, 0xB8]; // mov rax, imm64
    code.extend_from_slice(&terms[0].to_le_bytes());

    for term in terms.iter().skip(1) {
        if let Ok(imm32) = i32::try_from(*term) {
            // add rax, imm32 (sign-extended)
            code.extend_from_slice(&[0x48, 0x05]);
            code.extend_from_slice(&imm32.to_le_bytes());
        } else {
            // mov rcx, imm64 ; add rax, rcx
            code.extend_from_slice(&[0x48, 0xB9]);
            code.extend_from_slice(&term.to_le_bytes());
            code.extend_from_slice(&[0x48, 0x01, 0xC8]);
        }
    }

    code.push(0xC3); // ret
    Ok(code)
}

struct ExecutableBuffer {
    ptr: *mut u8,
}

impl ExecutableBuffer {
    fn new(code: &[u8]) -> Result<Self, CompileError> {
        let ptr = alloc_executable(code.len())?;
        // SAFETY: Destination is an allocated writable executable buffer of at least code.len().
        unsafe {
            ptr::copy_nonoverlapping(code.as_ptr(), ptr, code.len());
        }
        Ok(Self { ptr })
    }

    fn call_i64(&self) -> i64 {
        // SAFETY: The buffer contains valid machine code for signature extern "C" fn() -> i64.
        let func: extern "C" fn() -> i64 = unsafe { std::mem::transmute(self.ptr) };
        func()
    }
}

impl Drop for ExecutableBuffer {
    fn drop(&mut self) {
        let _ = free_executable(self.ptr);
    }
}

#[cfg(windows)]
fn alloc_executable(size: usize) -> Result<*mut u8, CompileError> {
    const MEM_COMMIT: u32 = 0x1000;
    const MEM_RESERVE: u32 = 0x2000;
    const PAGE_EXECUTE_READWRITE: u32 = 0x40;

    #[link(name = "kernel32")]
    extern "system" {
        fn VirtualAlloc(
            lpAddress: *mut std::ffi::c_void,
            dwSize: usize,
            flAllocationType: u32,
            flProtect: u32,
        ) -> *mut std::ffi::c_void;
    }

    // SAFETY: Calling Win32 allocation API with null base address and requested size.
    let p = unsafe {
        VirtualAlloc(
            std::ptr::null_mut(),
            size,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        )
    };
    if p.is_null() {
        Err(CompileError::Verify(
            "native backend: VirtualAlloc failed".to_string(),
        ))
    } else {
        Ok(p as *mut u8)
    }
}

#[cfg(windows)]
fn free_executable(ptr: *mut u8) -> Result<(), CompileError> {
    const MEM_RELEASE: u32 = 0x8000;

    #[link(name = "kernel32")]
    extern "system" {
        fn VirtualFree(lpAddress: *mut std::ffi::c_void, dwSize: usize, dwFreeType: u32) -> i32;
    }

    // SAFETY: Frees pointer previously returned by VirtualAlloc.
    let ok = unsafe { VirtualFree(ptr as *mut std::ffi::c_void, 0, MEM_RELEASE) };
    if ok == 0 {
        Err(CompileError::Verify(
            "native backend: VirtualFree failed".to_string(),
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn alloc_executable(_size: usize) -> Result<*mut u8, CompileError> {
    Err(CompileError::Verify(
        "native backend currently implemented for Windows only".to_string(),
    ))
}

#[cfg(not(windows))]
fn free_executable(_ptr: *mut u8) -> Result<(), CompileError> {
    Ok(())
}

fn run_native_backend(program: &Program) -> Result<(), CompileError> {
    if !cfg!(target_arch = "x86_64") {
        return Err(CompileError::Verify(
            "native backend requires x86_64 architecture".to_string(),
        ));
    }

    let handler = first_handler(program).ok_or_else(|| {
        CompileError::Verify("native backend: no contract handler found".to_string())
    })?;

    let expr = find_first_const_expr(&handler.body).ok_or_else(|| {
        CompileError::Verify(
            "native backend supports only handlers with at least one constant arithmetic expression"
                .to_string(),
        )
    })?;

    let expected = eval_const_expr(expr).ok_or_else(|| {
        CompileError::Verify("native backend could not evaluate expression".to_string())
    })?;

    let code = build_x86_64_return_expr(expr)?;
    println!("native bytes: {:02X?}", code);
    let exec = ExecutableBuffer::new(&code)?;
    let out = exec.call_i64();
    println!("native result: {} (expected {})", out, expected);
    if out != expected {
        return Err(CompileError::Verify(format!(
            "native backend mismatch: got {}, expected {}",
            out, expected
        )));
    }
    Ok(())
}

#[derive(Debug, Clone)]
enum Instr {
    Eval(String),
    Call(String),
    Send(String),
    Drop(String),
    JumpIfFalse { cond: String, target: usize },
    Jump { target: usize },
    Transition(String),
    Nop,
}

fn compile_program(program: &Program) -> Vec<Instr> {
    let mut out = Vec::new();
    if let Some(handler) = first_handler(program) {
        compile_block(&handler.body, &mut out);
    } else {
        out.push(Instr::Nop);
    }
    out
}

fn first_handler(program: &Program) -> Option<&HandlerDecl> {
    for item in &program.items {
        let contract = match item {
            Item::Contract(c) => c,
            Item::SharedContract(_) => continue,
        };
        for state in &contract.states {
            for member in &state.members {
                if let StateMember::Handler(h) = member {
                    return Some(h);
                }
            }
        }
    }
    None
}

fn compile_block(stmts: &[Stmt], out: &mut Vec<Instr>) {
    for stmt in stmts {
        compile_stmt(stmt, out);
    }
}

fn compile_stmt(stmt: &Stmt, out: &mut Vec<Instr>) {
    match stmt {
        Stmt::If {
            cond,
            then_branch,
            else_branch,
        } => {
            let jump_if_false_idx = out.len();
            out.push(Instr::JumpIfFalse {
                cond: cond.to_string(),
                target: usize::MAX,
            });
            compile_block(then_branch, out);

            if let Some(else_b) = else_branch {
                let jump_idx = out.len();
                out.push(Instr::Jump { target: usize::MAX });
                let else_start = out.len();
                if let Instr::JumpIfFalse { target, .. } = &mut out[jump_if_false_idx] {
                    *target = else_start;
                }
                compile_block(else_b, out);
                let end = out.len();
                if let Instr::Jump { target } = &mut out[jump_idx] {
                    *target = end;
                }
            } else {
                let end = out.len();
                if let Instr::JumpIfFalse { target, .. } = &mut out[jump_if_false_idx] {
                    *target = end;
                }
            }
        }
        Stmt::Call(expr) => out.push(Instr::Call(expr.to_string())),
        Stmt::Let { name, expr } => out.push(Instr::Eval(format!("let {} = {}", name, expr))),
        Stmt::Send(expr) => out.push(Instr::Send(expr.to_string())),
        Stmt::Transition(target) => out.push(Instr::Transition(target.clone())),
        Stmt::Violation(body) => compile_block(body, out),
        Stmt::Drop(expr) => out.push(Instr::Drop(expr.to_string())),
        Stmt::Expr(expr) => out.push(Instr::Eval(expr.to_string())),
    }
}

fn run_vm(bytecode: &[Instr]) {
    let mut ip = 0usize;
    while ip < bytecode.len() {
        match &bytecode[ip] {
            Instr::Eval(expr) => {
                println!("[vm] eval {}", expr);
                ip += 1;
            }
            Instr::Call(target) => {
                println!("[vm] call {}", target);
                ip += 1;
            }
            Instr::Send(expr) => {
                println!("[vm] send {}", expr);
                ip += 1;
            }
            Instr::Drop(expr) => {
                println!("[vm] drop {}", expr);
                ip += 1;
            }
            Instr::JumpIfFalse { cond, target } => {
                let cond_is_true = eval_condition(cond);
                if cond_is_true {
                    ip += 1;
                } else {
                    ip = *target;
                }
            }
            Instr::Jump { target } => {
                ip = *target;
            }
            Instr::Transition(state) => {
                println!("[vm] transition -> {}", state);
                ip += 1;
            }
            Instr::Nop => {
                ip += 1;
            }
        }
    }
}

fn eval_condition(cond: &str) -> bool {
    // The VM keeps this deterministic for the PoC; conditions can be wired to runtime data later.
    !cond.contains("false")
}

fn print_bytecode(code: &[Instr]) {
    for (i, instr) in code.iter().enumerate() {
        println!("{:04}: {:?}", i, instr);
    }
}

fn run_command(cmd: &str, source: &str) -> Result<(), CompileError> {
    let program = parse_source(source)?;
    verify_program(&program)?;

    match cmd {
        "check" => {
            println!("FlowLang check successful.");
        }
        "compile" => {
            let code = compile_program(&program);
            print_bytecode(&code);
        }
        "run" => {
            let code = compile_program(&program);
            print_bytecode(&code);
            run_vm(&code);
        }
        "jit-run" => {
            run_native_backend(&program)?;
        }
        _ => {
            println!("usage: flowlang <check|compile|run|jit-run> <file>");
        }
    }
    Ok(())
}

fn main() {
    if let Err(err) = real_main() {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}

fn real_main() -> Result<(), CompileError> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("usage: flowlang <check|compile|run|jit-run> <file>");
        return Ok(());
    }

    let cmd = &args[1];
    let path = &args[2];
    let source = fs::read_to_string(path)?;
    run_command(cmd, &source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_verifies_tcp_contract() {
        let source = r#"
            shared_contract GlobalRegistry {
                state DATA {
                    var active_connections: u32 = 0;
                    on increment() { active_connections + 1; }
                }
            }

            contract TCP_Stack {
                state LISTEN {
                    on receive(p: LinearBuffer) {
                        if (p.has_syn_flag()) {
                            call GlobalRegistry.increment();
                            let response = create_syn_ack(p);
                            send response;
                            transition -> SYN_SENT;
                        } else {
                            violation {
                                drop p;
                                send reset();
                            }
                        }
                    }
                }
            }
        "#;

        let program = parse_source(source).expect("parser should succeed");
        verify_program(&program).expect("linearity check should succeed");
    }

    #[test]
    fn fails_without_violation_block_for_linear_handler() {
        let source = r#"
            contract TCP_Stack {
                state LISTEN {
                    on receive(p: LinearBuffer) {
                        if (p.has_syn_flag()) {
                            drop p;
                        } else {
                            send reset();
                        }
                    }
                }
            }
        "#;

        let program = parse_source(source).expect("parser should succeed");
        let err = verify_program(&program).expect_err("verification should fail");
        let msg = err.to_string();
        assert!(msg.contains("no explicit violation block"));
        assert!(msg.contains("else-path without violation handling"));
    }

    #[test]
    fn fails_on_if_without_else_for_linear_handler() {
        let source = r#"
            contract TCP_Stack {
                state LISTEN {
                    on receive(p: LinearBuffer) {
                        if (p.has_syn_flag()) {
                            drop p;
                        }
                        violation {
                            send reset();
                        }
                    }
                }
            }
        "#;

        let program = parse_source(source).expect("parser should succeed");
        let err = verify_program(&program).expect_err("verification should fail");
        let msg = err.to_string();
        assert!(msg.contains("if without else"));
    }

    #[test]
    fn evals_const_expression_for_native_subset() {
        let expr = Expr::Sub(
            Box::new(Expr::Add(Box::new(Expr::Number(50)), Box::new(Expr::Number(10)))),
            Box::new(Expr::Number(18)),
        );
        assert_eq!(eval_const_expr(&expr), Some(42));
    }

    #[test]
    fn emits_machine_code_for_const_return() {
        let expr = Expr::Add(
            Box::new(Expr::Add(Box::new(Expr::Number(10)), Box::new(Expr::Number(20)))),
            Box::new(Expr::Number(12)),
        );
        let code = build_x86_64_return_expr(&expr).expect("machine code emission should succeed");
        assert_eq!(code[0], 0x48);
        assert_eq!(code[1], 0xB8);
        assert!(code.windows(2).any(|w| w == [0x48, 0x05]));
        assert_eq!(*code.last().unwrap_or(&0), 0xC3);
    }

    #[test]
    fn evals_const_comparison_conditions() {
        let gt = Expr::Gt(Box::new(Expr::Number(7)), Box::new(Expr::Number(3)));
        let eq = Expr::Eq(
            Box::new(Expr::Sub(Box::new(Expr::Number(10)), Box::new(Expr::Number(4)))),
            Box::new(Expr::Number(6)),
        );
        assert_eq!(eval_const_bool(&gt), Some(true));
        assert_eq!(eval_const_bool(&eq), Some(true));
    }

    #[test]
    fn chooses_const_if_branch_for_native_search() {
        let stmts = vec![Stmt::If {
            cond: Expr::Gt(Box::new(Expr::Number(5)), Box::new(Expr::Number(2))),
            then_branch: vec![Stmt::Expr(Expr::Sub(
                Box::new(Expr::Number(100)),
                Box::new(Expr::Number(58)),
            ))],
            else_branch: Some(vec![Stmt::Expr(Expr::Number(1))]),
        }];

        let expr = find_first_const_expr(&stmts).expect("branch expression should exist");
        assert_eq!(eval_const_expr(expr), Some(42));
    }
}
