use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;

pub struct GenpayCodeGen<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
}

impl<'ctx> GenpayCodeGen<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        Self {
            context,
            module,
            builder,
        }
    }

    pub fn compile_escrow_vault(&mut self) {
        let i64_type = self.context.i64_type();
        let fn_type = i64_type.fn_type(&[i64_type.into()], false);

        let load_s_fn = self.module.add_function("load_s", fn_type, None);
        let txdeposit_fn = self.module.add_function("txdeposit", fn_type, None);

        let vault_fn_type = self.context.void_type().fn_type(&[], false);
        let vault_fn = self
            .module
            .add_function("escrow_vault_entry", vault_fn_type, None);
        let bb = self.context.append_basic_block(vault_fn, "entry");
        self.builder.position_at_end(bb);

        // 1. Call load_s and extract basic value using .unwrap_basic()
        let wallet_ptr_loaded = i64_type.const_int(0x40, false);
        let current_balance_val = self
            .builder
            .build_call(load_s_fn, &[wallet_ptr_loaded.into()], "load_s_call")
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic();

        // 2. Call txdeposit and extract basic value using .unwrap_basic()
        let _txdeposit_res = self
            .builder
            .build_call(
                txdeposit_fn,
                &[current_balance_val.into()],
                "txdeposit_call",
            )
            .unwrap()
            .try_as_basic_value()
            .unwrap_basic();

        self.builder.build_return(None).unwrap();
    }
}

