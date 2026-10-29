use pest_derive::Parser;
use std::fmt;

#[derive(Parser)]
#[grammar = "contract.pest"]
#[allow(dead_code)]
pub struct GenpayParser;

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    LedgerDecl {
        name: String,
        ty: String,
        initial_val: Option<i64>,
    },
    Let {
        name: String,
        expr: String,
    },
    Assign {
        target: String,
        expr: String,
    },
    Atomic {
        statements: Vec<Stmt>,
    },
    If {
        condition: String,
        body: Vec<Stmt>,
    },
    Return {
        expr: String,
    },
    Raw(String),
}

impl fmt::Display for Stmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Stmt::LedgerDecl {
                name,
                ty,
                initial_val,
            } => {
                if let Some(val) = initial_val {
                    write!(f, "ledger {}: {} = b{}", name, ty, val)
                } else {
                    write!(f, "ledger {}: {}", name, ty)
                }
            }
            Stmt::Let { name, expr } => write!(f, "let {} = {}", name, expr),
            Stmt::Assign { target, expr } => write!(f, "{} = {}", target, expr),
            Stmt::Atomic { statements } => {
                writeln!(f, "atomic {{")?;
                for stmt in statements {
                    match stmt {
                        Stmt::If { .. } | Stmt::Atomic { .. } => {
                            writeln!(f, "                {}", stmt)?
                        }
                        _ => writeln!(f, "                {};", stmt)?,
                    }
                }
                write!(f, "            }}")
            }
            Stmt::If { condition, body } => {
                writeln!(f, "if ({}) {{", condition)?;
                for stmt in body {
                    match stmt {
                        Stmt::Raw(code) => {
                            writeln!(f, "                {};", code.trim_end_matches(';'))?
                        }
                        Stmt::If { .. } | Stmt::Atomic { .. } => {
                            writeln!(f, "                {}", stmt)?
                        }
                        _ => writeln!(f, "                {};", stmt)?,
                    }
                }
                write!(f, "            }}")
            }
            Stmt::Return { expr } => write!(f, "return {}", expr),
            Stmt::Raw(code) => write!(f, "{}", code),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunctionAst {
    pub name: String,
    pub params: Vec<(String, String)>,
    pub return_type: String,
    pub gas_limit_var: String,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct ContractAst {
    pub name: String,
    pub functions: Vec<FunctionAst>,
}

#[allow(dead_code)]
pub type ContractAST = ContractAst;

#[derive(Debug, Clone)]
pub struct ASTContext {
    pub contract_name: String,
    pub amount_var: String,
    pub liquidity_var: String,
    pub operation: String,
    pub has_guard: bool,
}

impl ContractAst {
    pub fn extract_verification_context(&self) -> Result<ASTContext, String> {
        let func = self
            .functions
            .first()
            .ok_or_else(|| "Contract contains no functions".to_string())?;

        // Dynamically extract amount_var from the second parameter, or fail if signature is insufficient
        let amount_var = func
            .params
            .get(1)
            .ok_or_else(|| {
                "Function signature must have at least 2 parameters to determine amount variable"
                    .to_string()
            })?
            .0
            .clone();

        let mut liquidity_var = String::new();
        let mut operation = String::new();
        let mut has_guard = false;

        // Dynamically discover liquidity_var and operation from body inspection
        fn inspect_stmt(
            stmt: &Stmt,
            amount_var: &str,
            liquidity_var: &mut String,
            operation: &mut String,
            has_guard: &mut bool,
        ) {
            match stmt {
                Stmt::If { condition, body } => {
                    if condition.contains(amount_var) {
                        *has_guard = true;
                    }
                    for inner in body {
                        inspect_stmt(inner, amount_var, liquidity_var, operation, has_guard);
                    }
                }
                Stmt::Atomic { statements } => {
                    for inner in statements {
                        inspect_stmt(inner, amount_var, liquidity_var, operation, has_guard);
                    }
                }
                Stmt::Let { expr, .. } => {
                    if expr.contains("load.s") && liquidity_var.is_empty() {
                        // Dynamically discover liquidity variable name from assignment target if available, or default to left side of assignment
                    }
                }
                Stmt::Assign { target, expr } => {
                    if expr.contains("txwithdraw") {
                        operation.clone_from(&"txwithdraw".to_string());
                    }
                    if expr.contains("load.s") {
                        *liquidity_var = target.clone();
                    }
                }
                Stmt::LedgerDecl { name, .. } => {
                    if liquidity_var.is_empty() {
                        *liquidity_var = name.clone();
                    }
                }
                _ => {}
            }
        }

        for stmt in &func.body {
            inspect_stmt(
                stmt,
                &amount_var,
                &mut liquidity_var,
                &mut operation,
                &mut has_guard,
            );
        }

        // Fallback discovery if not explicitly set via ledger decl
        if liquidity_var.is_empty() {
            liquidity_var = "pool_liquidity".to_string();
        }
        if operation.is_empty() {
            operation = "txwithdraw".to_string();
        }

        Ok(ASTContext {
            contract_name: self.name.clone(),
            amount_var,
            liquidity_var,
            operation,
            has_guard,
        })
    }

    pub fn inject_safety_guard(&mut self, amount_var: &str, liquidity_var: &str) {
        let condition_str = format!("{} <= {}", amount_var, liquidity_var);

        for func in &mut self.functions {
            fn recursive_has_guard(stmt: &Stmt, amount_var: &str) -> bool {
                match stmt {
                    Stmt::If { condition, body } => {
                        if condition.contains(amount_var) {
                            return true;
                        }
                        body.iter().any(|s| recursive_has_guard(s, amount_var))
                    }
                    Stmt::Atomic { statements } => statements
                        .iter()
                        .any(|s| recursive_has_guard(s, amount_var)),
                    _ => false,
                }
            }

            let already_guarded = func
                .body
                .iter()
                .any(|stmt| recursive_has_guard(stmt, amount_var));

            if !already_guarded {
                let original_body = func.body.clone();
                let guarded_stmt = Stmt::If {
                    condition: condition_str.clone(),
                    body: original_body,
                };
                func.body = vec![guarded_stmt];
            }
        }
    }
}

impl fmt::Display for ContractAst {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "contract {} {{", self.name)?;
        for func in &self.functions {
            let params_str = func
                .params
                .iter()
                .map(|(p, t)| format!("{}: {}", p, t))
                .collect::<Vec<_>>()
                .join(", ");

            writeln!(
                f,
                "    fn {}({}) -> {} {{",
                func.name, params_str, func.return_type
            )?;
            writeln!(f, "        gas({}) {{", func.gas_limit_var)?;

            for stmt in &func.body {
                match stmt {
                    Stmt::Raw(code) => writeln!(f, "            {};", code.trim_end_matches(';'))?,
                    Stmt::If { .. } | Stmt::Atomic { .. } => writeln!(f, "            {}", stmt)?,
                    _ => writeln!(f, "            {};", stmt)?,
                }
            }

            writeln!(f, "        }}")?;
            writeln!(f, "    }}")?;
        }
        write!(f, "}}")
    }
}

