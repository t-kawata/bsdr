// 共通の更新処理マクロ（sea-orm-cli による上書きを避けるため lib.rs に配置）
#[macro_export]
macro_rules! impl_jst_timestamp_behavior {
    ($model:ident) => {
        #[async_trait::async_trait]
        impl sea_orm::ActiveModelBehavior for $model {
            async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, sea_orm::DbErr>
            where
                C: sea_orm::ConnectionTrait,
            {
                use sea_orm::ActiveValue::Set;
                use chrono::Local;
                
                // MySQLがJSTで動作しているため、現在のローカル時刻（JST）を
                // NaiveDateTimeとしてそのまま送信
                self.updated_at = Set(Local::now().naive_local());
                Ok(self)
            }
        }
    };
}

pub mod config;
pub mod enums;
pub mod mode;
pub mod utils;
pub mod migration;
pub mod entities;
pub mod vo;

