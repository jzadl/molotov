use crate::ast::*;
use crate::tokenizer::{Token, TokenWithMeta};

pub struct Parser {
    tokens: Vec<TokenWithMeta>,
    pos: usize,
    context: Vec<String>,
}

impl Parser {
    pub fn new(tokens: Vec<TokenWithMeta>) -> Self {
        Parser { tokens, pos: 0, context: Vec::new() }
    }

    fn with_context<F, T>(&mut self, desc: &str, f: F) -> Result<T, String>
    where
        F: FnOnce(&mut Self) -> Result<T, String>,
    {
        self.context.push(desc.to_string());
        let result = f(self);
        self.context.pop();
        result
    }

    fn ctx_str(&self) -> String {
        if self.context.is_empty() {
            String::new()
        } else {
            format!(" while parsing {}", self.context.join(" > "))
        }
    }

    fn peek(&self) -> Option<&TokenWithMeta> {
        self.tokens.get(self.pos)
    }

    fn peek_at(&self, offset: usize) -> Option<&TokenWithMeta> {
        self.tokens.get(self.pos + offset)
    }

    fn advance(&mut self) -> Option<&TokenWithMeta> {
        let tok = self.tokens.get(self.pos);
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<(), String> {
        let ctx = self.ctx_str();
        match self.advance() {
            Some(tm) if &tm.token == expected => Ok(()),
            Some(tm) => Err(format!("line {}: expected {:?}, got {:?}{}", tm.line, expected, tm.token, ctx)),
            None => Err(format!("unexpected end of input, expected {:?}{}", expected, ctx)),
        }
    }

    fn expect_ident(&mut self) -> Result<String, String> {
        let ctx = self.ctx_str();
        match self.advance() {
            Some(tm) => match &tm.token {
                Token::Ident(s) => Ok(s.clone()),
                other => Err(format!("line {}: expected identifier, got {:?}{}", tm.line, other, ctx)),
            },
            None => Err(format!("unexpected end of input, expected identifier{}", ctx)),
        }
    }

    fn skip_newlines(&mut self) {
        while let Some(tm) = self.peek() {
            if matches!(tm.token, Token::Newline) {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn check(&self, token: &Token) -> bool {
        self.peek().map_or(false, |tm| &tm.token == token)
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
        self.skip_newlines();
        let mut stmts = Vec::new();
        while self.peek().is_some() {
            let stmt = self.with_context("program", |s| s.parse_stmt(0))?;
            stmts.push(stmt);
            self.skip_newlines();
        }
        Ok(Program { stmts })
    }

    fn parse_stmt(&mut self, _indent_level: usize) -> Result<Stmt, String> {
        self.skip_newlines();

        let mut decorators = Vec::new();
        if self.check(&Token::At) {
            while self.check(&Token::At) {
                self.advance();
                decorators.push(self.expect_ident()?);
                self.skip_newlines();
            }
        }

        let peek = match self.peek() {
            Some(t) => t.clone(),
            None => return Err(format!("unexpected end of input{}", self.ctx_str())),
        };

        let stmt = match &peek.token {
            Token::KwDef => self.with_context("function definition", |s| s.parse_func_def(decorators)),
            Token::KwClass => self.with_context("class definition", |s| s.parse_class_def(decorators)),
            Token::KwIf => self.with_context("if statement", |s| s.parse_if()),
            Token::KwWhile => self.with_context("while loop", |s| s.parse_while()),
            Token::KwFor => self.with_context("for loop", |s| s.parse_for()),
            Token::KwReturn => self.with_context("return statement", |s| s.parse_return()),
            Token::KwBreak => { self.advance(); Ok(Stmt::Break) }
            Token::KwContinue => { self.advance(); Ok(Stmt::Continue) }
            Token::KwPass => { self.advance(); Ok(Stmt::Pass) }
            Token::KwImport => self.with_context("import", |s| s.parse_import()),
            Token::KwFrom => self.with_context("from import", |s| s.parse_from_import()),
            Token::KwDel => self.with_context("del statement", |s| s.parse_del()),
            Token::KwTry => self.with_context("try statement", |s| s.parse_try()),
            Token::KwRaise => self.with_context("raise statement", |s| s.parse_raise()),
            Token::KwWith => self.with_context("with statement", |s| s.parse_with()),
            _ => {
                if !decorators.is_empty() {
                    return Err(format!("decorator without function or class{}", self.ctx_str()));
                }
                self.with_context("expression statement", |s| s.parse_simple_stmt())
            }
        };

        stmt
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        self.skip_newlines();

        if self.check(&Token::Indent) || self.check(&Token::Newline) {
            if self.check(&Token::Newline) {
                self.advance();
                self.skip_newlines();
            }
            if !self.check(&Token::Indent) {
                return Err("expected indented block".to_string());
            }
            self.advance();

            while self.peek().is_some() && !self.check(&Token::Dedent) {
                self.skip_newlines();
                if self.check(&Token::Dedent) {
                    break;
                }
                let stmt = self.parse_stmt(0)?;
                stmts.push(stmt);
                self.skip_newlines();
            }

            if self.check(&Token::Dedent) {
                self.advance();
            }
        } else {
            let stmt = self.parse_simple_stmt()?;
            stmts.push(stmt);
        }

        Ok(stmts)
    }

    fn parse_simple_stmt(&mut self) -> Result<Stmt, String> {
        let expr = self.parse_expr()?;

        if self.check(&Token::Comma) {
            let mut elems = vec![expr];
            while self.check(&Token::Comma) {
                self.advance();
                if self.check(&Token::Newline) || self.check(&Token::Dedent) || self.peek().is_none() {
                    break;
                }
                elems.push(self.parse_expr()?);
            }
            if self.check(&Token::Eq) {
                return self.parse_assign_tuple(elems);
            } else {
                return Err("tuple must be used in assignment".to_string());
            }
        }

        if self.check(&Token::Eq) {
            self.advance();
            let value = self.parse_expr()?;

            match expr {
                Expr::Ident(name) => {
                    if self.check(&Token::Newline) { self.advance(); }
                    Ok(Stmt::Assign(name, value, true))
                }
                Expr::Attribute(obj, attr) => {
                    if self.check(&Token::Newline) { self.advance(); }
                    if let Expr::Ident(self_name) = *obj {
                        if self_name == "self" {
                            Ok(Stmt::Assign(format!("self.{}", attr), value, true))
                        } else {
                            Err("assignment target must be an identifier or self.x".to_string())
                        }
                    } else {
                        Err("assignment target must be an identifier or self.x".to_string())
                    }
                }
                Expr::Subscript(target, index) => {
                    if self.check(&Token::Newline) { self.advance(); }
                    match *target {
                        Expr::Ident(name) => {
                            Ok(Stmt::Assign(format!("__setitem__({})", name),
                                Expr::FuncCall("__setitem_value__".to_string(), vec![*index, value]), true))
                        }
                        _ => Err("subscript target must be an identifier".to_string()),
                    }
                }
                _ => {
                    if self.check(&Token::Newline) { self.advance(); }
                    Err("assignment target must be an identifier".to_string())
                }
            }
        } else if self.check(&Token::PlusEq) { self.advance(); let v = self.parse_expr()?; self.skip_newlines(); Ok(Stmt::AugAssign(self.extract_name(&expr)?, BinOp::Add, v)) }
        else if self.check(&Token::MinusEq) { self.advance(); let v = self.parse_expr()?; self.skip_newlines(); Ok(Stmt::AugAssign(self.extract_name(&expr)?, BinOp::Sub, v)) }
        else if self.check(&Token::StarEq) { self.advance(); let v = self.parse_expr()?; self.skip_newlines(); Ok(Stmt::AugAssign(self.extract_name(&expr)?, BinOp::Mul, v)) }
        else if self.check(&Token::SlashEq) { self.advance(); let v = self.parse_expr()?; self.skip_newlines(); Ok(Stmt::AugAssign(self.extract_name(&expr)?, BinOp::Div, v)) }
        else if self.check(&Token::PercentEq) { self.advance(); let v = self.parse_expr()?; self.skip_newlines(); Ok(Stmt::AugAssign(self.extract_name(&expr)?, BinOp::Mod, v)) }
        else {
            if self.check(&Token::Newline) { self.advance(); }
            Ok(Stmt::Expr(expr))
        }
    }

    fn parse_assign_tuple(&mut self, elems: Vec<Expr>) -> Result<Stmt, String> {
        for e in &elems {
            if !matches!(e, Expr::Ident(_)) {
                return Err("tuple assignment targets must be identifiers".to_string());
            }
        }
        self.expect(&Token::Eq)?;
        let first = self.parse_expr()?;
        let value = if self.check(&Token::Comma) {
            let mut vals = vec![first];
            while self.check(&Token::Comma) {
                self.advance();
                if self.check(&Token::Newline) || self.check(&Token::Dedent) || self.peek().is_none() {
                    break;
                }
                vals.push(self.parse_expr()?);
            }
            Expr::Tuple(vals)
        } else {
            first
        };
        if self.check(&Token::Newline) { self.advance(); }
        let names = elems.into_iter().map(|e| match e { Expr::Ident(n) => n, _ => unreachable!() }).collect();
        Ok(Stmt::AssignTuple(names, value))
    }

    fn extract_name(&self, expr: &Expr) -> Result<String, String> {
        match expr {
            Expr::Ident(name) => Ok(name.clone()),
            _ => Err("expected identifier".to_string()),
        }
    }

    fn parse_func_def(&mut self, decorators: Vec<String>) -> Result<Stmt, String> {
        self.advance();
        let name = self.expect_ident()?;
        self.expect(&Token::LParen)?;

        let mut params = Vec::new();
        if !self.check(&Token::RParen) {
            loop {
                if self.check(&Token::Star) {
                    self.advance();
                    let varargs = self.expect_ident()?;
                    params.push((format!("*{}", varargs), None));
                } else {
                    let param_name = self.expect_ident()?;
                    params.push((param_name, None));
                }
                if self.check(&Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(&Token::RParen)?;
        self.expect(&Token::Colon)?;
        let body = self.parse_block()?;
        let return_type = None;

        Ok(Stmt::FuncDef { name, params, body, return_type, decorators })
    }

    fn parse_class_def(&mut self, decorators: Vec<String>) -> Result<Stmt, String> {
        self.advance();
        let name = self.expect_ident()?;
        let mut bases = Vec::new();

        if self.check(&Token::LParen) {
            self.advance();
            if !self.check(&Token::RParen) {
                loop {
                    bases.push(self.expect_ident()?);
                    if self.check(&Token::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
            self.expect(&Token::RParen)?;
        }
        self.expect(&Token::Colon)?;
        let body = self.parse_block()?;

        Ok(Stmt::ClassDef { name, bases, body, decorators })
    }

    fn parse_if(&mut self) -> Result<Stmt, String> {
        self.advance();
        let mut branches = Vec::new();

        let cond = self.parse_expr()?;
        self.expect(&Token::Colon)?;
        let body = self.parse_block()?;
        branches.push((cond, body));

        self.skip_newlines();
        while self.check(&Token::KwElif) {
            self.advance();
            let cond = self.parse_expr()?;
            self.expect(&Token::Colon)?;
            let body = self.parse_block()?;
            branches.push((cond, body));
            self.skip_newlines();
        }

        let else_body = if self.check(&Token::KwElse) {
            self.advance();
            self.expect(&Token::Colon)?;
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(Stmt::If(branches, else_body))
    }

    fn parse_while(&mut self) -> Result<Stmt, String> {
        self.advance();
        let cond = self.parse_expr()?;
        self.expect(&Token::Colon)?;
        let body = self.parse_block()?;
        Ok(Stmt::While(cond, body))
    }

    fn parse_for(&mut self) -> Result<Stmt, String> {
        self.advance();
        let mut vars = vec![self.expect_ident()?];
        while self.check(&Token::Comma) {
            self.advance();
            vars.push(self.expect_ident()?);
        }
        self.expect(&Token::KwIn)?;
        let iterable = self.parse_expr()?;
        self.expect(&Token::Colon)?;
        let body = self.parse_block()?;
        Ok(Stmt::For(vars, iterable, body))
    }

    fn parse_return(&mut self) -> Result<Stmt, String> {
        self.advance();
        if self.check(&Token::Newline) || self.check(&Token::Dedent) || self.peek().is_none() {
            Ok(Stmt::Return(None))
        } else {
            let expr = self.parse_expr()?;
            if self.check(&Token::Newline) { self.advance(); }
            Ok(Stmt::Return(Some(expr)))
        }
    }

    fn parse_module_path(&mut self) -> Result<String, String> {
        let mut path = self.expect_ident()?;
        while self.check(&Token::Colon) {
            self.advance();
            if self.check(&Token::Colon) {
                self.advance();
                path.push_str(&format!("::{}", self.expect_ident()?));
            } else {
                break;
            }
        }
        Ok(path)
    }

    fn parse_import(&mut self) -> Result<Stmt, String> {
        self.advance();
        let module = self.parse_module_path()?;
        let alias = if self.check(&Token::KwAs) {
            self.advance();
            Some(self.expect_ident()?)
        } else {
            None
        };
        Ok(Stmt::Import(module, alias))
    }

    fn parse_from_import(&mut self) -> Result<Stmt, String> {
        self.advance();
        let module = {
            let mut path = self.expect_ident()?;
            while self.check(&Token::Colon) {
                self.advance();
                if self.check(&Token::Colon) {
                    self.advance();
                    if self.check(&Token::KwImport) { break; }
                    path.push_str(&format!("::{}", self.expect_ident()?));
                } else { break; }
            }
            path
        };
        self.expect(&Token::KwImport)?;
        let mut names = Vec::new();
        loop {
            let name = self.expect_ident()?;
            let alias = if self.check(&Token::KwAs) {
                self.advance();
                Some(self.expect_ident()?)
            } else {
                None
            };
            names.push((name, alias));
            if self.check(&Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        Ok(Stmt::FromImport(module, names))
    }

    fn parse_del(&mut self) -> Result<Stmt, String> {
        self.advance();
        let mut names = Vec::new();
        names.push(self.expect_ident()?);
        while self.check(&Token::Comma) {
            self.advance();
            names.push(self.expect_ident()?);
        }
        Ok(Stmt::Delete(names))
    }

    fn parse_try(&mut self) -> Result<Stmt, String> {
        self.advance();
        self.expect(&Token::Colon)?;
        let body = self.parse_block()?;
        self.skip_newlines();

        let mut handlers = Vec::new();
        let mut else_body = None;
        let mut finally_body = None;

        while self.check(&Token::KwExcept) {
            self.advance();
            let exception = if matches!(self.peek().map(|t| &t.token), Some(Token::Ident(_)) | Some(Token::LParen)) {
                self.parse_expr().map(|e| format!("{:?}", e))?
            } else {
                "Exception".to_string()
            };
            let var = if self.check(&Token::KwAs) {
                self.advance();
                Some(self.expect_ident()?)
            } else {
                None
            };
            self.expect(&Token::Colon)?;
            let handler_body = self.parse_block()?;
            handlers.push(Handler { exception, var, body: handler_body });
            self.skip_newlines();
        }

        if self.check(&Token::KwElse) {
            self.advance();
            self.expect(&Token::Colon)?;
            else_body = Some(self.parse_block()?);
            self.skip_newlines();
        }

        if self.check(&Token::KwFinally) {
            self.advance();
            self.expect(&Token::Colon)?;
            finally_body = Some(self.parse_block()?);
        }

        Ok(Stmt::Try { body, handlers, else_body, finally_body })
    }

    fn parse_raise(&mut self) -> Result<Stmt, String> {
        self.advance();
        if self.check(&Token::Newline) || self.check(&Token::Dedent) || self.peek().is_none() {
            Ok(Stmt::Raise(None))
        } else {
            let expr = self.parse_expr()?;
            Ok(Stmt::Raise(Some(expr)))
        }
    }

    fn parse_with(&mut self) -> Result<Stmt, String> {
        self.advance();
        let expr = self.parse_expr()?;
        let var = if self.check(&Token::KwAs) {
            self.advance();
            Some(self.expect_ident()?)
        } else {
            None
        };
        self.expect(&Token::Colon)?;
        let body = self.parse_block()?;
        Ok(Stmt::With { expr, var, body })
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.with_context("expression", |s| s.parse_or_expr())
    }

    fn parse_or_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and_expr()?;
        while self.check(&Token::KwOr) {
            self.advance();
            let right = self.parse_and_expr()?;
            left = Expr::BinOp(Box::new(left), BinOp::Or, Box::new(right));
        }
        Ok(left)
    }

    fn parse_and_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_not_expr()?;
        while self.check(&Token::KwAnd) {
            self.advance();
            let right = self.parse_not_expr()?;
            left = Expr::BinOp(Box::new(left), BinOp::And, Box::new(right));
        }
        Ok(left)
    }

    fn parse_not_expr(&mut self) -> Result<Expr, String> {
        if self.check(&Token::KwNot) {
            self.advance();
            let expr = self.parse_not_expr()?;
            return Ok(Expr::UnaryOp(UnaryOp::Not, Box::new(expr)));
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_term_expr()?;

        loop {
            let op = if self.check(&Token::EqEq) { Some(BinOp::Eq) }
            else if self.check(&Token::BangEq) { Some(BinOp::Ne) }
            else if self.check(&Token::Less) { Some(BinOp::Lt) }
            else if self.check(&Token::Greater) { Some(BinOp::Gt) }
            else if self.check(&Token::LessEq) { Some(BinOp::Le) }
            else if self.check(&Token::GreaterEq) { Some(BinOp::Ge) }
            else if self.check(&Token::KwIn) { Some(BinOp::In) }
            else if self.check(&Token::KwIs) { Some(BinOp::Is) }
            else if self.check(&Token::KwNot) && self.peek_at(1).map_or(false, |t| t.token == Token::KwIn) {
                Some(BinOp::NotIn)
            }
            else { None };

            match op {
                Some(op) => {
                    self.advance();
                    if op == BinOp::NotIn { self.advance(); } // consume the 'in' too
                    let right = self.parse_term_expr()?;
                    left = Expr::BinOp(Box::new(left), op, Box::new(right));
                }
                None => break,
            }
        }
        Ok(left)
    }

    fn parse_term_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_arith_expr()?;
        loop {
            let op = if self.check(&Token::Plus) { Some(BinOp::Add) }
            else if self.check(&Token::Minus) { Some(BinOp::Sub) }
            else { None };

            match op {
                Some(op) => {
                    self.advance();
                    let right = self.parse_arith_expr()?;
                    left = Expr::BinOp(Box::new(left), op, Box::new(right));
                }
                None => break,
            }
        }
        Ok(left)
    }

    fn parse_arith_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary_expr()?;
        loop {
            let op = if self.check(&Token::Star) { Some(BinOp::Mul) }
            else if self.check(&Token::Slash) { Some(BinOp::Div) }
            else if self.check(&Token::DoubleSlash) { Some(BinOp::FloorDiv) }
            else if self.check(&Token::Percent) { Some(BinOp::Mod) }
            else { None };

            match op {
                Some(op) => {
                    self.advance();
                    let right = self.parse_unary_expr()?;
                    left = Expr::BinOp(Box::new(left), op, Box::new(right));
                }
                None => break,
            }
        }
        Ok(left)
    }

    fn parse_unary_expr(&mut self) -> Result<Expr, String> {
        if self.check(&Token::Minus) {
            self.advance();
            let expr = self.parse_unary_expr()?;
            return Ok(Expr::UnaryOp(UnaryOp::Neg, Box::new(expr)));
        }
        self.parse_pow_expr()
    }

    fn parse_pow_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_primary()?;
        if self.check(&Token::DoubleStar) {
            self.advance();
            let right = self.parse_unary_expr()?;
            left = Expr::BinOp(Box::new(left), BinOp::Pow, Box::new(right));
        }
        self.parse_trailers(left)
    }

    fn parse_trailers(&mut self, mut left: Expr) -> Result<Expr, String> {
        loop {
            if self.check(&Token::LParen) {
                self.advance();
                let mut args = Vec::new();
                if !self.check(&Token::RParen) {
                    loop {
                        if self.check(&Token::Star) {
                            self.advance();
                            let inner = self.parse_expr()?;
                            args.push(Expr::Starred(Box::new(inner)));
                        } else {
                            args.push(self.parse_expr()?);
                        }
                        if self.check(&Token::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(&Token::RParen)?;

                match left {
                    Expr::Ident(name) => {
                        left = Expr::FuncCall(name, args);
                    }
                    Expr::Attribute(_, _) => {
                        left = Expr::FuncCall("__method_call__".to_string(), vec![left, Expr::List(args)]);
                    }
                    _ => {
                        left = Expr::FuncCall("__call__".to_string(), vec![left, Expr::List(args)]);
                    }
                }
            } else if self.check(&Token::LBracket) {
                self.advance();

                if self.check(&Token::RBracket) {
                    self.advance();
                    left = Expr::Subscript(Box::new(left), Box::new(Expr::Slice(None, None, None)));
                    continue;
                }

                let first = self.parse_expr()?;

                if self.check(&Token::KwFor) {
                    let comp = self.parse_comp(ComprehensionKind::List, first)?;
                    left = Expr::Comp(comp);
                    continue;
                }

                if self.check(&Token::Colon) {
                    self.advance();
                    let end = if self.check(&Token::RBracket) { None } else {
                        if self.check(&Token::Colon) { None } else { Some(Box::new(self.parse_expr()?)) }
                    };
                    let step = if self.check(&Token::Colon) {
                        self.advance();
                        if self.check(&Token::RBracket) { None } else { Some(Box::new(self.parse_expr()?)) }
                    } else { None };
                    self.expect(&Token::RBracket)?;

                    let start = Some(Box::new(first));
                    left = Expr::Subscript(Box::new(left), Box::new(Expr::Slice(start, end, step)));
                } else {
                    self.expect(&Token::RBracket)?;
                    left = Expr::Subscript(Box::new(left), Box::new(first));
                }
            } else if self.check(&Token::Dot) {
                self.advance();
                let attr = if let Some(tm) = self.peek() {
                    match &tm.token {
                        Token::IntLit(n) => {
                            let s = n.to_string();
                            self.advance();
                            s
                        }
                        _ => self.expect_ident()?,
                    }
                } else {
                    return Err("expected identifier after dot".to_string());
                };
                if self.check(&Token::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    if !self.check(&Token::RParen) {
                        loop {
                            if self.check(&Token::Star) {
                                self.advance();
                                let inner = self.parse_expr()?;
                                args.push(Expr::Starred(Box::new(inner)));
                            } else {
                                args.push(self.parse_expr()?);
                            }
                            if self.check(&Token::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(&Token::RParen)?;
                    left = Expr::MethodCall(Box::new(left), attr, args);
                } else {
                    left = Expr::Attribute(Box::new(left), attr);
                }
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_comp(&mut self, kind: ComprehensionKind, first: Expr) -> Result<Comprehension, String> {
        let mut generators = Vec::new();

        loop {
            self.advance();
            let var = self.expect_ident()?;
            self.expect(&Token::KwIn)?;
            let iter = self.parse_expr()?;
            let cond = if self.check(&Token::KwIf) {
                self.advance();
                Some(Box::new(self.parse_expr()?))
            } else {
                None
            };
            generators.push(CompGenerator { var, iter: Box::new(iter), cond });

            if !self.check(&Token::KwFor) {
                break;
            }
        }

        if matches!(kind, ComprehensionKind::Dict) {
            if let Expr::Dict(pairs) = &first {
                if pairs.len() == 1 {
                    let (k, v) = &pairs[0];
                    if self.check(&Token::RBrace) {
                        self.advance();
                    }
                    return Ok(Comprehension {
                        kind,
                        element: Box::new(v.clone()),
                        key: Some(Box::new(k.clone())),
                        generators,
                    });
                }
            }
            if self.check(&Token::RBrace) {
                self.advance();
            }
            return Ok(Comprehension {
                kind,
                element: Box::new(first),
                key: None,
                generators,
            });
        }

        if self.check(&Token::RBracket) {
            self.advance();
        }
        Ok(Comprehension {
            kind,
            element: Box::new(first),
            key: None,
            generators,
        })
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let tm = match self.advance() {
            Some(t) => t.clone(),
            None => return Err("unexpected end of input in primary expression".to_string()),
        };

        match tm.token {
            Token::IntLit(s) => {
                let val: i64 = s.parse().map_err(|e| format!("invalid integer '{}': {}", s, e))?;
                Ok(Expr::IntLit(val))
            }
            Token::FloatLit(s) => {
                let val: f64 = s.parse().map_err(|e| format!("invalid float '{}': {}", s, e))?;
                Ok(Expr::FloatLit(val))
            }
            Token::StrLit(s) => Ok(Expr::StrLit(s)),
            Token::FStrLit(s) => {
                let parts = parse_fstring_literal(&s);
                if parts.len() == 1 {
                    Ok(parts.into_iter().next().unwrap())
                } else {
                    Ok(Expr::FStr(parts))
                }
            }
            Token::KwTrue => Ok(Expr::BoolLit(true)),
            Token::KwFalse => Ok(Expr::BoolLit(false)),
            Token::KwNone => Ok(Expr::NoneLit),
            Token::Ident(s) => Ok(Expr::Ident(s)),
            Token::KwLambda => {
                let mut params = Vec::new();
                if !self.check(&Token::Colon) {
                    loop {
                        params.push(self.expect_ident()?);
                        if self.check(&Token::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(&Token::Colon)?;
                let expr = self.parse_expr()?;
                Ok(Expr::Lambda(params, Box::new(expr)))
            }
            Token::LParen => {
                if self.check(&Token::RParen) {
                    self.advance();
                    Ok(Expr::NoneLit)
                } else {
                    let expr = self.parse_expr()?;
                    if self.check(&Token::Comma) {
                        self.advance();
                        let mut elems = vec![expr];
                        while !self.check(&Token::RParen) {
                            elems.push(self.parse_expr()?);
                            if self.check(&Token::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        self.expect(&Token::RParen)?;
                        Ok(Expr::Tuple(elems))
                    } else {
                        self.expect(&Token::RParen)?;
                        Ok(expr)
                    }
                }
            }
            Token::LBracket => {
                if self.check(&Token::RBracket) {
                    self.advance();
                    return Ok(Expr::List(vec![]));
                }
                let first = self.parse_expr()?;
                if self.check(&Token::KwFor) {
                    let comp = self.parse_comp(ComprehensionKind::List, first)?;
                    Ok(Expr::Comp(comp))
                } else if self.check(&Token::Comma) {
                    self.advance();
                    let mut elems = vec![first];
                    while !self.check(&Token::RBracket) {
                        elems.push(self.parse_expr()?);
                        if self.check(&Token::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    self.expect(&Token::RBracket)?;
                    Ok(Expr::List(elems))
                } else {
                    self.expect(&Token::RBracket)?;
                    Ok(Expr::List(vec![first]))
                }
            }
            Token::LBrace => {
                if self.check(&Token::RBrace) {
                    self.advance();
                    return Ok(Expr::Dict(vec![]));
                }
                let first = self.parse_expr()?;
                self.expect(&Token::Colon)?;
                let value = self.parse_expr()?;
                if self.check(&Token::KwFor) {
                    let dict_pair = Expr::Dict(vec![(first, value)]);
                    let comp = self.parse_comp(ComprehensionKind::Dict, dict_pair)?;
                    Ok(Expr::Comp(comp))
                } else {
                    let mut pairs = vec![(first, value)];
                    while self.check(&Token::Comma) {
                        self.advance();
                        if self.check(&Token::RBrace) {
                            break;
                        }
                        let k = self.parse_expr()?;
                        self.expect(&Token::Colon)?;
                        let v = self.parse_expr()?;
                        pairs.push((k, v));
                    }
                    self.expect(&Token::RBrace)?;
                    Ok(Expr::Dict(pairs))
                }
            }
            _ => {
                let ctx = self.ctx_str();
                Err(format!("line {}: unexpected token {:?} in expression{}", tm.line, tm.token, ctx))
            }
        }
    }
}

fn parse_fstring_literal(content: &str) -> Vec<Expr> {
    let mut parts = Vec::new();
    let mut rest = content;
    while !rest.is_empty() {
        if let Some(ob_start) = rest.find('{') {
            if ob_start > 0 {
                parts.push(Expr::StrLit(rest[..ob_start].to_string()));
            }
            let after_ob = &rest[ob_start + 1..];
            let cb_end = after_ob.find('}').unwrap_or(after_ob.len());
            let expr_str = after_ob[..cb_end].trim();
            let expr = parse_fstring_expr(expr_str);
            parts.push(expr);
            let consumed = ob_start + 1 + cb_end + 1;
            rest = &rest[consumed..];
        } else {
            parts.push(Expr::StrLit(rest.to_string()));
            break;
        }
    }
    parts
}

fn parse_fstring_expr(s: &str) -> Expr {
    if let Some(dot) = s.find('.') {
        let obj = s[..dot].trim().to_string();
        let attr = s[dot + 1..].trim().to_string();
        let obj_expr = parse_fstring_expr(&obj);
        Expr::Attribute(Box::new(obj_expr), attr)
    } else if let Some(bs) = s.find('[') {
        let obj = s[..bs].trim().to_string();
        let idx_str = s[bs + 1..].trim_end_matches(']').trim();
        let idx_expr = if let Ok(n) = idx_str.parse::<i64>() {
            Expr::IntLit(n)
        } else {
            Expr::Ident(idx_str.to_string())
        };
        Expr::Subscript(Box::new(Expr::Ident(obj)), Box::new(idx_expr))
    } else if let Ok(n) = s.parse::<i64>() {
        Expr::IntLit(n)
    } else if let Ok(f) = s.parse::<f64>() {
        Expr::FloatLit(f)
    } else if s == "True" {
        Expr::BoolLit(true)
    } else if s == "False" {
        Expr::BoolLit(false)
    } else if s == "None" {
        Expr::NoneLit
    } else {
        Expr::Ident(s.to_string())
    }
}


// why am i even doing this
// nobody is ever going to read or use this
// i should just give up
// but i wont
// i will finish this project
// and i will make it the best damn python parser ever
// and then i will be happy
// and everyone will be happy
// and the world will be a better place
// but until then
// i will keep coding
// even if it kills me
// which it probably will
// i love coding
// am i going crazy??

// Made by JZadl the GOAT
