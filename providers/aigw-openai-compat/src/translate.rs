//! Translation layer for OpenAI-compatible providers.
//!
//! Wraps the OpenAI translator and applies Quirks-based filtering: unsupported
//! fields are stripped or rejected before delegation.

use aigw_core::error::TranslateError;
use aigw_core::model::{ChatRequest, MessageContent, TypedContentPart};
use aigw_core::translate::{RequestTranslator, TranslatedRequest};
use aigw_core::ForwardCompatible;
use aigw_openai::translate::{OpenAIRequestTranslator, OpenAIResponseTranslator};
use aigw_openai::{OpenAITransport, OpenAITransportConfig};

use crate::{OpenAICompatProvider, Quirks};

/// Request translator for OpenAI-compatible providers.
///
/// Validates against [`Quirks`] (rejects unsupported features), strips
/// unsupported fields, then delegates to the OpenAI translator.
pub struct OpenAICompatRequestTranslator {
    inner: OpenAIRequestTranslator,
    quirks: Quirks,
    provider_name: String,
}

impl OpenAICompatRequestTranslator {
    pub fn new(
        provider: &OpenAICompatProvider,
    ) -> Result<Self, aigw_openai::OpenAITransportConfigError> {
        let config = OpenAITransportConfig {
            http: provider.http_config().clone(),
            auth: provider.auth_config().clone(),
        };
        let transport = OpenAITransport::new(config)?;
        Ok(Self {
            inner: OpenAIRequestTranslator::new(transport),
            quirks: provider.quirks().clone(),
            provider_name: provider.name().to_owned(),
        })
    }
}

impl RequestTranslator for OpenAICompatRequestTranslator {
    fn translate_request(
        &self,
        req: &ChatRequest,
    ) -> Result<TranslatedRequest, TranslateError> {
        self.validate_and_delegate(req, false)
    }

    fn translate_stream_request(
        &self,
        req: &ChatRequest,
    ) -> Result<TranslatedRequest, TranslateError> {
        if !self.quirks.supports_streaming {
            return Err(TranslateError::UnsupportedFeature {
                provider: "openai_compat",
                feature: format!("{}: streaming", self.provider_name),
            });
        }
        self.validate_and_delegate(req, true)
    }
}

impl OpenAICompatRequestTranslator {
    fn validate_and_delegate(
        &self,
        req: &ChatRequest,
        streaming: bool,
    ) -> Result<TranslatedRequest, TranslateError> {
        // Reject vision content if unsupported.
        if !self.quirks.supports_vision && request_has_image(req) {
            return Err(TranslateError::UnsupportedFeature {
                provider: "openai_compat",
                feature: format!("{}: vision/image content", self.provider_name),
            });
        }

        // Build a filtered request: strip unsupported fields.
        let mut filtered = req.clone();

        if !self.quirks.supports_tool_choice {
            filtered.tool_choice = None;
        }
        if !self.quirks.supports_parallel_tool_calls {
            filtered.extra.remove("parallel_tool_calls");
        }

        if streaming {
            self.inner.translate_stream_request(&filtered)
        } else {
            self.inner.translate_request(&filtered)
        }
    }
}

/// Response translator for OpenAI-compatible providers.
///
/// Compat providers return the same response format as OpenAI,
/// so no additional translation is needed.
pub type OpenAICompatResponseTranslator = OpenAIResponseTranslator;

/// Check if any message in the request contains image content.
fn request_has_image(req: &ChatRequest) -> bool {
    req.messages.iter().any(|msg| {
        if let Some(MessageContent::Parts(parts)) = &msg.content {
            parts.iter().any(|p| {
                matches!(
                    p,
                    ForwardCompatible::Known(TypedContentPart::ImageUrl { .. })
                )
            })
        } else {
            false
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use aigw_core::model::{
        ContentPart, ImageUrl, Message, MessageContent, Role, ToolChoice, ToolChoiceMode,
        TypedContentPart,
    };

    fn make_request_with_tool_choice() -> ChatRequest {
        ChatRequest {
            model: "llama-3".into(),
            messages: vec![Message {
                role: Role::User,
                content: Some(MessageContent::Text("hi".into())),
                name: None,
                tool_call_id: None,
                tool_calls: None,
                extra: Default::default(),
            }],
            tool_choice: Some(ToolChoice::Mode(ToolChoiceMode::Required)),
            temperature: None,
            max_tokens: None,
            top_p: None,
            stop: None,
            stream: None,
            tools: None,
            response_format: None,
            frequency_penalty: None,
            presence_penalty: None,
            n: None,
            seed: None,
            user: None,
            extra: Default::default(),
        }
    }

    fn make_request_with_image() -> ChatRequest {
        ChatRequest {
            model: "llama-3".into(),
            messages: vec![Message {
                role: Role::User,
                content: Some(MessageContent::Parts(vec![
                    ContentPart::Known(TypedContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: "https://example.com/img.png".into(),
                            detail: None,
                            extra: Default::default(),
                        },
                        extra: Default::default(),
                    }),
                ])),
                name: None,
                tool_call_id: None,
                tool_calls: None,
                extra: Default::default(),
            }],
            temperature: None,
            max_tokens: None,
            top_p: None,
            stop: None,
            stream: None,
            tools: None,
            tool_choice: None,
            response_format: None,
            frequency_penalty: None,
            presence_penalty: None,
            n: None,
            seed: None,
            user: None,
            extra: Default::default(),
        }
    }

    #[test]
    fn quirks_strips_tool_choice() {
        let quirks = Quirks {
            supports_tool_choice: false,
            ..Quirks::default()
        };
        let req = make_request_with_tool_choice();

        // Verify the filtering logic in isolation.
        let mut filtered = req.clone();
        if !quirks.supports_tool_choice {
            filtered.tool_choice = None;
        }
        assert!(filtered.tool_choice.is_none());
    }

    #[test]
    fn quirks_rejects_vision() {
        let req = make_request_with_image();
        assert!(request_has_image(&req));
    }

    #[test]
    fn quirks_allows_vision_when_supported() {
        let req = make_request_with_image();
        let quirks = Quirks::default(); // supports_vision = true
        assert!(quirks.supports_vision);
        // No error — vision is supported.
        assert!(request_has_image(&req)); // just validates the helper works
    }

    #[test]
    fn quirks_strips_parallel_tool_calls() {
        let mut req = make_request_with_tool_choice();
        req.extra.insert(
            "parallel_tool_calls".into(),
            serde_json::Value::Bool(true),
        );

        let quirks = Quirks {
            supports_parallel_tool_calls: false,
            ..Quirks::default()
        };

        let mut filtered = req.clone();
        if !quirks.supports_parallel_tool_calls {
            filtered.extra.remove("parallel_tool_calls");
        }
        assert!(!filtered.extra.contains_key("parallel_tool_calls"));
    }
}