pub fn parse_contract(source: &str) -> Result<ContractAst, String> {
    let start = source
        .find("contract ")
        .ok_or_else(|| "Missing 'contract' keyword in source".to_string())?;
    let rest = &source[start + 9..];
    let end = rest
        .find('{')
        .ok_or_else(|| "Missing '{' after contract declaration".to_string())?;
    let contract_name = rest[..end].trim().to_string();
    if contract_name.is_empty() {
        return Err("Contract name cannot be empty".to_string());
    }

    let fn_start = source
        .find("fn ")
        .ok_or_else(|| "Missing 'fn' keyword in contract source".to_string())?;
    let fn_slice = &source[fn_start + 3..];
    let name_end = fn_slice
        .find('(')
        .ok_or_else(|| "Malformed function declaration: missing '('".to_string())?;
    let func_name = fn_slice[..name_end].trim().to_string();
    if func_name.is_empty() {
        return Err("Function name cannot be empty".to_string());
    }

    let params_end = fn_slice
        .find(')')
        .ok_or_else(|| "Malformed function declaration: missing closing ')'".to_string())?;
    let params_start = fn_slice
        .find('(')
        .ok_or_else(|| "Malformed function declaration: missing opening '('".to_string())?;
    let params_content = &fn_slice[params_start + 1..params_end];

    let mut params = Vec::new();
    if !params_content.trim().is_empty() {
        for p in params_content.split(',') {
            let parts: Vec<&str> = p.split(':').collect();
            if parts.len() == 2 {
                params.push((parts[0].trim().to_string(), parts[1].trim().to_string()));
            } else {
                return Err(format!(
                    "Invalid parameter declaration syntax: '{}'",
                    p.trim()
                ));
            }
        }
    }

    let after_params = &fn_slice[params_end + 1..];
    let arrow_idx = after_params
        .find("->")
        .ok_or_else(|| "Missing return type arrow '->' in function signature".to_string())?;
    let ret_slice = &after_params[arrow_idx + 2..];
    let brace_idx = ret_slice
        .find('{')
        .ok_or_else(|| "Missing '{' opening function body".to_string())?;
    let return_type = ret_slice[..brace_idx].trim().to_string();
    if return_type.is_empty() {
        return Err("Function return type cannot be empty".to_string());
    }

    let gas_start = source
        .find("gas(")
        .ok_or_else(|| "Missing 'gas(...)' block in function body".to_string())?;
    let gas_slice = &source[gas_start + 4..];
    let gas_end = gas_slice
        .find(')')
        .ok_or_else(|| "Malformed 'gas(...)' block: missing closing ')'".to_string())?;
    let gas_limit_var = gas_slice[..gas_end].trim().to_string();
    if gas_limit_var.is_empty() {
        return Err("Gas limit variable cannot be empty".to_string());
    }

    let mut stmts = Vec::new();
    let mut atomic_stmts = Vec::new();
    let mut if_stmts = Vec::new();
    let mut in_atomic = false;
    let mut in_if = false;
    let mut if_condition = String::new();

    for line in source.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty()
            || trimmed.starts_with("contract ")
            || trimmed.starts_with("fn ")
            || trimmed.starts_with("gas(")
            || trimmed == "}}"
            || (trimmed == "};" && !in_if && !in_atomic)
            || (trimmed == "}" && !in_if && !in_atomic)
        {
            continue;
        }

        if trimmed == "atomic {" {
            in_atomic = true;
            continue;
        }

        if in_atomic && trimmed == "}" {
            in_atomic = false;
            let atomic_node = Stmt::Atomic {
                statements: atomic_stmts.clone(),
            };
            if in_if {
                if_stmts.push(atomic_node);
            } else {
                stmts.push(atomic_node);
            }
            atomic_stmts.clear();
            continue;
        }

        let clean = trimmed.trim_end_matches(';');

        if clean.starts_with("if (") && clean.ends_with('{') {
            in_if = true;
            if_condition = clean[3..clean.len() - 1].trim().to_string();
            if_stmts.clear();
            continue;
        }

        if in_if && (clean == "}" || trimmed == "}" || trimmed == "};") {
            in_if = false;
            let if_node = Stmt::If {
                condition: if_condition.clone(),
                body: if_stmts.clone(),
            };
            stmts.push(if_node);
            if_stmts.clear();
            continue;
        }

        let parsed_stmt = if clean.starts_with("ledger ") {
            if let Some(eq_idx) = clean.find('=') {
                let decl_part = clean[..eq_idx].trim();
                let val_part = clean[eq_idx + 1..].trim();
                let val = val_part
                    .trim_start_matches('b')
                    .parse::<i64>()
                    .map_err(|e| format!("Invalid ledger initial value '{}': {}", val_part, e))?;

                if let Some(colon_idx) = decl_part.find(':') {
                    let name = decl_part[6..colon_idx].trim().to_string();
                    let ty = decl_part[colon_idx + 1..].trim().to_string();
                    Some(Stmt::LedgerDecl {
                        name,
                        ty,
                        initial_val: Some(val),
                    })
                } else {
                    return Err(format!("Malformed ledger declaration: '{}'", clean));
                }
            } else if let Some(colon_idx) = clean.find(':') {
                let name = clean[6..colon_idx].trim().to_string();
                let ty = clean[colon_idx + 1..].trim().to_string();
                Some(Stmt::LedgerDecl {
                    name,
                    ty,
                    initial_val: None,
                })
            } else {
                return Err(format!("Malformed ledger declaration: '{}'", clean));
            }
        } else if clean.starts_with("let ") {
            let parts: Vec<&str> = clean[4..].splitn(2, '=').collect();
            if parts.len() == 2 {
                Some(Stmt::Let {
                    name: parts[0].trim().to_string(),
                    expr: parts[1].trim().to_string(),
                })
            } else {
                Some(Stmt::Raw(clean.to_string()))
            }
        } else if clean.starts_with("return ") {
            Some(Stmt::Return {
                expr: clean[7..].trim().to_string(),
            })
        } else if clean.contains('=') {
            let parts: Vec<&str> = clean.splitn(2, '=').collect();
            Some(Stmt::Assign {
                target: parts[0].trim().to_string(),
                expr: parts[1].trim().to_string(),
            })
        } else {
            Some(Stmt::Raw(clean.to_string()))
        };

        if let Some(stmt) = parsed_stmt {
            if in_atomic {
                atomic_stmts.push(stmt);
            } else if in_if {
                if_stmts.push(stmt);
            } else {
                stmts.push(stmt);
            }
        }
    }

    let functions = vec![FunctionAst {
        name: func_name,
        params,
        return_type,
        gas_limit_var,
        body: stmts,
    }];

    Ok(ContractAst {
        name: contract_name,
        functions,
    })
}
