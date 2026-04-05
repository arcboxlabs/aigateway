use std::error::Error;
use std::fmt::{self, Display, Formatter};

use bon::Builder;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

pub use super::responses_output::{ResponseContentPart, ResponseOutputItem};
use super::shared::{JsonObject, json_object_from_value, json_object_is_empty};

pub type ResponseInput = Value;
pub type ResponseConversation = Value;
pub type ResponseContextManagement = Value;
pub type ResponseReasoning = Value;
pub type ResponseInputItem = Value;
pub type ResponsePromptCacheRetention = Value;

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseTool {
    Function {
        name: String,
        parameters: Option<Value>,
        strict: Option<bool>,
        defer_loading: Option<bool>,
        description: Option<String>,
        extra: JsonObject,
    },
    FileSearch {
        vector_store_ids: Option<Vec<String>>,
        filters: Option<Value>,
        max_num_results: Option<u32>,
        ranking_options: Option<Value>,
        extra: JsonObject,
    },
    Computer {
        extra: JsonObject,
    },
    ComputerUsePreview {
        display_height: u32,
        display_width: u32,
        environment: String,
        extra: JsonObject,
    },
    WebSearch {
        filters: Option<Value>,
        search_context_size: Option<String>,
        user_location: Option<Value>,
        extra: JsonObject,
    },
    WebSearch20250826 {
        filters: Option<Value>,
        search_context_size: Option<String>,
        user_location: Option<Value>,
        extra: JsonObject,
    },
    Mcp {
        server_label: String,
        allowed_tools: Option<Value>,
        authorization: Option<String>,
        connector_id: Option<String>,
        defer_loading: Option<bool>,
        headers: Option<JsonObject>,
        require_approval: Option<Value>,
        server_description: Option<String>,
        server_url: Option<String>,
        extra: JsonObject,
    },
    CodeInterpreter {
        container: Option<Value>,
        extra: JsonObject,
    },
    ImageGeneration {
        action: Option<String>,
        background: Option<String>,
        input_fidelity: Option<String>,
        input_image_mask: Option<Value>,
        model: Option<Value>,
        moderation: Option<String>,
        output_compression: Option<u8>,
        output_format: Option<String>,
        partial_images: Option<u8>,
        quality: Option<String>,
        size: Option<String>,
        extra: JsonObject,
    },
    LocalShell {
        extra: JsonObject,
    },
    Custom {
        name: String,
        defer_loading: Option<bool>,
        description: Option<String>,
        format: Option<Value>,
        extra: JsonObject,
    },
    Namespace {
        description: String,
        name: String,
        tools: Vec<ResponseNamespaceTool>,
        extra: JsonObject,
    },
    ToolSearch {
        description: Option<String>,
        execution: Option<String>,
        parameters: Option<Value>,
        extra: JsonObject,
    },
    WebSearchPreview {
        search_content_types: Option<Vec<String>>,
        search_context_size: Option<String>,
        user_location: Option<Value>,
        extra: JsonObject,
    },
    WebSearchPreview20250311 {
        search_content_types: Option<Vec<String>>,
        search_context_size: Option<String>,
        user_location: Option<Value>,
        extra: JsonObject,
    },
    ApplyPatch {
        extra: JsonObject,
    },
    Shell {
        environment: Option<Value>,
        extra: JsonObject,
    },
    Raw(JsonObject),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseNamespaceTool {
    Function {
        name: String,
        defer_loading: Option<bool>,
        description: Option<String>,
        parameters: Option<Value>,
        strict: Option<bool>,
        extra: JsonObject,
    },
    Custom {
        name: String,
        defer_loading: Option<bool>,
        description: Option<String>,
        format: Option<Value>,
        extra: JsonObject,
    },
    Raw(JsonObject),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseToolChoice {
    Mode(ResponseToolChoiceMode),
    AllowedTools {
        mode: ResponseAllowedToolsMode,
        tools: Vec<ResponseTool>,
        extra: JsonObject,
    },
    FileSearch {
        extra: JsonObject,
    },
    WebSearchPreview {
        extra: JsonObject,
    },
    WebSearchPreview20250311 {
        extra: JsonObject,
    },
    Computer {
        extra: JsonObject,
    },
    ComputerUsePreview {
        extra: JsonObject,
    },
    ComputerUse {
        extra: JsonObject,
    },
    CodeInterpreter {
        extra: JsonObject,
    },
    ImageGeneration {
        extra: JsonObject,
    },
    Function {
        name: String,
        extra: JsonObject,
    },
    Mcp {
        server_label: String,
        name: Option<String>,
        extra: JsonObject,
    },
    Custom {
        name: String,
        extra: JsonObject,
    },
    ApplyPatch {
        extra: JsonObject,
    },
    Shell {
        extra: JsonObject,
    },
    Raw(JsonObject),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseToolChoiceMode {
    None,
    Auto,
    Required,
    Unknown(String),
}

impl ResponseToolChoiceMode {
    fn as_str(&self) -> &str {
        match self {
            Self::None => "none",
            Self::Auto => "auto",
            Self::Required => "required",
            Self::Unknown(value) => value.as_str(),
        }
    }
}

impl Serialize for ResponseToolChoiceMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ResponseToolChoiceMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "none" => Self::None,
            "auto" => Self::Auto,
            "required" => Self::Required,
            _ => Self::Unknown(value),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseAllowedToolsMode {
    Auto,
    Required,
    Unknown(String),
}

impl ResponseAllowedToolsMode {
    fn as_str(&self) -> &str {
        match self {
            Self::Auto => "auto",
            Self::Required => "required",
            Self::Unknown(value) => value.as_str(),
        }
    }
}

impl Serialize for ResponseAllowedToolsMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ResponseAllowedToolsMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "auto" => Self::Auto,
            "required" => Self::Required,
            _ => Self::Unknown(value),
        })
    }
}

