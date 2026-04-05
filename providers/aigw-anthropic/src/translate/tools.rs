//! Tool translation functions shared between request and response translators.
//!
//! Request direction: canonical (OpenAI format) → Anthropic
//! Response direction: Anthropic → canonical (OpenAI format)

use aigw_core::model::{
    FunctionCall, NamedToolChoice, Tool as CanonicalTool, ToolCall, ToolChoice as CanonicalToolChoice,
    ToolChoiceMode,
};

use crate::types::{Tool as AnthropicTool, ToolChoice as AnthropicToolChoice};

// ─── Request direction: canonical → Anthropic ───────────────────────────────

/// Translate canonical tool definitions to Anthropic format.
///
/// Unwraps the OpenAI `{ type: "function", function: { name, description, parameters } }`
/// wrapper into Anthropic's flat `{ name, description, input_schema }`.
pub fn translate_tools(tools: &[CanonicalTool]) -> Vec<AnthropicTool> {
    tools
        .iter()
        .map(|t| AnthropicTool {
            name: t.function.name.clone(),
            description: t.function.description.clone(),
            input_schema: t
                .function
                .parameters
                .clone()
                .unwrap_or(serde_json::json!({"type": "object"})),
        })
        .collect()
}

/// Translate canonical tool_choice to Anthropic format.
///
/// - `"auto"` → `{ type: "auto" }`
/// - `"none"` → `{ type: "none" }`
/// - `"required"` → `{ type: "any" }`
/// - `Named { name: "X" }` → `{ type: "tool", name: "X" }`
pub fn translate_tool_choice(tc: &CanonicalToolChoice) -> AnthropicToolChoice {
    match tc {
        CanonicalToolChoice::Mode(mode) => match mode {
            ToolChoiceMode::Auto | ToolChoiceMode::Unknown(_) => AnthropicToolChoice::Auto {
                disable_parallel_tool_use: None,
            },
            ToolChoiceMode::None => AnthropicToolChoice::None {
                extra: Default::default(),
            },
            ToolChoiceMode::Required => AnthropicToolChoice::Any {
                disable_parallel_tool_use: None,
            },
        },
        CanonicalToolChoice::Named(NamedToolChoice { function, .. }) => {
            AnthropicToolChoice::Tool {
                name: function.name.clone(),
                disable_parallel_tool_use: None,
            }
        }
        CanonicalToolChoice::Raw(_) => AnthropicToolChoice::Auto {
            disable_parallel_tool_use: None,
        },
    }
}

// ─── Response direction: Anthropic → canonical ──────────────────────────────

/// Convert an Anthropic `tool_use` block into a canonical `ToolCall`.
///
/// Anthropic sends `input` as a JSON object; OpenAI expects `arguments` as a
/// JSON string.
pub fn tool_use_to_canonical(id: &str, name: &str, input: &serde_json::Value) -> ToolCall {
    ToolCall {
        id: id.to_owned(),
        kind: "function".to_owned(),
        function: FunctionCall {
            name: name.to_owned(),
            arguments: serde_json::to_string(input).unwrap_or_default(),
            extra: Default::default(),
        },
        extra: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aigw_core::model::{FunctionDefinition, NamedToolChoiceFunction};

    #[test]
    fn translate_tool_definition() {
        let canonical = CanonicalTool {
            kind: "function".into(),
            function: FunctionDefinition {
                name: "get_weather".into(),
                description: Some("Get weather info".into()),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": { "location": { "type": "string" } }
                })),
                strict: None,
                extra: Default::default(),
            },
            extra: Default::default(),
        };

        let anthropic = translate_tools(&[canonical]);
        assert_eq!(anthropic.len(), 1);
        assert_eq!(anthropic[0].name, "get_weather");
        assert_eq!(anthropic[0].description.as_deref(), Some("Get weather info"));
        assert!(anthropic[0].input_schema.get("properties").is_some());
    }

    #[test]
    fn translate_tool_choice_variants() {
        // auto
        let auto = translate_tool_choice(&CanonicalToolChoice::Mode(ToolChoiceMode::Auto));
        assert!(matches!(auto, AnthropicToolChoice::Auto { .. }));

        // none
        let none = translate_tool_choice(&CanonicalToolChoice::Mode(ToolChoiceMode::None));
        assert!(matches!(none, AnthropicToolChoice::None { .. }));

        // required → any
        let any = translate_tool_choice(&CanonicalToolChoice::Mode(ToolChoiceMode::Required));
        assert!(matches!(any, AnthropicToolChoice::Any { .. }));

        // named
        let named = translate_tool_choice(&CanonicalToolChoice::Named(NamedToolChoice {
            kind: "function".into(),
            function: NamedToolChoiceFunction {
                name: "get_weather".into(),
                extra: Default::default(),
            },
            extra: Default::default(),
        }));
        match named {
            AnthropicToolChoice::Tool { name, .. } => assert_eq!(name, "get_weather"),
            other => panic!("expected Tool, got {other:?}"),
        }
    }

    #[test]
    fn tool_use_roundtrip() {
        let input = serde_json::json!({"location": "San Francisco"});
        let canonical = tool_use_to_canonical("toolu_01", "get_weather", &input);

        assert_eq!(canonical.id, "toolu_01");
        assert_eq!(canonical.kind, "function");
        assert_eq!(canonical.function.name, "get_weather");
        assert_eq!(
            canonical.function.arguments,
            r#"{"location":"San Francisco"}"#
        );
    }
}
