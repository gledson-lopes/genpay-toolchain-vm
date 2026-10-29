contract VulnerableVaultWithoutAtomic {
    fn process_withdrawal(wallet_ptr: *sustain i64, withdrawal_amount: i64) -> i64 {
        gas(300) {
            if (withdrawal_amount <= pool_liquidity) {
                ledger pool_liquidity: i64 = b502;
                let starting_balance = load.s(wallet_ptr);

                // Intentionally missing the `atomic { ... }` wrapper here!
                let status = txwithdraw(wallet_ptr, withdrawal_amount);

                pool_liquidity = 500;
                return starting_balance;
            }
        }
    }
}
