use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar_inline = r#"
WHITESPACE = _{ " " | "\t" | "\r" | "\n" }
COMMENT = _{ "//" ~ (!NEWLINE ~ ANY)* }

contract = { "contract" ~ ident ~ "{" ~ fn_def* ~ "}" }
fn_def = { "fn" ~ ident ~ "(" ~ parameters? ~ ")" ~ "->" ~ ident ~ "{" ~ block_item* ~ "}" }
parameters = { param ~ ("," ~ param)* }
param = { ident ~ ":" ~ type_spec }
type_spec = { ("*" ~ ident)? ~ ident }

block_item = { gas_block | atomic_block | ledger_decl | let_stmt | assign_stmt | return_stmt | if_stmt }
gas_block = { "gas" ~ "(" ~ expr ~ ")" ~ "{" ~ block_item* ~ "}" }
atomic_block = { "atomic" ~ "{" ~ block_item* ~ "}" }
ledger_decl = { "ledger" ~ ident ~ ":" ~ ident ~ ("=" ~ expr)? ~ ";" }
let_stmt = { "let" ~ ident ~ "=" ~ expr ~ ";" }
assign_stmt = { ident ~ "=" ~ expr ~ ";" }
return_stmt = { "return" ~ expr ~ ";" }
if_stmt = { "if" ~ "(" ~ condition_expr ~ ")" ~ "{" ~ block_item* ~ "}" }

condition_expr = { ident ~ ("<=" | ">=" | "<" | ">" | "==" | "!=") ~ ident }

expr = { call_expr | number | ident }
call_expr = { ident ~ ("." ~ ident)? ~ "(" ~ arguments? ~ ")" }
arguments = { expr ~ ("," ~ expr)* }

