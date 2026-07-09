use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table("users")
                    .if_not_exists()
                    .col(pk_uuid("id").default(Expr::custom_keyword("uuidv7()")))
                    .col(string("name").not_null())
                    .col(string("email").not_null())
                    .col(boolean("email_verified").not_null().default(false))
                    .col(timestamp_with_time_zone("created_at").default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;
        
        let mut foreign_key = ForeignKey::create()
            .name("user_id")
            .from("accounts", "user_id")
            .to("users", "id")
            .on_delete(ForeignKeyAction::Cascade)
            .to_owned();
        
        manager
            .create_table(
                Table::create()
                    .table("accounts")
                    .if_not_exists()
                    .col(pk_uuid("id").default(Expr::custom_keyword("uuidv7()")))
                    .col(uuid("user_id").not_null())
                    .col(string("provider").not_null())
                    .col(ColumnDef::new("password").string().null())
                    .col(ColumnDef::new("provider_id").string().null())
                    .col(timestamp_with_time_zone("created_at").default(Expr::current_timestamp()))
                    .foreign_key(&mut foreign_key)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table("users").to_owned())
            .await?;
        
        manager
            .drop_table(Table::drop().table("accounts").to_owned())
            .await?;
        
        Ok(())
    }
}
