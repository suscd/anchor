extern crate proc_macro;

use anchor_syn::{Program, ProgramArgs};
use quote::ToTokens;
use syn::parse_macro_input;

/// The `#[program]` attribute defines the module containing all instruction
/// handlers defining all entries into a Solana program.
#[proc_macro_attribute]
pub fn program(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = match args.is_empty() {
        false => Some(parse_macro_input!(args as ProgramArgs)),
        true => None,
    };
    let mut program = parse_macro_input!(input as Program);

    program.args = args;

    program.to_token_stream().into()
}
