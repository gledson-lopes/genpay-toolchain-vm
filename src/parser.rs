use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "parser.pest"]
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
            init: "b0".to_string(),
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
            let init = p.next()?.as_str().to_string();
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
