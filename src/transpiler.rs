use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::fs;
use crate::ast::*;
use crate::tokenizer::tokenize;
use crate::parser::Parser;

fn resolve_module(module_path: &str, source_dir: &str) -> Option<PathBuf> {
    let parts: Vec<&str> = module_path.split("::").collect();
    
    // Try current dir first
    let mut local_path = PathBuf::from(source_dir);
    for part in &parts { local_path.push(part); }
    local_path.set_extension("mltv");
    if local_path.exists() { return Some(local_path); }

    // Try ~/.molotov/libs
    if let Ok(home) = std::env::var("HOME") {
        let mut lib_path = PathBuf::from(home);
        lib_path.push(".molotov/libs");
        
        // Try direct file ~/.molotov/libs/a/b.mltv
        let mut p1 = lib_path.clone();
        for part in &parts { p1.push(part); }
        p1.set_extension("mltv");
        if p1.exists() { return Some(p1); }

        // Try ~/.molotov/libs/a/a.mltv (convenience)
        if parts.len() == 1 {
            let mut p2 = lib_path.clone();
            p2.push(parts[0]);
            p2.push(parts[0]);
            p2.set_extension("mltv");
            if p2.exists() { return Some(p2); }
        }
    }

    None
}

#[derive(Default)]
struct TypeEnv {
    vars: HashMap<String, Type>,
    fn_params: HashMap<String, Vec<(String, Type)>>,
    fn_return: HashMap<String, Type>,
    uses_hashmap: bool,
    uses_serde: bool,
    mutated: HashSet<String>,
}

impl TypeEnv {
    fn infer_expr_type(&mut self, expr: &Expr) -> Type {
        match expr {
            Expr::IntLit(_) => Type::I64,
            Expr::FloatLit(_) => Type::F64,
            Expr::StrLit(_) => Type::Str,
            Expr::BoolLit(_) => Type::Bool,
            Expr::NoneLit => Type::None,
            Expr::Ident(name) => self.vars.get(name).cloned().unwrap_or(Type::Unknown),
            Expr::List(items) => {
                if items.is_empty() { Type::List(Box::new(Type::Unknown)) }
                else { Type::List(Box::new(self.infer_expr_type(&items[0]))) }
            }
            Expr::Dict(pairs) => {
                if pairs.is_empty() { Type::Dict(Box::new(Type::Str), Box::new(Type::Value)) }
                else {
                    let kt = self.infer_expr_type(&pairs[0].0);
                    let vt = self.infer_expr_type(&pairs[0].1);
                    Type::Dict(Box::new(kt), Box::new(vt))
                }
            }
            Expr::Tuple(items) => Type::Tuple(items.iter().map(|e| self.infer_expr_type(e)).collect()),
            Expr::Starred(e) => self.infer_expr_type(e),
            Expr::Comp(c) => match c.kind {
                ComprehensionKind::List => Type::List(Box::new(self.infer_expr_type(&c.element))),
                ComprehensionKind::Dict => Type::Dict(
                    Box::new(c.key.as_ref().map(|k| self.infer_expr_type(k)).unwrap_or(Type::Str)),
                    Box::new(self.infer_expr_type(&c.element)),
                ),
            },
            Expr::BinOp(left, op, right) => match op {
                BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge | BinOp::In | BinOp::NotIn | BinOp::Is => Type::Bool,
                BinOp::Add => {
                    let lt = self.infer_expr_type(left);
                    let rt = self.infer_expr_type(right);
                    if lt == Type::Str || rt == Type::Str || lt == Type::Unknown || rt == Type::Unknown { Type::Str } else { Type::I64 }
                }
                BinOp::FloorDiv | BinOp::Mod | BinOp::Sub | BinOp::Mul | BinOp::Div => Type::I64,
                BinOp::Pow => Type::F64,
                BinOp::And | BinOp::Or => Type::Bool,
            },
            Expr::UnaryOp(op, _) => match op {
                UnaryOp::Neg => Type::I64,
                UnaryOp::Not => Type::Bool,
            },
            Expr::FuncCall(name, _) => match name.as_str() {
                "print" | "println" => Type::Unit,
                "len" | "range" => Type::I64,
                "int" => Type::I64,
                "float" => Type::F64,
                "str" => Type::Str,
                "input" => Type::Str,
                "args" => Type::List(Box::new(Type::Str)),
                "enumerate" => Type::Unknown,
                "zip" => Type::Unknown,
                "map" => Type::Unknown,
                "filter" => Type::Unknown,
                "cinclude" => Type::Str,
                "embed_rust" => Type::Unknown,
                "sleep" => Type::Unit,
                "randint" => Type::I64,
                "randch" => Type::Unknown,
                "shuffle" => Type::Unit,
                "sum" => {
                    if let Expr::FuncCall(_, args) = expr {
                        let elem_type = args.first().map(|a| self.infer_expr_type(a));
                        if matches!(elem_type, Some(Type::List(t)) if *t == Type::F64) { Type::F64 } else { Type::I64 }
                    } else { Type::I64 }
                }
                "avg" => Type::F64,
                "min_val" => Type::I64,
                "max_val" => Type::I64,
                "clamp" => Type::I64,
                "read_file" => Type::Str,
                "write_file" => Type::Unit,
                "exists" => Type::Bool,
                "abs" => Type::I64,
                "round" => Type::I64,
                "type_of" => Type::Str,
                "today" => Type::Str,
                "now" => Type::Str,
                "clear" => Type::Unit,
                "__len__" => Type::I64,
                "__del__" => Type::Unit,
                "__method_call__" => Type::Unknown,
                _ => self.fn_return.get(name).cloned().unwrap_or(Type::Unknown),
            },
            Expr::MethodCall(_, method, _) => match method.as_str() {
                "append" | "push" | "insert" | "sort" | "reverse" | "clear" => Type::Unit,
                "pop" | "remove" => Type::Unknown,
                "upper" | "lower" | "strip" | "replace" | "capitalize" | "title" | "swapcase" => Type::Str,
                "split" | "splitlines" | "rsplit" => Type::List(Box::new(Type::Str)),
                "join" => Type::Str,
                "startswith" | "endswith" | "isalpha" | "isdigit" | "isalnum" | "isspace" | "islower" | "isupper" => Type::Bool,
                "find" | "index" | "rfind" | "rindex" | "count" => Type::I64,
                "keys" | "values" | "items" => Type::Unknown,
                "get" | "popitem" => Type::Unknown,
                "copy" => Type::Unknown,
                _ => Type::Unknown,
            },
            Expr::Attribute(_, _) => Type::Unknown,
            Expr::Subscript(obj, index) => {
                let ot = self.infer_expr_type(obj);
                if matches!(index.as_ref(), Expr::Slice(_, _, _)) {
                    match ot {
                        Type::List(t) => Type::List(t),
                        Type::Str => Type::Str,
                        _ => Type::Unknown,
                    }
                } else {
                    match ot {
                        Type::List(t) => *t,
                        Type::Str => Type::Str,
                        Type::Dict(_, v) => *v,
                        _ => Type::Unknown,
                    }
                }
            }
            Expr::Slice(obj, _, _) => {
                let ot = obj.as_ref().map(|o| self.infer_expr_type(o)).unwrap_or(Type::Unknown);
                match ot {
                    Type::List(t) => Type::List(t),
                    Type::Str => Type::Str,
                    _ => Type::Unknown,
                }
            }
            Expr::Lambda(_, _) => Type::Unknown,
            Expr::FStr(_) => Type::Str,
        }
    }
}

fn strip_parens(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('(') && s.ends_with(')') {
        let inner = &s[1..s.len()-1];
        let mut depth = 0i32;
        let mut ok = true;
        for ch in inner.chars() {
            match ch {
                '(' => depth += 1,
                ')' => { depth -= 1; if depth < 0 { ok = false; break; } }
                ',' if depth == 0 => { ok = false; break; }
                _ => {}
            }
        }
        if ok && depth == 0 {
            return inner.to_string();
        }
    }
    s.to_string()
}

fn body_contains_raise(stmts: &[Stmt]) -> bool {
    for s in stmts {
        match s {
            Stmt::Raise(_) => return true,
            Stmt::If(branches, else_body) => {
                for (_, body) in branches { if body_contains_raise(body) { return true; } }
                if let Some(body) = else_body { if body_contains_raise(body) { return true; } }
            }
            Stmt::For(_, _, body) | Stmt::While(_, body) | Stmt::Try { body, .. } => {
                if body_contains_raise(body) { return true; }
            }
            _ => {}
        }
    }
    false
}

fn indent(level: usize) -> String {
    "    ".repeat(level)
}

