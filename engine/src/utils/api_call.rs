use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client, Method};
use std::time::Instant;

// DO NOT ADD AN ASYNCHRONOUS RUNTIME TO THIS FUNCTION, AS ACTIX WEB USES ITS OWN ASYNC RUNTIME WHEN YOU CREATE AN ENDPOINT.
pub async fn generic_api_call<T, D>(
    url: String,
    method: String,
    body: D,
    api_key: Option<String>,
) -> Result<T, Box<dyn std::error::Error>>
where
    // check that T can be deserilaised as owned values
    T: serde::de::DeserializeOwned,
    // chceck that D can be serilaised to JSON
    D: serde::Serialize,
{
    // this client is a rust object that allows you to create requests
    let client = Client::new();

    /* match the method, after putting it to uppercase and making sure its string,
    to compare it to the Method Enum and whichever it matches set the new method to be that, if its unsuported then throw the error */
    let method = match method.to_uppercase().as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "DELETE" => Method::DELETE,
        "PATCH" => Method::PATCH,
        _ => return Err("Unsupported HTTP method".into()),
    };

    // this will start a timer so then we can see how long the api call took
    let start = Instant::now();

    // Build the request with its method, URL and JSON content type.
    let mut request = client
        .request(method, &url)
        .header(CONTENT_TYPE, "application/json");

    // Add bearer authentication when the caller supplies a key.
    if let Some(key) = api_key {
        request = request.header(AUTHORIZATION, format!("Bearer {}", key));
    }

    // Serialize the body and send the request.
    let response = request.json(&body).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "No error body".to_string());
        return Err(format!("API call failed: {} - {}", status, text).into());
    }

    // Deserialize the JSON response into the caller's expected type.
    let result: T = response.json().await?;

    let duration = start.elapsed();
    println!(
        "API call plus json received for url: {} took: {:.2?}",
        &url, duration,
    );

    Ok(result)
}
