pub struct GenpayVM {
    pub memory: Vec<u8>,
    pub pc: usize,
    pub gas_counter: u64,
    pub ledgers: Vec<i64>,
}

impl GenpayVM {
    /// Initialize the Genpay Secure VM with a specified memory capacity
    pub fn new(memory_size: usize) -> Self {
        Self {
            memory: vec![0; memory_size],
            pc: 0,
            gas_counter: 0,
            ledgers: vec![2048, 100, 0, 0], // Default initial state for liquidity pool and vaults
        }
    }

    /// Set the gas budget for transaction execution
    pub fn set_gas(&mut self, gas: u64) {
        self.gas_counter = gas;
    }

    /// Execute bytecode from a starting program counter up to a given length
    pub fn run_from(&mut self, start_pc: usize, length: usize) -> Result<(), String> {
        self.pc = start_pc;
        let end_pc = start_pc + length;

        while self.pc < end_pc {
            // Check gas depletion
            if self.gas_counter == 0 {
                return Err("Out of gas error: gas budget exhausted".into());
            }
            self.gas_counter -= 1;

            let opcode = self.memory[self.pc];
            self.pc += 1;

            match opcode {
                0x01 => {
                    // OP_LOAD_S: Load state reserve
                    println!(
                        "[VM Decoded Exec] load.s executed. Reserve: {}",
                        self.ledgers[0]
                    );
                }
                0x02 => {
                    // OP_TXWITHDRAW: Read 4-byte immediate withdrawal amount
                    if self.pc + 4 > end_pc {
                        return Err("Unexpected end of bytecode for txwithdraw immediate".into());
                    }
                    let amount_bytes = &self.memory[self.pc..self.pc + 4];
                    let amount = u32::from_le_bytes(amount_bytes.try_into().unwrap()) as i64;
                    self.pc += 4;

                    // Deduct from liquidity pool (ledger index 0)
                    if !self.ledgers.is_empty() {
                        self.ledgers[0] -= amount;
                    }

                    println!(
                        "[VM Decoded Exec] txwithdraw executed with amount {}. Remaining pool: {}",
                        amount, self.ledgers[0]
                    );
                }
                0x03 => {
                    // OP_TXDEPOSIT: Read 4-byte immediate deposit amount
                    if self.pc + 4 > end_pc {
                        return Err("Unexpected end of bytecode for txdeposit immediate".into());
                    }
                    let amount_bytes = &self.memory[self.pc..self.pc + 4];
                    let amount = u32::from_le_bytes(amount_bytes.try_into().unwrap()) as i64;
                    self.pc += 4;

                    // Deposit into user vault (ledger index 1)
                    if self.ledgers.len() > 1 {
                        self.ledgers[1] += amount;
                    }

                    println!(
                        "[VM Decoded Exec] txdeposit executed with amount {}. User vault ledger state: {}",
                        amount,
                        if self.ledgers.len() > 1 {
                            self.ledgers[1]
                        } else {
                            0
                        }
                    );
                }
                0x04 => {
                    // OP_RETURN: Terminal or return opcode
                    // Can optionally consume following expression bytes if emitted
                }
                0xFF => {
                    // Padding / Halt Byte
                    break;
                }
                unknown => {
                    return Err(format!("Unknown opcode encountered: 0x{:02X}", unknown));
                }
            }
        }

        Ok(())
    }
}

