use crate::m20231030_000001_create_user_table::User;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Follower::Table)
                    .if_not_exists()
                    .primary_key(
                        Index::create()
                            .name("idx-follower")
                            .if_not_exists()
                            .table(Follower::Table)
                            .col(Follower::UserId)
                            .col(Follower::FollowerId),
                    )
                    .col(ColumnDef::new(Follower::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(Follower::FollowerId)
                            .uuid()
                            .not_null()
                            .check(
                                Expr::col(Follower::UserId)
                                    .eq(Expr::col(Follower::FollowerId))
                                    .not(),
                            ),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_follower-user")
                            .from(Follower::Table, Follower::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_follower-follower")
                            .from(Follower::Table, Follower::FollowerId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Follower::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Follower {
    Table,
    UserId,
    FollowerId,
}
