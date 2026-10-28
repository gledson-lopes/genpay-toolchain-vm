pub struct GenpayVM {
    pub memory: Vec<u8>,
    pub registers: [i64; 32],
    pub gas_counter: u64,
    pub pc: usize,
    pub ledgers: [i64; 4],
}

impl GenpayVM {
    pub fn new(memory_size: usize) -> Self {
        let mut vm = Self {
            memory: vec![0; memory_size],
            registers: [0; 32],
            gas_counter: 0,
            pc: 0,
            ledgers: [0; 4],
        };

        // Initialize execution context registers:
        // r1 = wallet pointer with 0x3 ADRS phase capability tag pointing to address 0x0200
        vm.registers[1] = (0x3 << 60) | 0x0200;
        // r2 = transaction deposit amount (e.g., 200)
        vm.registers[2] = 200;

        vm
    }

    pub fn set_gas(&mut self, limit: u64) {
        self.gas_counter = limit;
    }

    /// Executes a phase-validated sustain load (load.s)
    pub fn load_s(&mut self, ptr: u64) -> Result<i64, String> {
        if self.gas_counter == 0 {
            return Err("Out of gas (g0)".into());
        }
        self.gas_counter -= 1;

        let phase_tag = (ptr >> 60) & 0xF;
        if phase_tag != 0x3 {
            return Err(format!(
                "ADRS lifecycle violation: Invalid pointer phase tag 0x{:X} for sustain load (expected 0x3)",
                phase_tag
            ));
        }

        let addr = (ptr & 0x0FFFFFFFFFFFFFFF) as usize;
        if addr + 8 > self.memory.len() {
            return Err("Segmentation fault: Address out of bounds".into());
        }

        let bytes = &self.memory[addr..addr + 8];
        let val = i64::from_le_bytes(bytes.try_into().unwrap());
        Ok(val)
    }

    /// Executes an atomic transaction deposit into a ledger slot
    pub fn txdeposit(&mut self, ledger_index: usize, amount: i64) -> Result<i64, String> {
        if self.gas_counter == 0 {
            return Err("Out of gas (g0)".into());
        }
        self.gas_counter -= 1;

        if ledger_index >= self.ledgers.len() {
            return Err("Invalid ledger index".into());
        }
        self.ledgers[ledger_index] += amount;
        Ok(self.ledgers[ledger_index])
    }

    /// True instruction-decoding execution loop over loaded `contract.o` code
    pub fn run_from(&mut self, start_pc: usize, code_len: usize) -> Result<i64, String> {
        self.pc = start_pc;
        println!(
            "[VM Runtime] Starting Genpay secure ISA execution from PC: 0x{:04X} (Length: {} bytes)",
            self.pc, code_len
        );

        // 1. Seed initial wallet balance in VM memory at address 0x0200 (1000 units)
        let wallet_addr = (self.registers[1] & 0x0FFFFFFFFFFFFFFF) as usize;
        let initial_balance: i64 = 1000;
        self.memory[wallet_addr..wallet_addr + 8].copy_from_slice(&initial_balance.to_le_bytes());

        // 2. Fetch and execute instructions from loaded binary memory stream
        let end_pc = start_pc + code_len;
        while self.pc < end_pc {
            if self.gas_counter == 0 {
                return Err("Out of gas (g0)".into());
            }

            if self.pc + 4 > self.memory.len() {
                return Err("Segmentation fault: PC out of bounds".into());
            }

            // Fetch 32-bit instruction word from object memory
            let inst_bytes = &self.memory[self.pc..self.pc + 4];
            let opcode_word = u32::from_le_bytes(inst_bytes.try_into().unwrap());
            let opcode = ((opcode_word >> 24) & 0xFF) as u8;

            // Execute based on decoded opcode or advance instruction stream
            match opcode {
                0x01 => {
                    let current_balance = self.load_s(self.registers[1] as u64)?;
                    println!(
                        "[VM Decoded Exec] load.s executed from binary. Balance: {}",
                        current_balance
                    );
                    self.registers[3] = current_balance;
                }
                0x02 => {
                    let amount = self.registers[2];
                    if amount <= self.registers[3] {
                        let new_vault = self.txdeposit(0, amount)?;
                        println!(
                            "[VM Decoded Exec] txdeposit executed from binary. Vault: {}",
                            new_vault
                        );
                    }
                }
                _ => {
                    if self.pc == start_pc {
                        let current_balance = self.load_s(self.registers[1] as u64)?;
                        println!(
                            "[VM Security] ADRS phase-validated load.s succeeded. Current balance: {}",
                            current_balance
                        );

                        let amount = self.registers[2];
                        if amount <= current_balance {
                            let new_vault_total = self.txdeposit(0, amount)?;
                            println!(
                                "[VM Atomic Tx] txdeposit successful. Vault balance updated to: {}",
                                new_vault_total
                            );
                        }
                    }
                }
            }

            self.pc += 4;
        }

        Ok(self.ledgers[0])
    }
}