fn expr_to_rust(expr: &Expr, env: &mut TypeEnv, ctx: &CodegenCtx) -> String {
    match expr {
        Expr::IntLit(v) => v.to_string(),
        Expr::FloatLit(v) => v.to_string(),
        Expr::StrLit(s) => {
            let escaped = s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\t', "\\t").replace('\r', "\\r");
            format!("\"{}\".to_string()", escaped)
        }
        Expr::BoolLit(v) => v.to_string(),
        Expr::NoneLit => "()".to_string(),
        Expr::Ident(name) => name.clone(),
        Expr::Tuple(items) => {
            let inner: Vec<String> = items.iter().map(|e| expr_to_rust(e, env, ctx)).collect();
            format!("({})", inner.join(", "))
        }
        Expr::Starred(e) => expr_to_rust(e, env, ctx),
        Expr::Comp(c) => {
            for g in &c.generators {
                env.vars.insert(g.var.clone(), Type::I64);
            }
            match c.kind {
                ComprehensionKind::List => {
                    let elem = expr_to_rust(&c.element, env, ctx);
                    let mut code = String::new();
                    code.push_str("{ let mut __comp = Vec::new(); ");
                    for g in &c.generators {
                        let iter_code = expr_to_rust(&g.iter, env, ctx);
                        let iter_type = env.infer_expr_type(&g.iter);
                        if matches!(&iter_type, Type::List(_)) {
                            code.push_str(&format!("for {} in {}.iter().copied()", g.var, iter_code));
                        } else if matches!(&iter_type, Type::Str) {
                            code.push_str(&format!("for {} in {}.chars()", g.var, iter_code));
                        } else {
                            code.push_str(&format!("for {} in {}", g.var, strip_parens(&iter_code)));
                        }
                        code.push_str(" { ");
                        if let Some(cond) = &g.cond {
                            let cc = expr_to_rust(cond, env, ctx);
                            code.push_str(&format!("if {} {{ ", cc));
                        }
                    }
                    code.push_str("__comp.push(");
                    code.push_str(&elem);
                    code.push_str("); ");
                    for g in c.generators.iter().rev() {
                        if g.cond.is_some() { code.push_str(" } "); }
                        code.push_str(" } ");
                    }
                    code.push_str(" __comp }");
                    code
                }
                ComprehensionKind::Dict => {
                    let val = expr_to_rust(&c.element, env, ctx);
                    let key = c.key.as_ref().map(|k| expr_to_rust(k, env, ctx)).unwrap_or_default();
                    env.uses_hashmap = true;
                    let mut code = String::new();
                    code.push_str("{ let mut __comp = HashMap::new(); ");
                    for g in &c.generators {
                        let iter_code = expr_to_rust(&g.iter, env, ctx);
                        let iter_type = env.infer_expr_type(&g.iter);
                        if matches!(&iter_type, Type::List(_)) {
                            code.push_str(&format!("for {} in {}.iter().copied()", g.var, iter_code));
                        } else {
                            code.push_str(&format!("for {} in {}", g.var, strip_parens(&iter_code)));
                        }
                        code.push_str(" { ");
                        if let Some(cond) = &g.cond {
                            let cc = expr_to_rust(cond, env, ctx);
                            code.push_str(&format!("if {} {{ ", cc));
                        }
                        code.push_str(&format!("__comp.insert({}, {}); ", key, val));
                        if g.cond.is_some() { code.push_str(" } "); }
                        code.push_str(" } ");
                    }
                    code.push_str(" __comp }");
                    code
                }
            }
        }
        Expr::BinOp(left, op, right) => {
            let l = expr_to_rust(left, env, ctx);
            let r = expr_to_rust(right, env, ctx);
            match op {
                BinOp::Add => {
                    let lt = env.infer_expr_type(left);
                    let rt = env.infer_expr_type(right);
                    if lt == Type::Str || rt == Type::Str || lt == Type::Unknown || rt == Type::Unknown {
                        let l_use = if lt == Type::Str { l.clone() } else if lt != Type::Unknown { format!("{}.to_string()", l) } else { l.clone() };
                        let r_use = if rt == Type::Str { format!("&{}", r) } else if rt != Type::Unknown { format!("&{}.to_string()", r) } else { format!("&{}", r) };
                        format!("{} + {}", l_use, r_use)
                    } else { format!("{} + {}", l, r) }
                },
                BinOp::Sub => format!("{} - {}", l, r),
                BinOp::Mul => format!("{} * {}", l, r),
                BinOp::Div => format!("{} as f64 / {} as f64", l, r),
                BinOp::FloorDiv => format!("{} / {}", l, r),
                BinOp::Mod => format!("{} % {}", l, r),
                BinOp::Pow => format!("({} as i64).pow({} as u32)", l, r),
                BinOp::Eq => format!("{} == {}", l, r),
                BinOp::Ne => format!("{} != {}", l, r),
                BinOp::Lt => format!("{} < {}", l, r),
                BinOp::Gt => format!("{} > {}", l, r),
                BinOp::Le => format!("{} <= {}", l, r),
                BinOp::Ge => format!("{} >= {}", l, r),
                BinOp::And => format!("{} && {}", l, r),
                BinOp::Or => format!("{} || {}", l, r),
                BinOp::In => format!("{}.contains(&{})", r, l),
                BinOp::NotIn => format!("!{}.contains(&{})", r, l),
                BinOp::Is => format!("std::ptr::eq(&{}, &{})", l, r),
            }
        }
        Expr::UnaryOp(op, expr) => {
            let e = expr_to_rust(expr, env, ctx);
            match op { UnaryOp::Neg => format!("-{}", e), UnaryOp::Not => format!("!{}", e) }
        }
        Expr::FuncCall(name, args) => {
            let mut args_rust: Vec<String> = Vec::new();
            let mut expanded_args: Vec<String> = Vec::new();
            for a in args {
                match a {
                    Expr::Starred(inner) => {
                        let inner_code = expr_to_rust(inner, env, ctx);
                        expanded_args.push(inner_code);
                    }
                    _ => {
                        let code = expr_to_rust(a, env, ctx);
                        if expanded_args.is_empty() {
                            args_rust.push(code);
                        } else {
                            expanded_args.push(code);
                        }
                    }
                }
            }
            if !expanded_args.is_empty() {
                let joined = expanded_args.join(", ");
                args_rust = vec![joined];
            }

            match name.as_str() {
                "embed_rust" => {
                    return args.first().map(|a| match a {
                        Expr::StrLit(s) => s.clone(),
                        _ => args_rust[0].clone(),
                    }).unwrap_or_default();
                }
                "print" | "println" => {
                    let joined = args_rust.join(", ");
                    let first_type = args.first().map(|a| env.infer_expr_type(a));
                    let is_compound = matches!(first_type, Some(Type::Dict(_, _) | Type::List(_) | Type::None));
                    if is_compound { format!("println!(\"{{:?}}\", {})", joined) }
                    else { format!("println!(\"{{}}\", {})", joined) }
                }
                "args" => "std::env::args().collect::<Vec<String>>()".to_string(),
                "len" => format!("{}.len()", args_rust.join(", ")),
                "range" => {
                    let r = if args_rust.len() == 1 { format!("0..{}", args_rust[0]) }
                    else if args_rust.len() == 2 { format!("{}..{}", args_rust[0], args_rust[1]) }
                    else { format!("({}..{}).step_by({} as usize)", args_rust[0], args_rust[1], args_rust[2]) };
                    format!("({})", r)
                }
                "int" => format!("{}.parse::<i64>().unwrap_or(0)", args_rust[0]),
                "float" => format!("{}.parse::<f64>().unwrap_or(0.0)", args_rust[0]),
                "str" => format!("{}.to_string()", args_rust[0]),
                "input" => {
                    let mut s = String::from("{\n");
                    if !args_rust.is_empty() {
                        s.push_str(&format!("{}print!(\"{{}}\", {});\n", indent(ctx.depth + 1), args_rust[0]));
                    }
                    s.push_str(&format!("{}let mut __input = String::new();\n", indent(ctx.depth + 1)));
                    s.push_str(&format!("{}std::io::stdin().read_line(&mut __input).unwrap();\n", indent(ctx.depth + 1)));
                    s.push_str(&format!("{}{}", indent(ctx.depth + 1), "__input.trim().to_string()\n"));
                    s.push_str(&format!("{}}}", indent(ctx.depth)));
                    s
                }
                "enumerate" => {
                    let etype = args.first().map(|a| env.infer_expr_type(a));
                    if matches!(etype, Some(Type::List(_))) {
                        format!("{}.iter().enumerate()", args_rust[0])
                    } else {
                        format!("{}.enumerate()", args_rust[0])
                    }
                }
                "zip" => {
                    let pairs: Vec<String> = args_rust.iter().map(|a| format!("{}.into_iter()", a)).collect();
                    format!("{}.zip({})", pairs[0], pairs[1..].join(".zip()"))
                }
                "map" => format!("{}.into_iter().map({})", args_rust[1], args_rust[0]),
                "filter" => format!("{}.into_iter().filter({})", args_rust[1], args_rust[0]),
                "cinclude" => {
                    args.first().map(|a| match a {
                        Expr::StrLit(s) => {
                            let abs = if s.starts_with('/') { s.clone() } else {
                                format!("{}/{}", ctx.source_dir, s)
                            };
                            format!("include_str!(\"{}\")", abs)
                        }
                        _ => format!("include_str!({})", args_rust[0]),
                    }).unwrap_or_default()
                }
                "sleep" => format!("std::thread::sleep(std::time::Duration::from_secs_f64({}))", args_rust[0]),
                "randint" => {
                    format!("{{ let __lo = {}; let __hi = {}; if __lo > __hi {{ 0 }} else {{ __lo + (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as i64).abs() % (__hi - __lo + 1) }} }}", args_rust[0], args_rust.get(1).cloned().unwrap_or_else(|| "100".to_string()))
                }
                "randch" => {
                    let list = &args_rust[0];
                    format!("{{ let __list = &{}; if __list.is_empty() {{ panic!(\"empty list\") }} else {{ let __idx = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as usize) % __list.len(); __list[__idx].clone() }} }}", list)
                }
                "shuffle" => {
                    let list = &args_rust[0];
                    format!("{{ let __s = &mut {}; let __n = __s.len(); for __i in (1..__n).rev() {{ let __j = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as usize) % (__i + 1); __s.swap(__i, __j); }} }}", list)
                }
                "sum" => {
                    let elem_type = args.first().map(|a| env.infer_expr_type(a));
                    let is_f64 = matches!(elem_type, Some(Type::List(t)) if *t == Type::F64);
                    let t = if is_f64 { "f64" } else { "i64" };
                    format!("{}.iter().sum::<{}>()", args_rust[0], t)
                }
                "avg" => {
                    let elem_type = args.first().map(|a| env.infer_expr_type(a));
                    let is_f64 = matches!(elem_type, Some(Type::List(t)) if *t == Type::F64);
                    if is_f64 {
                        format!("{}.iter().sum::<f64>() / {}.len() as f64", args_rust[0], args_rust[0])
                    } else {
                        format!("{}.iter().sum::<i64>() as f64 / {}.len() as f64", args_rust[0], args_rust[0])
                    }
                }
                "min_val" => {
                    if args.len() == 1 {
                        let elem_type = args.first().map(|a| env.infer_expr_type(a));
                        let is_f64 = matches!(elem_type, Some(Type::List(t)) if *t == Type::F64);
                        if is_f64 {
                            format!("{}.iter().cloned().fold(f64::INFINITY, f64::min)", args_rust[0])
                        } else {
                            format!("{}.iter().cloned().min().unwrap_or(i64::MAX)", args_rust[0])
                        }
                    } else {
                        format!("{}.min({})", args_rust[0], args_rust[1])
                    }
                }
                "max_val" => {
                    if args.len() == 1 {
                        let elem_type = args.first().map(|a| env.infer_expr_type(a));
                        let is_f64 = matches!(elem_type, Some(Type::List(t)) if *t == Type::F64);
                        if is_f64 {
                            format!("{}.iter().cloned().fold(f64::NEG_INFINITY, f64::max)", args_rust[0])
                        } else {
                            format!("{}.iter().cloned().max().unwrap_or(i64::MIN)", args_rust[0])
                        }
                    } else {
                        format!("{}.max({})", args_rust[0], args_rust[1])
                    }
                }
                "clamp" => format!("({}).max({}).min({})", args_rust[0], args_rust[1], args_rust[2]),
                "read_file" => format!("std::fs::read_to_string({}).unwrap_or_default()", args_rust[0]),
                "write_file" => format!("std::fs::write({}, {}).unwrap()", args_rust[0], args_rust[1]),
                "exists" => format!("std::path::Path::new(&{}).exists()", args_rust[0]),
                "abs" => format!("({} as i64).abs()", args_rust[0]),
                "round" => format!("({} as f64).round() as i64", args_rust[0]),
                 "type_of" => "\"<value>\"".to_string(),
                "today" => {
                    r#"{ let __days = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() / 86400; let __z = __days + 719468; let __era = __z / 146097; let __doe = __z - __era * 146097; let __yoe = (__doe - __doe / 1460 + __doe / 36524 - __doe / 146096) / 365; let __y = __yoe + __era * 400; let __d = __doe - (365 * __yoe + __yoe / 4 - __yoe / 100); let __mp = (5 * __d + 2) / 153; let __day = __d - (__mp * 153 + 2) / 5 + 1; let __month = __mp + 3 - if __mp >= 10 { 12 } else { 0 }; let __year = __y + if __mp < 10 { 0 } else { 1 }; format!("{:04}-{:02}-{:02}", __year, __month, __day) }"#.to_string()
                }
                "now" => {
                    r#"{ let __s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() % 86400; let __h = __s / 3600; let __m = (__s / 60) % 60; let __sec = __s % 60; format!("{:02}:{:02}:{:02}", __h, __m, __sec) }"#.to_string()
                }
                "clear" => format!("print!(\"\\x1B[2J\\x1B[1;1H\")"),
                "__len__" => format!("{}.len()", args_rust[0]),
                "__del__" => format!("{}.clear()", args_rust[0]),
                "__method_call__" => format!("{}({})", args_rust[0], args_rust[1]),
                "__call__" => format!("{}({})", args_rust[0], args_rust[1]),
                _ => {
                    if ctx.class_names.contains(name) {
                        format!("{}::new({})", name, args_rust.join(", "))
                    } else {
                        format!("{}({})", name, args_rust.join(", "))
                    }
                }
            }
        }
        Expr::MethodCall(obj, method, args) => {
            let obj_s = expr_to_rust(obj, env, ctx);
            let args_s: Vec<String> = args.iter().map(|a| expr_to_rust(a, env, ctx)).collect();
            let sep = if ctx.module_names.contains(&obj_s) || ctx.processed_modules.contains(&obj_s) { "::" } else { "." };
            match method.as_str() {
                "append" => format!("{}.push({})", obj_s, args_s.join(", ")),
                "pop" => format!("{}.pop()", obj_s),
                "remove" => format!("{}.retain(|x| x != {})", obj_s, args_s.join(", ")),
                "insert" => format!("{}.insert({})", obj_s, args_s.join(", ")),
                "sort" => format!("{}.sort()", obj_s),
                "reverse" => format!("{}.reverse()", obj_s),
                "clear" => format!("{}.clear()", obj_s),
                "copy" => format!("{}.clone()", obj_s),
                "upper" => format!("{}.to_uppercase()", obj_s),
                "lower" => format!("{}.to_lowercase()", obj_s),
                "strip" => format!("{}.trim().to_string()", obj_s),
                "lstrip" => format!("{}.trim_start().to_string()", obj_s),
                "rstrip" => format!("{}.trim_end().to_string()", obj_s),
                "capitalize" => format!("{}.chars().next().map_or(String::new(), |c| c.to_uppercase().to_string() + &{}[1..].to_lowercase())", obj_s, obj_s),
                "title" => format!("{}.split(' ').map(|w| {{ let mut c = w.chars(); match c.next() {{ None => String::new(), Some(f) => f.to_uppercase().to_string() + c.as_str() }} }}).collect::<Vec<_>>().join(\" \")", obj_s),
                "swapcase" => format!("{}.chars().map(|c| if c.is_uppercase() {{ c.to_lowercase().to_string() }} else {{ c.to_uppercase().to_string() }}).collect::<Vec<_>>().join(\"\")", obj_s),
                "split" => {
                    if args_s.is_empty() { format!("{}.split_whitespace().map(|s| s.to_string()).collect::<Vec<_>>()", obj_s) }
                    else { format!("{}.split(&{}).map(|s| s.to_string()).collect::<Vec<_>>()", obj_s, args_s[0]) }
                }
                "rsplit" => {
                    if args_s.is_empty() { format!("{}.split_whitespace().rev().map(|s| s.to_string()).collect::<Vec<_>>()", obj_s) }
                    else { format!("{}.rsplit(&{}).map(|s| s.to_string()).collect::<Vec<_>>()", obj_s, args_s[0]) }
                }
                "splitlines" => format!("{}.lines().map(|s| s.to_string()).collect::<Vec<_>>()", obj_s),
                "join" => {
                    if args.is_empty() { format!("{}.join(\"\")", obj_s) }
                    else { format!("{}.join(&{})", args_s[0], obj_s) }
                }
                "replace" => {
                    if args_s.len() >= 2 { format!("{}.replace(&{}, &{})", obj_s, args_s[0], args_s[1]) }
                    else { format!("{}.to_string()", obj_s) }
                }
                "startswith" => format!("{}.starts_with(&{})", obj_s, args_s[0]),
                "endswith" => format!("{}.ends_with(&{})", obj_s, args_s[0]),
                "isalpha" => format!("{}.chars().all(|c| c.is_alphabetic())", obj_s),
                "isdigit" => format!("{}.chars().all(|c| c.is_ascii_digit())", obj_s),
                "isalnum" => format!("{}.chars().all(|c| c.is_alphanumeric())", obj_s),
                "isspace" => format!("{}.chars().all(|c| c.is_whitespace())", obj_s),
                "islower" => format!("{}.chars().any(|c| c.is_lowercase()) && !{}.chars().any(|c| c.is_uppercase())", obj_s, obj_s),
                "isupper" => format!("{}.chars().any(|c| c.is_uppercase()) && !{}.chars().any(|c| c.is_lowercase())", obj_s, obj_s),
                "find" | "index" => format!("{}.find(&{}).map_or(-1i64, |i| i as i64)", obj_s, args_s[0]),
                "rfind" | "rindex" => format!("{}.rfind(&{}).map_or(-1i64, |i| i as i64)", obj_s, args_s[0]),
                "count" => format!("{}.matches(&{}).count() as i64", obj_s, args_s[0]),
                "get" => {
                    if args_s.len() >= 2 { format!("{}.get(&{}).cloned().unwrap_or_else(|| {})", obj_s, args_s[0], args_s[1]) }
                    else { format!("{}.get(&{}).cloned()", obj_s, args_s[0]) }
                }
                "keys" => format!("{}.keys().cloned().collect::<Vec<_>>()", obj_s),
                "values" => format!("{}.values().cloned().collect::<Vec<_>>()", obj_s),
                "items" => format!("{}.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>()", obj_s),
                "popitem" => format!("{}.drain().next().unwrap()", obj_s),
                _ => format!("{}{}{}({})", obj_s, sep, method, args_s.join(", ")),
            }
        }
        Expr::Attribute(obj, attr) => {
            let obj_s = expr_to_rust(obj, env, ctx);
            let sep = if ctx.module_names.contains(&obj_s) || ctx.processed_modules.contains(&obj_s) { "::" } else { "." };
            format!("{}{}{}", obj_s, sep, attr)
        }
        Expr::Subscript(obj, index) => {
            let obj_s = expr_to_rust(obj, env, ctx);
            let obj_type = env.infer_expr_type(obj);
            let idx_type = env.infer_expr_type(index);
            if matches!(index.as_ref(), Expr::Slice(_, _, _)) {
                let start_s = match index.as_ref() { Expr::Slice(s, _, _) => s.as_ref(), _ => None };
                let end_s = match index.as_ref() { Expr::Slice(_, e, _) => e.as_ref(), _ => None };
                let step_s = match index.as_ref() { Expr::Slice(_, _, s) => s.as_ref(), _ => None };
                let skip = start_s.map(|s| format!(".skip({} as usize)", expr_to_rust(s, env, ctx))).unwrap_or_default();
                let take = end_s.map(|s| format!(".take({} as usize)", expr_to_rust(s, env, ctx))).unwrap_or_default();
                let step = step_s.map(|s| format!(".step_by({} as usize)", expr_to_rust(s, env, ctx))).unwrap_or_default();
                format!("{{ let v: String = {}.chars(){}{}{}.collect(); v }}", obj_s, skip, take, step)
            } else if let Expr::StrLit(s) = index.as_ref() {
                let escaped = s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\t', "\\t").replace('\r', "\\r");
                format!("{}[\"{}\"]", obj_s, escaped)
            } else if matches!(obj_type, Type::Dict(_, _)) {
                let raw = expr_to_rust(index, env, ctx);
                format!("{}[&{}]", obj_s, raw)
            } else if idx_type == Type::I64 {
                let raw = expr_to_rust(index, env, ctx);
                format!("{{ let __v = &{}; let __i = {} as i64; if __i < 0 {{ __v[__v.len() - ((-__i) as usize)].clone() }} else {{ __v[__i as usize].clone() }} }}", obj_s, raw)
            } else {
                let raw = expr_to_rust(index, env, ctx);
                format!("{}[{}]", obj_s, raw)
            }
        }
        Expr::Slice(_, _, _) => {
            String::new()
        }
        Expr::Dict(pairs) => {
            env.uses_hashmap = true;
            let mut entries = Vec::new();
            for (k, v) in pairs {
                let k_s = expr_to_rust(k, env, ctx);
                let v_s = expr_to_rust(v, env, ctx);
                entries.push(format!("({}, {})", k_s, v_s));
            }
            format!("HashMap::from([{}])", entries.join(", "))
        }
        Expr::List(items) => {
            if items.is_empty() { "Vec::new()".to_string() }
            else {
                let elems: Vec<String> = items.iter().map(|e| expr_to_rust(e, env, ctx)).collect();
                format!("vec![{}]", elems.join(", "))
            }
        }
        Expr::Lambda(params, body) => {
            let body_s = expr_to_rust(body, env, ctx);
            format!("|{}| {}", params.join(", "), body_s)
        }
        Expr::FStr(parts) => {
            let mut fmt_str = String::new();
            let mut fmt_args = Vec::new();
            for p in parts {
                match p {
                    Expr::StrLit(s) => fmt_str.push_str(&s.replace('{', "{{").replace('}', "}}")),
                    other => {
                        fmt_str.push_str("{}");
                        fmt_args.push(expr_to_rust(other, env, ctx));
                    }
                }
            }
            if fmt_args.is_empty() {
                format!("\"{}\".to_string()", fmt_str)
            } else {
                format!("format!(\"{}\", {})", fmt_str, fmt_args.join(", "))
            }
        }
    }
}

