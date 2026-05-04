use serde::{Deserialize, Serialize};

/// A single recurring transaction — persisted as a ledger periodic-transaction
/// (`~ <period>`) block in the workspace's `recurring.ledger` file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecurringDefinition {
    pub period: String,
    /// Destination account, e.g. `Expenses:Subscriptions:Netflix`.
    pub account: String,
    /// The paying-side account, e.g. `Assets:Bank:Revolut`.
    pub counter_account: String,
    pub amount: f64,
    pub currency: String,
    /// Optional payee label captured as a comment next to the posting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payee: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateRecurringRequest {
    pub period: String,
    pub account: String,
    pub counter_account: String,
    pub amount: String,
    pub currency: Option<String>,
    pub payee: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateRecurringRequest {
    pub period: String,
    pub account: String,
    pub counter_account: String,
    pub amount: String,
    pub currency: Option<String>,
    pub payee: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecurringForecastQuery {
    pub end_date: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecurringDefinitionResponse {
    pub formatted_text: String,
    pub definition: RecurringDefinition,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecurringForecastResponse {
    /// Raw `ledger register --forecast` output; frontend parses same as other register queries.
    pub output: String,
}
