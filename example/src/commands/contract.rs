/// Hidden command for integration tests.
///
/// Deterministically triggers each exit code so the test suite can verify
/// the full 0-4 contract without depending on network or filesystem state.
use crate::error::AppError;
use crate::output::{self, Ctx};

pub fn run(ctx: Ctx, code: i32) -> Result<(), AppError> {
    match code {
        0 => {
            let data = serde_json::json!({"contract": true, "exit_code": 0});
            output::print_success_or(ctx, &data, |_| {
                println!("contract: success");
            });
            Ok(())
        }
        1 => Err(AppError::Transient("contract: transient error".into())),
        2 => Err(AppError::Config("contract: config error".into())),
        3 => Err(AppError::InvalidInput("contract: bad input".into())),
        4 => Err(AppError::RateLimited("contract: rate limited".into())),
        other => Err(AppError::InvalidInput(format!(
            "unknown contract code: {other}. Valid: 0-4"
        ))),
    }
}