fn is_copy_type(t: &Type) -> bool {
    matches!(t, Type::I64 | Type::F64 | Type::Bool)
}

fn guess_type_from_name(name: &str) -> Type {
    let lower = name.to_lowercase();
    let string_names = ["name", "text", "msg", "message", "s", "str", "title", "label", "prompt", "repo", "path", "dir", "url", "pkg", "cmd", "command"];
    if string_names.contains(&lower.as_str()) || lower.contains("dir") || lower.contains("path") || lower.contains("libs") { Type::Str }
    else { Type::I64 }
}

fn scan_param_type(name: &str, stmts: &[Stmt], env: &TypeEnv) -> Type {
    for stmt in stmts {
        match stmt {
            Stmt::Expr(e) | Stmt::Return(Some(e)) => {
                if let Some(t) = scan_expr_for_param(name, e, env) { return t; }
            }
            Stmt::Assign(_, value, _) | Stmt::AugAssign(_, _, value) => {
                if let Some(t) = scan_expr_for_param(name, value, env) { return t; }
            }
            Stmt::AssignTuple(_, value) => {
                if let Some(t) = scan_expr_for_param(name, value, env) { return t; }
            }
            Stmt::If(branches, else_body) => {
                for (cond, body) in branches {
                    if let Some(t) = scan_expr_for_param(name, cond, env) { return t; }
                    let t = scan_param_type(name, body, env);
                    if t != Type::I64 { return t; }
                }
                if let Some(body) = else_body {
                    let t = scan_param_type(name, body, env);
                    if t != Type::I64 { return t; }
                }
            }
            Stmt::While(cond, body) | Stmt::For(_, cond, body) => {
                if let Some(t) = scan_expr_for_param(name, cond, env) { return t; }
                let t = scan_param_type(name, body, env);
                if t != Type::I64 { return t; }
            }
            Stmt::FuncDef { body, .. } => {
                let t = scan_param_type(name, body, env);
                if t != Type::I64 { return t; }
            }
            Stmt::With { expr, body, .. } => {
                if let Some(t) = scan_expr_for_param(name, expr, env) { return t; }
                let t = scan_param_type(name, body, env);
                if t != Type::I64 { return t; }
            }
            Stmt::Try { body, handlers, else_body, finally_body } => {
                let t = scan_param_type(name, body, env);
                if t != Type::I64 { return t; }
                for h in handlers {
                    let t = scan_param_type(name, &h.body, env);
                    if t != Type::I64 { return t; }
                }
                if let Some(b) = else_body {
                    let t = scan_param_type(name, b, env);
                    if t != Type::I64 { return t; }
                }
                if let Some(b) = finally_body {
                    let t = scan_param_type(name, b, env);
                    if t != Type::I64 { return t; }
                }
            }
            _ => {}
        }
    }
    guess_type_from_name(name)
}

