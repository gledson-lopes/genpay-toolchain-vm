use genpay::codegen::GenpayCodeGen;
use genpay::vm::GenpayVM;
use inkwell::context::Context;

fn main() {
    println!("=== Genpay v2.1 Toolchain & VM Verification ===");

    // 1. Compile EscrowVault AST to LLVM IR
    let context = Context::create();
    let mut codegen = GenpayCodeGen::new(&context, "genpay_module");
    codegen.compile_escrow_vault();

    println!("\nGenerated LLVM IR Output:\n");
    println!("{}", codegen.module.print_to_string().to_string());

    // 2. Execute Compiled Contract Mechanics on Genpay VM
    let mut vm = GenpayVM::new(1024);
    println!("Executing EscrowVault.deposit_funds in VM...");

    let limit = 1000u64;
    vm.set_gas(limit); // Removed .unwrap() here
    println!(
        "1. Initialized g0 hardware gas counter to: {}",
        vm.gas_counter
    );

    // Prepare memory with valid 0x3 sustain pointer tag
    let wallet_address: usize = 0x40;
    let valid_sustain_ptr: u64 = (0x3 << 60) | (wallet_address as u64);
    let initial_balance: i64 = 5000;
    vm.memory[wallet_address..wallet_address + 8].copy_from_slice(&initial_balance.to_le_bytes());

    // Execute load.s
    let balance = vm.load_s(valid_sustain_ptr).unwrap();
    println!("2. Executed load.s: Read wallet balance = {}", balance);

    // Execute txdeposit into ledger b0
    let deposit_amount = 1200u64;
    let new_vault_balance = vm.txdeposit(0, deposit_amount as i64).unwrap();
    println!(
        "3. Executed txdeposit: Escrowed {} into ledger b0. New vault total = {}",
        deposit_amount, new_vault_balance
    );

    println!("4. Remaining g0 hardware gas: {}", vm.gas_counter);
    println!("\n=== Execution Completed Successfully ===");
}
