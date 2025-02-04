use bilrost::Message;
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Message)]
pub struct Birth {
  name: String,
  born_at_epoch: i64,
}

impl Birth {
  pub fn new_now(name: String) -> Self {
    Self {
      born_at_epoch: OffsetDateTime::now_utc().unix_timestamp(),
      name,
    }
  }
}
