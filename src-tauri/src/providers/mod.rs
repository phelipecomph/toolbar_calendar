pub mod google;
pub mod ics;

use crate::model::{Account, AccountKind, NormalizedEvent};
use chrono::{DateTime, Utc};

/// Fonte de calendário. Toda integração (ICS, Google, Graph) implementa isto.
#[async_trait::async_trait]
pub trait CalendarSource: Send + Sync {
    async fn list_events(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<NormalizedEvent>, String>;
}

/// Constrói o provider a partir da conta.
pub fn build(account: &Account) -> Result<Box<dyn CalendarSource>, String> {
    match account.kind {
        AccountKind::Ics => Ok(Box::new(ics::IcsSource::from_account(account)?)),
        AccountKind::Google => Ok(Box::new(google::GoogleSource::from_account(account)?)),
        AccountKind::Graph => Err("provider Microsoft Graph não implementado".into()),
    }
}
