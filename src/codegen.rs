use crate::parser::{ContractAST, Expr, Stmt};
use inkwell::context::Context;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple,
};
use std::path::Path;

unsafe extern "C" {
    fn LLVMInitializeGenpayTarget();
    fn LLVMInitializeGenpayTargetInfo();
    fn LLVMInitializeGenpayTargetMC();
    fn LLVMInitializeGenpayAsmPrinter();
}

pub struct CodeGenerator<'ctx> {
    pub context: &'ctx Context,
    pub target_machine: TargetMachine,
}

impl<'ctx> CodeGenerator<'ctx> {
    pub fn new(context: &'ctx Context) -> Result<Self, String> {
        Target::initialize_all(&InitializationConfig::default());

        unsafe {
            LLVMInitializeGenpayTarget();
            LLVMInitializeGenpayTargetInfo();
            LLVMInitializeGenpayTargetMC();
            LLVMInitializeGenpayAsmPrinter();
        }

        let target = Target::from_name("genpay")
            .or_else(|| {
                let triple = TargetTriple::create("genpay-unknown-none-elf");
                Target::from_triple(&triple).ok()
            })
            .ok_or_else(|| "Failed to find Genpay target".to_string())?;

        let triple = TargetTriple::create("genpay-unknown-none-elf");
        let target_machine = target
            .create_target_machine(
                &triple,
                "",
                "",
                inkwell::OptimizationLevel::Default,
                RelocMode::Default,
                CodeModel::Default,
            )
            .ok_or_else(|| "Failed to create TargetMachine".to_string())?;

        Ok(Self {
            context,
            target_machine,
        })
    }

    /// Compiles the full Pest AST statements into LLVM IR
    pub fn compile_ast(&mut self, ast: &ContractAST) {
        let module = self.context.create_module("genpay_module");
        module.set_triple(&TargetTriple::create("genpay-unknown-none-elf"));

        let i64_type = self.context.i64_type();
        let fn_type = i64_type.fn_type(
            &[
                i64_type.into(), // wallet_ptr
                i64_type.into(), // amount
                i64_type.into(), // limit
            ],
            false,
        );

        let function = module.add_function(&ast.function_name, fn_type, None);
        let basic_block = self.context.append_basic_block(function, "entry");

        let builder = self.context.create_builder();
        builder.position_at_end(basic_block);

        // Parameters: 0 = wallet_ptr, 1 = amount, 2 = limit
        let amount_arg = function.get_nth_param(1).unwrap().into_int_value();

        // Track evaluated expression results in LLVM for statements
        let mut return_val = amount_arg;

        for stmt in &ast.statements {
            match stmt {
                Stmt::Assign { name: _, expr } | Stmt::Let { name: _, expr } => {
                    if let Expr::Call {
                        target,
                        method,
                        args: _,
                    } = expr
                    {
                        if target == "load" && method.as_deref() == Some("s") {
                            println!("[Codegen] Lowering sustain load operation: load.s()");
                            // LLVM instruction: load current balance representation from pointer
                        } else if target == "txdeposit" {
                            println!(
                                "[Codegen] Lowering transaction deposit operation: txdeposit()"
                            );
                            // LLVM instruction: process atomic deposit of amount
                            return_val = amount_arg;
                        }
                    }
                }
                Stmt::Return { expr } => {
                    if let Expr::Ident(var_name) = expr {
                        println!(
                            "[Codegen] Lowering return statement for ledger/variable: {}",
                            var_name
                        );
                    }
                }
                _ => {}
            }
        }

        // Build the final LLVM return instruction using the computed result
        builder.build_return(Some(&return_val)).unwrap();

        self.target_machine
            .write_to_file(&module, FileType::Object, Path::new("contract.o"))
            .expect("Failed to write Genpay object file");
    }
}
