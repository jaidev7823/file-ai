use sea_orm::*;
use chrono::Utc;

use crate::entities::user::{self, Entity as User, Model as UserModel};

pub struct UserService {
    pub db: DatabaseConnection,
}

impl UserService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn create_user(&self, name: String, email: String) -> Result<UserModel, DbErr> {
        let now = Utc::now();
        
        let new_user = user::ActiveModel {
            name: Set(name),
            email: Set(email),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        let user = new_user.insert(&self.db).await?;
        Ok(user)
    }

    pub async fn get_all_users(&self) -> Result<Vec<UserModel>, DbErr> {
        let users = User::find().all(&self.db).await?;
        Ok(users)
    }

    pub async fn get_user_by_id(&self, id: i32) -> Result<Option<UserModel>, DbErr> {
        let user = User::find_by_id(id).one(&self.db).await?;
        Ok(user)
    }

    pub async fn update_user(&self, id: i32, name: Option<String>, email: Option<String>) -> Result<UserModel, DbErr> {
        let user = User::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(DbErr::RecordNotFound("User not found".to_string()))?;

        let mut user: user::ActiveModel = user.into();
        
        if let Some(name) = name {
            user.name = Set(name);
        }
        if let Some(email) = email {
            user.email = Set(email);
        }
        user.updated_at = Set(Utc::now());

        let updated_user = user.update(&self.db).await?;
        Ok(updated_user)
    }

    pub async fn delete_user(&self, id: i32) -> Result<(), DbErr> {
        User::delete_by_id(id).exec(&self.db).await?;
        Ok(())
    }
}