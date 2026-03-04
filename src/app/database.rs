use super::config;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement,
};
use std::cmp::max;
use std::time::Duration;

pub async fn init() -> anyhow::Result<DatabaseConnection> {
    let database_config = &config::get().database();

    // 拼接 PostgreSQL 的连接字符串
    let mut options = ConnectOptions::new(format!(
        "postgres://{}:{}@{}:{}/{}",
        database_config.user(),
        database_config.password(),
        database_config.host(),
        database_config.port(),
        database_config.database()
    ));

    let cpus = num_cpus::get() as u32;
    options
        .min_connections(max(cpus * 4, 10))
        .max_connections(max(cpus * 8, 20))
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(300))
        // 一个连接最多活 24 小时
        .max_lifetime(Duration::from_secs(3600 * 24))
        // 关闭底层的 sqlx 默认日志，因为已经配了 tracing
        .sqlx_logging(false)
        .set_schema_search_path(database_config.schema());

    let db = Database::connect(options).await?;
    db.ping().await?;
    tracing::info!("Database connected successfully");

    log_database_version(&db).await?;

    Ok(db)
}

/// 打印数据库版本
async fn log_database_version(db: &DatabaseConnection) -> anyhow::Result<()> {
    let version_result = db
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            String::from("SELECT version()"),
        ))
        .await?
        .ok_or_else(|| anyhow::anyhow!("Failed to get database version"))?;

    tracing::info!(
        "Database version: {}",
        version_result.try_get_by_index::<String>(0)?
    );

    Ok(())
}
