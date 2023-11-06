use crate::entities::follower;
use crate::entities::prelude::*;
use sea_orm::{Linked, RelationDef, RelationTrait};

pub struct UserAsFollower;

impl Linked for UserAsFollower {
    type FromEntity = User;

    type ToEntity = User;

    fn link(&self) -> Vec<RelationDef> {
        vec![
            follower::Relation::User1.def().rev(),
            follower::Relation::User2.def(),
        ]
    }
}