impl Serialize for ResponseNamespaceTool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Function {
                name,
                defer_loading,
                description,
                parameters,
                strict,
                extra,
            } => KnownResponseNamespaceTool::Function {
                name: name.clone(),
                defer_loading: *defer_loading,
                description: description.clone(),
                parameters: parameters.clone(),
                strict: *strict,
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Custom {
                name,
                defer_loading,
                description,
                format,
                extra,
            } => KnownResponseNamespaceTool::Custom {
                name: name.clone(),
                defer_loading: *defer_loading,
                description: description.clone(),
                format: format.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Raw(raw) => raw.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ResponseNamespaceTool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if let Ok(typed) = serde_json::from_value::<KnownResponseNamespaceTool>(value.clone()) {
            return Ok(match typed {
                KnownResponseNamespaceTool::Function {
                    name,
                    defer_loading,
                    description,
                    parameters,
                    strict,
                    extra,
                } => Self::Function {
                    name,
                    defer_loading,
                    description,
                    parameters,
                    strict,
                    extra,
                },
                KnownResponseNamespaceTool::Custom {
                    name,
                    defer_loading,
                    description,
                    format,
                    extra,
                } => Self::Custom {
                    name,
                    defer_loading,
                    description,
                    format,
                    extra,
                },
            });
        }

        Ok(Self::Raw(
            json_object_from_value(value).map_err(D::Error::custom)?,
        ))
    }
}