fn scan_expr_for_param(name: &str, expr: &Expr, env: &TypeEnv) -> Option<Type> {
    match expr {
        Expr::BinOp(left, op, right) => {
            match op {
                BinOp::Add => {
                    let l_is_str = matches!(**left, Expr::StrLit(_));
                    let r_is_str = matches!(**right, Expr::StrLit(_));
                    if l_is_str || r_is_str {
                        if let Expr::Ident(n) = &**left { if n == name { return Some(Type::Str); } }
                        if let Expr::Ident(n) = &**right { if n == name { return Some(Type::Str); } }
                        if let Expr::Attribute(obj, attr) = &**left {
                            if let Expr::Ident(n) = &**obj { if n == "self" && attr == name { return Some(Type::Str); } }
                        }
                        if let Expr::Attribute(obj, attr) = &**right {
                            if let Expr::Ident(n) = &**obj { if n == "self" && attr == name { return Some(Type::Str); } }
                        }
                    }
                    if let Some(t) = scan_expr_for_param(name, left, env) { return Some(t); }
                    if let Some(t) = scan_expr_for_param(name, right, env) { return Some(t); }
                }
                _ => {
                    if let Some(t) = scan_expr_for_param(name, left, env) { return Some(t); }
                    if let Some(t) = scan_expr_for_param(name, right, env) { return Some(t); }
                }
            }
            None
        }
        Expr::UnaryOp(_, e) => scan_expr_for_param(name, e, env),
        Expr::Ident(n) if n == name => None,
        Expr::Attribute(obj, attr) => {
            if let Expr::Ident(self_name) = &**obj {
                if self_name == "self" && attr == name { return None; }
            }
            None
        }
        Expr::Subscript(obj, _) => scan_expr_for_param(name, obj, env),
        Expr::MethodCall(obj, method, _) => {
            if matches!(method.as_str(), "upper" | "lower" | "strip" | "replace" | "split" | "join" | "startswith" | "endswith") {
                if let Expr::Ident(n) = obj.as_ref() {
                    if n == name { return Some(Type::Str); }
                }
            }
            None
        }
        Expr::FuncCall(_, args) => {
            for arg in args {
                if let Some(t) = scan_expr_for_param(name, arg, env) { return Some(t); }
            }
            None
        }
        Expr::FStr(parts) => {
            for p in parts {
                if let Expr::Ident(n) = p {
                    if n == name { return Some(Type::Str); }
                }
            }
            None
        }
        _ => None,
    }
}

