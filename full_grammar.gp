contract FullGrammarVaultVulnerable {
    fn execute_vault_ops(wallet_ptr: *sustain i64, harvest_amount: i64) -> i64 {
        ledger pool_liquidity: i64 = 502;
        let current_balance = load.s(wallet_ptr);

        gas(250) {
            atomic {
                let status = txwithdraw(wallet_ptr, harvest_amount);
            }
        }

        pool_liquidity = 500;
        return current_balance;
    }
}
