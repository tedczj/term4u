use base64::Engine as _;
use base64::prelude::BASE64_URL_SAFE;
use prost::Message as _;
use warp_multi_agent_api::response_event::stream_finished::{InvalidApiKey, Reason};
use warp_multi_agent_api::response_event::{StreamFinished, Type};
use warp_multi_agent_api::{LlmProvider, ResponseEvent};

use super::{Error, decode_response_event};

#[test]
fn decoded_bedrock_provider_is_rejected() {
    let event = ResponseEvent {
        r#type: Some(Type::Finished(StreamFinished {
            reason: Some(Reason::InvalidApiKey(InvalidApiKey {
                provider: LlmProvider::AwsBedrock as i32,
                model_name: "removed-model".to_owned(),
            })),
            ..Default::default()
        })),
    };
    let encoded = BASE64_URL_SAFE.encode(event.encode_to_vec());

    let error = decode_response_event(&encoded).expect_err("Bedrock must be rejected after decode");

    assert!(matches!(error, Error::AwsBedrockUnsupported));
    assert_eq!(error.to_string(), "AWS Bedrock is not supported");
}

#[test]
fn decoded_supported_provider_is_accepted() {
    let event = ResponseEvent {
        r#type: Some(Type::Finished(StreamFinished {
            reason: Some(Reason::InvalidApiKey(InvalidApiKey {
                provider: LlmProvider::Anthropic as i32,
                model_name: "supported-model".to_owned(),
            })),
            ..Default::default()
        })),
    };
    let encoded = BASE64_URL_SAFE.encode(event.encode_to_vec());

    assert_eq!(decode_response_event(&encoded).unwrap(), event);
}