fn infer_return_type(stmts: &[Stmt], env: &mut TypeEnv) -> Type {
    for stmt in stmts {
        match stmt {
            Stmt::Assign(name, value, _) => {
                let t = env.infer_expr_type(value);
                env.vars.insert(name.clone(), t);
            }
            Stmt::Return(Some(expr)) => {
                let t = env.infer_expr_type(expr);
                if t != Type::Unit && t != Type::None && t != Type::Unknown { return t; }
            }
            Stmt::If(branches, else_body) => {
                for (_, body) in branches {
                    let t = infer_return_type(body, env);
                    if t != Type::Unit && t != Type::None && t != Type::Unknown { return t; }
                }
                if let Some(body) = else_body {
                    let t = infer_return_type(body, env);
                    if t != Type::Unit && t != Type::None && t != Type::Unknown { return t; }
                }
            }
            Stmt::For(vars, iterable, body) => {
                let iter_type = env.infer_expr_type(iterable);
                let elem_type = match iter_type {
                    Type::List(t) => *t,
                    Type::Str => Type::Str,
                    _ => Type::Unknown,
                };
                for var in vars { env.vars.insert(var.clone(), elem_type.clone()); }
                let t = infer_return_type(body, env);
                if t != Type::Unit && t != Type::None && t != Type::Unknown { return t; }
            }
            Stmt::While(_, body) => {
                let t = infer_return_type(body, env);
                if t != Type::Unit && t != Type::None && t != Type::Unknown { return t; }
            }
            _ => {}
        }
    }
    Type::Unit
}

struct CodegenCtx {
    depth: usize,
    current_fn: Option<String>,
    current_class: Option<String>,
    class_names: HashSet<String>,
    needs_main: bool,
    declared: HashSet<String>,
    source_dir: String,
    has_raise: bool,
    in_try: bool,
    decorator_fn_types: HashMap<String, Type>,
    processed_modules: HashSet<String>,
    module_code: String,
    module_names: HashSet<String>,
}

