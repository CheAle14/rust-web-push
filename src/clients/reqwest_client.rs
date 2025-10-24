use async_trait::async_trait;
use http::header::RETRY_AFTER;

use crate::{
    clients::MAX_RESPONSE_SIZE, error::RetryAfter, request_builder, WebPushClient, WebPushError, WebPushMessage,
};

#[async_trait]
impl WebPushClient for reqwest::Client {
    /// Sends a notification. Never times out.
    async fn send(&self, message: WebPushMessage) -> Result<(), WebPushError> {
        trace!("Message: {:?}", message);

        let request = request_builder::build_request::<reqwest::Body>(message);
        let request = reqwest::Request::try_from(request)?;

        trace!("Request: {:?}", request);

        let response = self.execute(request).await?;

        trace!("Response: {:?}", response);

        let retry_after = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|ra| ra.to_str().ok())
            .and_then(RetryAfter::from_str);

        let response_status = response.status();
        trace!("Response status: {}", response_status);

        if response.content_length().unwrap_or_default() > MAX_RESPONSE_SIZE as u64 {
            return Err(WebPushError::ResponseTooLarge);
        }

        let response = request_builder::parse_response(response_status, response.bytes().await?.to_vec());

        trace!("Response: {:?}", response);

        if let Err(WebPushError::ServerError {
            retry_after: None,
            info,
        }) = response
        {
            Err(WebPushError::ServerError { retry_after, info })
        } else {
            Ok(response?)
        }
    }
}
