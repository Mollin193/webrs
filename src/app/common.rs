use serde::Deserialize;
use validator::Validate;

const DEFAULT_PAGE: u64 = 1;
const DEFAULT_SIZE: u64 = 15;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Validate)]
pub struct PaginationParams {
    #[validate(range(min = 1, message = "页码必须大于0"))]
    pub page: u64,
    pub size: u64,
}
