//! Google Gemini API client for the AI Gateway.
//!
//! This crate provides a typed Rust client for the [Gemini API](https://ai.google.dev/api/generate-content),
//! including non-streaming and SSE streaming support.
//!
//! # Quick Start
//!
//! ```no_run
//! use aigw_gemini::{Client, ClientConfig, GenerateContentRequest, Content, Part, Role};
//!
//! # async fn example() -> Result<(), aigw_gemini::Error> {
//! let client = Client::new(
//!     ClientConfig::builder().api_key("AIza...").build()
//! )?;
//!
//! let req = GenerateContentRequest::builder()
//!     .model("gemini-2.5-flash")
//!     .contents(vec![Content {
//!         role: Some(Role::User),
//!         parts: vec![Part::text("Hello, Gemini!")],
//!     }])
//!     .build();
//!
//! let response = client.generate_content(&req).await?;
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod error;
pub mod streaming;
pub mod types;

pub use client::{Client, ClientConfig};
pub use error::Error;
pub use types::*;
