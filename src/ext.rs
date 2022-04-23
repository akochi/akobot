use std::fmt;
use twilight_model::user::{CurrentUser, User};

pub struct UserTuple<'r>(&'r str, &'r u16);

impl<'r> fmt::Display for UserTuple<'r> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}#{:04}", self.0, self.1)
    }
}

pub trait UserExt {
    fn as_tuple(&self) -> UserTuple;
}

impl UserExt for User {
    fn as_tuple(&self) -> UserTuple {
        UserTuple(&self.name, &self.discriminator)
    }
}

impl UserExt for CurrentUser {
    fn as_tuple(&self) -> UserTuple {
        UserTuple(&self.name, &self.discriminator)
    }
}