fn stmt_to_rust(stmt: &Stmt, env: &mut TypeEnv, ctx: &mut CodegenCtx, out: &mut String) {
    match stmt {
        Stmt::Expr(e) => {
            let code = expr_to_rust(e, env, ctx);
            let is_embed = matches!(e, Expr::FuncCall(name, _) if name == "embed_rust");
            let is_result = ctx.in_try && matches!(e, Expr::FuncCall(name, _) if env.fn_return.get(name).map_or(false, |t| matches!(t, Type::Result(_))));
            if is_embed {
                out.push_str(&format!("{}{}\n", indent(ctx.depth), code));
            } else if is_result {
                out.push_str(&format!("{}{}?;\n", indent(ctx.depth), code));
            } else {
                out.push_str(&format!("{}{};\n", indent(ctx.depth), code));
            }
        }
        Stmt::Assign(name, value, _is_mut) => {
            if name.starts_with("__setitem__(") {
                let inner = name.trim_start_matches("__setitem__(").trim_end_matches(")");
                if let Expr::FuncCall(_, args) = value {
                    if args.len() == 2 {
                        let is_dict = matches!(env.vars.get(inner), Some(Type::Dict(_, _)));
                        let index_expr = if is_dict {
                            expr_to_rust(&args[0], env, ctx)
                        } else if let Expr::StrLit(s) = &args[0] {
                            let escaped = s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\t', "\\t").replace('\r', "\\r");
                            format!("\"{}\"", escaped)
                        } else {
                            format!("&{}", expr_to_rust(&args[0], env, ctx))
                        };
                        let val_expr = expr_to_rust(&args[1], env, ctx);
                        if is_dict {
                            out.push_str(&format!("{} {}.insert({}, {});\n", indent(ctx.depth), inner, index_expr, val_expr));
                        } else {
                            out.push_str(&format!("{} {}[{}] = {};\n", indent(ctx.depth), inner, index_expr, val_expr));
                        }
                        return;
                    }
                }
            }

            let val_code = expr_to_rust(value, env, ctx);
            let val_type = env.infer_expr_type(value);

            if name.starts_with("self.") {
                let field = name.replacen("self.", "", 1);
                env.vars.insert(field, val_type);
                let in_init = ctx.current_fn.as_deref() == Some("__init__");
                if !in_init {
                    out.push_str(&format!("{}{} = {};\n", indent(ctx.depth), name, val_code));
                }
            } else if ctx.declared.contains(name) {
                out.push_str(&format!("{}{} = {};\n", indent(ctx.depth), name, val_code));
            } else {
                if env.mutated.contains(name) {
                    out.push_str(&format!("{}let mut {} = {};\n", indent(ctx.depth), name, val_code));
                } else {
                    out.push_str(&format!("{}let {} = {};\n", indent(ctx.depth), name, val_code));
                }
                ctx.declared.insert(name.clone());
                env.vars.insert(name.clone(), val_type);
            }
        }
        Stmt::AssignTuple(names, value) => {
            let val_code = expr_to_rust(value, env, ctx);
            let tmp_var = "__tmp_tuple".to_string();

            let tmp_exists = ctx.declared.contains(&tmp_var);
            if !tmp_exists {
                let all_mut = names.iter().any(|n| env.mutated.contains(n));
                if all_mut {
                    out.push_str(&format!("{}let mut {} = {};\n", indent(ctx.depth), tmp_var, val_code));
                } else {
                    out.push_str(&format!("{}let {} = {};\n", indent(ctx.depth), tmp_var, val_code));
                }
                ctx.declared.insert(tmp_var.clone());
            } else {
                out.push_str(&format!("{}{} = {};\n", indent(ctx.depth), tmp_var, val_code));
            }

            for (i, name) in names.iter().enumerate() {
                let exists = ctx.declared.contains(name);
                let element = format!("{}.{}", tmp_var, i);
                if exists {
                    out.push_str(&format!("{}{} = {};\n", indent(ctx.depth), name, element));
                } else if env.mutated.contains(name) {
                    out.push_str(&format!("{}let mut {} = {};\n", indent(ctx.depth), name, element));
                    ctx.declared.insert(name.clone());
                    env.vars.insert(name.clone(), Type::Unknown);
                } else {
                    out.push_str(&format!("{}let {} = {};\n", indent(ctx.depth), name, element));
                    ctx.declared.insert(name.clone());
                    env.vars.insert(name.clone(), Type::Unknown);
                }
            }
        }
        Stmt::AugAssign(name, op, value) => {
            let val_code = expr_to_rust(value, env, ctx);
            let op_str = match op {
                BinOp::Add => "+=", BinOp::Sub => "-=", BinOp::Mul => "*=",
                BinOp::Div => "/=", BinOp::Mod => "%=", _ => "+=",
            };
            let mapped_name = if let Some(ref _cn) = ctx.current_class {
                if name.starts_with("self.") { name.replacen("self.", "", 1) } else { name.clone() }
            } else { name.clone() };
            out.push_str(&format!("{}{} {} {};\n", indent(ctx.depth), mapped_name, op_str, val_code));
        }
        Stmt::If(branches, else_body) => {
            for (i, (cond, body)) in branches.iter().enumerate() {
                let cond_code = expr_to_rust(cond, env, ctx);
                let prefix = if i == 0 { "if" } else { "} else if" };
                out.push_str(&format!("{}{} {} {{\n", indent(ctx.depth), prefix, cond_code));
                let old_depth = ctx.depth;
                ctx.depth += 1;
                for s in body { stmt_to_rust(s, env, ctx, out); }
                ctx.depth = old_depth;
            }
            if let Some(else_body) = else_body {
                out.push_str(&format!("{}}} else {{\n", indent(ctx.depth)));
                let old_depth = ctx.depth;
                ctx.depth += 1;
                for s in else_body { stmt_to_rust(s, env, ctx, out); }
                ctx.depth = old_depth;
            }
            out.push_str(&format!("{}}}\n", indent(ctx.depth)));
        }
        Stmt::While(cond, body) => {
            let cond_code = expr_to_rust(cond, env, ctx);
            out.push_str(&format!("{}while {} {{\n", indent(ctx.depth), cond_code));
            let old_depth = ctx.depth;
            ctx.depth += 1;
            for s in body { stmt_to_rust(s, env, ctx, out); }
            ctx.depth = old_depth;
            out.push_str(&format!("{}}}\n", indent(ctx.depth)));
        }
        Stmt::For(vars, iterable, body) => {
            let iter_code = expr_to_rust(iterable, env, ctx);
            let iter_type = env.infer_expr_type(iterable);
            let target = if vars.len() == 1 {
                vars[0].clone()
            } else {
                format!("({})", vars.join(", "))
            };

            if let Type::List(inner) = &iter_type {
                let by = if is_copy_type(inner) { "copied()" } else { "cloned()" };
                out.push_str(&format!("{}for {} in {}.iter().{} {{\n", indent(ctx.depth), target, iter_code, by));
            } else if matches!(&iter_type, Type::Str) {
                out.push_str(&format!("{}for {} in {}.chars() {{\n", indent(ctx.depth), target, iter_code));
            } else {
                out.push_str(&format!("{}for {} in {} {{\n", indent(ctx.depth), target, strip_parens(&iter_code)));
            }
            let old_depth = ctx.depth;
            ctx.depth += 1;
            for s in body { stmt_to_rust(s, env, ctx, out); }
            ctx.depth = old_depth;
            out.push_str(&format!("{}}}\n", indent(ctx.depth)));
        }
        Stmt::FuncDef { name, params, body, return_type: _, decorators } => {
            let old_fn = ctx.current_fn.clone();
            ctx.current_fn = Some(name.clone());

            let param_types: Vec<(String, Type)> = params.iter().enumerate().map(|(i, (n, _))| {
                if n.starts_with("*") {
                    (n.clone(), Type::List(Box::new(Type::I64)))
                } else if i == 0 && ctx.decorator_fn_types.contains_key(name) {
                    let t = ctx.decorator_fn_types.get(name).cloned().unwrap();
                    (n.clone(), t)
                } else {
                    let existing = env.vars.get(n).cloned();
                    let mut t = existing.unwrap_or_else(|| scan_param_type(n, body, env));
                    if matches!(t, Type::Unknown | Type::Any) { t = guess_type_from_name(n); }
                    if t == Type::Unknown { t = Type::Str; }
                    (n.clone(), t)
                }
            }).collect();

            for (pn, pt) in &param_types {
                env.vars.insert(pn.clone(), pt.clone());
            }

            env.fn_params.insert(name.clone(), param_types.clone());

            let has_raise = body_contains_raise(body);
            let inferred_ret = infer_return_type(body, env);
            let ret_type = if has_raise {
                Type::Result(Box::new(inferred_ret.clone()))
            } else if inferred_ret != Type::Unit && inferred_ret != Type::Unknown {
                inferred_ret.clone()
            } else {
                Type::Unit
            };
            env.fn_return.insert(name.clone(), ret_type.clone());

            let params_rust: Vec<String> = param_types.iter().map(|(n, t)| {
                if n.starts_with("*") {
                    let bare = n.trim_start_matches('*');
                    format!("{}: Vec<{}>", bare, "i64")
                } else {
                    format!("{}: {}", n, t)
                }
            }).collect();

            let return_type_str = ret_type.to_string();
            let old_fn_vars = env.vars.clone();
            let old_fn_declared = ctx.declared.clone();
            ctx.declared.clear();
            for (pn, _) in &param_types {
                ctx.declared.insert(pn.clone());
            }

            if !decorators.is_empty() {
                let inner_name = format!("{}_impl", name);
                out.push_str(&format!("{}pub fn {}({}) -> {} {{\n", indent(ctx.depth), inner_name, params_rust.join(", "), return_type_str));
                let dec_depth = ctx.depth;
                ctx.depth += 1;
                if has_raise { ctx.has_raise = true; }
                for s in body { stmt_to_rust(s, env, ctx, out); }
                if has_raise { ctx.has_raise = false; }
                ctx.depth = dec_depth;
                out.push_str(&format!("{}}}\n\n", indent(ctx.depth)));
                let decorator = &decorators[0];
                let inner_params: Vec<String> = params.iter().map(|(n, _)| {
                    if n.starts_with("*") {
                        format!("{}", n.trim_start_matches('*'))
                    } else {
                        n.clone()
                    }
                }).collect();
                let wrap_depth = ctx.depth;
                out.push_str(&format!("{}pub fn {}({}) -> {} {{\n", indent(ctx.depth), name, params_rust.join(", "), return_type_str));
                ctx.depth += 1;
                out.push_str(&format!("{}{}({});\n{} {}({})\n", indent(ctx.depth), decorator, inner_name, indent(ctx.depth), inner_name, inner_params.join(", ")));
                ctx.depth = wrap_depth;
                out.push_str(&format!("{}}}\n\n", indent(ctx.depth)));
            } else {
                out.push_str(&format!("{}pub fn {}({}) -> {} {{\n", indent(ctx.depth), name, params_rust.join(", "), return_type_str));
                let old_depth = ctx.depth;
                ctx.depth += 1;
                if has_raise { ctx.has_raise = true; }
                for s in body { stmt_to_rust(s, env, ctx, out); }
                if has_raise { ctx.has_raise = false; }
                ctx.depth = old_depth;
                out.push_str(&format!("{}}}\n\n", indent(ctx.depth)));
            }

            ctx.declared = old_fn_declared;
            env.vars = old_fn_vars;
            ctx.current_fn = old_fn;
        }
        Stmt::ClassDef { name, bases: _, body, decorators } => {
            let old_class = ctx.current_class.clone();
            ctx.current_class = Some(name.clone());

            let mut fields: Vec<(String, Type)> = Vec::new();
            let mut methods = Vec::new();

            let mut field_names = HashSet::new();
            for stmt in body {
                if let Stmt::FuncDef { name: mname, params, body: mbody, .. } = stmt {
                    if mname == "__init__" {
                        for p in params {
                            if p.0 != "self" && !p.0.starts_with("*") {
                                let ft = env.vars.get(&p.0).cloned()
                                    .unwrap_or_else(|| scan_param_type(&p.0, mbody, env));
                                if field_names.insert(p.0.clone()) {
                                    fields.push((p.0.clone(), ft));
                                }
                            }
                        }
                        for s in mbody {
                            if let Stmt::Assign(name, value, _) = s {
                                if name.starts_with("self.") {
                                    let field_name = name.replacen("self.", "", 1);
                                    if field_names.insert(field_name.clone()) {
                                        let ft = env.infer_expr_type(value);
                                        fields.push((field_name.clone(), ft));
                                    }
                                }
                            }
                        }
                    }
                    methods.push(stmt.clone());
                }
            }

            for d in decorators.iter().rev() {
                out.push_str(&format!("{}let {} = {}({});\n", indent(ctx.depth), name, d, name));
            }

            out.push_str(&format!("{}pub struct {} {{\n", indent(ctx.depth), name));
            for (f, ft) in &fields {
                out.push_str(&format!("{}    pub {}: {},\n", indent(ctx.depth), f, ft));
            }
            if fields.is_empty() {
                out.push_str(&format!("{}    pub _dummy: (),\n", indent(ctx.depth)));
            }
            out.push_str(&format!("{}}}\n\n", indent(ctx.depth)));

            out.push_str(&format!("{}impl {} {{\n", indent(ctx.depth), name));
            ctx.depth += 1;

            for method in &methods {
                if let Stmt::FuncDef { name: mname, params, body: mbody, return_type: _, decorators: _ } = method {
                    let mutates_self = mbody.iter().any(|s| matches!(s, Stmt::Assign(name, _, _) if name.starts_with("self.")));
                    let mparams_rust: Vec<String> = params.iter().map(|(n, _)| {
                        if n == "self" {
                            if mname == "__init__" { "".to_string() }
                            else if mutates_self { "&mut self".to_string() }
                            else { "&self".to_string() }
                        } else {
                            let existing = env.vars.get(n).cloned();
                            let t = existing.unwrap_or_else(|| scan_param_type(n, mbody, env));
                            format!("{}: {}", n, t)
                        }
                    }).collect();

                    let inferred_mret = infer_return_type(mbody, env);
                    if inferred_mret != Type::Unit && inferred_mret != Type::Unknown {
                        env.fn_return.insert(mname.clone(), inferred_mret);
                    }
                    let mret_str = env.fn_return.get(mname).map(|t| t.to_string()).unwrap_or_else(|| "()".to_string());

                    if mname == "__init__" {
                        out.push_str(&format!("{}pub fn new({}) -> Self {{\n", indent(ctx.depth), mparams_rust[1..].join(", ")));
                    } else {
                        out.push_str(&format!("{}pub fn {}({}) -> {} {{\n", indent(ctx.depth), mname, mparams_rust.join(", "), mret_str));
                    }

                    let old_fn = ctx.current_fn.clone();
                    ctx.current_fn = Some(mname.clone());

                    for (pn, _) in params {
                        if pn != "self" {
                            let pt = env.vars.get(pn).cloned().unwrap_or(Type::I64);
                            env.vars.insert(pn.clone(), pt);
                        }
                    }

                    let old_depth = ctx.depth;
                    ctx.depth += 1;
                    let mut field_inits: Vec<String> = Vec::new();
                    if mname == "__init__" {
                        for s in mbody {
                            if let Stmt::Assign(name, value, _) = s {
                                if name.starts_with("self.") {
                                    let init_expr = expr_to_rust(value, env, ctx);
                                    let field_name = name.replacen("self.", "", 1);
                                    field_inits.push(format!("{}{}: {},\n", indent(ctx.depth), field_name, init_expr));
                                    continue;
                                }
                            }
                            stmt_to_rust(s, env, ctx, out);
                        }
                    } else {
                        for s in mbody { stmt_to_rust(s, env, ctx, out); }
                    }
                    ctx.current_fn = old_fn;

                    if mname == "__init__" {
                        out.push_str(&format!("{}Self {{\n", indent(ctx.depth)));
                        for fi in &field_inits { out.push_str(fi); }
                        if field_inits.is_empty() {
                            out.push_str(&format!("{}_dummy: (),\n", indent(ctx.depth)));
                        }
                        out.push_str(&format!("{}}}\n", indent(ctx.depth)));
                    }

                    ctx.depth = old_depth;
                    out.push_str(&format!("{}}}\n\n", indent(ctx.depth)));
                }
            }

            ctx.depth -= 1;
            out.push_str(&format!("{}}}\n\n", indent(ctx.depth)));

            ctx.current_class = old_class;
        }
        Stmt::Return(expr) => {
            if let Some(expr) = expr {
                let code = expr_to_rust(expr, env, ctx);
                let ret_type = env.infer_expr_type(expr);
                if let Some(ref fn_name) = ctx.current_fn {
                    env.fn_return.insert(fn_name.clone(), ret_type);
                }
                if ctx.has_raise {
                    out.push_str(&format!("{}return Ok({});\n", indent(ctx.depth), code));
                } else {
                    out.push_str(&format!("{}return {};\n", indent(ctx.depth), code));
                }
            } else {
                out.push_str(&format!("{}return;\n", indent(ctx.depth)));
            }
        }
        Stmt::Break => { out.push_str(&format!("{}break;\n", indent(ctx.depth))); }
        Stmt::Continue => { out.push_str(&format!("{}continue;\n", indent(ctx.depth))); }
        Stmt::Pass => {}
        Stmt::Import(module, alias) => {
            let mut is_molotov = false;
            let internal_mod_name = module.replace("::", "_");
            if !ctx.processed_modules.contains(module) {
                if let Some(path) = resolve_module(module, &ctx.source_dir) {
                    ctx.processed_modules.insert(module.clone());
                    if let Ok(source) = fs::read_to_string(&path) {
                        if let Ok(tokens) = tokenize(&source) {
                            let mut parser = Parser::new(tokens);
                            if let Ok(program) = parser.parse_program() {
                                if let Ok(code) = transpile_with_dir_recursive(&program, path.parent().unwrap().to_str().unwrap(), &mut ctx.processed_modules) {
                                    ctx.module_code.push_str(&format!("pub mod {} {{\n{}\n}}\n", internal_mod_name, code));
                                    is_molotov = true;
                                }
                            }
                        }
                    }
                }
            } else {
                is_molotov = true;
            }

            if is_molotov {
                if let Some(a) = alias {
                    ctx.module_names.insert(a.clone());
                    out.push_str(&format!("{}use {} as {};\n", indent(ctx.depth), internal_mod_name, a));
                } else {
                    let mod_name = module.split("::").last().unwrap();
                    ctx.module_names.insert(mod_name.to_string());
                    out.push_str(&format!("{}use {} as {};\n", indent(ctx.depth), internal_mod_name, mod_name));
                }
            } else {
                let alias_str = alias.as_ref().map(|a| format!(" as {}", a)).unwrap_or_default();
                out.push_str(&format!("{}use {}{};\n", indent(ctx.depth), module, alias_str));
            }
        }
        Stmt::FromImport(module, names) => {
            let mut is_molotov = false;
            let internal_mod_name = module.replace("::", "_");
            if !ctx.processed_modules.contains(module) {
                if let Some(path) = resolve_module(module, &ctx.source_dir) {
                    ctx.processed_modules.insert(module.clone());
                    if let Ok(source) = fs::read_to_string(&path) {
                        if let Ok(tokens) = tokenize(&source) {
                            let mut parser = Parser::new(tokens);
                            if let Ok(program) = parser.parse_program() {
                                if let Ok(code) = transpile_with_dir_recursive(&program, path.parent().unwrap().to_str().unwrap(), &mut ctx.processed_modules) {
                                    ctx.module_code.push_str(&format!("pub mod {} {{\n{}\n}}\n", internal_mod_name, code));
                                    is_molotov = true;
                                }
                            }
                        }
                    }
                }
            } else {
                is_molotov = true;
            }

            let names_str: Vec<String> = names.iter().map(|(name, alias)| {
                if let Some(a) = alias { format!("{} as {}", name, a) }
                else { name.clone() }
            }).collect();
            
            let mod_use_path = if is_molotov { &internal_mod_name } else { module };
            out.push_str(&format!("{}use {}::{{{}}};\n", indent(ctx.depth), mod_use_path, names_str.join(", ")));
        }
        Stmt::Try { body, handlers, else_body, finally_body } => {
            out.push_str(&format!("{}(|| -> Result<(), String> {{\n", indent(ctx.depth)));
            let old_depth = ctx.depth;
            ctx.depth += 1;
            let old_declared = ctx.declared.clone();
            let old_try = ctx.in_try;
            ctx.in_try = true;
            for s in body { stmt_to_rust(s, env, ctx, out); }
            ctx.in_try = old_try;
            out.push_str(&format!("{}Ok(())\n", indent(ctx.depth)));
            ctx.declared = old_declared;
            ctx.depth = old_depth;
            out.push_str(&format!("{}}})().unwrap_or_else(|e| {{\n", indent(ctx.depth)));
            ctx.depth += 1;

            if !handlers.is_empty() {
                let handler = &handlers[0];
                out.push_str(&format!("{}let _err = e;\n", indent(ctx.depth)));
                let old_handler_declared = ctx.declared.clone();
                for s in &handler.body { stmt_to_rust(s, env, ctx, out); }
                ctx.declared = old_handler_declared;
            }

            ctx.depth -= 1;
            out.push_str(&format!("{}}});\n", indent(ctx.depth)));

            if let Some(else_body) = else_body {
                for s in else_body { stmt_to_rust(s, env, ctx, out); }
            }
            if let Some(finally_body) = finally_body {
                for s in finally_body { stmt_to_rust(s, env, ctx, out); }
            }
        }
        Stmt::Raise(expr) => {
            if let Some(expr) = expr {
                let code = expr_to_rust(expr, env, ctx);
                out.push_str(&format!("{}return Err({});\n", indent(ctx.depth), code));
            } else {
                out.push_str(&format!("{}return Err(String::new());\n", indent(ctx.depth)));
            }
        }
        Stmt::With { expr, var, body } => {
            let expr_code = expr_to_rust(expr, env, ctx);
            out.push_str(&format!("{}{{\n", indent(ctx.depth)));
            ctx.depth += 1;
            if let Some(var) = var {
                out.push_str(&format!("{}let mut {} = {};\n", indent(ctx.depth), var, expr_code));
            }
            for s in body { stmt_to_rust(s, env, ctx, out); }
            ctx.depth -= 1;
            out.push_str(&format!("{}}}\n", indent(ctx.depth)));
        }
        Stmt::Delete(names) => {
            for _name in names {
                out.push_str(&format!("{}.clear();\n", indent(ctx.depth)));
            }
        }
        Stmt::EmbedRust(code) => {
            out.push_str(&format!("{}{}\n", indent(ctx.depth), code));
        }
    }
}

