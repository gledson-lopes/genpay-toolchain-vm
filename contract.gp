contract EscrowVault {
    fn withdraw_funds(wallet_ptr: *sustain i64, harvest_amount: i64, limit: i64) -> i64 {
        gas(limit) {
            ledger pool_liquidity: i64;
            atomic {
                let current_balance = load.s(wallet_ptr);
                if (harvest_amount <= pool_liquidity) {
                    pool_liquidity = txwithdraw(wallet_ptr, harvest_amount);
                }
            }
            return pool_liquidity;
        }
    }
}
