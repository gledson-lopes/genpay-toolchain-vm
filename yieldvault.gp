contract YieldVault {
    fn harvest_yield(account_ptr: *sustain i64, harvest_amount: i64, limit: i64) -> i64 {
        gas(limit) {
            ledger pool: i64 = b0;
            atomic {
                let pool_liquidity = load.s(account_ptr);
                pool = txwithdraw(account_ptr, harvest_amount);
            }
            return pool;
        }
    }
}
