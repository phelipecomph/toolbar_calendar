pub mod google;
pub mod ics;

use crate::model::{Account, AccountKind, NormalizedEvent};
use chrono::{DateTime, Utc};

/// A calendar source. Every integration (ICS, Google, Graph) implements this.
#[async_trait::async_trait]
pub trait CalendarSource: Send + Sync {
    async fn list_events(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<NormalizedEvent>, String>;
}

/// Builds the provider from the account.
pub fn build(account: &Account) -> Result<Box<dyn CalendarSource>, String> {
    match account.kind {
        AccountKind::Ics => Ok(Box::new(ics::IcsSource::from_account(account)?)),
        AccountKind::Google => Ok(Box::new(google::GoogleSource::from_account(account)?)),
        AccountKind::Graph => Err("Microsoft Graph provider not implemented".into()),
    }
}
