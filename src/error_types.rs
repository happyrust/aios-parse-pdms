use anyhow::anyhow;
use std::io;

#[derive(Debug, thiserror::Error)]
#[error("...")]
pub enum ResolveError {
    #[error("Axis index not exist {0}")]
    AxisIndeNotExist(u32),

    #[error("{0}")]
    IoError(#[from] io::Error),
    IoError1(io::Error),
    // #[error("{0}")]
    // Query(QueryPayloadError),
    // #[error("The json payload provided is malformed. `{0}`.")]
    // MalformedPayload(serde_json::error::Error),
    // #[error("A json payload is missing.")]
    // MissingPayload,
}

fn gen_resolve_error() -> anyhow::Result<u32>{

     // Err(ResolveError::AxisIndeNotExist(1).into());
    let io = io::Error::new(io::ErrorKind::Other, "oh no!");
    Err(ResolveError::from(io).into())
}

#[test]
fn test_resolve_error() {
    match gen_resolve_error() {
        Ok(_) => {}
        Err(e) => {
            println!("{:?}", e.to_string());
        }
    }

    assert_eq!(1, 1);

}

#[test]
#[should_panic]
fn test_resolve_panic() {
    gen_resolve_error().unwrap();
}

//


#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    // #[error("{0}")]
    // Json(JsonPayloadError),
    // #[error("{0}")]
    // Query(QueryPayloadError),
    // #[error("The json payload provided is malformed. `{0}`.")]
    // MalformedPayload(serde_json::error::Error),
    // #[error("A json payload is missing.")]
    // MissingPayload,
}