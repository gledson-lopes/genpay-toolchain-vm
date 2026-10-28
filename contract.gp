contract EscrowVault {
    fn deposit_funds(wallet_ptr: *sustain i64, amount: i64, limit: i64) -> i64 {
        gas(limit) {
            ledger vault: i64 = b0;
            atomic {
                let current_balance = load.s(wallet_ptr);
                if (amount <= current_balance) {
            vault = txdeposit(wallet_ptr, amount);
        }
            }
            return vault;
        }
    }
}
