use crate::types::{
    ImageOutputFormat, InvalidRequestBody, RequestBody, RequestParams, ResponseBody,
};
use async_openai::config::OpenAIConfig;
use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestMessageContentPart,
    ChatCompletionRequestMessageContentPartImageArgs, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
};
use async_openai::Client;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use warp::Filter;

pub(crate) type MessageSender = Sender<ServiceMessage>;
pub(crate) type MessageReceiver = Receiver<ServiceMessage>;

pub(crate) struct AiService {
    client: Arc<Client<OpenAIConfig>>,
    sender: Option<MessageSender>,
}

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct ServiceRequest {
    pub(crate) params: RequestParams,
    pub(crate) body: RequestBody,
}

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct ServiceResponse {
    pub(crate) body: ResponseBody,
}

#[derive(Deserialize, Serialize, Debug)]
pub(crate) enum ServiceMessage {
    Request(ServiceRequest),
    Response(ServiceResponse),
}

impl AiService {
    pub(crate) fn service(
        client: Arc<Client<OpenAIConfig>>,
        sender: Option<MessageSender>,
    ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
        let service = Arc::new(Self { client, sender });

        warp::post() // Accept only POST requests...
            // ...at the root path...
            .and(warp::path::end())
            // ...with query parameters that suit RequestParams...
            .and(warp::query::<RequestParams>())
            // ...regardless of the declared content type.
            .and(warp::body::bytes())
            // RetroArch declares application/x-www-form-urlencoded for its AI service requests,
            // but the body is actually JSON;
            // hence we deserialize explicitly because warp doesn't know how to handle this discrepancy.
            .and_then(|params, body: Bytes| async move {
                log::info!(target: "groan", "{:?}", params);
                if let Ok(body) = serde_json::from_slice::<RequestBody>(body.iter().as_slice()) {
                    log::info!(target: "groan", "{:?}", body);
                    Ok((params, body))
                } else {
                    Err(warp::reject::custom(InvalidRequestBody))
                }
            })
            // Then we untuple the parameters and body...
            .untuple_one()
            // ...and pass along the service object itself.
            .and(warp::any().map(move || service.clone()))
            // query_service may run on another thread, possibly with multiple instances;
            // therefore we create the client in an `Arc` and clone it for each call to this endpoint
            .then(move |params, body, service| AiService::query_service(service, params, body))
            // Now that we've got the response, convert it to JSON...
            .map(|response| warp::reply::json(&response))
            .with(warp::trace::named("groan"))
    }

    async fn query_service(
        service: Arc<AiService>,
        params: RequestParams,
        body: RequestBody,
    ) -> ResponseBody {
        match params
            .output
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>()
            .as_slice()
        {
            ["text", ..] => AiService::send_chat_request(service, params, body).await,
            ["sound", "wav", ..] => ResponseBody::error("Sound not implemented"),
            ["image", "png", "png-a", ..] => ResponseBody::error("Image not implemented"),
            _ => ResponseBody::error(format!("Unknown output format {:?}", params.output)),
        }
    }

    async fn send_chat_request(
        service: Arc<AiService>,
        params: RequestParams,
        body: RequestBody,
    ) -> ResponseBody {
        let system = ChatCompletionRequestSystemMessageArgs::default()
            .content(
                "You are a narration service helping a visually impaired player \
            understand the scene for the game they're playing. \
            Describe the contents of the base64-encoded screenshots you will be given. \
            Your response will be read aloud by a text-to-speech system; \
            limit your response to at most two sentences. \
            Do not use headings or explicit section makers. \
            Do not speculate about the image's contents.",
            ) // TODO: Make customizable
            .build()
            .map(ChatCompletionRequestMessage::System)
            .unwrap();

        let message = ChatCompletionRequestMessageContentPartImageArgs::default()
            .image_url(format!(
                "data:image/{:?};base64,{}",
                body.format.unwrap_or(ImageOutputFormat::Png),
                body.image
            ))
            .build()
            .map(ChatCompletionRequestMessageContentPart::ImageUrl)
            .unwrap();

        let user = ChatCompletionRequestUserMessageArgs::default()
            .content(vec![message])
            .build()
            .map(ChatCompletionRequestMessage::User)
            .unwrap();

        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-4o-mini") // TODO: Make customizable
            .max_tokens(300u32) // TODO: Make customizable
            .messages(vec![system, user])
            .build()
            .unwrap();

        match service.client.chat().create(request).await.as_ref() {
            Ok(response) => {
                log::info!(target: "groan", "{:?}", response);
                ResponseBody::text(response.choices[0].message.content.as_ref().unwrap())
            }
            Err(error) => ResponseBody::error(format!("Error: {:?}", error)),
        }
    }
}
