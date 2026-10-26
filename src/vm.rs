pub struct GenpayVM {
    pub memory: Vec<u8>,
    pub gas_counter: u64,
    pub ledgers: [i64; 4],
}

impl GenpayVM {
    pub fn new(memory_size: usize) -> Self {
        Self {
            memory: vec![0; memory_size],
            gas_counter: 0,
            ledgers: [0; 4],
        }
    }

    pub fn set_gas(&mut self, limit: u64) {
        self.gas_counter = limit;
    }

    /// Executes a phase-validated sustain load (load.s)
    /// Validates that the pointer bears the required 0x3 sustain capability tag.
    pub fn load_s(&mut self, ptr: u64) -> Result<i64, String> {
        // Check and consume hardware gas (g0)
        if self.gas_counter == 0 {
            return Err("Out of gas (g0)".into());
        }
        self.gas_counter -= 1;

        // Extract the 4-bit phase capability tag from bits 63-60
        let phase_tag = (ptr >> 60) & 0xF;
        if phase_tag != 0x3 {
            return Err(format!(
                "ADRS lifecycle violation: Invalid pointer phase tag 0x{:X} for sustain load (expected 0x3)",
                phase_tag
            ));
        }

        // Extract the lower 60 bits as the physical memory address
        let addr = (ptr & 0x0FFFFFFFFFFFFFFF) as usize;
        if addr + 8 > self.memory.len() {
            return Err("Segmentation fault: Address out of bounds".into());
        }

        // Read 8 bytes (i64) in little-endian format
        let bytes = &self.memory[addr..addr + 8];
        let val = i64::from_le_bytes(bytes.try_into().unwrap());
        Ok(val)
    }

    /// Executes an atomic transaction deposit into a ledger slot
    pub fn txdeposit(&mut self, ledger_index: usize, amount: i64) -> Result<i64, String> {
        if ledger_index >= self.ledgers.len() {
            return Err("Invalid ledger index".into());
        }
        self.ledgers[ledger_index] += amount;
        Ok(self.ledgers[ledger_index])
    }
}

