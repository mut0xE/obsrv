//! Token Program Module
//!
//! This module provides decoders for Solana's token programs:
//! - SPL Token: The original token standard (TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA)
//! - Token2022: Extended token program with additional features (TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb)
//!
//! # Architecture
//!
//! - `core.rs`: Shared instruction decoders used by both SPL Token and Token2022
//! - `spl.rs`: SPL Token-specific decoder and routing
//! - `token2022.rs`: Token2022-specific decoder with extensions
//! - `token.rs`: Public API for decoding both token programs

pub mod core;
pub mod spl;
pub mod token;
pub mod token2022;

pub use token::*;
