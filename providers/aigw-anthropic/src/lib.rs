//! Anthropic Messages API client for the AI Gateway.
//!
//! This crate provides a typed Rust client for the [Anthropic Messages API](https://docs.anthropic.com/en/api/messages),
//! including non-streaming and SSE streaming support.
//!
//! # Quick Start
//!
//! ```no_run
//! use aigw_anthropic::{Client, ClientConfig, MessagesRequest, Message, MessageContent, Role};
//!
//! # async fn example() -> Result<(), aigw_anthropic::Error> {
//! let client = Client::new(
//!     ClientConfig::builder().api_key("sk-ant-...").build()
//! )?;
//!
//! let req = MessagesRequest::builder()
//!     .model("claude-sonnet-4-20250514")
//!     .messages(vec![Message {
//!         role: Role::User,
//!         content: MessageContent::Text("Hello, Claude!".into()),
//!     }])
//!     .max_tokens(1024)
//!     .build();
//!
//! let response = client.messages(&req).await?;
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