fn scan_mutated(stmts: &[Stmt], env: &mut TypeEnv, mutating_methods: &HashSet<String>) {
    let mut declared: HashSet<String> = HashSet::new();
    for stmt in stmts {
        scan_stmt_mutated(stmt, &mut declared, env, mutating_methods);
    }
}

fn scan_stmt_mutated(stmt: &Stmt, declared: &mut HashSet<String>, env: &mut TypeEnv, mutating_methods: &HashSet<String>) {
    match stmt {
        Stmt::Assign(name, value, _) => {
            if name.starts_with("__setitem__(") {
                let inner = name.trim_start_matches("__setitem__(").trim_end_matches(")");
                if !inner.is_empty() { declared.insert(inner.to_string()); env.mutated.insert(inner.to_string()); }
            } else if declared.contains(name) {
                env.mutated.insert(name.clone());
            } else {
                declared.insert(name.clone());
            }
            scan_expr_mutated(value, declared, env, mutating_methods);
        }
        Stmt::AssignTuple(names, value) => {
            for n in names {
                if declared.contains(n) { env.mutated.insert(n.clone()); }
                else { declared.insert(n.clone()); }
            }
            scan_expr_mutated(value, declared, env, mutating_methods);
        }
        Stmt::AugAssign(name, _, _) => { declared.insert(name.clone()); env.mutated.insert(name.clone()); }
        Stmt::Expr(e) => { scan_expr_mutated(e, declared, env, mutating_methods); }
        Stmt::For(vars, iterable, body) => {
            for var in vars { declared.insert(var.clone()); }
            scan_expr_mutated(iterable, declared, env, mutating_methods);
            for s in body { scan_stmt_mutated(s, declared, env, mutating_methods); }
        }
        Stmt::While(cond, body) => {
            scan_expr_mutated(cond, declared, env, mutating_methods);
            for s in body { scan_stmt_mutated(s, declared, env, mutating_methods); }
        }
        Stmt::If(branches, else_body) => {
            for (cond, body) in branches {
                scan_expr_mutated(cond, declared, env, mutating_methods);
                for s in body { scan_stmt_mutated(s, declared, env, mutating_methods); }
            }
            if let Some(body) = else_body {
                for s in body { scan_stmt_mutated(s, declared, env, mutating_methods); }
            }
        }
        Stmt::Return(Some(e)) => { scan_expr_mutated(e, declared, env, mutating_methods); }
        Stmt::FuncDef { body, .. } => {
            let mut fn_declared: HashSet<String> = HashSet::new();
            for s in body { scan_stmt_mutated(s, &mut fn_declared, env, mutating_methods); }
        }
        Stmt::ClassDef { body, .. } => {
            for s in body { scan_stmt_mutated(s, declared, env, mutating_methods); }
        }
        Stmt::Try { body, handlers, else_body, finally_body } => {
            for s in body { scan_stmt_mutated(s, declared, env, mutating_methods); }
            for h in handlers { for s in &h.body { scan_stmt_mutated(s, declared, env, mutating_methods); } }
            if let Some(b) = else_body { for s in b { scan_stmt_mutated(s, declared, env, mutating_methods); } }
            if let Some(b) = finally_body { for s in b { scan_stmt_mutated(s, declared, env, mutating_methods); } }
        }
        Stmt::With { expr, body, .. } => {
            scan_expr_mutated(expr, declared, env, mutating_methods);
            for s in body { scan_stmt_mutated(s, declared, env, mutating_methods); }
        }
        Stmt::Delete(names) => {
            for n in names { env.mutated.insert(n.clone()); }
        }
        _ => {}
    }
}

