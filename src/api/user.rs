use crate::app::AppState;
use crate::app::common::{Page, PaginationParams};
use crate::app::enumeration::Gender;
use crate::app::error::{ApiError, ApiResult};
use crate::app::path::Path;
use crate::app::response::ApiResponse;
use crate::app::utils::encode_password;
use crate::app::valid::{ValidJson, ValidQuery};
use crate::entity::prelude::*;
use crate::entity::sys_user;
use crate::entity::sys_user::ActiveModel;
use axum::extract::State;
use axum::{Router, debug_handler, routing};
use sea_orm::prelude::*;
use sea_orm::{ActiveValue, Condition, IntoActiveModel, QueryOrder, QueryTrait};
use serde::Deserialize;
use validator::Validate;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/", routing::get(find_page))
        .route("/", routing::post(create))
        .route("/{id}", routing::put(update))
        .route("/{id}", routing::delete(delete))
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UserQueryParams {
    keyword: Option<String>,
    #[validate(nested)] // 嵌套校验分页参数
    #[serde(flatten)] // 扁平化展开
    pagination: PaginationParams,
}

#[debug_handler]
async fn find_page(
    State(AppState { db }): State<AppState>,
    ValidQuery(UserQueryParams {
        keyword,
        pagination,
    }): ValidQuery<UserQueryParams>,
) -> ApiResult<ApiResponse<Page<sys_user::Model>>> {
    // SeaORM 的查询构造器
    let paginator = SysUser::find()
        // apply_if: 如果前端传了 keyword，就把闭包里的搜索条件拼上去
        .apply_if(keyword.as_ref(), |query, keyword| {
            query.filter(
                Condition::any() // OR 条件：名字包含关键字，或者账号包含关键字
                    .add(sys_user::Column::Name.contains(keyword))
                    .add(sys_user::Column::Account.contains(keyword)),
            )
        })
        .order_by_desc(sys_user::Column::CreatedAt)
        .paginate(&db, pagination.size);

    // 查出总数
    let total = paginator.num_items().await?;
    // 查出这一页的真实数据 (注意：SeaORM 页码从 0 开始，所以前端传来的 page 要 -1)
    let items = paginator.fetch_page(pagination.page - 1).await?;
    // 打包成标准 Page 对象
    let page = Page::from_pagination(pagination, total, items);

    Ok(ApiResponse::ok("ok", Some(page)))
}

/// DeriveIntoActiveModel允许直接把前端传来的 JSON 结构体，转换成 SeaORM 的操作模型
#[derive(Debug, Deserialize, Validate, DeriveIntoActiveModel)]
#[serde(rename_all = "camelCase")]
pub struct UserParams {
    #[validate(length(min = 1, max = 16, message = "姓名长度为1-16"))]
    pub name: String,
    pub gender: Gender, // 前端传男/女，全自动变枚举
    #[validate(length(min = 1, max = 16, message = "账号长度为1-16"))]
    pub account: String,
    #[validate(length(max = 16, message = "密码长度为6-16"))]
    pub password: String,
    #[validate(custom(function = "crate::app::validation::is_mobile_phone"))]
    pub mobile_phone: String,
    pub birthday: Date,
    #[serde(default)]
    pub enabled: bool,
}

#[debug_handler]
async fn create(
    State(AppState { db }): State<AppState>,
    ValidJson(params): ValidJson<UserParams>,
) -> ApiResult<ApiResponse<sys_user::Model>> {
    if params.password.is_empty() {
        return Err(ApiError::Biz("密码不能为空".to_string()));
    }

    // 一键把 Payload 转成能操作数据库的 ActiveModel
    let mut active_model = params.into_active_model();

    // 把明文密码抽出来，加密，再放回 ActiveValue::Set 盒子里
    active_model.password =
        ActiveValue::Set(encode_password(&active_model.password.take().unwrap())?);

    let result = active_model.insert(&db).await?;

    Ok(ApiResponse::ok("ok", Some(result)))
}

#[debug_handler]
async fn update(
    State(AppState { db }): State<AppState>,
    Path(id): Path<String>,                   // 从 URL /123 拿到 ID
    ValidJson(params): ValidJson<UserParams>, // 从 Body 拿到修改后的数据
) -> ApiResult<ApiResponse<sys_user::Model>> {
    // 先查查数据库里有没有这个人
    let existed_user = SysUser::find_by_id(&id)
        .one(&db)
        .await?
        .ok_or_else(|| ApiError::Biz(String::from("待修改的用户不存在")))?;

    let old_password = existed_user.password.clone();
    let password = params.password.clone();

    // 把查出来的旧数据转成 ActiveModel
    let mut existed_active_model = existed_user.into_active_model();
    // 把前端传来的新数据转成 ActiveModel
    let mut active_model = params.into_active_model();

    // 直接用前端的新数据覆盖旧数据！
    existed_active_model.clone_from(&active_model);

    // ID 不准被修改，强制设置为 Unchanged 状态。
    existed_active_model.id = ActiveValue::Unchanged(id);

    if password.is_empty() {
        // 如果前端没传密码，说明不想改密码，强制把密码字段还原成旧的
        existed_active_model.password = ActiveValue::Unchanged(old_password);
    } else {
        // 如果传了新密码，重新加密一遍
        existed_active_model.password =
            ActiveValue::Set(encode_password(&active_model.password.take().unwrap())?);
    }

    // 更新进数据库
    let result = existed_active_model.update(&db).await?;

    Ok(ApiResponse::ok("ok", Some(result)))
}

#[debug_handler]
async fn delete(
    State(AppState { db }): State<AppState>,
    Path(id): Path<String>, // 安全提取 ID
) -> ApiResult<ApiResponse<()>> {
    // 依然是先查这个人存不存在
    let existed_user = SysUser::find_by_id(&id)
        .one(&db)
        .await?
        .ok_or_else(|| ApiError::Biz(String::from("待删除的用户不存在")))?;

    let result = existed_user.delete(&db).await?;

    // 打个日志：某某用户被删除了
    tracing::info!(
        "Deleted user: {}, affected rows: {}",
        id,
        result.rows_affected
    );

    Ok(ApiResponse::ok("ok", None))
}
