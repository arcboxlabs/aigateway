//! Anthropic API client for the AI Gateway.
//!
//! This crate provides a typed Rust client for the [Anthropic API](https://docs.anthropic.com/en/api/messages),
//! including non-streaming and SSE streaming support.
//!
//! # Features
//!
//! - **`claude-code`** — Enables non-standard endpoints used by Claude Code
//!   (`/api/event_logging/batch`, `/v1/oauth/token`).
//!
//! # Quick Start
//!
//! ```no_run
//! use aigw_anthropic::{Client, Transport, TransportConfig, MessagesRequest, Message, MessageContent, Role};
//! use secrecy::SecretString;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let transport = Transport::new(TransportConfig {
//!     api_key: SecretString::from("sk-ant-..."),
//!     ..Default::default()
//! })?;
//! let client = Client::new(transport)?;
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
//! let resp = client.messages(&req).await?;
//! println!("{}", resp.body.id);
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod error;
pub mod rate_limit;
pub mod streaming;
pub mod translate;
pub mod transport;
pub mod types;

pub use client::Client;
pub use error::Error;
pub use rate_limit::{ApiResponse, RateLimitInfo};
pub use transport::{AuthMode, Transport, TransportConfig, TransportConfigError};
pub use types::*;
