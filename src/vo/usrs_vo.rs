use sea_orm::FromQueryResult;

#[derive(FromQueryResult)]
pub struct AuthUsrVo {
    pub id: u32,
    pub email: String,
    pub password: String,
}