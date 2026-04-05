use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::shared::{JsonObject, json_object_from_value, json_object_is_empty};

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseContentPart {
    OutputText {
        annotations: Option<Vec<ResponseOutputTextAnnotation>>,
        logprobs: Option<Vec<Value>>,
        text: String,
        extra: JsonObject,
    },
    Refusal {
        refusal: String,
        extra: JsonObject,
    },
    Raw(JsonObject),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseOutputTextAnnotation {
    FileCitation {
        file_id: String,
        filename: String,
        index: i64,
        extra: JsonObject,
    },
    UrlCitation {
        end_index: i64,
        start_index: i64,
        title: String,
        url: String,
        extra: JsonObject,
    },
    ContainerFileCitation {
        container_id: String,
        end_index: i64,
        file_id: String,
        filename: String,
        start_index: i64,
        extra: JsonObject,
    },
    FilePath {
        file_id: String,
        index: i64,
        extra: JsonObject,
    },
    Raw(JsonObject),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseReasoningSummaryPart {
    SummaryText { text: String, extra: JsonObject },
    Raw(JsonObject),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseReasoningContentPart {
    ReasoningText { text: String, extra: JsonObject },
    Raw(JsonObject),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseFileSearchResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attributes: Option<JsonObject>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseSafetyCheck {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseCodeInterpreterOutput {
    Logs { logs: String, extra: JsonObject },
    Image { url: String, extra: JsonObject },
    Raw(JsonObject),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseShellAction {
    pub commands: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_length: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<i64>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseShellCallOutcome {
    Timeout { extra: JsonObject },
    Exit { exit_code: i64, extra: JsonObject },
    Raw(JsonObject),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseShellCallOutputContent {
    pub outcome: ResponseShellCallOutcome,
    pub stderr: String,
    pub stdout: String,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseOutputItem {
    Message {
        id: String,
        content: Vec<ResponseContentPart>,
        role: String,
        status: Option<String>,
        phase: Option<String>,
        extra: JsonObject,
    },
    FileSearchCall {
        id: String,
        queries: Vec<String>,
        status: Option<String>,
        results: Option<Vec<ResponseFileSearchResult>>,
        extra: JsonObject,
    },
    ComputerCall {
        id: String,
        call_id: String,
        pending_safety_checks: Option<Vec<ResponseSafetyCheck>>,
        status: Option<String>,
        action: Option<Value>,
        extra: JsonObject,
    },
    ComputerCallOutput {
        call_id: String,
        output: Value,
        id: Option<String>,
        acknowledged_safety_checks: Option<Vec<ResponseSafetyCheck>>,
        status: Option<String>,
        extra: JsonObject,
    },
    WebSearchCall {
        id: String,
        action: Option<Value>,
        status: Option<String>,
        extra: JsonObject,
    },
    FunctionCall {
        arguments: String,
        call_id: String,
        name: String,
        id: Option<String>,
        namespace: Option<String>,
        status: Option<String>,
        extra: JsonObject,
    },
    FunctionCallOutput {
        call_id: String,
        output: Value,
        id: Option<String>,
        status: Option<String>,
        extra: JsonObject,
    },
    ToolSearchCall {
        arguments: Option<Value>,
        id: Option<String>,
        call_id: Option<String>,
        execution: Option<String>,
        status: Option<String>,
        extra: JsonObject,
    },
    ToolSearchOutput {
        tools: Vec<Value>,
        id: Option<String>,
        call_id: Option<String>,
        execution: Option<String>,
        status: Option<String>,
        extra: JsonObject,
    },
    Reasoning {
        id: String,
        summary: Vec<ResponseReasoningSummaryPart>,
        content: Option<Vec<ResponseReasoningContentPart>>,
        encrypted_content: Option<String>,
        status: Option<String>,
        extra: JsonObject,
    },
    Compaction {
        encrypted_content: String,
        id: Option<String>,
        extra: JsonObject,
    },
    ImageGenerationCall {
        id: String,
        result: Option<String>,
        status: Option<String>,
        extra: JsonObject,
    },
    CodeInterpreterCall {
        id: String,
        code: Option<String>,
        container_id: Option<String>,
        outputs: Option<Vec<ResponseCodeInterpreterOutput>>,
        status: Option<String>,
        extra: JsonObject,
    },
    LocalShellCall {
        id: String,
        action: Option<Value>,
        call_id: String,
        status: Option<String>,
        extra: JsonObject,
    },
    LocalShellCallOutput {
        id: String,
        output: String,
        status: Option<String>,
        extra: JsonObject,
    },
    ShellCall {
        action: Option<ResponseShellAction>,
        call_id: String,
        id: Option<String>,
        environment: Option<Value>,
        status: Option<String>,
        extra: JsonObject,
    },
    ShellCallOutput {
        call_id: String,
        output: Vec<ResponseShellCallOutputContent>,
        id: Option<String>,
        max_output_length: Option<i64>,
        status: Option<String>,
        extra: JsonObject,
    },
    McpApprovalRequest {
        id: String,
        arguments: String,
        name: String,
        server_label: String,
        extra: JsonObject,
    },
    McpApprovalResponse {
        approval_request_id: String,
        approve: bool,
        id: Option<String>,
        reason: Option<String>,
        extra: JsonObject,
    },
    McpCall {
        id: String,
        arguments: String,
        name: String,
        server_label: String,
        approval_request_id: Option<String>,
        error: Option<String>,
        output: Option<String>,
        status: Option<String>,
        extra: JsonObject,
    },
    CustomToolCallOutput {
        call_id: String,
        output: Value,
        id: Option<String>,
        extra: JsonObject,
    },
    CustomToolCall {
        call_id: String,
        input: String,
        name: String,
        id: Option<String>,
        namespace: Option<String>,
        extra: JsonObject,
    },
    Raw(JsonObject),
}

impl Serialize for ResponseContentPart {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::OutputText {
                annotations,
                logprobs,
                text,
                extra,
            } => KnownResponseContentPart::OutputText {
                annotations: annotations.clone(),
                logprobs: logprobs.clone(),
                text: text.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Refusal { refusal, extra } => KnownResponseContentPart::Refusal {
                refusal: refusal.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Raw(raw) => raw.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ResponseContentPart {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if let Ok(typed) = serde_json::from_value::<KnownResponseContentPart>(value.clone()) {
            return Ok(match typed {
                KnownResponseContentPart::OutputText {
                    annotations,
                    logprobs,
                    text,
                    extra,
                } => Self::OutputText {
                    annotations,
                    logprobs,
                    text,
                    extra,
                },
                KnownResponseContentPart::Refusal { refusal, extra } => {
                    Self::Refusal { refusal, extra }
                }
            });
        }

        Ok(Self::Raw(
            json_object_from_value(value).map_err(D::Error::custom)?,
        ))
    }
}

impl Serialize for ResponseOutputTextAnnotation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::FileCitation {
                file_id,
                filename,
                index,
                extra,
            } => KnownResponseOutputTextAnnotation::FileCitation {
                file_id: file_id.clone(),
                filename: filename.clone(),
                index: *index,
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::UrlCitation {
                end_index,
                start_index,
                title,
                url,
                extra,
            } => KnownResponseOutputTextAnnotation::UrlCitation {
                end_index: *end_index,
                start_index: *start_index,
                title: title.clone(),
                url: url.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::ContainerFileCitation {
                container_id,
                end_index,
                file_id,
                filename,
                start_index,
                extra,
            } => KnownResponseOutputTextAnnotation::ContainerFileCitation {
                container_id: container_id.clone(),
                end_index: *end_index,
                file_id: file_id.clone(),
                filename: filename.clone(),
                start_index: *start_index,
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::FilePath {
                file_id,
                index,
                extra,
            } => KnownResponseOutputTextAnnotation::FilePath {
                file_id: file_id.clone(),
                index: *index,
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Raw(raw) => raw.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ResponseOutputTextAnnotation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if let Ok(typed) =
            serde_json::from_value::<KnownResponseOutputTextAnnotation>(value.clone())
        {
            return Ok(match typed {
                KnownResponseOutputTextAnnotation::FileCitation {
                    file_id,
                    filename,
                    index,
                    extra,
                } => Self::FileCitation {
                    file_id,
                    filename,
                    index,
                    extra,
                },
                KnownResponseOutputTextAnnotation::UrlCitation {
                    end_index,
                    start_index,
                    title,
                    url,
                    extra,
                } => Self::UrlCitation {
                    end_index,
                    start_index,
                    title,
                    url,
                    extra,
                },
                KnownResponseOutputTextAnnotation::ContainerFileCitation {
                    container_id,
                    end_index,
                    file_id,
                    filename,
                    start_index,
                    extra,
                } => Self::ContainerFileCitation {
                    container_id,
                    end_index,
                    file_id,
                    filename,
                    start_index,
                    extra,
                },
                KnownResponseOutputTextAnnotation::FilePath {
                    file_id,
                    index,
                    extra,
                } => Self::FilePath {
                    file_id,
                    index,
                    extra,
                },
            });
        }

        Ok(Self::Raw(
            json_object_from_value(value).map_err(D::Error::custom)?,
        ))
    }
}

impl Serialize for ResponseReasoningSummaryPart {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::SummaryText { text, extra } => KnownResponseReasoningSummaryPart::SummaryText {
                text: text.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Raw(raw) => raw.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ResponseReasoningSummaryPart {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if let Ok(typed) =
            serde_json::from_value::<KnownResponseReasoningSummaryPart>(value.clone())
        {
            return Ok(match typed {
                KnownResponseReasoningSummaryPart::SummaryText { text, extra } => {
                    Self::SummaryText { text, extra }
                }
            });
        }

        Ok(Self::Raw(
            json_object_from_value(value).map_err(D::Error::custom)?,
        ))
    }
}

impl Serialize for ResponseReasoningContentPart {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::ReasoningText { text, extra } => {
                KnownResponseReasoningContentPart::ReasoningText {
                    text: text.clone(),
                    extra: extra.clone(),
                }
                .serialize(serializer)
            }
            Self::Raw(raw) => raw.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ResponseReasoningContentPart {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if let Ok(typed) =
            serde_json::from_value::<KnownResponseReasoningContentPart>(value.clone())
        {
            return Ok(match typed {
                KnownResponseReasoningContentPart::ReasoningText { text, extra } => {
                    Self::ReasoningText { text, extra }
                }
            });
        }

        Ok(Self::Raw(
            json_object_from_value(value).map_err(D::Error::custom)?,
        ))
    }
}

impl Serialize for ResponseCodeInterpreterOutput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Logs { logs, extra } => KnownResponseCodeInterpreterOutput::Logs {
                logs: logs.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Image { url, extra } => KnownResponseCodeInterpreterOutput::Image {
                url: url.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Raw(raw) => raw.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ResponseCodeInterpreterOutput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if let Ok(typed) =
            serde_json::from_value::<KnownResponseCodeInterpreterOutput>(value.clone())
        {
            return Ok(match typed {
                KnownResponseCodeInterpreterOutput::Logs { logs, extra } => {
                    Self::Logs { logs, extra }
                }
                KnownResponseCodeInterpreterOutput::Image { url, extra } => {
                    Self::Image { url, extra }
                }
            });
        }

        Ok(Self::Raw(
            json_object_from_value(value).map_err(D::Error::custom)?,
        ))
    }
}

impl Serialize for ResponseShellCallOutcome {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Timeout { extra } => KnownResponseShellCallOutcome::Timeout {
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Exit { exit_code, extra } => KnownResponseShellCallOutcome::Exit {
                exit_code: *exit_code,
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Raw(raw) => raw.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ResponseShellCallOutcome {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if let Ok(typed) = serde_json::from_value::<KnownResponseShellCallOutcome>(value.clone()) {
            return Ok(match typed {
                KnownResponseShellCallOutcome::Timeout { extra } => Self::Timeout { extra },
                KnownResponseShellCallOutcome::Exit { exit_code, extra } => {
                    Self::Exit { exit_code, extra }
                }
            });
        }

        Ok(Self::Raw(
            json_object_from_value(value).map_err(D::Error::custom)?,
        ))
    }
}

impl Serialize for ResponseOutputItem {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Message {
                id,
                content,
                role,
                status,
                phase,
                extra,
            } => KnownResponseOutputItem::Message {
                id: id.clone(),
                content: content.clone(),
                role: role.clone(),
                status: status.clone(),
                phase: phase.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::FileSearchCall {
                id,
                queries,
                status,
                results,
                extra,
            } => KnownResponseOutputItem::FileSearchCall {
                id: id.clone(),
                queries: queries.clone(),
                status: status.clone(),
                results: results.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::ComputerCall {
                id,
                call_id,
                pending_safety_checks,
                status,
                action,
                extra,
            } => KnownResponseOutputItem::ComputerCall {
                id: id.clone(),
                call_id: call_id.clone(),
                pending_safety_checks: pending_safety_checks.clone(),
                status: status.clone(),
                action: action.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::ComputerCallOutput {
                call_id,
                output,
                id,
                acknowledged_safety_checks,
                status,
                extra,
            } => KnownResponseOutputItem::ComputerCallOutput {
                call_id: call_id.clone(),
                output: output.clone(),
                id: id.clone(),
                acknowledged_safety_checks: acknowledged_safety_checks.clone(),
                status: status.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::WebSearchCall {
                id,
                action,
                status,
                extra,
            } => KnownResponseOutputItem::WebSearchCall {
                id: id.clone(),
                action: action.clone(),
                status: status.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::FunctionCall {
                arguments,
                call_id,
                name,
                id,
                namespace,
                status,
                extra,
            } => KnownResponseOutputItem::FunctionCall {
                arguments: arguments.clone(),
                call_id: call_id.clone(),
                name: name.clone(),
                id: id.clone(),
                namespace: namespace.clone(),
                status: status.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::FunctionCallOutput {
                call_id,
                output,
                id,
                status,
                extra,
            } => KnownResponseOutputItem::FunctionCallOutput {
                call_id: call_id.clone(),
                output: output.clone(),
                id: id.clone(),
                status: status.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::ToolSearchCall {
                arguments,
                id,
                call_id,
                execution,
                status,
                extra,
            } => KnownResponseOutputItem::ToolSearchCall {
                arguments: arguments.clone(),
                id: id.clone(),
                call_id: call_id.clone(),
                execution: execution.clone(),
                status: status.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::ToolSearchOutput {
                tools,
                id,
                call_id,
                execution,
                status,
                extra,
            } => KnownResponseOutputItem::ToolSearchOutput {
                tools: tools.clone(),
                id: id.clone(),
                call_id: call_id.clone(),
                execution: execution.clone(),
                status: status.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Reasoning {
                id,
                summary,
                content,
                encrypted_content,
                status,
                extra,
            } => KnownResponseOutputItem::Reasoning {
                id: id.clone(),
                summary: summary.clone(),
                content: content.clone(),
                encrypted_content: encrypted_content.clone(),
                status: status.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Compaction {
                encrypted_content,
                id,
                extra,
            } => KnownResponseOutputItem::Compaction {
                encrypted_content: encrypted_content.clone(),
                id: id.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::ImageGenerationCall {
                id,
                result,
                status,
                extra,
            } => KnownResponseOutputItem::ImageGenerationCall {
                id: id.clone(),
                result: result.clone(),
                status: status.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::CodeInterpreterCall {
                id,
                code,
                container_id,
                outputs,
                status,
                extra,
            } => KnownResponseOutputItem::CodeInterpreterCall {
                id: id.clone(),
                code: code.clone(),
                container_id: container_id.clone(),
                outputs: outputs.clone(),
                status: status.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::LocalShellCall {
                id,
                action,
                call_id,
                status,
                extra,
            } => KnownResponseOutputItem::LocalShellCall {
                id: id.clone(),
                action: action.clone(),
                call_id: call_id.clone(),
                status: status.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::LocalShellCallOutput {
                id,
                output,
                status,
                extra,
            } => KnownResponseOutputItem::LocalShellCallOutput {
                id: id.clone(),
                output: output.clone(),
                status: status.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::ShellCall {
                action,
                call_id,
                id,
                environment,
                status,
                extra,
            } => KnownResponseOutputItem::ShellCall {
                action: action.clone(),
                call_id: call_id.clone(),
                id: id.clone(),
                environment: environment.clone(),
                status: status.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::ShellCallOutput {
                call_id,
                output,
                id,
                max_output_length,
                status,
                extra,
            } => KnownResponseOutputItem::ShellCallOutput {
                call_id: call_id.clone(),
                output: output.clone(),
                id: id.clone(),
                max_output_length: *max_output_length,
                status: status.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::McpApprovalRequest {
                id,
                arguments,
                name,
                server_label,
                extra,
            } => KnownResponseOutputItem::McpApprovalRequest {
                id: id.clone(),
                arguments: arguments.clone(),
                name: name.clone(),
                server_label: server_label.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::McpApprovalResponse {
                approval_request_id,
                approve,
                id,
                reason,
                extra,
            } => KnownResponseOutputItem::McpApprovalResponse {
                approval_request_id: approval_request_id.clone(),
                approve: *approve,
                id: id.clone(),
                reason: reason.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::McpCall {
                id,
                arguments,
                name,
                server_label,
                approval_request_id,
                error,
                output,
                status,
                extra,
            } => KnownResponseOutputItem::McpCall {
                id: id.clone(),
                arguments: arguments.clone(),
                name: name.clone(),
                server_label: server_label.clone(),
                approval_request_id: approval_request_id.clone(),
                error: error.clone(),
                output: output.clone(),
                status: status.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::CustomToolCallOutput {
                call_id,
                output,
                id,
                extra,
            } => KnownResponseOutputItem::CustomToolCallOutput {
                call_id: call_id.clone(),
                output: output.clone(),
                id: id.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::CustomToolCall {
                call_id,
                input,
                name,
                id,
                namespace,
                extra,
            } => KnownResponseOutputItem::CustomToolCall {
                call_id: call_id.clone(),
                input: input.clone(),
                name: name.clone(),
                id: id.clone(),
                namespace: namespace.clone(),
                extra: extra.clone(),
            }
            .serialize(serializer),
            Self::Raw(raw) => raw.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ResponseOutputItem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if let Ok(typed) = serde_json::from_value::<KnownResponseOutputItem>(value.clone()) {
            return Ok(match typed {
                KnownResponseOutputItem::Message {
                    id,
                    content,
                    role,
                    status,
                    phase,
                    extra,
                } => Self::Message {
                    id,
                    content,
                    role,
                    status,
                    phase,
                    extra,
                },
                KnownResponseOutputItem::FileSearchCall {
                    id,
                    queries,
                    status,
                    results,
                    extra,
                } => Self::FileSearchCall {
                    id,
                    queries,
                    status,
                    results,
                    extra,
                },
                KnownResponseOutputItem::ComputerCall {
                    id,
                    call_id,
                    pending_safety_checks,
                    status,
                    action,
                    extra,
                } => Self::ComputerCall {
                    id,
                    call_id,
                    pending_safety_checks,
                    status,
                    action,
                    extra,
                },
                KnownResponseOutputItem::ComputerCallOutput {
                    call_id,
                    output,
                    id,
                    acknowledged_safety_checks,
                    status,
                    extra,
                } => Self::ComputerCallOutput {
                    call_id,
                    output,
                    id,
                    acknowledged_safety_checks,
                    status,
                    extra,
                },
                KnownResponseOutputItem::WebSearchCall {
                    id,
                    action,
                    status,
                    extra,
                } => Self::WebSearchCall {
                    id,
                    action,
                    status,
                    extra,
                },
                KnownResponseOutputItem::FunctionCall {
                    arguments,
                    call_id,
                    name,
                    id,
                    namespace,
                    status,
                    extra,
                } => Self::FunctionCall {
                    arguments,
                    call_id,
                    name,
                    id,
                    namespace,
                    status,
                    extra,
                },
                KnownResponseOutputItem::FunctionCallOutput {
                    call_id,
                    output,
                    id,
                    status,
                    extra,
                } => Self::FunctionCallOutput {
                    call_id,
                    output,
                    id,
                    status,
                    extra,
                },
                KnownResponseOutputItem::ToolSearchCall {
                    arguments,
                    id,
                    call_id,
                    execution,
                    status,
                    extra,
                } => Self::ToolSearchCall {
                    arguments,
                    id,
                    call_id,
                    execution,
                    status,
                    extra,
                },
                KnownResponseOutputItem::ToolSearchOutput {
                    tools,
                    id,
                    call_id,
                    execution,
                    status,
                    extra,
                } => Self::ToolSearchOutput {
                    tools,
                    id,
                    call_id,
                    execution,
                    status,
                    extra,
                },
                KnownResponseOutputItem::Reasoning {
                    id,
                    summary,
                    content,
                    encrypted_content,
                    status,
                    extra,
                } => Self::Reasoning {
                    id,
                    summary,
                    content,
                    encrypted_content,
                    status,
                    extra,
                },
                KnownResponseOutputItem::Compaction {
                    encrypted_content,
                    id,
                    extra,
                } => Self::Compaction {
                    encrypted_content,
                    id,
                    extra,
                },
                KnownResponseOutputItem::ImageGenerationCall {
                    id,
                    result,
                    status,
                    extra,
                } => Self::ImageGenerationCall {
                    id,
                    result,
                    status,
                    extra,
                },
                KnownResponseOutputItem::CodeInterpreterCall {
                    id,
                    code,
                    container_id,
                    outputs,
                    status,
                    extra,
                } => Self::CodeInterpreterCall {
                    id,
                    code,
                    container_id,
                    outputs,
                    status,
                    extra,
                },
                KnownResponseOutputItem::LocalShellCall {
                    id,
                    action,
                    call_id,
                    status,
                    extra,
                } => Self::LocalShellCall {
                    id,
                    action,
                    call_id,
                    status,
                    extra,
                },
                KnownResponseOutputItem::LocalShellCallOutput {
                    id,
                    output,
                    status,
                    extra,
                } => Self::LocalShellCallOutput {
                    id,
                    output,
                    status,
                    extra,
                },
                KnownResponseOutputItem::ShellCall {
                    action,
                    call_id,
                    id,
                    environment,
                    status,
                    extra,
                } => Self::ShellCall {
                    action,
                    call_id,
                    id,
                    environment,
                    status,
                    extra,
                },
                KnownResponseOutputItem::ShellCallOutput {
                    call_id,
                    output,
                    id,
                    max_output_length,
                    status,
                    extra,
                } => Self::ShellCallOutput {
                    call_id,
                    output,
                    id,
                    max_output_length,
                    status,
                    extra,
                },
                KnownResponseOutputItem::McpApprovalRequest {
                    id,
                    arguments,
                    name,
                    server_label,
                    extra,
                } => Self::McpApprovalRequest {
                    id,
                    arguments,
                    name,
                    server_label,
                    extra,
                },
                KnownResponseOutputItem::McpApprovalResponse {
                    approval_request_id,
                    approve,
                    id,
                    reason,
                    extra,
                } => Self::McpApprovalResponse {
                    approval_request_id,
                    approve,
                    id,
                    reason,
                    extra,
                },
                KnownResponseOutputItem::McpCall {
                    id,
                    arguments,
                    name,
                    server_label,
                    approval_request_id,
                    error,
                    output,
                    status,
                    extra,
                } => Self::McpCall {
                    id,
                    arguments,
                    name,
                    server_label,
                    approval_request_id,
                    error,
                    output,
                    status,
                    extra,
                },
                KnownResponseOutputItem::CustomToolCallOutput {
                    call_id,
                    output,
                    id,
                    extra,
                } => Self::CustomToolCallOutput {
                    call_id,
                    output,
                    id,
                    extra,
                },
                KnownResponseOutputItem::CustomToolCall {
                    call_id,
                    input,
                    name,
                    id,
                    namespace,
                    extra,
                } => Self::CustomToolCall {
                    call_id,
                    input,
                    name,
                    id,
                    namespace,
                    extra,
                },
            });
        }

        Ok(Self::Raw(
            json_object_from_value(value).map_err(D::Error::custom)?,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum KnownResponseContentPart {
    OutputText {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        annotations: Option<Vec<ResponseOutputTextAnnotation>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        logprobs: Option<Vec<Value>>,
        text: String,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    Refusal {
        refusal: String,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum KnownResponseOutputTextAnnotation {
    FileCitation {
        file_id: String,
        filename: String,
        index: i64,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    UrlCitation {
        end_index: i64,
        start_index: i64,
        title: String,
        url: String,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    ContainerFileCitation {
        container_id: String,
        end_index: i64,
        file_id: String,
        filename: String,
        start_index: i64,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    FilePath {
        file_id: String,
        index: i64,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum KnownResponseReasoningSummaryPart {
    SummaryText {
        text: String,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum KnownResponseReasoningContentPart {
    ReasoningText {
        text: String,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum KnownResponseCodeInterpreterOutput {
    Logs {
        logs: String,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    Image {
        url: String,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum KnownResponseShellCallOutcome {
    Timeout {
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    Exit {
        exit_code: i64,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum KnownResponseOutputItem {
    Message {
        id: String,
        content: Vec<ResponseContentPart>,
        role: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        phase: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    FileSearchCall {
        id: String,
        queries: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        results: Option<Vec<ResponseFileSearchResult>>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    ComputerCall {
        id: String,
        call_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pending_safety_checks: Option<Vec<ResponseSafetyCheck>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        action: Option<Value>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    ComputerCallOutput {
        call_id: String,
        output: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        acknowledged_safety_checks: Option<Vec<ResponseSafetyCheck>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    WebSearchCall {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        action: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    FunctionCall {
        arguments: String,
        call_id: String,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        namespace: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    FunctionCallOutput {
        call_id: String,
        output: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    ToolSearchCall {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        arguments: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        call_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        execution: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    ToolSearchOutput {
        tools: Vec<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        call_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        execution: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    Reasoning {
        id: String,
        summary: Vec<ResponseReasoningSummaryPart>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content: Option<Vec<ResponseReasoningContentPart>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        encrypted_content: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    Compaction {
        encrypted_content: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    ImageGenerationCall {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        result: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    CodeInterpreterCall {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        code: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        container_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        outputs: Option<Vec<ResponseCodeInterpreterOutput>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    LocalShellCall {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        action: Option<Value>,
        call_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    LocalShellCallOutput {
        id: String,
        output: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    ShellCall {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        action: Option<ResponseShellAction>,
        call_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        environment: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    ShellCallOutput {
        call_id: String,
        output: Vec<ResponseShellCallOutputContent>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_output_length: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    McpApprovalRequest {
        id: String,
        arguments: String,
        name: String,
        server_label: String,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    McpApprovalResponse {
        approval_request_id: String,
        approve: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    McpCall {
        id: String,
        arguments: String,
        name: String,
        server_label: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        approval_request_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    CustomToolCallOutput {
        call_id: String,
        output: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    CustomToolCall {
        call_id: String,
        input: String,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        namespace: Option<String>,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{
        ResponseContentPart, ResponseOutputItem, ResponseReasoningContentPart,
        ResponseReasoningSummaryPart,
    };

    #[test]
    fn response_output_message_deserializes_typed_content() {
        let item: ResponseOutputItem = serde_json::from_str(
            r#"{
                "type":"message",
                "id":"msg_123",
                "role":"assistant",
                "phase":"final_answer",
                "content":[
                    {
                        "type":"output_text",
                        "text":"hello",
                        "annotations":[
                            {
                                "type":"file_citation",
                                "file_id":"file_123",
                                "filename":"spec.md",
                                "index":0
                            }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();

        match item {
            ResponseOutputItem::Message { content, phase, .. } => {
                assert_eq!(phase.as_deref(), Some("final_answer"));
                assert!(matches!(content[0], ResponseContentPart::OutputText { .. }));
            }
            other => panic!("expected message item, got {other:?}"),
        }
    }

    #[test]
    fn response_reasoning_item_deserializes_summary_and_content() {
        let item: ResponseOutputItem = serde_json::from_str(
            r#"{
                "type":"reasoning",
                "id":"rs_123",
                "summary":[{"type":"summary_text","text":"step 1"}],
                "content":[{"type":"reasoning_text","text":"details"}],
                "encrypted_content":"abc"
            }"#,
        )
        .unwrap();

        match item {
            ResponseOutputItem::Reasoning {
                summary,
                content,
                encrypted_content,
                ..
            } => {
                assert!(matches!(
                    summary[0],
                    ResponseReasoningSummaryPart::SummaryText { .. }
                ));
                assert!(matches!(
                    content.unwrap()[0],
                    ResponseReasoningContentPart::ReasoningText { .. }
                ));
                assert_eq!(encrypted_content.as_deref(), Some("abc"));
            }
            other => panic!("expected reasoning item, got {other:?}"),
        }
    }

    #[test]
    fn response_function_call_output_preserves_string_payload() {
        let item: ResponseOutputItem = serde_json::from_str(
            r#"{
                "type":"function_call_output",
                "call_id":"call_123",
                "output":"{\"ok\":true}"
            }"#,
        )
        .unwrap();

        match item {
            ResponseOutputItem::FunctionCallOutput { output, .. } => {
                assert_eq!(output, Value::String("{\"ok\":true}".to_owned()));
            }
            other => panic!("expected function_call_output item, got {other:?}"),
        }
    }

    #[test]
    fn response_output_item_preserves_unknown_type() {
        let item: ResponseOutputItem = serde_json::from_str(
            r#"{
                "type":"deep_research_trace",
                "trace_id":"tr_123"
            }"#,
        )
        .unwrap();

        let reserialized = serde_json::to_value(&item).unwrap();
        assert_eq!(reserialized["type"], "deep_research_trace");
        assert_eq!(reserialized["trace_id"], "tr_123");
    }
}
