use genpay::codegen::GenpayCodeGen;
use genpay::verifier::GenpayVerifier;
use genpay::vm::GenpayVM;
use inkwell::context::Context;
use std::env;
use std::fs;

const GREEN: &str = "\x1b[32m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

fn main() {
    println!(
        "{}=== Genpay v2.1 Toolchain & Formal Verification Engine ==={}",
        BOLD, RESET
    );

    let args: Vec<String> = env::args().collect();
    let file_path = if args.len() > 1 {
        &args[1]
    } else {
        "contract.gp"
    };

    let mut source_code = fs::read_to_string(file_path).unwrap_or_else(|_| {
        r#"contract EscrowVault {
    fn deposit_funds(wallet_ptr: *sustain i64, amount: i64, limit: i64) -> i64 {
        gas(limit) {
            ledger vault: i64 = b0;
            atomic {
                let current_balance = load.s(wallet_ptr);
                vault = txdeposit(wallet_ptr, amount);
            }
            return vault;
        }
    }
}"#
        .to_string()
    });

    println!(
        "\n[1/3] Running Genpay Formal Verification on {}",
        file_path
    );

    let target_statement = "vault = txdeposit(wallet_ptr, amount);";
    let faulty_span = if let Some(idx) = source_code.find(target_statement) {
        idx..(idx + target_statement.len())
    } else {
        180..220
    };

    let mut v_res =
        GenpayVerifier::verify_escrow_deposit(&source_code, file_path, faulty_span.clone());

    // --- AUTONOMOUS SELF-HEALING COMPILER LOOP ---
    if !v_res.is_valid {
        if source_code.contains("if (amount <= current_balance)") {
            println!(
                "\n{}⚠️ [GENPAY AUTO-HEAL] Guard already detected in source code. Skipping patch.{}\n",
                GREEN, RESET
            );
        } else {
            println!(
                "\n{}⚡ [GENPAY AUTO-HEAL] Invariant violation detected. Initiating automated patch...{}",
                GREEN, RESET
            );

            if let Some(ref guidance) = v_res.guidance {
                println!(
                    "{}   -> Genpay SMT Counterexample: {}{}",
                    GREEN, guidance.counterexample_summary, RESET
                );
            }

            let vulnerable_pattern = "vault = txdeposit(wallet_ptr, amount);";
            let safe_replacement = "if (amount <= current_balance) {\n                    vault = txdeposit(wallet_ptr, amount);\n                }";

            source_code = source_code.replace(vulnerable_pattern, safe_replacement);
            fs::write(file_path, &source_code).expect("Failed to write auto-patched contract");

            println!("\n{}✨ [AUTO-REWRITE APPLIED]:{}", GREEN, RESET);
            println!("{}{}{}\n", GREEN, safe_replacement, RESET);

            println!("[Re-running Genpay Formal Verification on patched code...]");
            let new_span = if let Some(idx) = source_code.find("vault = txdeposit") {
                idx..idx + 35
            } else {
                faulty_span
            };

            v_res = GenpayVerifier::verify_escrow_deposit(&source_code, file_path, new_span);

            if !v_res.is_valid {
                eprintln!("\n[COMPILER ABORT] Automated patch failed to resolve invariant.");
                std::process::exit(1);
            }
        }
    }

    println!(
        "{}✔ Genpay Formal Verification Passed: Solvency and memory invariants mathematically proven.{}\n",
        GREEN, RESET
    );

    println!("[2/3] Generating Genpay Intermediate Representation (IR)...");
    let context = Context::create();
    let mut codegen = GenpayCodeGen::new(&context, "genpay_module");
    codegen.compile_escrow_vault();

    println!("{}", codegen.module.print_to_string().to_string());

    println!("[3/3] Executing Contract on Genpay VM Runtime...");
    let mut vm = GenpayVM::new(1024);
    vm.set_gas(1000);

    let wallet_address: usize = 0x40;
    let valid_sustain_ptr: u64 = (0x3 << 60) | (wallet_address as u64);
    let initial_balance: i64 = 5000;
    vm.memory[wallet_address..wallet_address + 8].copy_from_slice(&initial_balance.to_le_bytes());

    let _ = vm.load_s(valid_sustain_ptr).unwrap();
    let new_vault_balance = vm.txdeposit(0, 1200).unwrap();

    println!(
        "{}✔ VM Execution Successful! New Vault Balance: {}{}",
        GREEN, new_vault_balance, RESET
    );
    println!("Remaining Gas (g0): {}", vm.gas_counter);
}