impl Serialize for ResponseTool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Function {
                name,
                parameters,
                strict,
                defer_loading,
                description,
                extra,
            } => KnownResponseTool::Function {
                name: name.clone(),
                parameters: parameters.clone(),
                strict: *strict,
                defer_loading: *defer_loading,
                description: description.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::FileSearch {
                vector_store_ids,
                filters,
                max_num_results,
                ranking_options,
                extra,
            } => KnownResponseTool::FileSearch {
                vector_store_ids: vector_store_ids.clone(),
                filters: filters.clone(),
                max_num_results: *max_num_results,
                ranking_options: ranking_options.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Computer { extra } => KnownResponseTool::Computer {
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::ComputerUsePreview {
                display_height,
                display_width,
                environment,
                extra,
            } => KnownResponseTool::ComputerUsePreview {
                display_height: *display_height,
                display_width: *display_width,
                environment: environment.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::WebSearch {
                filters,
                search_context_size,
                user_location,
                extra,
            } => KnownResponseTool::WebSearch {
                filters: filters.clone(),
                search_context_size: search_context_size.clone(),
                user_location: user_location.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::WebSearch20250826 {
                filters,
                search_context_size,
                user_location,
                extra,
            } => KnownResponseTool::WebSearch20250826 {
                filters: filters.clone(),
                search_context_size: search_context_size.clone(),
                user_location: user_location.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Mcp {
                server_label,
                allowed_tools,
                authorization,
                connector_id,
                defer_loading,
                headers,
                require_approval,
                server_description,
                server_url,
                extra,
            } => KnownResponseTool::Mcp {
                server_label: server_label.clone(),
                allowed_tools: allowed_tools.clone(),
                authorization: authorization.clone(),
                connector_id: connector_id.clone(),
                defer_loading: *defer_loading,
                headers: headers.clone(),
                require_approval: require_approval.clone(),
                server_description: server_description.clone(),
                server_url: server_url.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::CodeInterpreter { container, extra } => KnownResponseTool::CodeInterpreter {
                container: container.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::ImageGeneration {
                action,
                background,
                input_fidelity,
                input_image_mask,
                model,
                moderation,
                output_compression,
                output_format,
                partial_images,
                quality,
                size,
                extra,
            } => KnownResponseTool::ImageGeneration {
                action: action.clone(),
                background: background.clone(),
                input_fidelity: input_fidelity.clone(),
                input_image_mask: input_image_mask.clone(),
                model: model.clone(),
                moderation: moderation.clone(),
                output_compression: *output_compression,
                output_format: output_format.clone(),
                partial_images: *partial_images,
                quality: quality.clone(),
                size: size.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::LocalShell { extra } => KnownResponseTool::LocalShell {
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Custom {
                name,
                defer_loading,
                description,
                format,
                extra,
            } => KnownResponseTool::Custom {
                name: name.clone(),
                defer_loading: *defer_loading,
                description: description.clone(),
                format: format.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Namespace {
                description,
                name,
                tools,
                extra,
            } => KnownResponseTool::Namespace {
                description: description.clone(),
                name: name.clone(),
                tools: tools.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::ToolSearch {
                description,
                execution,
                parameters,
                extra,
            } => KnownResponseTool::ToolSearch {
                description: description.clone(),
                execution: execution.clone(),
                parameters: parameters.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::WebSearchPreview {
                search_content_types,
                search_context_size,
                user_location,
                extra,
            } => KnownResponseTool::WebSearchPreview {
                search_content_types: search_content_types.clone(),
                search_context_size: search_context_size.clone(),
                user_location: user_location.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::WebSearchPreview20250311 {
                search_content_types,
                search_context_size,
                user_location,
                extra,
            } => KnownResponseTool::WebSearchPreview20250311 {
                search_content_types: search_content_types.clone(),
                search_context_size: search_context_size.clone(),
                user_location: user_location.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::ApplyPatch { extra } => KnownResponseTool::ApplyPatch {
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Shell { environment, extra } => KnownResponseTool::Shell {
                environment: environment.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Raw(raw) => raw.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ResponseTool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if let Ok(typed) = serde_json::from_value::<KnownResponseTool>(value.clone()) {
            return Ok(match typed {
                KnownResponseTool::Function {
                    name,
                    parameters,
                    strict,
                    defer_loading,
                    description,
                    extra,
                } => Self::Function {
                    name,
                    parameters,
                    strict,
                    defer_loading,
                    description,
                    extra,
                },
                KnownResponseTool::FileSearch {
                    vector_store_ids,
                    filters,
                    max_num_results,
                    ranking_options,
                    extra,
                } => Self::FileSearch {
                    vector_store_ids,
                    filters,
                    max_num_results,
                    ranking_options,
                    extra,
                },
                KnownResponseTool::Computer { extra } => Self::Computer { extra },
                KnownResponseTool::ComputerUsePreview {
                    display_height,
                    display_width,
                    environment,
                    extra,
                } => Self::ComputerUsePreview {
                    display_height,
                    display_width,
                    environment,
                    extra,
                },
                KnownResponseTool::WebSearch {
                    filters,
                    search_context_size,
                    user_location,
                    extra,
                } => Self::WebSearch {
                    filters,
                    search_context_size,
                    user_location,
                    extra,
                },
                KnownResponseTool::WebSearch20250826 {
                    filters,
                    search_context_size,
                    user_location,
                    extra,
                } => Self::WebSearch20250826 {
                    filters,
                    search_context_size,
                    user_location,
                    extra,
                },
                KnownResponseTool::Mcp {
                    server_label,
                    allowed_tools,
                    authorization,
                    connector_id,
                    defer_loading,
                    headers,
                    require_approval,
                    server_description,
                    server_url,
                    extra,
                } => Self::Mcp {
                    server_label,
                    allowed_tools,
                    authorization,
                    connector_id,
                    defer_loading,
                    headers,
                    require_approval,
                    server_description,
                    server_url,
                    extra,
                },
                KnownResponseTool::CodeInterpreter { container, extra } => {
                    Self::CodeInterpreter { container, extra }
                }
                KnownResponseTool::ImageGeneration {
                    action,
                    background,
                    input_fidelity,
                    input_image_mask,
                    model,
                    moderation,
                    output_compression,
                    output_format,
                    partial_images,
                    quality,
                    size,
                    extra,
                } => Self::ImageGeneration {
                    action,
                    background,
                    input_fidelity,
                    input_image_mask,
                    model,
                    moderation,
                    output_compression,
                    output_format,
                    partial_images,
                    quality,
                    size,
                    extra,
                },
                KnownResponseTool::LocalShell { extra } => Self::LocalShell { extra },
                KnownResponseTool::Custom {
                    name,
                    defer_loading,
                    description,
                    format,
                    extra,
                } => Self::Custom {
                    name,
                    defer_loading,
                    description,
                    format,
                    extra,
                },
                KnownResponseTool::Namespace {
                    description,
                    name,
                    tools,
                    extra,
                } => Self::Namespace {
                    description,
                    name,
                    tools,
                    extra,
                },
                KnownResponseTool::ToolSearch {
                    description,
                    execution,
                    parameters,
                    extra,
                } => Self::ToolSearch {
                    description,
                    execution,
                    parameters,
                    extra,
                },
                KnownResponseTool::WebSearchPreview {
                    search_content_types,
                    search_context_size,
                    user_location,
                    extra,
                } => Self::WebSearchPreview {
                    search_content_types,
                    search_context_size,
                    user_location,
                    extra,
                },
                KnownResponseTool::WebSearchPreview20250311 {
                    search_content_types,
                    search_context_size,
                    user_location,
                    extra,
                } => Self::WebSearchPreview20250311 {
                    search_content_types,
                    search_context_size,
                    user_location,
                    extra,
                },
                KnownResponseTool::ApplyPatch { extra } => Self::ApplyPatch { extra },
                KnownResponseTool::Shell { environment, extra } => {
                    Self::Shell { environment, extra }
                }
            });
        }

        Ok(Self::Raw(
            json_object_from_value(value).map_err(D::Error::custom)?,
        ))
    }
}

impl Serialize for ResponseToolChoice {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Mode(mode) => mode.serialize(serializer),
            Self::AllowedTools { mode, tools, extra } => KnownResponseToolChoice::AllowedTools {
                mode: mode.clone(),
                tools: tools.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::FileSearch { extra } => KnownResponseToolChoice::FileSearch {
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::WebSearchPreview { extra } => KnownResponseToolChoice::WebSearchPreview {
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::WebSearchPreview20250311 { extra } => {
                KnownResponseToolChoice::WebSearchPreview20250311 {
                    extra: extra.clone(),
                }
                .serialize(serializer)
            }
            Self::Computer { extra } => KnownResponseToolChoice::Computer {
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::ComputerUsePreview { extra } => KnownResponseToolChoice::ComputerUsePreview {
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::ComputerUse { extra } => KnownResponseToolChoice::ComputerUse {
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::CodeInterpreter { extra } => KnownResponseToolChoice::CodeInterpreter {
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::ImageGeneration { extra } => KnownResponseToolChoice::ImageGeneration {
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Function { name, extra } => KnownResponseToolChoice::Function {
                name: name.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Mcp {
                server_label,
                name,
                extra,
            } => KnownResponseToolChoice::Mcp {
                server_label: server_label.clone(),
                name: name.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Custom { name, extra } => KnownResponseToolChoice::Custom {
                name: name.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::ApplyPatch { extra } => KnownResponseToolChoice::ApplyPatch {
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Shell { extra } => KnownResponseToolChoice::Shell {
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Raw(raw) => raw.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ResponseToolChoice {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        match value {
            Value::String(raw) => Ok(Self::Mode(ResponseToolChoiceMode::from(raw))),
            Value::Object(object) => {
                if let Ok(typed) =
                    serde_json::from_value::<KnownResponseToolChoice>(Value::Object(object.clone()))
                {
                    return Ok(match typed {
                        KnownResponseToolChoice::AllowedTools { mode, tools, extra } => {
                            Self::AllowedTools { mode, tools, extra }
                        }
                        KnownResponseToolChoice::FileSearch { extra } => Self::FileSearch { extra },
                        KnownResponseToolChoice::WebSearchPreview { extra } => {
                            Self::WebSearchPreview { extra }
                        }
                        KnownResponseToolChoice::WebSearchPreview20250311 { extra } => {
                            Self::WebSearchPreview20250311 { extra }
                        }
                        KnownResponseToolChoice::Computer { extra } => Self::Computer { extra },
                        KnownResponseToolChoice::ComputerUsePreview { extra } => {
                            Self::ComputerUsePreview { extra }
                        }
                        KnownResponseToolChoice::ComputerUse { extra } => {
                            Self::ComputerUse { extra }
                        }
                        KnownResponseToolChoice::CodeInterpreter { extra } => {
                            Self::CodeInterpreter { extra }
                        }
                        KnownResponseToolChoice::ImageGeneration { extra } => {
                            Self::ImageGeneration { extra }
                        }
                        KnownResponseToolChoice::Function { name, extra } => {
                            Self::Function { name, extra }
                        }
                        KnownResponseToolChoice::Mcp {
                            server_label,
                            name,
                            extra,
                        } => Self::Mcp {
                            server_label,
                            name,
                            extra,
                        },
                        KnownResponseToolChoice::Custom { name, extra } => {
                            Self::Custom { name, extra }
                        }
                        KnownResponseToolChoice::ApplyPatch { extra } => Self::ApplyPatch { extra },
                        KnownResponseToolChoice::Shell { extra } => Self::Shell { extra },
                    });
                }

                Ok(Self::Raw(object.into_iter().collect()))
            }
            _ => Err(D::Error::custom("expected string or object")),
        }
    }
}

impl From<String> for ResponseToolChoiceMode {
    fn from(value: String) -> Self {
        match value.as_str() {
            "none" => Self::None,
            "auto" => Self::Auto,
            "required" => Self::Required,
            _ => Self::Unknown(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
enum KnownResponseNamespaceTool {
    #[serde(rename = "function")]
    Function {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        defer_loading: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parameters: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        strict: Option<bool>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "custom")]
    Custom {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        defer_loading: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        format: Option<Value>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
enum KnownResponseTool {
    #[serde(rename = "function")]
    Function {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parameters: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        strict: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        defer_loading: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "file_search")]
    FileSearch {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        vector_store_ids: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filters: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_num_results: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ranking_options: Option<Value>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "computer")]
    Computer {
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "computer_use_preview")]
    ComputerUsePreview {
        display_height: u32,
        display_width: u32,
        environment: String,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "web_search")]
    WebSearch {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filters: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        search_context_size: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        user_location: Option<Value>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "web_search_2025_08_26")]
    WebSearch20250826 {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filters: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        search_context_size: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        user_location: Option<Value>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "mcp")]
    Mcp {
        server_label: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        allowed_tools: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        authorization: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        connector_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        defer_loading: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        headers: Option<JsonObject>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        require_approval: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        server_description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        server_url: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "code_interpreter")]
    CodeInterpreter {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        container: Option<Value>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "image_generation")]
    ImageGeneration {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        action: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        background: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        input_fidelity: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        input_image_mask: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        moderation: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output_compression: Option<u8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output_format: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        partial_images: Option<u8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        quality: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        size: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "local_shell")]
    LocalShell {
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "custom")]
    Custom {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        defer_loading: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        format: Option<Value>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "namespace")]
    Namespace {
        description: String,
        name: String,
        tools: Vec<ResponseNamespaceTool>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "tool_search")]
    ToolSearch {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        execution: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parameters: Option<Value>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "web_search_preview")]
    WebSearchPreview {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        search_content_types: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        search_context_size: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        user_location: Option<Value>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "web_search_preview_2025_03_11")]
    WebSearchPreview20250311 {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        search_content_types: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        search_context_size: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        user_location: Option<Value>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "apply_patch")]
    ApplyPatch {
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "shell")]
    Shell {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        environment: Option<Value>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
enum KnownResponseToolChoice {
    #[serde(rename = "allowed_tools")]
    AllowedTools {
        mode: ResponseAllowedToolsMode,
        tools: Vec<ResponseTool>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "file_search")]
    FileSearch {
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "web_search_preview")]
    WebSearchPreview {
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "web_search_preview_2025_03_11")]
    WebSearchPreview20250311 {
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "computer")]
    Computer {
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "computer_use_preview")]
    ComputerUsePreview {
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "computer_use")]
    ComputerUse {
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "code_interpreter")]
    CodeInterpreter {
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "image_generation")]
    ImageGeneration {
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "function")]
    Function {
        name: String,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "mcp")]
    Mcp {
        server_label: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "custom")]
    Custom {
        name: String,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "apply_patch")]
    ApplyPatch {
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    #[serde(rename = "shell")]
    Shell {
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Builder)]
#[builder(on(String, into))]
pub struct ResponseCreateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation: Option<ResponseConversation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_management: Option<ResponseContextManagement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<ResponseInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonObject>,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_cache_retention: Option<ResponsePromptCacheRetention>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ResponseReasoning>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safety_identifier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<ResponseStreamOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<ResponseTextConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ResponseToolChoice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ResponseTool>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    #[builder(default)]
    pub extra: JsonObject,
}

impl ResponseCreateRequest {
    pub fn validate(&self) -> Result<(), ResponseCreateRequestError> {
        validate_previous_response_id_and_conversation(
            self.previous_response_id.as_deref(),
            self.conversation.as_ref(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseCreateRequestError {
    PreviousResponseIdAndConversationConflict,
}

impl Display for ResponseCreateRequestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::PreviousResponseIdAndConversationConflict => {
                f.write_str("`previous_response_id` and `conversation` cannot both be set")
            }
        }
    }
}

impl Error for ResponseCreateRequestError {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Builder)]
#[builder(on(String, into))]
pub struct ResponseInputTokensRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation: Option<ResponseConversation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<ResponseInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ResponseReasoning>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<ResponseTextConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ResponseToolChoice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ResponseTool>>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    #[builder(default)]
    pub extra: JsonObject,
}

impl ResponseInputTokensRequest {
    pub fn validate(&self) -> Result<(), ResponseCreateRequestError> {
        validate_previous_response_id_and_conversation(
            self.previous_response_id.as_deref(),
            self.conversation.as_ref(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseInputTokensResponse {
    pub object: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Builder)]
#[builder(on(String, into))]
pub struct ResponseCompactRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<ResponseInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<Value>,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    #[builder(default)]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseTextConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseStreamOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_obfuscation: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_usage: Option<bool>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseObject {
    pub id: String,
    pub object: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation: Option<ResponseConversation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incomplete_details: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonObject>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<Vec<ResponseOutputItem>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_cache_retention: Option<ResponsePromptCacheRetention>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ResponseReasoning>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safety_identifier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<ResponseTextConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ResponseToolChoice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ResponseTool>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<ResponseUsage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseUsage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens_details: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens_details: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCompaction {
    pub id: String,
    pub object: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<Vec<ResponseOutputItem>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<ResponseUsage>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseInputItemsPage {
    pub object: String,
    pub data: Vec<ResponseInputItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseRetrieveStreamQuery {
    pub stream: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,
}

impl Default for ResponseRetrieveStreamQuery {
    fn default() -> Self {
        Self {
            stream: true,
            starting_after: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseStreamEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

fn validate_previous_response_id_and_conversation(
    previous_response_id: Option<&str>,
    conversation: Option<&ResponseConversation>,
) -> Result<(), ResponseCreateRequestError> {
    if previous_response_id.is_some() && conversation.is_some() {
        return Err(ResponseCreateRequestError::PreviousResponseIdAndConversationConflict);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        ResponseAllowedToolsMode, ResponseCompactRequest, ResponseCreateRequest,
        ResponseInputTokensRequest, ResponseRetrieveStreamQuery, ResponseStreamEvent, ResponseTool,
        ResponseToolChoice, ResponseToolChoiceMode,
    };

    #[test]
    fn validate_rejects_previous_response_and_conversation() {
        let mut request = ResponseCreateRequest::builder()
            .model("gpt-4.1")
            .input(serde_json::json!("hello"))
            .build();
        request.conversation = Some(serde_json::json!({"id":"conv_123"}));
        request.previous_response_id = Some("resp_123".to_string());

        assert!(request.validate().is_err());
    }

    #[test]
    fn input_tokens_validate_rejects_previous_response_and_conversation() {
        let mut request = ResponseInputTokensRequest::builder()
            .model("gpt-4.1")
            .build();
        request.conversation = Some(serde_json::json!({"id":"conv_123"}));
        request.previous_response_id = Some("resp_123".to_string());

        assert!(request.validate().is_err());
    }

    #[test]
    fn response_stream_event_preserves_unknown_fields() {
        let event: ResponseStreamEvent = serde_json::from_str(
            r#"{
                "type":"response.output_text.delta",
                "delta":"hi",
                "obfuscation":"xx"
            }"#,
        )
        .unwrap();

        assert_eq!(event.event_type, "response.output_text.delta");
        assert_eq!(event.extra.get("obfuscation").unwrap(), "xx");
    }

    #[test]
    fn retrieve_stream_query_defaults_stream_true() {
        let query = ResponseRetrieveStreamQuery::default();
        assert!(query.stream);
        assert!(query.starting_after.is_none());
    }

    #[test]
    fn response_create_request_builder_sets_model() {
        let request = ResponseCreateRequest::builder().model("gpt-4.1").build();
        assert_eq!(request.model, "gpt-4.1");
        assert!(request.input.is_none());
    }

    #[test]
    fn response_compact_request_builder_sets_model() {
        let request = ResponseCompactRequest::builder()
            .model("gpt-5.1-codex-max")
            .build();
        assert_eq!(request.model, "gpt-5.1-codex-max");
        assert!(request.input.is_none());
    }

    #[test]
    fn response_tool_deserializes_known_function_tool() {
        let tool: ResponseTool = serde_json::from_str(
            r#"{
                "type":"function",
                "name":"get_weather",
                "description":"weather lookup",
                "strict":true,
                "x_provider":"compat"
            }"#,
        )
        .unwrap();

        match tool {
            ResponseTool::Function {
                name,
                strict,
                extra,
                ..
            } => {
                assert_eq!(name, "get_weather");
                assert_eq!(strict, Some(true));
                assert_eq!(extra.get("x_provider").unwrap(), "compat");
            }
            other => panic!("expected function tool, got {other:?}"),
        }
    }

    #[test]
    fn response_tool_preserves_unknown_tool_type() {
        let tool: ResponseTool = serde_json::from_str(
            r#"{
                "type":"deep_research_preview",
                "plan":"fast"
            }"#,
        )
        .unwrap();

        let reserialized = serde_json::to_value(&tool).unwrap();
        assert_eq!(reserialized["type"], "deep_research_preview");
        assert_eq!(reserialized["plan"], "fast");
    }

    #[test]
    fn response_tool_round_trips_versioned_web_search_type() {
        let tool: ResponseTool = serde_json::from_str(
            r#"{
                "type":"web_search_2025_08_26",
                "search_context_size":"medium"
            }"#,
        )
        .unwrap();

        let reserialized = serde_json::to_value(&tool).unwrap();
        assert_eq!(reserialized["type"], "web_search_2025_08_26");
        assert_eq!(reserialized["search_context_size"], "medium");
    }

    #[test]
    fn response_tool_choice_deserializes_allowed_tools_with_nested_typed_tools() {
        let choice: ResponseToolChoice = serde_json::from_str(
            r#"{
                "type":"allowed_tools",
                "mode":"required",
                "tools":[
                    {"type":"function","name":"get_weather"},
                    {"type":"mcp","server_label":"deepwiki"}
                ],
                "x_trace":"123"
            }"#,
        )
        .unwrap();

        match choice {
            ResponseToolChoice::AllowedTools { mode, tools, extra } => {
                assert_eq!(mode, ResponseAllowedToolsMode::Required);
                assert!(matches!(tools[0], ResponseTool::Function { .. }));
                assert!(matches!(tools[1], ResponseTool::Mcp { .. }));
                assert_eq!(extra.get("x_trace").unwrap(), "123");
            }
            other => panic!("expected allowed_tools choice, got {other:?}"),
        }
    }

    #[test]
    fn response_tool_choice_unknown_string_mode_round_trips() {
        let choice: ResponseToolChoice = serde_json::from_str(r#""parallel_required""#).unwrap();
        assert_eq!(
            choice,
            ResponseToolChoice::Mode(ResponseToolChoiceMode::Unknown(
                "parallel_required".to_owned()
            ))
        );
        assert_eq!(
            serde_json::to_string(&choice).unwrap(),
            r#""parallel_required""#
        );
    }

    #[test]
    fn response_tool_choice_preserves_unknown_object_type() {
        let choice: ResponseToolChoice = serde_json::from_str(
            r#"{
                "type":"new_builtin_tool",
                "tier":"beta"
            }"#,
        )
        .unwrap();

        let reserialized = serde_json::to_value(&choice).unwrap();
        assert_eq!(reserialized["type"], "new_builtin_tool");
        assert_eq!(reserialized["tier"], "beta");
    }
}
