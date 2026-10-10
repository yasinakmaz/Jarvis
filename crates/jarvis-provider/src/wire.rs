//! Alan modeli ↔ `OpenAI` tel tipleri eşlemesi (ADR 0029). Eşlenemeyen her şey açık hatadır.

use async_openai::types::chat::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls,
    ChatCompletionRequestAssistantMessage, ChatCompletionRequestAssistantMessageContent,
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestSystemMessageContent, ChatCompletionRequestToolMessage,
    ChatCompletionRequestToolMessageContent, ChatCompletionRequestUserMessage,
    ChatCompletionRequestUserMessageContent, ChatCompletionResponseMessage, ChatCompletionTool,
    ChatCompletionTools, CompletionUsage, CreateChatCompletionRequest,
    CreateChatCompletionResponse, FinishReason as WireFinish, FunctionCall, FunctionObject,
    Role as WireRole,
};
use jarvis_types::{Content, Message, Role, ToolCall, ToolCallId, ToolName, ToolSpec, Trust};

use crate::{ChatRequest, ChatResponse, FinishReason, ProviderError, Usage};

fn invalid_request(detail: impl Into<String>) -> ProviderError {
    ProviderError::InvalidRequest(detail.into())
}

fn invalid_response(detail: impl Into<String>) -> ProviderError {
    ProviderError::InvalidResponse(detail.into())
}

/// Alan isteğini tel isteğine çevirir. `max_tokens`, `max_completion_tokens` değildir:
/// uyumlu sağlayıcıların çoğu yenisini bilmez.
#[expect(
    deprecated,
    reason = "max_tokens: OpenAI uyumlu sağlayıcıların ortak alanı"
)]
pub fn to_wire_request(
    model: &str,
    request: &ChatRequest,
) -> Result<CreateChatCompletionRequest, ProviderError> {
    if request.messages.is_empty() {
        return Err(invalid_request("istekte hiç mesaj yok"));
    }
    let messages = request
        .messages
        .iter()
        .map(to_wire_message)
        .collect::<Result<Vec<_>, _>>()?;
    let tools = (!request.tools.is_empty()).then(|| request.tools.iter().map(wire_tool).collect());
    Ok(CreateChatCompletionRequest {
        model: model.to_owned(),
        messages,
        tools,
        max_tokens: request.max_tokens,
        ..Default::default()
    })
}

fn wire_tool(spec: &ToolSpec) -> ChatCompletionTools {
    ChatCompletionTools::Function(ChatCompletionTool {
        function: FunctionObject {
            name: spec.name.to_string(),
            description: Some(spec.description.clone()),
            parameters: Some(spec.parameters.clone()),
            strict: None,
        },
    })
}

/// Görüntü parçaları M4'e kadar desteklenmez; sessizce atlanmaz.
fn text_of(message: &Message) -> Result<String, ProviderError> {
    if message
        .content
        .iter()
        .any(|part| matches!(part, Content::ImagePng(_)))
    {
        return Err(invalid_request("görüntü girdisi M4'e kadar desteklenmiyor"));
    }
    Ok(message.joined_text())
}

/// Tek bir alan mesajını tel mesajına çevirir.
pub fn to_wire_message(message: &Message) -> Result<ChatCompletionRequestMessage, ProviderError> {
    let text = text_of(message)?;
    Ok(match message.role {
        Role::System => ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
            content: ChatCompletionRequestSystemMessageContent::Text(text),
            name: None,
        }),
        Role::User => ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
            content: ChatCompletionRequestUserMessageContent::Text(text),
            name: None,
        }),
        Role::Assistant => assistant_message(message, text)?,
        Role::Tool => {
            let id = message
                .tool_call_id
                .as_ref()
                .ok_or_else(|| invalid_request("araç sonucu mesajında tool_call_id yok"))?;
            ChatCompletionRequestMessage::Tool(ChatCompletionRequestToolMessage {
                content: ChatCompletionRequestToolMessageContent::Text(text),
                tool_call_id: id.to_string(),
            })
        }
    })
}

fn assistant_message(
    message: &Message,
    text: String,
) -> Result<ChatCompletionRequestMessage, ProviderError> {
    let calls: Vec<ChatCompletionMessageToolCalls> = message
        .tool_calls
        .iter()
        .map(wire_call)
        .collect::<Result<_, _>>()?;
    Ok(ChatCompletionRequestMessage::Assistant(
        ChatCompletionRequestAssistantMessage {
            content: (!text.is_empty())
                .then_some(ChatCompletionRequestAssistantMessageContent::Text(text)),
            tool_calls: (!calls.is_empty()).then_some(calls),
            ..Default::default()
        },
    ))
}

