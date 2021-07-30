use crate::Error;
use quote::quote;

pub fn generate(error: Error) -> proc_macro2::TokenStream {
    let error_enum = &error.raw_enum;
    let enum_name = &error.ident;

    let offset = match &error.args {
        None => quote! { anchor_lang::__private::ERROR_CODE_OFFSET},
        Some(args) => {
            let offset = &args.offset;
            quote! { #offset }
        }
    };

    let display_impl = generate_display_impl(&error);
    let from_u32_impl = generate_u32_conversion(&error, &offset);

    quote! {
        /// Anchor generated Result to be used as the return type for the
        /// program.
        pub type Result<T> = std::result::Result<T, Error>;

        /// Anchor generated error allowing one to easily return a
        /// `ProgramError` or a custom, user defined error code by utilizing
        /// its `From` implementation.
        #[doc(hidden)]
        #[derive(thiserror::Error, Debug)]
        pub enum Error {
            #[error(transparent)]
            ProgramError(#[from] anchor_lang::solana_program::program_error::ProgramError),
            #[error(transparent)]
            ErrorCode(#[from] #enum_name),
        }

        #[derive(std::fmt::Debug, Clone, Copy)]
        #[repr(u32)]
        #error_enum

        impl std::error::Error for #enum_name {}

        #display_impl

        #from_u32_impl

        impl std::convert::From<Error> for anchor_lang::solana_program::program_error::ProgramError {
            fn from(e: Error) -> anchor_lang::solana_program::program_error::ProgramError {
                match e {
                    Error::ProgramError(e) => e,
                    Error::ErrorCode(c) => anchor_lang::solana_program::program_error::ProgramError::Custom(c as u32 + #offset),
                }
            }
        }

        impl std::convert::From<#enum_name> for anchor_lang::solana_program::program_error::ProgramError {
            fn from(e: #enum_name) -> anchor_lang::solana_program::program_error::ProgramError {
                let err: Error = e.into();
                err.into()
            }
        }
    }
}

fn generate_display_impl(error: &Error) -> proc_macro2::TokenStream {
    let enum_name = &error.ident;

    // Each arm of the `match` statement for implementing `std::fmt::Display`
    // on the user defined error code.
    let variant_dispatch: Vec<proc_macro2::TokenStream> = error
        .raw_enum
        .variants
        .iter()
        .enumerate()
        .map(|(idx, variant)| {
            let ident = &variant.ident;
            let error_code = &error.codes[idx];
            let msg = match &error_code.msg {
                None => {
                    quote! {
                        <Self as std::fmt::Debug>::fmt(self, fmt)
                    }
                }
                Some(msg) => {
                    quote! {
                        write!(fmt, #msg)
                    }
                }
            };
            quote! {
                #enum_name::#ident => #msg
            }
        })
        .collect();

    quote! {
        impl std::fmt::Display for #enum_name {
            fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                match self {
                    #(#variant_dispatch),*
                }
            }
        }
    }
}

fn generate_u32_conversion(
    error: &Error,
    offset: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let enum_name = &error.ident;

    // Each constant defined to map the error code back to the enum variant
    let (code_map, code_dispatch): (Vec<_>, Vec<_>) = error
        .codes
        .iter()
        .map(|code| {
            let number = code.id;
            let ident = &code.ident;

            let map_entry = quote! {
                pub const #ident: u32 = #number + #offset;
            };

            let dispatch_entry = quote! {
                __error_code_map::#ident => Ok(#enum_name::#ident)
            };

            (map_entry, dispatch_entry)
        })
        .unzip();

    quote! {
        #[allow(non_upper_case_globals)]
        mod __error_code_map {
            #(#code_map)*
        }

        impl std::convert::TryFrom<u32> for #enum_name {
            type Error = ();

            fn try_from(value: u32) -> std::result::Result<Self, Self::Error> {
                match value {
                    #(#code_dispatch),*,
                    _ => Err(())
                }
            }
        }
    }
}
