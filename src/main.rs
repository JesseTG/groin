mod types;

use clap::Parser;
use warp::Filter;
use bytes::Bytes;
use crate::types::{InvalidRequestBody, RequestBody, RequestParams};
// NOTE: These doc comments are parsed and embedded into the CLI itself.

/// groan - Good RetroArch OpenAI iNtegration
#[derive(Parser, Debug)]
#[command(version, about, long_about)]
struct Cli {
    /// The API key used to authenticate with OpenAI.
    /// Provide on the command-line or with the OPENAI_API_KEY environment variable.
    #[arg(short, long, env = "OPENAI_API_KEY")]
    key: String,

    #[arg(short, long, default_value_t = 4404)]
    port: u16,

    // TODO: Select a host
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    pretty_env_logger::init();

    let hello = warp::post() // Accept only POST requests...
        // ...at the root path...
        .and(warp::path::end())
        // ...with query parameters that suit RequestParams...
        .and(warp::query::<RequestParams>())
        // ...regardless of the declared content type.
        // RetroArch declares application/x-www-form-urlencoded,
        // but the body is actually JSON;
        // hence we deserialize explicitly because warp doesn't know how to
        .and(warp::body::bytes())
        .and_then(|params, body: Bytes| async move {
            if let Ok(body) = serde_json::from_slice::<RequestBody>(body.iter().as_slice()) {
                Ok((params, body))
            } else {
                Err(warp::reject::custom(InvalidRequestBody))
            }
        })
        .untuple_one()
        .map(|params, body: RequestBody| {
            format!("{:?}, {:?}", params, body)
        })
        .with(warp::trace::named("groan"));

    warp::serve(hello)
        .run(([127, 0, 0, 1], cli.port))
        .await;
}