/// Metin argümanlar (ayrıştırılamayan çağrılar) olduğu gibi geri gönderilir.
fn wire_call(call: &ToolCall) -> Result<ChatCompletionMessageToolCalls, ProviderError> {
    let arguments = match &call.arguments {
        serde_json::Value::String(raw) => raw.clone(),
        other => serde_json::to_string(other)
            .map_err(|e| invalid_request(format!("araç argümanları yazılamadı: {e}")))?,
    };
    Ok(ChatCompletionMessageToolCalls::Function(
        ChatCompletionMessageToolCall {
            id: call.id.to_string(),
            function: FunctionCall {
                name: call.name.to_string(),
                arguments,
            },
        },
    ))
}

/// Tel yanıtını alan yanıtına çevirir; ilk seçenek kullanılır.
pub fn from_wire_response(
    response: CreateChatCompletionResponse,
) -> Result<ChatResponse, ProviderError> {
    let choice = response
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| invalid_response("yanıtta hiç seçenek yok"))?;
    let finish = choice
        .finish_reason
        .map(finish_reason)
        .ok_or_else(|| invalid_response("yanıtta finish_reason yok"))?;
    Ok(ChatResponse {
        message: from_wire_message(choice.message)?,
        finish,
        usage: response.usage.as_ref().map(usage),
        model: response.model,
    })
}

const fn finish_reason(wire: WireFinish) -> FinishReason {
    match wire {
        WireFinish::Stop => FinishReason::Stop,
        WireFinish::Length => FinishReason::Length,
        WireFinish::ToolCalls | WireFinish::FunctionCall => FinishReason::ToolCalls,
        WireFinish::ContentFilter => FinishReason::ContentFilter,
    }
}

const fn usage(wire: &CompletionUsage) -> Usage {
    Usage {
        prompt_tokens: wire.prompt_tokens,
        completion_tokens: wire.completion_tokens,
        total_tokens: wire.total_tokens,
    }
}

/// Tel asistan mesajını alan mesajına çevirir. Ret metni (`refusal`) içerik olarak görünür.
pub fn from_wire_message(message: ChatCompletionResponseMessage) -> Result<Message, ProviderError> {
    if !matches!(message.role, WireRole::Assistant) {
        return Err(invalid_response(
            "yanıt mesajı asistan rolünde değil; asistan bekleniyordu",
        ));
    }
    let text = message
        .content
        .filter(|text| !text.is_empty())
        .or_else(|| message.refusal.filter(|text| !text.is_empty()));
    let tool_calls = message
        .tool_calls
        .unwrap_or_default()
        .into_iter()
        .map(from_wire_call)
        .collect::<Result<Vec<_>, _>>()?;
    if text.is_none() && tool_calls.is_empty() {
        return Err(invalid_response("yanıtta ne metin ne araç çağrısı var"));
    }
    Ok(Message {
        role: Role::Assistant,
        content: text.map(Content::Text).into_iter().collect(),
        tool_calls,
        tool_call_id: None,
        trust: Trust::Trusted,
    })
}

/// Argüman metni JSON değilse ham metin `Value::String` olarak taşınır: çağrıyı reddedip
/// modele "geçersiz argüman" sonucu dönmek `jarvis-core`'un işidir (Tasarım 0008).
fn from_wire_call(call: ChatCompletionMessageToolCalls) -> Result<ToolCall, ProviderError> {
    let ChatCompletionMessageToolCalls::Function(call) = call else {
        return Err(invalid_response(
            "özel araç (custom tool) çağrıları desteklenmiyor",
        ));
    };
    let id = ToolCallId::new(call.id)
        .map_err(|e| invalid_response(format!("araç çağrısı kimliği geçersiz: {e}")))?;
    let name = ToolName::parse(&call.function.name)
        .map_err(|e| invalid_response(format!("araç adı geçersiz: {e}")))?;
    let raw = call.function.arguments;
    let arguments = serde_json::from_str(&raw).unwrap_or(serde_json::Value::String(raw));
    Ok(ToolCall {
        id,
        name,
        arguments,
    })
}

#[cfg(test)]
#[path = "wire_tests.rs"]
mod tests;
