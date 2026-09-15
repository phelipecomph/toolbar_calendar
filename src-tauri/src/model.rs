use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountKind {
    Ics,
    Google,
    Graph,
}

impl AccountKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccountKind::Ics => "ics",
            AccountKind::Google => "google",
            AccountKind::Graph => "graph",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ics" => Some(AccountKind::Ics),
            "google" => Some(AccountKind::Google),
            "graph" => Some(AccountKind::Graph),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attendee {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// Conta de calendário. `config` guarda o específico do provider (ex.: URL do ICS).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub kind: AccountKind,
    pub display_name: String,
    pub color: String,
    pub config: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_token: Option<String>,
    pub enabled: bool,
}

/// Evento normalizado — contrato único entre providers, cache e frontend.
/// Espelhado em src/lib/types.ts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedEvent {
    pub id: String,
    pub account_id: String,
    pub source_id: String,
    pub title: String,
    pub start: String, // ISO8601 UTC
    pub end: String,   // ISO8601 UTC
    pub all_day: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_link: Option<String>,
    pub attendees: Vec<Attendee>,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub window_before_minutes: i64,
    pub window_after_minutes: i64,
    pub sync_interval_minutes: u64,
    pub strip_height_logical: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            window_before_minutes: 60,
            window_after_minutes: 480,
            sync_interval_minutes: 3,
            strip_height_logical: 28,
        }
    }
}

/// Fase 1/dev: eventos mockados relativos a agora. Usados só quando não há contas.
pub fn mock_events() -> Vec<NormalizedEvent> {
    let now = Utc::now();
    let at = |min: i64| (now + Duration::minutes(min)).to_rfc3339();

    let mk = |id: &str, acc: &str, color: &str, title: &str, s: i64, e: i64| NormalizedEvent {
        id: id.into(),
        account_id: acc.into(),
        source_id: acc.into(),
        color: color.into(),
        title: title.into(),
        start: at(s),
        end: at(e),
        all_day: false,
        description: None,
        location: None,
        meeting_link: None,
        html_link: None,
        attendees: vec![],
    };

    vec![
        mk("m1", "acc-personal", "#4285f4", "Standup", -20, 10),
        mk("m2", "acc-work", "#0b8043", "Design review", 30, 120),
        mk("m3", "acc-work", "#0b8043", "1:1 com Ana", 60, 90),
        mk("m4", "acc-personal", "#4285f4", "Dentista", 75, 105),
        mk("m5", "acc-side", "#f4511e", "Deploy window", 150, 210),
        mk("m6", "acc-work", "#0b8043", "Almoço", 240, 300),
        mk("m7", "acc-personal", "#4285f4", "Foco: relatório", 320, 440),
    ]
}