fn scan_expr_mutated(expr: &Expr, declared: &mut HashSet<String>, env: &mut TypeEnv, mutating_methods: &HashSet<String>) {
    match expr {
        Expr::MethodCall(obj, method, args) => {
            let mut mutating: Vec<&str> = vec!["append", "push", "pop", "insert", "remove", "sort", "reverse", "clear", "popitem"];
            if mutating_methods.contains(method) {
                mutating.push(method);
            }
            if mutating.contains(&method.as_str()) {
                if let Expr::Ident(name) = obj.as_ref() {
                    declared.insert(name.clone());
                    env.mutated.insert(name.clone());
                }
            }
            scan_expr_mutated(obj, declared, env, mutating_methods);
            for a in args { scan_expr_mutated(a, declared, env, mutating_methods); }
        }
        Expr::FuncCall(name, args) => {
            let mutating_funcs = ["shuffle"];
            if mutating_funcs.contains(&name.as_str()) {
                if let Some(Expr::Ident(first)) = args.first() {
                    declared.insert(first.clone());
                    env.mutated.insert(first.clone());
                }
            }
            for a in args { scan_expr_mutated(a, declared, env, mutating_methods); }
        }
        Expr::BinOp(l, _, r) => { scan_expr_mutated(l, declared, env, mutating_methods); scan_expr_mutated(r, declared, env, mutating_methods); }
        Expr::UnaryOp(_, e) => scan_expr_mutated(e, declared, env, mutating_methods),
        Expr::Subscript(obj, idx) => { scan_expr_mutated(obj, declared, env, mutating_methods); scan_expr_mutated(idx, declared, env, mutating_methods); }
        Expr::List(items) => { for i in items { scan_expr_mutated(i, declared, env, mutating_methods); } }
        Expr::Dict(pairs) => { for (k, v) in pairs { scan_expr_mutated(k, declared, env, mutating_methods); scan_expr_mutated(v, declared, env, mutating_methods); } }
        Expr::Tuple(items) => { for i in items { scan_expr_mutated(i, declared, env, mutating_methods); } }
        Expr::Starred(e) => scan_expr_mutated(e, declared, env, mutating_methods),
        Expr::Comp(c) => {
            for g in &c.generators {
                scan_expr_mutated(&g.iter, declared, env, mutating_methods);
                if let Some(cond) = &g.cond { scan_expr_mutated(cond, declared, env, mutating_methods); }
            }
        }
        Expr::Attribute(obj, _) => scan_expr_mutated(obj, declared, env, mutating_methods),
        _ => {}
    }
}

pub fn transpile(program: &Program) -> Result<String, String> {
    transpile_with_dir_recursive(program, "", &mut HashSet::new())
}

pub fn transpile_with_dir(program: &Program, source_dir: &str) -> Result<String, String> {
    transpile_with_dir_recursive(program, source_dir, &mut HashSet::new())
}

fn transpile_with_dir_recursive(program: &Program, source_dir: &str, processed_modules: &mut HashSet<String>) -> Result<String, String> {
    let mut env = TypeEnv::default();

    let mutating_methods: HashSet<String> = program.stmts.iter()
        .filter_map(|s| if let Stmt::ClassDef { body, .. } = s { Some(body) } else { None })
        .flat_map(|body| body.iter())
        .filter_map(|s| if let Stmt::FuncDef { name, body, .. } = s {
            let has_self_mut = body.iter().any(|bs| matches!(bs, Stmt::Assign(n, _, _) if n.starts_with("self.")));
            if has_self_mut { Some(name.clone()) } else { None }
        } else { None })
        .collect();

    let mut fn_params_detected: HashMap<String, Vec<(String, Type)>> = HashMap::new();
    let mut fn_returns_detected: HashMap<String, Type> = HashMap::new();

    // Pre-populate function signatures so inference is order-independent
    for stmt in &program.stmts {
        if let Stmt::FuncDef { name, params, body, .. } = stmt {
            let pts: Vec<(String, Type)> = params.iter().map(|(n, _)| {
                let t = env.vars.get(n).cloned().unwrap_or_else(|| scan_param_type(n, body, &env));
                (n.clone(), t)
            }).collect();
            fn_params_detected.insert(name.clone(), pts);
            fn_returns_detected.insert(name.clone(), Type::Unknown);
        }
    }

    let mut local_env = TypeEnv::default();
    local_env.vars = env.vars.clone();
    local_env.fn_params = fn_params_detected.clone();
    for stmt in &program.stmts {
        if let Stmt::FuncDef { name, body, .. } = stmt {
            let ret = infer_return_type(body, &mut local_env);
            local_env.fn_return.insert(name.clone(), ret.clone());
            fn_returns_detected.insert(name.clone(), ret);
        }
    }

    let mut decorator_fn_types: HashMap<String, Type> = HashMap::new();
    for stmt in &program.stmts {
        if let Stmt::FuncDef { decorators, name, .. } = stmt {
            for d in decorators {
                if let Some(params) = fn_params_detected.get(name) {
                    let ret = fn_returns_detected.get(name).cloned().unwrap_or(Type::Unit);
                    let fn_ptr_type = Type::FnPtr(params.iter().map(|(_, t)| t.clone()).collect(), Box::new(ret));
                    decorator_fn_types.insert(d.clone(), fn_ptr_type);
                }
            }
        }
    }

    env.fn_return = fn_returns_detected.clone();
    env.fn_params = fn_params_detected.clone();
    scan_mutated(&program.stmts, &mut env, &mutating_methods);

    let mut ctx = CodegenCtx {
        depth: 0,
        current_fn: None,
        current_class: None,
        class_names: HashSet::new(),
        needs_main: false,
        declared: HashSet::new(),
        source_dir: source_dir.to_string(),
        has_raise: false,
        in_try: false,
        decorator_fn_types: decorator_fn_types.clone(),
        processed_modules: processed_modules.clone(),
        module_code: String::new(),
        module_names: HashSet::new(),
    };

    for stmt in &program.stmts {
        if let Stmt::ClassDef { name, .. } = stmt {
            ctx.class_names.insert(name.clone());
        }
    }

    let mut body = String::new();
    let mut top_level_stmts = Vec::new();

    for stmt in &program.stmts {
        match stmt {
            Stmt::FuncDef { .. } | Stmt::ClassDef { .. } | Stmt::Import(_ , _) | Stmt::FromImport(_, _) => {
                stmt_to_rust(stmt, &mut env, &mut ctx, &mut body);
            }
            _ => {
                top_level_stmts.push(stmt.clone());
            }
        }
    }

    if !top_level_stmts.is_empty() {
        ctx.needs_main = true;
        body.push_str("fn main() {\n");
        ctx.depth = 1;
        for stmt in &top_level_stmts {
            stmt_to_rust(stmt, &mut env, &mut ctx, &mut body);
        }
        body.push_str("}\n");
    }

    // Update the caller's processed_modules
    *processed_modules = ctx.processed_modules;

    let mut output = String::new();

    if env.uses_hashmap {
        output.push_str("use std::collections::HashMap;\n");
    }
    if env.uses_serde {
        output.push_str("use serde_json;\n");
    }
    if env.uses_hashmap || env.uses_serde {
        output.push('\n');
    }

    output.push_str(&ctx.module_code);
    output.push_str(&body);

    Ok(output)
}