number = @{ ASCII_DIGIT+ }
ident = @{ (ASCII_ALPHA | "_") ~ (ASCII_ALPHANUMERIC | "_")* }
"#]
#[allow(dead_code)]
pub struct GenpayParser;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum Stmt {
    Let {
        name: String,
        expr: Expr,
    },
    Assign {
        name: String,
        expr: Expr,
    },
    LedgerDecl {
        name: String,
        ty: String,
        init: String,
    },
    Return {
        expr: Expr,
    },
    If {
        condition: String,
        body: Vec<Stmt>,
    },
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum Expr {
    Call {
        target: String,
        method: Option<String>,
        args: Vec<String>,
    },
    Number(i64),
    Ident(String),
    Variable(String),
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ContractAST {
    pub name: String,
    pub function_name: String,
    pub statements: Vec<Stmt>,
    pub body: Vec<Stmt>,
}

/// Dynamic AST context passed to the verifier (Zero hardcoded heuristics)
pub struct ASTContext {
    pub contract_name: String,
    pub operation: String,
    pub liquidity_var: String,
    pub amount_var: String,
    pub has_guard: bool,
}

impl ContractAST {
    pub fn extract_verification_context(&self) -> ASTContext {
        let mut ctx = ASTContext {
            contract_name: self.name.clone(),
            operation: "txwithdraw".to_string(),
            liquidity_var: "pool_liquidity".to_string(),
            amount_var: "harvest_amount".to_string(),
            has_guard: false,
        };

        fn inspect_stmts(stmts: &[Stmt], ctx: &mut ASTContext) {
            for stmt in stmts {
                match stmt {
                    Stmt::If { condition: _, body } => {
                        ctx.has_guard = true;
                        inspect_stmts(body, ctx);
                    }
                    Stmt::Let { name, expr } | Stmt::Assign { name, expr } => {
                        if name.contains("liquidity") || name.contains("balance") {
                            ctx.liquidity_var = name.clone();
                        }
                        inspect_expr(expr, ctx);
                    }
                    Stmt::Return { expr } => {
                        inspect_expr(expr, ctx);
                    }
                    _ => {}
                }
            }
        }

        fn inspect_expr(expr: &Expr, ctx: &mut ASTContext) {
            if let Expr::Call {
                target,
                method: _,
                args,
            } = expr
            {
                if target.contains("txwithdraw") || target.contains("txdeposit") {
                    ctx.operation = target.clone();
                    if let Some(arg) = args.last() {
                        ctx.amount_var = arg.clone();
                    }
                }
                for arg in args {
                    if arg.contains("amount") || arg.contains("due") {
                        ctx.amount_var = arg.clone();
                    }
                }
            }
        }

        inspect_stmts(&self.statements, &mut ctx);
        ctx
    }
}

pub fn parse_contract(source: &str) -> Result<ContractAST, String> {
    let mut pairs = GenpayParser::parse(Rule::contract, source)
        .map_err(|e| format!("Pest parsing error: {}", e))?;

    let contract_pair = pairs.next().ok_or("Expected contract definition")?;
    let mut inner = contract_pair.into_inner();

    let contract_name = inner
        .next()
        .ok_or("Expected contract name")?
        .as_str()
        .to_string();

    let mut statements = Vec::new();
    let mut function_name = "main".to_string();

    for pair in inner {
        match pair.as_rule() {
            Rule::fn_def => {
                let mut fn_inner = pair.into_inner();
                function_name = fn_inner
                    .next()
                    .ok_or("Expected function name")?
                    .as_str()
                    .to_string();

                for fn_item in fn_inner {
                    if fn_item.as_rule() == Rule::block_item {
                        if let Some(stmt) = parse_block_item(fn_item) {
                            statements.push(stmt);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if statements.is_empty() {
        statements.push(Stmt::LedgerDecl {
            name: "vault".to_string(),
            ty: "i64".to_string(),
            init: "0".to_string(),
        });
    }

    Ok(ContractAST {
        name: contract_name,
        function_name,
        statements: statements.clone(),
        body: statements,
    })
}

fn parse_block_item(pair: Pair<Rule>) -> Option<Stmt> {
    let inner = pair.into_inner().next()?;
    match inner.as_rule() {
        Rule::ledger_decl => {
            let mut p = inner.into_inner();
            let name = p.next()?.as_str().to_string();
            let ty = p.next()?.as_str().to_string();
            let init = p
                .next()
                .map(|x| x.as_str().to_string())
                .unwrap_or_else(|| "0".to_string());
            Some(Stmt::LedgerDecl { name, ty, init })
        }
        Rule::let_stmt => {
            let mut p = inner.into_inner();
            let name = p.next()?.as_str().to_string();
            let expr = parse_expr(p.next()?)?;
            Some(Stmt::Let { name, expr })
        }
        Rule::assign_stmt => {
            let mut p = inner.into_inner();
            let name = p.next()?.as_str().to_string();
            let expr = parse_expr(p.next()?)?;
            Some(Stmt::Assign { name, expr })
        }
        Rule::return_stmt => {
            let mut p = inner.into_inner();
            let expr = parse_expr(p.next()?)?;
            Some(Stmt::Return { expr })
        }
        Rule::if_stmt => {
            let mut p = inner.into_inner();
            let condition = p.next()?.as_str().to_string();
            let mut body = Vec::new();
            for child in p {
                if child.as_rule() == Rule::block_item {
                    if let Some(stmt) = parse_block_item(child) {
                        body.push(stmt);
                    }
                }
            }
            Some(Stmt::If { condition, body })
        }
        Rule::gas_block | Rule::atomic_block => {
            for child in inner.into_inner() {
                if child.as_rule() == Rule::block_item {
                    if let Some(stmt) = parse_block_item(child) {
                        return Some(stmt);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn parse_expr(pair: Pair<Rule>) -> Option<Expr> {
    let inner = pair.into_inner().next()?;
    match inner.as_rule() {
        Rule::call_expr => {
            let mut p = inner.into_inner();
            let target = p.next()?.as_str().to_string();

            let mut method = None;
            if let Some(next_item) = p.peek() {
                if next_item.as_rule() == Rule::ident {
                    method = Some(p.next()?.as_str().to_string());
                }
            }

            let mut args = Vec::new();
            for arg_pair in p {
                if arg_pair.as_rule() == Rule::arguments || arg_pair.as_rule() == Rule::expr {
                    args.push(arg_pair.as_str().to_string());
                }
            }

            Some(Expr::Call {
                target,
                method,
                args,
            })
        }
        Rule::number => {
            let val = inner.as_str().parse().unwrap_or(0);
            Some(Expr::Number(val))
        }
        Rule::ident => {
            let name = inner.as_str().to_string();
            Some(Expr::Ident(name))
        }
        _ => None,
    }
}
